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
use serde::{Deserialize, Serialize};
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
    questions: &'a Vec<Question>,
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
    let questions = Question::find_all(&state.db)
        .await
        .map_err(|error| ListError {
            message: format!("Problem fetching questions: {}", error),
        })?;

    let s = List {
        questions: &questions,
        messages: &Message::to_messages(&messages),
        page: page(),
    }
    .render()
    .unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuestionForm {
    title: String,
    link: String,
}

#[derive(Template)]
#[template(path = "questions/new.jinja")]
struct New<'a> {
    form: &'a QuestionForm,
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

#[get("/questions/{external_id}")]
async fn show_or_new(
    state: Data<AppState>,
    path: Path<String>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let external_id = path.into_inner();
    let messages = &Message::to_messages(&messages);

    let s = match external_id.as_ref() {
        "new" => {
            let form = &QuestionForm {
                title: "".to_string(),
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
            let question = Question::find(&external_id, &state.db)
                .await
                .map_err(|error| ShowError {
                    message: format!("Problem fetching question: {}", error),
                })?;
            Show {
                question: &question,
                messages,
                page: page(),
            }
            .render()
            .unwrap()
        }
    };

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

async fn fetch_page(link: &String) -> Result<Page> {
    let original_url = Url::parse(link)?;
    info!("Fetching text at link {}", original_url);
    let html = reqwest::get(link).await?.text().await?;
    let page = Page::parse(html, link.to_string())?;
    Ok(page)
}

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem")]
struct CreateError {
    form: QuestionForm,
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
async fn create(state: Data<AppState>, form: Form<QuestionForm>) -> Result<HttpResponse, Error> {
    let form = form.into_inner();
    let page = fetch_page(&form.link).await.map_err(|error| CreateError {
        form: form.clone(),
        message: format!("Problem fetching the logo: {}", error),
    })?;
    let author_id = 1;

    let question = CreateQuestion {
        author_id,
        link: form.link.clone(),
        link_logo: page.meta_image().map(|url| url.to_string()),
        title: form.title.clone(),
    };
    Question::create(question, &state.db)
        .await
        .map_err(|error| CreateError {
            form,
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
        error!("There was a problem: {}", self.message);
        FlashMessage::warning(self.message.clone()).send();
        HttpResponse::SeeOther()
            .append_header((http::header::LOCATION, "/questions"))
            .finish()
    }
}

#[post("/questions/{external_id}/queues")]
async fn start_queue(path: Path<String>, state: Data<AppState>) -> Result<HttpResponse, Error> {
    let external_id = path.into_inner();

    let queue = CreateQueue {
        description: "Questions related to algorithms and data structures".to_string(),
        starting_question_external_id: external_id.clone(),
        title: "Algorithms and data strucures".to_string(),
        user_id: 1,
    };
    let result = Queue::find_or_create(queue, &state.db)
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
            format!("/queues/{}", result.record.external_id),
        ))
        .finish();
    Ok(redirect)
}
