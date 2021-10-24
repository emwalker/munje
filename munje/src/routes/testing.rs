#![allow(dead_code)]

use crate::types::{AppState, Pool};
use actix_web::cookie::Key;
use actix_web::dev::{HttpServiceFactory, Service};
use actix_web::{http, test, web, App};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use anyhow::Error;
use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use sqlx::sqlite::SqlitePoolOptions;
use std::str;

pub type TestResult = Result<(), Error>;

#[derive(Clone)]
pub struct Document {
    doc: Html,
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
}

impl Document {
    fn from(html: &str) -> Self {
        Self {
            doc: Html::parse_document(html),
        }
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
    pub pool: Pool,
}

impl Runner {
    pub async fn new() -> Self {
        match Self::fetch_pool().await {
            Ok(pool) => Runner { pool: pool },
            Err(err) => panic!("There was a problem: {}", err),
        }
    }

    pub async fn get<F>(&self, service: F, path: &str) -> Result<Document, Error>
    where
        F: HttpServiceFactory + 'static,
    {
        let signing_key = Key::generate(); // This will usually come from configuration!
        let message_store = CookieMessageStore::builder(signing_key).build();
        let message_framework = FlashMessagesFramework::builder(message_store).build();

        let app = App::new()
            .app_data(web::Data::new(AppState {
                pool: self.pool.clone(),
            }))
            .wrap(message_framework.clone())
            .service(service);
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri(path).to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let body = match resp.response().body() {
            actix_web::body::Body::Bytes(bytes) => bytes,
            _ => panic!("Response error"),
        };

        let html = str::from_utf8(&body).unwrap();
        Ok(Document::from(html))
    }

    async fn fetch_pool() -> Result<Pool> {
        let result = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await;
        match result {
            Ok(pool) => {
                sqlx::migrate!("./migrations").run(&pool).await?;
                Ok(pool)
            }
            Err(_) => panic!("Unable to fetch database pool"),
        }
    }
}
