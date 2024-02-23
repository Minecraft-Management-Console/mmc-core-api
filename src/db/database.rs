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
    pub async fn init(address:&str) -> Result<Self,Error>{
        let client = Surreal::new::<Ws>(address).await?;
        client.signin(Root{
            username:"root",
            password:"root"
        }).await?;

        client.use_ns("minecraft_manager").use_db("storage").await.unwrap();
        Ok(Database { client, name_space: String::from("minecraft_manager"), db_name: String::from("storage") })
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

    pub async fn validate_token(&self,token:&str) -> bool{
        let query = format!("SELECT * from users WHERE token=\"{token}\"");
        info!("{} {}","Sending Query: ",query);
        let mut user_in_db = self.client.query(query).await.unwrap();
        let created: Option<User> = user_in_db.take(0).expect("Unable to query DB");
        info!("{:?} {}",created,"found user with token");
        match created{
            Some(_) => true,
            None => false
        }
        
    }
}
