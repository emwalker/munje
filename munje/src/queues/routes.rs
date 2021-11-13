use actix_web::{
    get, post, web,
    web::{Data, Form, Path},
};
use askama::Template;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    queues::{AnswerQuestion, NextQuestion, Queue, WideAnswer},
    types::{AppState, CurrentPage, Message},
    users::User,
};

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

#[derive(Template)]
#[template(path = "queues/list.jinja")]
struct List<'a> {
    queues: &'a Vec<Queue>,
    page: CurrentPage,
    messages: &'a Vec<Message>,
}

#[get("/{handle}/queues")]
async fn list(state: Data<AppState>, path: Path<String>) -> Result<HttpResponse, Error> {
    let handle = path.into_inner();
    let messages = Message::none();
    let queues = User::find_by_handle(handle.clone(), &state.db)
        .await?
        .queues(&state.db)
        .await?;

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
async fn show(state: Data<AppState>, path: Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let messages = &Message::none();

    let queue = &Queue::find(&id, &state.db).await?;
    let next_question = queue.next_question(&state.db).await?;
    let recent_answers = queue.recent_answers(&state.db).await?;

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
    fn translated_state(&self) -> Result<String, Error> {
        let state = match self.state.as_ref() {
            "Correct" => Ok("correct"),
            "Incorrect" => Ok("incorrect"),
            "Too hard" => Ok("unsure"),
            other => Err(Error::Generic(format!("Incorrect state: {}", other))),
        }?
        .to_string();

        Ok(state)
    }
}

#[post("/queues/{queue_id}/questions/{question_id}")]
async fn answer_question(
    form: Form<AnswerQuestionForm>,
    path: Path<(String, String)>,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (queue_external_id, question_external_id) = path.into_inner();
    let form = form.into_inner();
    let queue = Queue::find(&queue_external_id, request.db()?).await?;
    let state_name = form.translated_state()?;

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
            request.db()?,
        )
        .await?;

    request.redirect(format!("/queues/{}", queue_external_id).as_ref())
}
