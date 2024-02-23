use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use validator::Validate;

use chrono::{Duration, Utc};
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

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub struct Token {
    // #[serde(skip_deserializing)]
    pub expiry: String,
    // #[serde(skip_deserializing)]
    pub secret: String,
}
pub fn generate_sha512_string(string: String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(string.as_bytes());
    let result = hasher.clone().finalize();
    format!("{:x}", result)
}

impl User {
    pub fn new(username: String, password: String, email: String,token:&str) -> User {
        User {
            token: None,
            username: username,
            hashed_pass: generate_sha512_string(password),
            email: email,
        }
    }
}
