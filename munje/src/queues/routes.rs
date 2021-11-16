use actix_identity::Identity;
use actix_web::{
    get, post, web,
    web::{Form, Path},
};
use askama::Template;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    queues::{AnswerQuestion, NextQuestion, Queue, WideAnswer},
    types::{CurrentPage, Message},
    users::User,
};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(show).service(answer_question).service(list);
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
async fn list(
    path: Path<String>,
    request: HttpRequest,
    id: Identity,
) -> Result<HttpResponse, Error> {
    let handle = path.into_inner();
    let messages = Message::none();
    let db = request.db()?;
    let queues = User::find_by_handle(handle.clone(), db)
        .await?
        .queues(db)
        .await?;

    let s = List {
        messages: &messages,
        page: CurrentPage::from("/queues", auth::user(&id)?),
        queues: &queues,
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[get("/{handle}/queues/{queue_id}")]
async fn show(
    path: Path<(String, String)>,
    request: HttpRequest,
    id: Identity,
) -> Result<HttpResponse, Error> {
    let (_handle, queue_id) = path.into_inner();
    let messages = &Message::none();
    let db = request.db()?;

    let queue = &Queue::find(&queue_id, db).await?;
    let next_question = queue.next_question(db).await?;
    let recent_answers = queue.recent_answers(db).await?;

    let s = Show {
        queue,
        messages,
        page: CurrentPage::from("/queues", auth::user(&id)?),
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

#[post("/{handle}/queues/{queue_id}/questions/{question_id}")]
async fn answer_question(
    form: Form<AnswerQuestionForm>,
    path: Path<(String, String, String)>,
    request: HttpRequest,
    id: Identity,
) -> Result<HttpResponse, Error> {
    if id.identity().is_none() {
        return request.redirect("/");
    }

    let (handle, queue_external_id, question_external_id) = path.into_inner();
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

    request.redirect(format!("/{}/queues/{}", handle, queue_external_id).as_ref())
}
