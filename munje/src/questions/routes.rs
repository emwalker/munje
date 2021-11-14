use actix_web::{
    get, post, web,
    web::{Form, Path},
};
use anyhow::Result;
use askama::Template;
use reqwest;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    page::Page,
    prelude::*,
    questions::{CreateQuestion, Question},
    queues::{CreateQueue, Queue},
    types::{CurrentPage, Message},
    users::User,
};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(list)
        .service(show_or_new)
        .service(create)
        .service(start_queue);
}

#[derive(Template)]
#[template(path = "questions/list.jinja")]
struct List<'a> {
    questions: &'a Vec<Question>,
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[get("/questions")]
async fn list(request: HttpRequest) -> Result<HttpResponse, Error> {
    let questions = Question::find_all(request.db()?).await?;
    let s = List {
        questions: &questions,
        messages: &Message::none(),
        page: CurrentPage::from("/questions", request.user()?),
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

#[get("/questions/{external_id}")]
async fn show_or_new(path: Path<String>, request: HttpRequest) -> Result<HttpResponse, Error> {
    let external_id = path.into_inner();
    let messages = &Message::none();
    let user = request.user().unwrap_or(User::default());

    let s = match external_id.as_ref() {
        "new" => {
            let form = &QuestionForm {
                title: "".to_string(),
                link: "".to_string(),
            };
            New {
                form,
                messages,
                page: CurrentPage::from("/questions", user),
            }
            .render()
            .unwrap()
        }
        _ => {
            let db = request.db()?;
            let question = Question::find(&external_id, db).await?;
            Show {
                question: &question,
                messages,
                page: CurrentPage::from("/questions", user),
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

#[post("/questions")]
async fn create(form: Form<QuestionForm>, request: HttpRequest) -> Result<HttpResponse, Error> {
    let form = form.into_inner();
    let page = fetch_page(&form.link).await?;

    let question = CreateQuestion {
        author_id: request.user()?.id,
        link: form.link.clone(),
        link_logo: page.meta_image().map(|url| url.to_string()),
        title: form.title.clone(),
    };
    Question::create(question, request.db()?).await?;

    request.redirect("/questions")
}

#[post("/questions/{external_id}/queues")]
async fn start_queue(path: Path<String>, request: HttpRequest) -> Result<HttpResponse, Error> {
    let external_id = path.into_inner();
    let user = request.user()?;

    let queue = CreateQueue {
        description: "Questions related to algorithms and data structures".to_string(),
        starting_question_external_id: external_id.clone(),
        title: "Algorithms and data strucures".to_string(),
        user_id: 1,
    };
    let result = Queue::find_or_create(queue, request.db()?).await?;

    let path = format!("/{}/queues/{}", user.handle, result.record.external_id);
    request.redirect(path.as_ref())
}
