mod db;
mod models;

use actix_cors::Cors;
use actix_web::{
    cookie::Cookie,
    get,
    http::header::ContentType,
    post,
    web::{Data, Json},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use std::env;
use tracing::{info, trace, warn, Level};
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};
use validator::Validate;

use crate::{
    db::Database,
    models::{
        token::TokenData, users::{AuthUserLoginRequest, User, UserData}, AuthUserSignupRequest
    },
};

// THIS WILL BE USED TO THE CLIENT
#[post("/create_user")]
async fn create_user(user: Json<AuthUserSignupRequest>, db: Data<Database>) -> impl Responder {
    // info!(user,"/create_user http request received with body");
    let is_valid = user.validate();
    match is_valid {
        Ok(_) => {
            let user_name = user.username.clone();
            let email = user.email.clone();
            let mut buffer = uuid::Uuid::encode_buffer();
            let new_uuid = uuid::Uuid::new_v4().simple().encode_lower(&mut buffer);

            let new_user = Database::add_user(
                &db,
                User::new(user_name, user.password.clone(), email),
                &new_uuid,
            )
            .await;

            match new_user {
                Ok(_) => {
                    // let json =
                    //     serde_json::to_string_pretty(&created).expect("unable to serialize user");
                    // let cookie = Cookie::build("Auth-Token", new_uuid.to_string().clone())
                    //     .domain(domain)
                    //     .path("/")
                    //     .same_site(actix_web::cookie::SameSite::None)
                    //     .secure(true)
                    //     .http_only(true)
                    //     .finish();
                    // // dbg!(&cookie);

                    HttpResponse::Ok()
                        // .cookie(cookie)
                        .content_type(ContentType::json())
                        .body("Created User!")
                }
                Err(e) => HttpResponse::Forbidden()
                    .content_type(ContentType::plaintext())
                    .body(e),
            }
        }
        Err(e) => {
            return HttpResponse::Forbidden()
                .content_type(ContentType::json())
                .body(format!("{}", e))
        }
    }
}

#[post("/login")]
async fn login(user: Json<AuthUserLoginRequest>, db: Data<Database>) -> impl Responder {
    // info!(user,"/create_user http request received with body");
    let domain = std::env::var("CORS_DOMAIN").unwrap();
    let is_valid = user.validate();
    match is_valid {
        Ok(_) => {
            let user_name = user.username.clone();
            let new_user = Database::login(&db,&user_name, &user.password).await;

            match new_user {
                Ok(created) => {
                    let json =
                        serde_json::to_string_pretty(&created).expect("unable to serialize user");

                    let cookie = Cookie::build(
                        "Auth-Token",
                        created
                            .token
                            .expect("TOKEN IS A NONE VALUE")
                            .secret
                            .to_string()
                            .clone(),
                    )
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
                Err(e) => HttpResponse::Forbidden().body(format!("{}", e)),
            }
        }
        Err(e) => {
            return HttpResponse::Forbidden()
                .content_type(ContentType::json())
                .body(format!("{}", e))
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
        None => None,
    }
}

#[get("/validate")]
async fn validate(auth: HttpRequest, db: Data<Database>) -> impl Responder {
    match get_auth_cookie(&auth) {
        Some(key) => {
            match Database::validate_token(&db,&key).await {
                Ok(()) => {
                    return HttpResponse::Ok().body("Ok!");
                }
                Err(e) => {
                    return HttpResponse::BadRequest().body(format!("{}", e));
                }
            };
        }
        None => {
            warn!("No authorisation cookie found for requst");
            HttpResponse::BadRequest().body("Malformed request!")
        }
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
                .with_writer(std::io::stdout.with_max_level(Level::DEBUG))
                .with_file(true)
                .with_line_number(true),
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
            .service(validate)
            .service(login)
    })
    .bind(bind_address)?
    .run()
    .await
}
