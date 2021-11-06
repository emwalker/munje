use actix_web::{
    error, get, http, post, web,
    web::{Data, Form, Path},
    Error, HttpResponse,
};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use askama::Template;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::queues::{AnswerQuestion, NextQuestion, Queue, WideAnswer};
use crate::routes::redirect_to;
use crate::types::{AppState, CurrentPage, Message};
use crate::users::User;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(show).service(answer_question).service(list);
}

fn page() -> CurrentPage {
    CurrentPage {
        path: "/gnusto/queues".to_string(),
    }
}

#[derive(Template)]
#[template(path = "queues/show.jinja")]
struct Show<'a> {
    queue: &'a Queue,
    messages: &'a Vec<Message>,
    page: CurrentPage,
    next_question: NextQuestion,
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

#[derive(Template)]
#[template(path = "queues/list.jinja")]
struct List<'a> {
    queues: &'a Vec<Queue>,
    page: CurrentPage,
    messages: &'a Vec<Message>,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "There was a problem getting the list of queues")]
struct ListError {
    message: String,
    handle: String,
}

impl error::ResponseError for ListError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self.message);
        FlashMessage::error(self.message.clone()).send();
        redirect_to("/questions".to_string())
    }
}

// FIXME: /{user-handle}/queues
#[get("/{handle}/queues")]
async fn list(
    state: Data<AppState>,
    messages: IncomingFlashMessages,
    path: Path<String>,
) -> Result<HttpResponse, Error> {
    let handle = path.into_inner();
    let messages = Message::to_messages(&messages);
    let queues = User::find_by_handle(handle.clone(), &state.db)
        .await
        .map_err(|error| ListError {
            message: format!("Problem getting list of queues: {}", error),
            handle: handle.clone(),
        })?
        .queues(&state.db)
        .await
        .map_err(|error| ListError {
            message: format!("Problem getting list of queues: {}", error),
            handle: handle.clone(),
        })?;

    let s = List {
        messages: &messages,
        page: page(),
        queues: &queues,
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[get("/queues/{id}")]
async fn show(
    state: Data<AppState>,
    path: Path<String>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let messages = &Message::to_messages(&messages);

    let queue = &Queue::find(&id, &state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching queue: {}", error),
        })?;

    let next_question = queue
        .next_question(&state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching next question: {}", error),
        })?;

    let recent_answers = queue
        .recent_answers(&state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching recent answers: {}", error),
        })?;

    let s = Show {
        queue,
        messages,
        page: page(),
        next_question,
        recent_answers,
    }
    .render()
    .unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Serialize, Deserialize)]
pub struct AnswerQuestionForm {
    pub state: String,
}

impl AnswerQuestionForm {
    fn translated_state(&self, queue_external_id: String) -> Result<String, Error> {
        let state = match self.state.as_ref() {
            "Correct" => Ok("correct"),
            "Incorrect" => Ok("incorrect"),
            "Too hard" => Ok("unsure"),
            other => Err(AnswerQuestionError {
                message: format!("Incorrect state: {}", other),
                queue_external_id,
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
    queue_external_id: String,
}

impl error::ResponseError for AnswerQuestionError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self.message);
        FlashMessage::error(self.message.clone()).send();
        redirect_to(format!("/queues/{}", self.queue_external_id))
    }
}

#[post("/queues/{queue_id}/questions/{question_id}")]
async fn answer_question(
    state: Data<AppState>,
    path: Path<(String, String)>,
    form: Form<AnswerQuestionForm>,
) -> Result<HttpResponse, Error> {
    let (queue_external_id, question_external_id) = path.into_inner();
    let state = state.into_inner();
    let form = form.into_inner();

    let queue = Queue::find(&queue_external_id, &state.db)
        .await
        .map_err(|error| AnswerQuestionError {
            message: format!("Problem fetching question: {}", error),
            queue_external_id: queue_external_id.clone(),
        })?;
    let state_name = form.translated_state(queue_external_id.clone())?;

    info!(
        r#"Answering question {} as "{}"#,
        question_external_id, state_name
    );
    queue
        .answer_question(
            AnswerQuestion {
                question_external_id: question_external_id.clone(),
                state: state_name,
                user_id: queue.user_id,
                queue_id: queue.id,
            },
            &state.db,
        )
        .await
        .map_err(|error| AnswerQuestionError {
            message: format!("Problem answering question: {}", error),
            queue_external_id: queue_external_id.clone(),
        })?;

    let redirect = HttpResponse::SeeOther()
        .append_header((
            http::header::LOCATION,
            format!("/queues/{}", queue_external_id),
        ))
        .finish();
    Ok(redirect)
}
