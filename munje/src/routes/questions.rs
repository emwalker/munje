use crate::models::{Question, QuestionData};
use crate::types::{Document, Message};
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

#[derive(Template)]
#[template(path = "questions/list.jinja")]
struct List<'a> {
    items: &'a Vec<Question>,
    messages: &'a Vec<Message>,
}

async fn fetch_all(pool: &Pool, messages: IncomingFlashMessages) -> Result<HttpResponse, Error> {
    let result = Question::find_all(pool).await;
    let s = match result {
        Ok(items) => List {
            items: &items,
            messages: &Message::to_messages(&messages),
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
}

#[derive(Template)]
#[template(path = "questions/show.jinja")]
struct Show<'a> {
    question: &'a Question,
    messages: &'a Vec<Message>,
}

#[derive(Template)]
#[template(path = "questions/not-found.jinja")]
struct NotFound<'a> {
    messages: &'a Vec<Message>,
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
                form: form,
                messages,
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
                }
                .render()
                .unwrap(),
                Ok(None) => NotFound { messages }.render().unwrap(),
                Err(_) => {
                    FlashMessage::error("There was a problem").send();
                    NotFound { messages }.render().unwrap()
                }
            }
        }
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

async fn fetch_logo(link: &String) -> Result<String> {
    let original_url = Url::parse(link).unwrap();
    info!("Fetching text at link {}", original_url);
    let text = reqwest::get(link).await?.text().await?;
    let doc = Document::from(text.as_ref());
    let result = doc.select_attr(r#"meta[property="og:image"]"#, "content");
    let link_logo = match result {
        Some(fetched_url) => {
            let parsed_url = Url::parse(fetched_url.as_ref())?;
            match parsed_url.host() {
                Some(_) => fetched_url.to_string(),
                None => original_url.host_str().unwrap().to_string() + &fetched_url,
            }
        }
        None => "default-logo".to_string(),
    };
    info!("Using logo url: {}", link_logo);
    Ok(link_logo)
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
        Err(err) => return error_result(format!("Unable to fetch logo: {:?}", err)),
    }
}
