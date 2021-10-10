#[macro_use]
extern crate log;

mod models;
mod routes;
mod types;

use actix_web::{middleware, App, HttpServer};
use anyhow::Result;
use dotenv::dotenv;
use sqlx::SqlitePool;
use types::{AppState, Pool};

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    info!("using sqlite database at: {}", &database_url);
    let pool = SqlitePool::new(&database_url).await?;

    let server = HttpServer::new(move || {
        App::new()
            .data(AppState { pool: pool.clone() })
            .wrap(middleware::Logger::default())
            .configure(routes::init)
    })
    .bind("127.0.0.1:8080")?;

    info!("Starting server");
    server.run().await?;

    Ok(())
}
