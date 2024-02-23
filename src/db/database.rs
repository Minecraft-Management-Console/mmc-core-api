use std::collections::{BTreeMap, HashMap};
use std::fmt::{write, Display};
use std::str::FromStr;

use actix_web::cookie::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::{Client, Ws};
// use surrealdb::sql::query;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use surrealdb::{sql, Error};
use tracing::{debug, info, warn};
use tracing_subscriber::fmt::format;
use tracing_subscriber::registry::Data;

use crate::models::users::{generate_sha512_string, Token, User};

#[derive(Clone)]
pub struct Database {
    pub client: Surreal<Client>,
    pub name_space: String,
    pub db_name: String,
}

#[derive(Debug)]
pub enum SessionTokenErrors {
    ExpiredSessionToken,
    InvalidSessionToken,
    NoSessionToken,
}

pub enum LoginErrors {
    NoSuchEntry,
}

impl Display for SessionTokenErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SessionTokenErrors::ExpiredSessionToken => write!(f, "Session token has expired!"),
            SessionTokenErrors::InvalidSessionToken => write!(f, "Session token is invalid"),
            SessionTokenErrors::NoSessionToken => write!(f, "No session token has been provided"),
        }
    }
}

impl Display for LoginErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            LoginErrors::NoSuchEntry => {
                write!(f, "No such username/password found in the database!")
            }
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
        let query = "SELECT * FROM users FETCH token;";
        let mut response = self.client.query(query).await.unwrap();

        let result: Result<Vec<User>, Error> = response.take(0);
        // let result = self.client.select("users").await;
        match result {
            Ok(a) => Some(a),
            Err(e) => {
                dbg!(e);
                None
            }
        }
    }
    pub async fn create_token(&self, token: &str, username: &str) {
        let created_token: Result<Option<Token>, Error> = self
            .client
            .create(("token", token))
            .content(Token {
                expiry: (Utc::now() + chrono::Duration::days(2)).to_string(),
                secret: String::from(token),
            })
            .await;

        let query = format!("UPDATE token:{} SET owner=user:{}", token, username);

        let fuck_this_crap = format!("UPDATE users:{} SET token=token:{}", username, token);

        debug!("{} {}", "Sending ownership query: ", query);
        self.client
            .query(query)
            .query(fuck_this_crap)
            .await
            .unwrap();
        match created_token {
            Ok(_) => {
                info!("Created Token successfully");
            }
            Err(e) => {
                info!("{e} {}", "Error");
            }
        };
    }

    pub async fn add_user(&self, new_user: User, token: &str) -> Result<User, String> {
        let username = new_user.username.clone();
        let created_user = self
            .client
            .create(("users", new_user.username.clone()))
            .content(new_user)
            .await;

        self.create_token(token, &username).await;
        match created_user {
            Ok(created) => Ok(created.unwrap()),
            Err(e) => {
                info!("{e} {}", "Error");
                Err(format!("{e}"))
            }
        }
    }

    // returns true if the session id expired
    // THIS FUNCTION CANNOT RETURN SessionTokenErrors::ExpiredSessionToken
    // PLEASE DO NOT FUCKING MAKE THIS RETURN SessionTokenErrors::ExpiredSessionToken
    async fn is_sessionid_expired(&self, token: &str) -> Result<bool, SessionTokenErrors> {
        #[derive(Deserialize)]
        struct Expiry {
            expiry: String,
        }
        let query = format!("SELECT expiry FROM token:{}", token);
        info!({ query }, "Sending query to database");

        let mut expiry = self.client.query(query).await.unwrap();
        let expiry: Option<Expiry> = expiry.take(0).expect("Unable to query DB");
        match expiry {
            Some(expiration) => {
                let time = expiration.expiry;
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
        info!("Refreshing the session token.");
        let new_expiry = (Utc::now() + chrono::Duration::days(1)).to_string();
        debug!("{new_expiry}");
        let sql = format!("UPDATE token:{} SET expiry = \"{}\"", token, new_expiry);

        debug!("Sending query: {}", sql);
        let _ = self.client.query(sql).await.unwrap();
    }
    pub async fn validate_token(&self, token: &str) -> Result<(), SessionTokenErrors> {
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

    pub async fn login(
        &self,
        username: &str,
        db_store_pass: &str,
    ) -> Result<User, LoginErrors> {
        #[derive(Deserialize,Debug)]
        struct HashedPass {
            hashed_pass: String,
        }
        let sql = format!("SELECT hashed_pass FROM users:{}", username);
        debug!("Sending query: {}", sql);
        let mut query = self.client.query(sql).await.unwrap();
        let pass_in_db: Option<HashedPass> = query.take(0).unwrap();

        match dbg!(pass_in_db) {
            Some(database_hash) => {
                let db_in_pass = database_hash.hashed_pass;
                if db_in_pass == generate_sha512_string(db_store_pass.to_string()) {
                    // SESSION VALIDITY FOUND... get user from db, serialise and return to client

                    // THIS has to be infallible since this is the only way we will refresh tokens!
                    // 
                    #[derive(Deserialize)]
                    struct DbResponseToken{token:Token};

                    let sql = format!("SELECT token FROM users:{} fetch token",username);
                    let mut response = self.client.query(&sql).await.unwrap();
                    debug!("Sending query: {}", sql);

                    let db_response:Option<DbResponseToken> = response.take(0).unwrap();

                    let token = db_response.unwrap().token.secret;
                    
                    let is_expired = self.is_sessionid_expired(&token).await.unwrap();
                    if is_expired{
                        info!("Current session for {} has expired. Generating a new sessionID",username);
                        // We need to delete the older token and update it with a new one!.
                        let delete_token_sql = format!("DELETE token where secret==\"{}\"",&token);
                        debug!("Sending query: {}", delete_token_sql);
                        let _ = self.client.query(delete_token_sql).await; // NOW CREATE A NEW TOKEN FOR THE USER HURRR

                        let mut buffer = uuid::Uuid::encode_buffer();
                        let new_uuid = uuid::Uuid::now_v7().simple().encode_lower(&mut buffer);

                        self.create_token(&new_uuid, &username).await;
                    }

                    let sql = format!("SELECT * FROM users:{} FETCH token", username); // WE WILL HAVE TO GENERATE A NEW TOKEN IN THIS CASE
                    let mut response = self.client.query(sql).await.unwrap();
                    let result: Option<User> = dbg!(response.take(0)).unwrap();
                    return Ok(result.unwrap());
                }
                
            }
            None => (),
        }

        // info!("Hashed pass in db: {}",pass_in_db.unwrap().hashed_pass);
        return Err(LoginErrors::NoSuchEntry);
    }
}
