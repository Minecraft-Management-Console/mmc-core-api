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
    pub token: Option<Token>,
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[serde(skip_deserializing)]
    hashed_pass: String,
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub struct Token {
    // #[serde(skip_deserializing)]
    pub expiry: String,
    // #[serde(skip_deserializing)]
    pub secret: String,
}

impl User {
    pub fn new(username: String, password: String, email: String,token:&str) -> User {
        let hashed_pass = format!("{}:{}", username, password);
        User {
            token: None,
            username: username,
            hashed_pass,
            email: email,
        }
    }
}
