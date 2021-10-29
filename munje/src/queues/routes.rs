use actix_web::{
    error, get, http, post, web,
    web::{Data, Form, Path},
    Error, HttpResponse,
};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use askama::Template;
use derive_more::{Display, Error};
use serde::Deserialize;

use crate::queues::{AnswerQuestion, Queue, WideAnswer};
use crate::types::{AppState, CurrentPage, Message};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(show).service(answer_question);
}

fn page() -> CurrentPage {
    CurrentPage {
        path: "/queues".to_string(),
    }
}

#[derive(Template)]
#[template(path = "queues/show.jinja")]
struct Show<'a> {
    queue: &'a Queue,
    messages: &'a Vec<Message>,
    page: CurrentPage,
    next_answer: WideAnswer,
    recent_answers: Vec<WideAnswer>,
}

#[derive(Template)]
#[template(path = "queues/not-found.jinja")]
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

#[get("/queues/{id}")]
async fn show(
    state: Data<AppState>,
    path: Path<String>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let messages = &Message::to_messages(&messages);

    let queue = Queue::find_by_id(&id, &state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching question: {}", error),
        })?;

    let next_answer = queue
        .next_answer(&state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching next answer: {}", error),
        })?;

    let recent_answers = next_answer
        .recent_answers(&state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching recent answers: {}", error),
        })?;

    let s = Show {
        queue: &queue,
        messages,
        page: page(),
        next_answer: next_answer,
        recent_answers: recent_answers,
    }
    .render()
    .unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Deserialize)]
struct AnswerQuestionForm {
    state: String,
}

impl AnswerQuestionForm {
    fn translated_state(&self, queue_id: String) -> Result<String, Error> {
        let state = match self.state.as_ref() {
            "Correct" => Ok("correct"),
            "Incorrect" => Ok("incorrect"),
            "Too hard" => Ok("unsure"),
            other => Err(AnswerQuestionError {
                message: format!("Incorrect state: {}", other),
                queue_id,
            }),
        }?
        .to_string();

        Ok(state)
    }
}

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem")]
struct AnswerQuestionError {
    message: String,
    queue_id: String,
}

impl error::ResponseError for AnswerQuestionError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self.message);
        FlashMessage::error(self.message.clone()).send();
        HttpResponse::SeeOther()
            .append_header((http::header::LOCATION, format!("/queues/{}", self.queue_id)))
            .finish()
    }
}

#[post("/queues/{id}/answers/{answer_id}")]
async fn answer_question(
    state: Data<AppState>,
    path: Path<(String, String)>,
    form: Form<AnswerQuestionForm>,
) -> Result<HttpResponse, Error> {
    let (queue_id, answer_id) = path.into_inner();
    let state = state.into_inner();
    let form = form.into_inner();

    let queue = Queue::find_by_id(&queue_id, &state.db)
        .await
        .map_err(|error| AnswerQuestionError {
            message: format!("Problem fetching question: {}", error),
            queue_id: queue_id.clone(),
        })?;
    let state_name = form.translated_state(queue_id.clone())?;

    info!("Updating answer {} to state {}", answer_id, state_name);
    queue
        .answer_question(
            AnswerQuestion {
                answer_id,
                state: state_name,
                user_id: queue.user_id.clone(),
                queue_id: queue_id.clone(),
            },
            &state.db,
        )
        .await
        .map_err(|error| AnswerQuestionError {
            message: format!("Problem answering question: {}", error),
            queue_id: queue_id.clone(),
        })?;

    let redirect = HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, format!("/queues/{}", queue_id)))
        .finish();
    Ok(redirect)
}
