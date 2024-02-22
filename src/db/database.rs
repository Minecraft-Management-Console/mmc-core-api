use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::{Client,Ws};
use surrealdb::Surreal;
use surrealdb::opt::auth::Root;
use surrealdb::Error;
use tracing::info;
use tracing_subscriber::registry::Data;

use crate::models::users::User;

#[derive(Clone)]
pub struct Database{
    pub client: Surreal<Client>,
    pub name_space: String,   
    pub db_name: String,
}


impl Database {
    pub async fn init() -> Result<Self,Error>{
        let client = Surreal::new::<Ws>("127.0.0.1:8080").await?;
        client.signin(Root{
            username:"root",
            password:"root"
        }).await?;

        client.use_ns("surreal").use_db("users").await.unwrap();
        Ok(Database { client, name_space: String::from("surreal"), db_name: String::from("users") })
    }


    pub async fn get_all_users(&self) -> Option<Vec<User>>{
        let result = self.client.select("users").await;
        match result{
            Ok(a) => Some(a),
            Err(_) => None,
        }
    }

    pub async fn add_user(&self,new_user:User) -> Result<User,String>{
        let created_user = self.client.create(("users",new_user.username.clone())).content(new_user).await;
        match created_user{
            Ok(created) => {
                Ok(created.unwrap())
            },
            Err(e) => {info!("{e} {}","Error"); Err(format!("{e}"))},
        }
    }
}
