use crate::models::{Question, QuestionData};
use crate::page::Page;
use crate::types::{CurrentPage, Message};
use crate::{AppState, Pool};

use actix_web::{get, http, post, web, Error, HttpResponse};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use anyhow::Result;
use askama::Template;
use reqwest;
use url::Url;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(show_or_new).service(create);
}

fn page() -> CurrentPage {
    CurrentPage {
        path: "/questions".to_string(),
    }
}

#[derive(Template)]
#[template(path = "questions/list.jinja")]
struct List<'a> {
    items: &'a Vec<Question>,
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

async fn fetch_all(pool: &Pool, messages: IncomingFlashMessages) -> Result<HttpResponse, Error> {
    let result = Question::find_all(pool).await;
    let s = match result {
        Ok(items) => List {
            items: &items,
            messages: &Message::to_messages(&messages),
            page: page(),
        }
        .render()
        .unwrap(),
        Err(err) => format!("error: {}", err).to_string(),
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[get("/questions")]
async fn list(
    state: web::Data<AppState>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    fetch_all(&state.pool, messages).await
}

#[derive(Template)]
#[template(path = "questions/new.jinja")]
struct New<'a> {
    form: &'a QuestionData,
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[derive(Template)]
#[template(path = "questions/show.jinja")]
struct Show<'a> {
    question: &'a Question,
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[derive(Template)]
#[template(path = "questions/not-found.jinja")]
struct NotFound<'a> {
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[get("/questions/{id}")]
async fn show_or_new(
    state: web::Data<AppState>,
    path: web::Path<(String,)>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner().0;
    let messages = &Message::to_messages(&messages);
    let s = match id.as_ref() {
        "new" => {
            let form = &QuestionData {
                link: "".to_string(),
            };
            New {
                form,
                messages,
                page: page(),
            }
            .render()
            .unwrap()
        }
        _ => {
            let result = Question::find_by_id(id, &state.pool).await;
            match result {
                Ok(Some(question)) => Show {
                    question: &question,
                    messages,
                    page: page(),
                }
                .render()
                .unwrap(),
                Ok(None) => NotFound {
                    messages,
                    page: page(),
                }
                .render()
                .unwrap(),
                Err(_) => {
                    FlashMessage::error("There was a problem").send();
                    NotFound {
                        messages,
                        page: page(),
                    }
                    .render()
                    .unwrap()
                }
            }
        }
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

async fn fetch_logo(link: &String) -> Result<Option<String>> {
    let original_url = Url::parse(link)?;
    info!("Fetching text at link {}", original_url);
    let html = reqwest::get(link).await?.text().await?;
    match Page::parse(html, link.to_string()) {
        Ok(page) => match page.meta_image() {
            Some(url) => {
                let url_str = url.to_string();
                info!("Using logo url: {}", url_str);
                Ok(Some(url_str))
            }
            None => Ok(None),
        },
        Err(err) => {
            error!("Problem parsing page: {:?}", err);
            Ok(None)
        }
    }
}

#[post("/questions")]
async fn create(
    state: web::Data<AppState>,
    form: web::Form<QuestionData>,
) -> Result<HttpResponse, Error> {
    let data = form.into_inner();

    let error_result = |message: String| -> Result<HttpResponse, Error> {
        let messages = vec![Message {
            content: message,
            level: "warning".to_string(),
        }];
        let s = New {
            form: &data,
            messages: &messages,
            page: page(),
        }
        .render()
        .unwrap();
        Ok(HttpResponse::Ok().content_type("text/html").body(s))
    };

    match fetch_logo(&data.link).await {
        Ok(link_logo) => {
            let result = Question::create(&data, link_logo, &state.pool).await;
            match result {
                Err(err) => error_result(format!("There was a problem: {:?}", err)),
                Ok(_) => {
                    FlashMessage::info("Question created").send();
                    let redirect = HttpResponse::SeeOther()
                        .append_header((http::header::LOCATION, "/questions"))
                        .finish();
                    Ok(redirect)
                }
            }
        }
        Err(err) => return error_result(format!("Problem fetching logo: {:?}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{Runner, TestResult};
    use super::*;

    #[actix_rt::test]
    async fn test_list() -> TestResult {
        let doc = Runner::new().await.get(list, "/questions").await?;
        assert_eq!("Questions", doc.select_text("h2").unwrap());
        Ok(())
    }

    #[actix_rt::test]
    async fn new() -> TestResult {
        let doc = Runner::new()
            .await
            .get(show_or_new, "/questions/new")
            .await?;
        assert_eq!("Add a question", doc.select_text("h2").unwrap());
        Ok(())
    }

    #[actix_rt::test]
    async fn unknown() -> TestResult {
        let doc = Runner::new()
            .await
            .get(show_or_new, "/questions/unknown")
            .await?;
        let title = doc.select_text("title").unwrap();
        assert_eq!("Question not found", title);
        Ok(())
    }

    #[actix_rt::test]
    async fn show() -> TestResult {
        let harness = Runner::new().await;
        let data = QuestionData {
            link: "some-link".to_string(),
        };
        let question = Question::create(&data, Some("logo-url".to_string()), &harness.pool).await?;
        let path = format!("/questions/{}", question.id);
        let doc = harness.get(show_or_new, &path).await?;

        assert_eq!("Question", doc.css("h2")?.first().unwrap().inner_html());
        assert_eq!(
            "some-link",
            doc.css("a.link")?
                .first()
                .unwrap()
                .value()
                .attr("href")
                .unwrap(),
        );
        assert!(doc.css("div.link-logo")?.exists());
        assert!(doc.css("button.start-queue")?.exists());
        Ok(())
    }

    #[actix_rt::test]
    async fn start_queue() -> TestResult {
        Ok(())
    }
}
