use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::Error;
use tracing::{debug, info, warn};
use validator::Validate;


use crate::db::{database::SessionTokenErrors, Database};

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub struct Token {
    // #[serde(skip_deserializing)]
    pub expiry: String,
    // #[serde(skip_deserializing)]
    pub secret: String,
}

pub trait TokenData {
    async fn create_token(db: &Database, token: &str, username: &str);
    async fn is_sessionid_expired(&self, token: &str) -> Result<bool, SessionTokenErrors>;
    async fn refresh_token(&self, token: &str);
    async fn validate_token(&self, token: &str) -> Result<(), SessionTokenErrors>;
}

impl TokenData for Database {
    async fn create_token(db: &Database, token: &str, username: &str) {
        let created_token: Result<Option<Token>, Error> = db
            .client
            .create(("token", token))
            .content(Token {
                expiry: (Utc::now() + chrono::Duration::days(2)).to_string(),
                secret: String::from(token),
            })
            .await;

        let query = format!("UPDATE token:{} SET owner=users:{}", token, username);

        let fuck_this_crap = format!("UPDATE users:{} SET token=token:{}", username, token);

        debug!("{} {}", "Sending ownership query: ", query);
        db.client.query(query).query(fuck_this_crap).await.unwrap();
        match created_token {
            Ok(_) => {
                info!("Created token:{token} successfully for {username}");
            }
            Err(e) => {
                warn!("{} {e}", "Error:");

            }
        };
    }

    // returns true if the session id expired
    // THIS FUNCTION CANNOT RETURN SessionTokenErrors::ExpiredSessionToken
    // PLEASE DO NOT FUCKING MAKE THIS RETURN SessionTokenErrors::ExpiredSessionToken
    async fn is_sessionid_expired(&self, token: &str) -> Result<bool, SessionTokenErrors> {
        let query = format!("SELECT VALUE expiry FROM token:{}", token);
        debug!({ query }, "Sending query to database");

        let mut expiry = self.client.query(query).await.unwrap();
        let expiry: Option<String> = expiry.take(0).expect("Unable to query DB");
        match expiry {
            Some(time) => {
                // let time = expiration.expiry;
                info!({ time }, "User's token will expire at: ");
                let time_in_db = DateTime::<Utc>::from_str(&time).expect("Unable to parse date");
                let time_now = Utc::now();

                debug!("Expiry for token: {:?} = {}", token, time_in_db);
                debug!("Current UTC time: {}", time_now);

                if time_now >= time_in_db {
                    info!("Session token has expired.");
                    return Ok(true);
                }
                return Ok(false);
            }
            None => {
                warn!("Authentication token not found in the database!");
                return Err(SessionTokenErrors::InvalidSessionToken);
            }
        }
    }

    async fn refresh_token(&self, token: &str) {
        info!(token,"Refreshing the session");
        let new_expiry = (Utc::now() + chrono::Duration::days(1)).to_string();
        debug!("New expiry for {token}: {new_expiry}");
        let sql = format!("UPDATE token:{} SET expiry = \"{}\"", token, new_expiry);

        debug!("Sending query: {}", sql);
        let _ = self.client.query(sql).await.unwrap();
    }
    async fn validate_token(&self, token: &str) -> Result<(), SessionTokenErrors> {
        match self.is_sessionid_expired(token).await {
            Ok(expired) => {
                if expired {
                    return Err(SessionTokenErrors::ExpiredSessionToken);
                }
                self.refresh_token(token).await;
                return Ok(());
            }
            Err(e) => return Err(e),
        };
    }
}
