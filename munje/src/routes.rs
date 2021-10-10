mod questions;

use actix_web::{get, web, Error, HttpResponse};
use anyhow::Result;
use askama::Template;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(home);
    questions::configure(cfg);
}

#[derive(Template)]
#[template(path = "home.jinja")]
struct Home;

#[get("/")]
async fn home() -> Result<HttpResponse, Error> {
    let s = Home.render().unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Question, QuestionData};
    use crate::types::{AppState, Pool};
    use actix_web::dev::{HttpServiceFactory, Service};
    use actix_web::{http, test, App};
    use anyhow::Error;
    use scraper::{Html, Selector};
    use std::str;

    #[derive(Clone)]
    struct Document {
        inner: Html,
    }

    impl Document {
        fn select_text(&self, selector: &str) -> String {
            let sel = Selector::parse(selector).unwrap();
            match self.inner.select(&sel).next() {
                Some(element) => element.inner_html(),
                None => "".to_string(),
            }
        }
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
            let result = sqlx::SqlitePool::builder()
                .max_size(10)
                .build("sqlite:///tmp/munje/test.db".as_ref())
                .await;
            match result {
                Ok(pool) => {
                    let mut tx = pool.acquire().await?;
                    sqlx::query!("delete from questions")
                        .execute(&mut tx)
                        .await?;
                    Ok(pool)
                }
                Err(_) => panic!("Unable to fetch database pool"),
            }
        }

        async fn get<F>(&self, service: F, path: &str) -> Result<Document, Error>
        where
            F: HttpServiceFactory + 'static,
        {
            let app = App::new()
                .data(AppState {
                    pool: self.pool.clone(),
                })
                .service(service);
            let mut app = test::init_service(app).await;

            let req = test::TestRequest::get().uri(path).to_request();
            let resp = app.call(req).await.unwrap();

            assert_eq!(resp.status(), http::StatusCode::OK);

            let body = match resp.response().body().as_ref() {
                Some(actix_web::body::Body::Bytes(bytes)) => bytes,
                _ => panic!("Response error"),
            };

            let html = str::from_utf8(&body).unwrap();
            Ok(Document {
                inner: Html::parse_document(html),
            })
        }
    }

    #[actix_rt::test]
    async fn test_home() -> Result<(), Error> {
        let doc = Harness::new().await.get(home, "/").await?;
        assert_eq!("Munje", doc.select_text("p.title"));
        Ok(())
    }

    #[actix_rt::test]
    async fn test_list() -> Result<(), Error> {
        let doc = Harness::new()
            .await
            .get(questions::list, "/questions")
            .await?;
        assert_eq!("Questions", doc.select_text("h2"));
        Ok(())
    }

    #[actix_rt::test]
    async fn test_new() -> Result<(), Error> {
        let doc = Harness::new()
            .await
            .get(questions::show_or_new, "/questions/new")
            .await?;
        assert_eq!("Add a question", doc.select_text("h2"));
        Ok(())
    }

    #[actix_rt::test]
    async fn test_unknown() -> Result<(), Error> {
        let doc = Harness::new()
            .await
            .get(questions::show_or_new, "/questions/unknown")
            .await?;
        let title = doc.select_text("title");
        assert_eq!("Question not found", title);
        Ok(())
    }

    #[actix_rt::test]
    async fn test_show() -> Result<(), Error> {
        let harness = Harness::new().await;
        let data = QuestionData {
            link: "some-link".to_string(),
        };
        let question = Question::create(&data, &harness.pool).await?;
        let path = String::from("/questions/") + question.id.as_ref();
        let doc = harness.get(questions::show_or_new, &path).await?;

        assert_eq!("Question", doc.select_text("h2"));
        Ok(())
    }
}
