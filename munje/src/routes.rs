use actix_identity::Identity;
use actix_web::{get, web, Error, HttpResponse};
use anyhow::Result;
use askama::Template;

use crate::{
    prelude::*,
    types::{CurrentPage, Message},
};

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
async fn home(id: Identity) -> Result<HttpResponse, Error> {
    let s = Home {
        messages: Message::none(),
        page: CurrentPage::from("/", auth::user_or_guest(&id)?),
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
async fn overview(id: Identity) -> Result<HttpResponse, Error> {
    let s = Overview {
        messages: Message::none(),
        page: CurrentPage::from("/overview", auth::user(&id)?),
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
