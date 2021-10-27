#[macro_use]
extern crate log;

use actix_web::cookie::Key;
use actix_web::{middleware, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework, Level};
use anyhow::Result;
use dotenv::dotenv;
use munje::{questions, queues, routes, types::AppState};
use sqlx::sqlite::SqlitePoolOptions;

fn message_framework(session_key: &String) -> FlashMessagesFramework {
    let bytes = base64::decode(session_key).unwrap();
    let signing_key = Key::derive_from(&bytes);
    let store = CookieMessageStore::builder(signing_key).build();
    // Show debug-level messages when developing locally
    let minimum_level = match std::env::var("APP_ENV") {
        Ok(s) if &s == "local" => Level::Debug,
        _ => Level::Info,
    };
    FlashMessagesFramework::builder(store)
        .minimum_level(minimum_level)
        .build()
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let session_key = std::env::var("SESSION_KEY").expect("SESSION_KEY not set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    info!("using sqlite database at: {}", &database_url);
    let db = SqlitePoolOptions::new().connect(&database_url).await?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { db: db.clone() }))
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(message_framework(&session_key))
            .configure(routes::register)
            .configure(questions::routes::register)
            .configure(queues::routes::register)
    })
    .bind("127.0.0.1:8080")?;

    info!("Starting server");
    server.run().await?;

    Ok(())
}