use crate::types::{CurrentPage, Message};
use actix_web::{get, web, Error, HttpResponse};
use anyhow::Result;
use askama::Template;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(home).service(robots);
}

#[derive(Template)]
#[template(path = "home.jinja")]
struct Home<'a> {
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[get("/")]
async fn home() -> Result<HttpResponse, Error> {
    let s = Home {
        messages: &Message::none(),
        page: CurrentPage {
            path: "/".to_string(),
        },
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Template)]
#[template(path = "robots.jinja")]
struct Robots;

#[get("/robots.txt")]
async fn robots() -> Result<HttpResponse, Error> {
    let s = Robots.render().unwrap();
    Ok(HttpResponse::Ok().content_type("text/plain").body(s))
}
