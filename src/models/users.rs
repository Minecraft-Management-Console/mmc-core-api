use serde::{Deserialize, Serialize};
use validator::Validate;


#[derive(Validate,Serialize,Deserialize,Debug)]
pub struct AuthUserRequest{
    #[validate(length(min=3,message="Username required to be more than 3 characters"))]
    pub username: String,
    #[validate(length(min=8,message="Password required to be more than 8 characters"))]
    pub password: String,
    // pub token: String
    #[validate(email)]
    pub email: String,
}

#[derive(Serialize,Deserialize,Validate,Debug)]
pub struct User{
    #[serde(skip_deserializing)]
    pub token: String,
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[serde(skip_deserializing)]
    pub hashed_pass: String,
}

impl User{
    pub fn new(uuid:String,username:String,password:String,email:String) -> User{
        let hashed_pass= format!("{}:{}",username,password);
        User{token:uuid,username:username,hashed_pass,email:email}
    }
}