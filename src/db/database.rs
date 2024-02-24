use std::fmt::Display;

use surrealdb::engine::remote::ws::{Client, Ws};
// use surrealdb::sql::query;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use surrealdb::Error;
use tracing::info;


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
    _NoSessionToken,
}

pub enum LoginErrors {
    NoSuchEntry,
}

impl Display for SessionTokenErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SessionTokenErrors::ExpiredSessionToken => write!(f, "Session token has expired!"),
            SessionTokenErrors::InvalidSessionToken => write!(f, "Session token is invalid"),
            SessionTokenErrors::_NoSessionToken => write!(f, "No session token has been provided"),
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


    

    
}
