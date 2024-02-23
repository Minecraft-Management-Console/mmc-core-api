use serde::{Deserialize, Serialize};
use validator::Validate;

use chrono::{Duration, Utc};
#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct AuthUserRequest {
    #[validate(length(min = 3, message = "Username required to be more than 3 characters"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password required to be more than 8 characters"))]
    pub password: String,
    // pub token: String
    #[validate(email)]
    pub email: String,
}

#[derive(Serialize, Deserialize, Validate, Debug)]
pub struct User {
    // #[serde(skip_deserializing)]
    pub token: Token,
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[serde(skip_deserializing)]
    hashed_pass: String,
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub struct Token {
    pub expiry: String,
    token: String,
}

impl User {
    pub fn new(username: String, password: String, email: String,token:&str) -> User {
        let hashed_pass = format!("{}:{}", username, password);
        User {
            token: Token {
                token: token.to_string(),
                expiry: (Utc::now() + Duration::days(1)).to_string(),
            },
            username: username,
            hashed_pass,
            email: email,
        }
    }
}
