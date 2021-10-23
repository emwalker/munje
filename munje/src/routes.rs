mod questions;

use crate::types::Message;
use actix_web::{get, web, Error, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use anyhow::Result;
use askama::Template;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(home);
    questions::configure(cfg);
}

#[derive(Template)]
#[template(path = "home.jinja")]
struct Home<'a> {
    messages: &'a Vec<Message>,
}

#[get("/")]
async fn home(messages: IncomingFlashMessages) -> Result<HttpResponse, Error> {
    let s = Home {
        messages: &Message::to_messages(&messages),
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Question, QuestionData};
    use crate::types::{AppState, Document, Pool};
    use actix_web::cookie::Key;
    use actix_web::dev::{HttpServiceFactory, Service};
    use actix_web::{http, test, web, App};
    use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
    use anyhow::Error;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::str;

    #[cfg(test)]
    #[ctor::ctor]
    fn init() {
        env_logger::init();
    }

    struct Harness {
        pool: Pool,
    }

    impl Harness {
        async fn new() -> Self {
            match Self::fetch_pool().await {
                Ok(pool) => Harness { pool: pool },
                Err(err) => panic!("There was a problem: {}", err),
            }
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

        async fn get<F>(&self, service: F, path: &str) -> Result<Document, Error>
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
    }

    #[actix_rt::test]
    async fn test_home() -> Result<(), Error> {
        let doc = Harness::new().await.get(home, "/").await?;
        assert_eq!("Munje", doc.select_text("p.title").unwrap());
        Ok(())
    }

    #[actix_rt::test]
    async fn test_list() -> Result<(), Error> {
        let doc = Harness::new()
            .await
            .get(questions::list, "/questions")
            .await?;
        assert_eq!("Questions", doc.select_text("h2").unwrap());
        Ok(())
    }

    #[actix_rt::test]
    async fn test_new() -> Result<(), Error> {
        let doc = Harness::new()
            .await
            .get(questions::show_or_new, "/questions/new")
            .await?;
        assert_eq!("Add a question", doc.select_text("h2").unwrap());
        Ok(())
    }

    #[actix_rt::test]
    async fn test_unknown() -> Result<(), Error> {
        let doc = Harness::new()
            .await
            .get(questions::show_or_new, "/questions/unknown")
            .await?;
        let title = doc.select_text("title").unwrap();
        assert_eq!("Question not found", title);
        Ok(())
    }

    #[actix_rt::test]
    async fn test_show() -> Result<(), Error> {
        let harness = Harness::new().await;
        let data = QuestionData {
            link: "some-link".to_string(),
        };
        let question = Question::create(&data, "logo-url".to_string(), &harness.pool).await?;
        let path = String::from("/questions/") + question.id.as_ref();
        let doc = harness.get(questions::show_or_new, &path).await?;

        assert_eq!("Question", doc.select_text("h2").unwrap());
        Ok(())
    }
}
