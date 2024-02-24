use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use surrealdb::Error;
use tracing::{debug, info, warn};
use validator::Validate;


use crate::{db::{database::LoginErrors, Database}, models::token::Token};

use super::token::TokenData;
#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct AuthUserSignupRequest {
    #[validate(length(min = 3, message = "Username required to be more than 3 characters"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password required to be more than 8 characters"))]
    pub password: String,
    // pub token: String
    #[validate(email)]
    pub email: String,
}

#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct AuthUserLoginRequest {
    #[validate(length(min = 3, message = "Username required to be more than 3 characters"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password required to be more than 8 characters"))]
    pub password: String,
}

#[derive(Serialize, Deserialize, Validate, Debug)]
pub struct User {
    pub token: Option<Token>,
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[serde(skip_deserializing)]
    hashed_pass: String,
}

pub fn generate_sha512_string(string: String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(string.as_bytes());
    let result = hasher.clone().finalize();
    format!("{:x}", result)
}

impl User {
    pub fn new(username: String, password: String, email: String) -> User {
        User {
            token: None,
            username: username,
            hashed_pass: generate_sha512_string(password),
            email: email,
        }
    }
}

pub trait UserData {
    async fn add_user(db: &Database, new_user: User, token: &str) -> Result<User, String>;
    async fn _get_all_users(db: &Database) -> Option<Vec<User>>;
    async fn login(db: &Database, username: &str, db_store_pass: &str)
        -> Result<User, LoginErrors>;
}
impl UserData for Database {
    async fn add_user(db: &Database, new_user: User, token: &str) -> Result<User, String> {
        let username = new_user.username.clone();
        let created_user = db
            .client
            .create(("users", new_user.username.clone()))
            .content(new_user)
            .await;

        Database::create_token(&db,token, &username).await;
        match created_user {
            Ok(created) => Ok(created.unwrap()),
            Err(e) => {
                warn!("{e} {}", "Error");
                Err(format!("{e}"))
            }
        }
    }

    async fn _get_all_users(db: &Database) -> Option<Vec<User>> {
        let query = "SELECT * FROM users FETCH token;";
        let mut response = db.client.query(query).await.unwrap();

        let result: Result<Vec<User>, Error> = response.take(0);
        // let result = self.client.select("users").await;
        match result {
            Ok(a) => Some(a),
            Err(e) => {
                warn!("{:#?}", e);
                None
            }
        }
    }

    async fn login(
        db: &Database,
        username: &str,
        db_store_pass: &str,
    ) -> Result<User, LoginErrors> {
        let sql = format!("SELECT VALUE hashed_pass FROM users:{}", username);
        debug!("Sending query: {}", sql);
        let mut query = db.client.query(sql).await.unwrap();
        let pass_in_db: Option<String> = query.take(0).unwrap();

        match pass_in_db {
            Some(db_in_pass) => {
                // let db_in_pass = database_hash.hashed_pass;
                if db_in_pass == generate_sha512_string(db_store_pass.to_string()) {
                    // SESSION VALIDITY FOUND... get user from db, serialise and return to client

                    // THIS has to be infallible since this is the only way we will refresh tokens!
                    let sql = format!("SELECT VALUE token.secret FROM users:{}", username);
                    let mut response = db.client.query(&sql).await.unwrap();
                    debug!("Sending query: {}", sql);

                    let db_response: Option<String> = response.take(0).unwrap();

                    let token = db_response.unwrap();

                    let is_expired = db.is_sessionid_expired(&token).await.unwrap();
                    if is_expired {
                        info!(
                            "Current session for {} has expired. Generating a new sessionID",
                            username
                        );
                        // We need to delete the older token and update it with a new one!.
                        let delete_token_sql = format!("DELETE token where secret==\"{}\"", &token);
                        debug!("Sending query: {}", delete_token_sql);
                        let _ = db.client.query(delete_token_sql).await; // NOW CREATE A NEW TOKEN FOR THE USER HURRR

                        let mut buffer = uuid::Uuid::encode_buffer();
                        let new_uuid = uuid::Uuid::new_v4().simple().encode_lower(&mut buffer);

                        Database::create_token(&db,&new_uuid, &username).await;
                    } else {
                        db.refresh_token(&token).await;
                    }

                    let sql = format!("SELECT * FROM users:{} FETCH token", username); // WE WILL HAVE TO GENERATE A NEW TOKEN IN THIS CASE
                    let mut response = db.client.query(sql).await.unwrap();
                    let result: Option<User> = response.take(0).unwrap();
                    return Ok(result.unwrap());
                }
            }
            None => (),
        }

        // info!("Hashed pass in db: {}",pass_in_db.unwrap().hashed_pass);
        return Err(LoginErrors::NoSuchEntry);
    }
}
