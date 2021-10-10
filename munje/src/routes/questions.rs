use crate::models::{Question, QuestionData};
use crate::{AppState, Pool};

use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use anyhow::Result;
use askama::Template;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(show_or_new).service(create);
}

#[derive(Template)]
#[template(path = "questions/list.jinja")]
struct List<'a> {
    items: &'a Vec<Question>,
}

async fn fetch_all(pool: &Pool) -> Result<HttpResponse, Error> {
    let result = Question::find_all(pool).await;
    let s = match result {
        Ok(items) => List { items: &items }.render().unwrap(),
        Err(err) => format!("error: {}", err).to_string(),
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[get("/questions")]
async fn list(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    fetch_all(&state.pool).await
}

#[derive(Template)]
#[template(path = "questions/new.jinja")]
struct New<'a> {
    form: &'a QuestionData,
}

#[derive(Template)]
#[template(path = "questions/show.jinja")]
struct Show<'a> {
    question: &'a Question,
}

#[derive(Template)]
#[template(path = "questions/not-found.jinja")]
struct NotFound;

#[get("/questions/{id}")]
async fn show_or_new(
    state: web::Data<AppState>,
    path: web::Path<(String,)>,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner().0;
    let s = match id.as_ref() {
        "new" => {
            let form = &QuestionData {
                link: "".to_string(),
            };
            New { form: form }.render().unwrap()
        }
        _ => {
            let result = Question::find_by_id(id, &state.pool).await;
            match result {
                Ok(Some(question)) => Show {
                    question: &question,
                }
                .render()
                .unwrap(),
                Ok(None) => NotFound.render().unwrap(),
                Err(_) => NotFound.render().unwrap(),
            }
        }
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[post("/questions")]
async fn create(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Form<QuestionData>,
) -> Result<HttpResponse, Error> {
    println!("REQ: {:?}", req);
    let data = form.into_inner();
    let result = Question::create(&data, &state.pool).await;
    match result {
        Ok(_) => fetch_all(&state.pool).await,
        err => {
            error!("There was a problem: {:?}", err);
            let s = New { form: &data }.render().unwrap();
            Ok(HttpResponse::Ok().content_type("text/html").body(s))
        }
    }
}
