#![allow(dead_code)]

use actix_http::Request;
use actix_web::cookie::Key;
use actix_web::dev::Service;
use actix_web::{cookie, http, test, web, App};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use anyhow::Error;
use anyhow::Result;
use munje::{
    questions, queues, routes,
    types::{AppState, Config, Pool},
    users,
};
use scraper::{ElementRef, Html, Selector};
use sqlx::postgres::PgPoolOptions;
use std::str;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    env_logger::init();
}

pub type TestResult = Result<(), Error>;

#[derive(Debug, Clone)]
pub struct Document {
    source: String,
    doc: Html,
}

pub struct HttpResult {
    pub doc: Document,
    pub status: http::StatusCode,
}

pub struct Matches<'a> {
    selector: Selector,
    matches: Vec<ElementRef<'a>>,
}

impl<'a> Matches<'a> {
    pub fn first(&mut self) -> Option<&ElementRef> {
        self.matches.iter().next()
    }

    pub fn exists(&mut self) -> bool {
        self.matches.iter().next() != None
    }

    pub fn none(&mut self) -> bool {
        !self.exists()
    }
}

impl Document {
    fn from(html: &str) -> Self {
        Self {
            source: html.to_string(),
            doc: Html::parse_document(html),
        }
    }

    pub fn to_string(&self) -> String {
        self.source.to_string()
    }

    pub fn select_text(&self, selector: &str) -> Option<String> {
        match self.css(selector).unwrap().first() {
            Some(elem) => Some(elem.inner_html()),
            None => None,
        }
    }

    pub fn css(&self, selector_str: &str) -> Result<Matches> {
        let selector = Selector::parse(selector_str).unwrap();
        Ok(Matches {
            selector: selector.clone(),
            matches: self.doc.select(&selector).collect(),
        })
    }
}

pub struct Runner {
    pub db: Pool,
    pub user: users::User,
    signing_key: cookie::Key,
}

impl Runner {
    pub async fn new() -> Self {
        let signing_key = Key::generate(); // This will usually come from configuration!

        match Self::fetch_db().await {
            Ok(db) => {
                let user = users::User::find_by_handle("gnusto".to_string(), &db)
                    .await
                    .unwrap();
                Runner {
                    db: db,
                    user: user,
                    signing_key,
                }
            }
            Err(err) => panic!("There was a problem: {}", err),
        }
    }

    pub async fn get(&self, path: &str) -> Result<HttpResult, Error> {
        let message_store = CookieMessageStore::builder(self.signing_key.clone()).build();
        let message_framework = FlashMessagesFramework::builder(message_store).build();

        let app = App::new()
            .app_data(web::Data::new(AppState {
                db: self.db.clone(),
            }))
            .wrap(message_framework.clone())
            .service(routes::home)
            .configure(questions::routes::register)
            .configure(queues::routes::register);
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri(path).to_request();
        let resp = app.call(req).await.unwrap();

        let body = match resp.response().body() {
            actix_web::body::Body::Bytes(bytes) => bytes,
            actix_web::body::AnyBody::Empty => "".as_bytes(),
            other => panic!("Response error: {:?}", other),
        };
        let html = str::from_utf8(&body).unwrap();

        Ok(HttpResult {
            doc: Document::from(html),
            status: resp.status(),
        })
    }

    pub async fn post(&self, req: Request) -> Result<HttpResult, Error> {
        let message_store = CookieMessageStore::builder(self.signing_key.clone()).build();
        let message_framework = FlashMessagesFramework::builder(message_store).build();

        let app = App::new()
            .app_data(web::Data::new(AppState {
                db: self.db.clone(),
            }))
            .wrap(message_framework.clone())
            .configure(questions::routes::register)
            .configure(queues::routes::register);

        let app = test::init_service(app).await;
        let resp = app.call(req).await.unwrap();
        Ok(HttpResult {
            doc: Document::from(""),
            status: resp.status(),
        })
    }

    async fn fetch_db() -> Result<Pool> {
        let config = Config::load()?;
        let result = PgPoolOptions::new()
            .max_connections(1)
            .connect(&config.database_url)
            .await;
        match result {
            Ok(pool) => {
                sqlx::migrate!("./migrations").run(&pool).await?;
                Ok(pool)
            }
            Err(err) => panic!("Unable to fetch database pool: {}", err),
        }
    }
}
