use std::collections::{BTreeMap, HashMap};
use std::fmt::{write, Display};
use std::str::FromStr;

use actix_web::cookie::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::{Client, Ws};
// use surrealdb::sql::query;
use surrealdb::opt::auth::Root;
use surrealdb::Error;
use surrealdb::Surreal;
use tracing::{debug, info, warn};
use tracing_subscriber::fmt::format;
use tracing_subscriber::registry::Data;

use crate::models::users::{Token, User};

#[derive(Clone)]
pub struct Database {
    pub client: Surreal<Client>,
    pub name_space: String,
    pub db_name: String,
}


pub enum SessionTokenErrors{
    ExpiredSessionToken,
    InvalidSessionToken,
    NoSessionToken,
}

impl Display for SessionTokenErrors{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self{
            SessionTokenErrors::ExpiredSessionToken => write!(f,"Session token has expired!"),
            SessionTokenErrors::InvalidSessionToken => write!(f, "Session token is invalid"),
            SessionTokenErrors::NoSessionToken => write!(f,"No session token has been provided"),
        }
    }
}


impl Database {
    pub async fn init(address: &str) -> Result<Self, Error> {
        info!({ address }, "Initialising SurrealDB on address:");

        let client = Surreal::new::<Ws>(address).await?;
        client
            .signin(Root {
                username: "root",
                password: "root",
            })
            .await?;

        client
            .use_ns("minecraft_manager")
            .use_db("storage")
            .await
            .unwrap();
        Ok(Database {
            client,
            name_space: String::from("minecraft_manager"),
            db_name: String::from("storage"),
        })
    }

    pub async fn get_all_users(&self) -> Option<Vec<User>> {
        let result = self.client.select("users").await;
        match result {
            Ok(a) => Some(a),
            Err(_) => None,
        }
    }

    pub async fn add_user(&self, new_user: User, token: &str) -> Result<User, String> {
        let created_token: Result<Option<Token>, Error> = self
            .client
            .create(("token", token))
            .content(new_user.token.clone())
            .await;

        let query = format!(
            "UPDATE token:{} SET owner=user:{}",
            token,
            new_user.username.clone()
        );
        debug!("{} {}", "Sending ownership query: ", query);
        self.client.query(query).await.unwrap();
        let created_user = self
            .client
            .create(("users", new_user.username.clone()))
            .content(new_user)
            .await;
        match created_token {
            Ok(_) => {
                info!("Created Token successfully");
            }
            Err(e) => {
                info!("{e} {}", "Error");
                return Err(format!("{e}"));
            }
        };

        match created_user {
            Ok(created) => Ok(created.unwrap()),
            Err(e) => {
                info!("{e} {}", "Error");
                Err(format!("{e}"))
            }
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<(),SessionTokenErrors> {
        // let query = format!("SELECT * from users WHERE token=\"{token}\"");
        // debug!("{} {}","Sending Query: ",query);
        // let mut user_in_db = self.client.query(query).await.unwrap();
        // let created: Option<User> = user_in_db.take(0).expect("Unable to query DB");
        // debug!("{:?} {}",created,"found user with token");
        // match created{
        //     Some(_) => true,
        //     None => false
        // }

        let query = format!("SELECT * FROM users where token.token==\"{}\"", token);
        info!({ query }, "Sending query to database");

        let mut expiry = self.client.query(query).await.unwrap();
        let expiry: Option<User> = expiry.take(0).expect("Unable to query DB");

        match expiry {
            Some(user) => {
                let time = user.token.expiry;
                info!({ time }, "User's token will expire at: ");
                let time_in_db = DateTime::<Utc>::from_str(&time).expect("Unable to parse date");
                let time_now = Utc::now();

                debug!("Expiry for token: {:?} = {}", token, time_in_db);
                debug!("Current UTC time: {}", time_now);

                if time_now >= time_in_db {
                    info!("Session token has expired.");
                    return Err(SessionTokenErrors::ExpiredSessionToken);
                }
                info!("Refreshing the session token.");
                let new_expiry = (Utc::now() + chrono::Duration::days(1)).to_string();
                debug!("{new_expiry}");
                let sql = format!(
                    "UPDATE users:{} SET token.expiry = \"{}\"",
                    user.username,
                    new_expiry
                );

                debug!("Sending query: {}",sql);
                let _ = self.client.query(sql).await.unwrap();
                return Ok(());
            }
            None => {
                warn!("Authentication token not found in the database!");
                return Err(SessionTokenErrors::InvalidSessionToken);
            }
        }
    }
}
