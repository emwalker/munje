#[macro_use]
extern crate log;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{cookie::SameSite, middleware, web, App, HttpServer};
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use time::Duration;

use munje::{
    questions, queues, routes,
    types::{AppState, Config},
    users,
};

#[actix_web::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    env_logger::init();

    info!("Connecting to database: {}", &config.database_url);
    let db = PgPoolOptions::new().connect(&config.database_url).await?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { db: db.clone() }))
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(config.session_key.as_bytes())
                    .name("auth-cookie")
                    .max_age(Duration::days(356))
                    .same_site(SameSite::Strict)
                    .secure(true),
            ))
            .configure(routes::register)
            .configure(users::routes::register)
            .configure(questions::routes::register)
            .configure(queues::routes::register)
    })
    .bind("0.0.0.0:8080")?;

    info!("Starting server");
    server.run().await?;

    Ok(())
}
