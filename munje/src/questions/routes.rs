use actix_web::{
    error, get, http, post, web,
    web::{Data, Form, Path},
    Error, HttpResponse,
};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use anyhow::Result;
use askama::Template;
use derive_more::{Display, Error};
use reqwest;
use url::Url;

use crate::page::Page;
use crate::questions::{CreateQuestion, Question};
use crate::queues::{CreateQueue, Queue};
use crate::types::{AppState, CurrentPage, Message};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(list)
        .service(show_or_new)
        .service(create)
        .service(start_queue);
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

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem")]
struct ListError {
    message: String,
}

impl error::ResponseError for ListError {
    fn error_response(&self) -> HttpResponse {
        let messages = vec![Message {
            content: self.message.clone(),
            level: "warning".to_string(),
        }];
        let s = NotFound {
            messages: &messages,
            page: page(),
        }
        .render()
        .unwrap();
        HttpResponse::Ok().content_type("text/html").body(s)
    }
}

#[get("/questions")]
async fn list(
    state: Data<AppState>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let items = Question::find_all(&state.db)
        .await
        .map_err(|error| ListError {
            message: format!("Problem fetching questions: {}", error),
        })?;

    let s = List {
        items: &items,
        messages: &Message::to_messages(&messages),
        page: page(),
    }
    .render()
    .unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Template)]
#[template(path = "questions/new.jinja")]
struct New<'a> {
    form: &'a CreateQuestion,
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

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem")]
struct ShowError {
    message: String,
}

impl error::ResponseError for ShowError {
    fn error_response(&self) -> HttpResponse {
        let messages = vec![Message {
            content: self.message.clone(),
            level: "warning".to_string(),
        }];
        let s = NotFound {
            messages: &messages,
            page: page(),
        }
        .render()
        .unwrap();
        HttpResponse::Ok().content_type("text/html").body(s)
    }
}

#[get("/questions/{id}")]
async fn show_or_new(
    state: Data<AppState>,
    path: Path<String>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let messages = &Message::to_messages(&messages);

    let s = match id.as_ref() {
        "new" => {
            let form = &CreateQuestion {
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
            let result = Question::find_by_id(id, &state.db)
                .await
                .map_err(|error| ShowError {
                    message: format!("Problem fetching question: {}", error),
                })?;
            match result {
                Some(question) => Show {
                    question: &question,
                    messages,
                    page: page(),
                }
                .render()
                .unwrap(),
                None => NotFound {
                    messages,
                    page: page(),
                }
                .render()
                .unwrap(),
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

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem")]
struct CreateError {
    form: CreateQuestion,
    message: String,
}

impl error::ResponseError for CreateError {
    fn error_response(&self) -> HttpResponse {
        let messages = vec![Message {
            content: self.message.clone(),
            level: "warning".to_string(),
        }];
        let s = New {
            form: &self.form,
            messages: &messages,
            page: page(),
        }
        .render()
        .unwrap();
        HttpResponse::Ok().content_type("text/html").body(s)
    }
}

#[post("/questions")]
async fn create(state: Data<AppState>, form: Form<CreateQuestion>) -> Result<HttpResponse, Error> {
    let form = form.into_inner();
    let link_logo = fetch_logo(&form.link).await.map_err(|error| CreateError {
        form: form.clone(),
        message: format!("Problem fetching the logo: {}", error),
    })?;
    let author_id = "21546b43-dcde-43b2-a251-e736194de0a0";

    Question::create(author_id.to_string(), &form, link_logo, &state.db)
        .await
        .map_err(|error| CreateError {
            form: form,
            message: format!("Problem saving the question: {}", error),
        })?;
    FlashMessage::info("Question created").send();

    let redirect = HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, "/questions"))
        .finish();
    Ok(redirect)
}

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem starting a queue")]
struct StartQueueError {
    message: String,
}

impl error::ResponseError for StartQueueError {
    fn error_response(&self) -> HttpResponse {
        FlashMessage::warning(self.message.clone()).send();
        HttpResponse::SeeOther()
            .append_header((http::header::LOCATION, "/questions"))
            .finish()
    }
}

#[post("/questions/{id}/queues")]
async fn start_queue(path: Path<String>, state: Data<AppState>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();

    let queue = CreateQueue {
        user_id: "21546b43-dcde-43b2-a251-e736194de0a0".to_string(),
        starting_question_id: id.clone(),
    };
    let result = Queue::find_or_create(&queue, &state.db)
        .await
        .map_err(|error| StartQueueError {
            message: format!("Problem starting queue: {}", error),
        })?;
    if result.created {
        FlashMessage::info("New queue started").send();
    }

    let redirect = HttpResponse::SeeOther()
        .append_header((
            http::header::LOCATION,
            format!("/queues/{}", result.queue.id),
        ))
        .finish();
    Ok(redirect)
}
