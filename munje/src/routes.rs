mod questions;
mod testing;

use crate::types::{CurrentPage, Message};
use actix_web::{get, web, Error, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use anyhow::Result;
use askama::Template;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(home);
    questions::configure(cfg);
}

#[derive(Template)]
#[template(path = "home.jinja")]
struct Home<'a> {
    messages: &'a Vec<Message>,
    page: CurrentPage,
}

#[get("/")]
async fn home(messages: IncomingFlashMessages) -> Result<HttpResponse, Error> {
    let s = Home {
        messages: &Message::to_messages(&messages),
        page: CurrentPage {
            path: "/".to_string(),
        },
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::testing::{Runner, TestResult};

    #[actix_rt::test]
    async fn test_home() -> TestResult {
        let doc = Runner::new().await.get(home, "/").await?;
        assert_eq!("Munje", doc.select_text("p.title").unwrap());
        Ok(())
    }
}
