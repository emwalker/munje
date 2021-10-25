use actix_web::{
    error, get, web,
    web::{Data, Path},
    Error, HttpResponse,
};
use actix_web_flash_messages::IncomingFlashMessages;
use askama::Template;
use derive_more::{Display, Error};

use crate::queues::Queue;
use crate::types::{AppState, CurrentPage, Message};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(show);
}

fn page() -> CurrentPage {
    CurrentPage {
        path: "/queues".to_string(),
    }
}

#[derive(Template)]
#[template(path = "queues/show.jinja")]
struct Show<'a> {
    queue: &'a Queue,
    messages: &'a Vec<Message>,
    page: CurrentPage,
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

#[get("/queues/{id}")]
async fn show(
    state: Data<AppState>,
    path: Path<String>,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let messages = &Message::to_messages(&messages);
    let result = Queue::find_by_id(&id, &state.db)
        .await
        .map_err(|error| ShowError {
            message: format!("Problem fetching question: {}", error),
        })?;

    let s = match result {
        Some(queue) => Show {
            queue: &queue,
            messages,
            page: page(),
        }
        .render()
        .unwrap(),
        None => NotFound {
            messages,
            page: page(),
        }
        .render()
        .unwrap(),
    };

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}
