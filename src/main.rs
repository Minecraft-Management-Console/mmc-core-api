mod db;
mod models;

use actix_cors::Cors;
use actix_web::{
    cookie::{time::Duration, Cookie},
    get,
    http::{self, header::ContentType, StatusCode},
    post,
    web::{Data, Form, Json},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use std::{env, io};
use tracing::{debug, info, trace, Level};
use tracing_subscriber::{
    fmt::{layer, writer::MakeWriterExt, MakeWriter},
    layer::{self, SubscriberExt},
    util::SubscriberInitExt,
};
use validator::Validate;

use crate::{
    db::Database,
    models::{users::User, AuthUserRequest},
};

// THIS WILL BE USED TO THE CLIENT
#[post("/create_user")]
async fn create_user(user: Json<AuthUserRequest>, db: Data<Database>) -> impl Responder {
    // info!(user,"/create_user http request received with body");
    let domain = std::env::var("CORS_DOMAIN").unwrap();
    let is_valid = user.validate();
    match is_valid {
        Ok(_) => {
            let user_name = user.username.clone();
            let email = user.email.clone();
            let mut buffer = uuid::Uuid::encode_buffer();
            let new_uuid = uuid::Uuid::new_v4().simple().encode_lower(&mut buffer);

            let new_user = db
                .add_user(User::new(
                    new_uuid.to_string().clone(),
                    user_name,
                    user.password.clone(),
                    email,
                ))
                .await;

            match new_user {
                Ok(created) => {
                    let json =
                        serde_json::to_string_pretty(&created).expect("unable to serialize user");
                    let cookie = Cookie::build("Auth-Token", new_uuid.to_string().clone())
                        .domain(domain)
                        .path("/")
                        .same_site(actix_web::cookie::SameSite::None)
                        .secure(true)
                        .http_only(true)
                        .finish();
                    // dbg!(&cookie);

                    HttpResponse::Ok()
                        .cookie(cookie)
                        .content_type(ContentType::json())
                        .body(json)
                }
                Err(e) => HttpResponse::Forbidden().body(e),
            }
        }
        Err(e) => {
            return HttpResponse::Forbidden()
                .content_type(ContentType::json())
                .body(format!("{{errors=[{}]}}", e))
        }
    }
}

fn get_auth_cookie<'a>(req: &'a HttpRequest) -> Option<String> {
    let cookie = req.cookie("Auth-Token");
    match cookie {
        Some(c) => {
            info!("{:#?}", c.to_string());
            Some(c.value().to_string())
        }
        None => {
            None
        }
    }
}

#[get("/get_all_users")]
async fn get_users(auth: HttpRequest, db: Data<Database>) -> impl Responder {
    match get_auth_cookie(&auth) {
        Some(key) => {
            info!(key, "Received header key from the request.");

            let cookie = get_auth_cookie(&auth);
            match cookie{
                Some(token) => {
                    if db.validate_token(&token).await{
                        let users = db.get_all_users().await.unwrap();
                        let json = serde_json::to_string(&users).expect("Unable to serialise the data");
        
                        return HttpResponse::Ok()
                            .content_type(ContentType::json())
                            .body(json);
                    }
                    return HttpResponse::Forbidden().body("Token Authentication Failed").into();
                },
                None => return HttpResponse::Forbidden().body("Token not found. Your session may have expired!").into()
            };

           
        }
        None => HttpResponse::BadRequest().body("Malformed request!"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let debug_file =
        tracing_appender::rolling::hourly("./logs/", "debug").with_max_level(tracing::Level::INFO);

    let warn_file = tracing_appender::rolling::hourly("./logs/", "warnings")
        .with_max_level(tracing::Level::WARN);
    let all_files = debug_file.and(warn_file);

    // let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        // .with(console_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(all_files)
                .with_ansi(false),
        )
        .with(
            tracing_subscriber::fmt::Layer::new()
                .with_writer(std::io::stdout.with_max_level(Level::DEBUG)),
        )
        .init();

    let _ = dotenvy::dotenv().expect("No env file found");
    trace!("{:?} {}", env::vars(), "Loaded enviroment variables");

    let bind_address = env::var("BIND_ADDRESS").expect("No BIND_ADDRESS set in the enviroment");
    info!(bind_address, "Beginning HTTP server on address:");

    let surreal_db_address =
        env::var("SURREALDB_ADDRESS").expect("SURREALDB_ADDRESS enviroment variable not sent");
    info!(surreal_db_address);
    let db = Database::init(&surreal_db_address)
        .await
        .expect("Unable to connect to surrealDB database");

    let db_data = Data::new(db);

    HttpServer::new(move || {
        let cors = Cors::permissive().supports_credentials();
        App::new()
            .wrap(cors)
            .app_data(db_data.clone())
            .service(create_user)
            .service(get_users)
    })
    .bind(bind_address)?
    .run()
    .await
}
