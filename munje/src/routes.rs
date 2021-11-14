use crate::types::{CurrentPage, Message};
use actix_web::{get, web, Error, HttpResponse};
use anyhow::Result;
use askama::Template;

use crate::prelude::*;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(home).service(overview).service(robots);
}

#[derive(Template)]
#[template(path = "home.jinja")]
struct Home {
    messages: Vec<Message>,
    page: CurrentPage,
}

#[get("/")]
async fn home(request: HttpRequest) -> Result<HttpResponse, Error> {
    let s = Home {
        messages: Message::none(),
        page: CurrentPage::from("/", request.user()?),
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Template)]
#[template(path = "overview.jinja")]
struct Overview {
    messages: Vec<Message>,
    page: CurrentPage,
}

#[get("/overview")]
async fn overview(request: HttpRequest) -> Result<HttpResponse, Error> {
    let s = Overview {
        messages: Message::none(),
        page: CurrentPage::from("/overview", request.user()?),
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
