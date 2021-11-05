use actix_web::{get, web, Error, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use askama::Template;

use crate::types::{CurrentPage, Message};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(signup);
}

fn page() -> CurrentPage {
    CurrentPage {
        path: "/users".to_string(),
    }
}

#[derive(Template)]
#[template(path = "users/signup.jinja")]
struct Signup<'a> {
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[get("/users/signup")]
async fn signup(messages: IncomingFlashMessages) -> Result<HttpResponse, Error> {
    let messages = Message::to_messages(&messages);
    let s = Signup {
        messages: &messages,
        page: page(),
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}
