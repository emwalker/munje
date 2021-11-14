#[macro_use]
extern crate log;

use actix_session::CookieSession;
use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

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
        let key = config.session_key.clone();

        // !production needs no domain set, because browsers.
        #[cfg(not(feature = "production"))]
        let session_storage = CookieSession::signed(key.as_bytes())
            .name("sessionid")
            .secure(false)
            .path("/");

        #[cfg(feature = "production")]
        let session_storage = CookieSession::signed(key.as_bytes())
            .name("sessionid")
            .path("/")
            .same_site(actix_web::cookie::SameSite::Lax)
            .domain(&config.session_domain.clone())
            .secure(true);

        App::new()
            .app_data(web::Data::new(AppState { db: db.clone() }))
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(session_storage)
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
