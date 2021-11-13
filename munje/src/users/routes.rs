use actix_web::{
    error, get, post, web,
    web::{Data, Form},
    Error, HttpResponse,
};
use anyhow::Result;
use askama::Template;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::{
    routes::redirect_to,
    types::{AppState, CurrentPage, Message},
    users::User,
};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(signup).service(create_user);
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
async fn signup() -> Result<HttpResponse, Error> {
    let messages = Message::none();
    let s = Signup {
        messages: &messages,
        page: page(),
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Serialize, Deserialize)]
pub struct RegisterUserForm {
    pub handle: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Display, Error)]
struct RegistrationError {
    message: String,
}

impl error::ResponseError for RegistrationError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self.message);
        let s = Signup {
            messages: &vec![Message {
                content: self.message.clone(),
                level: "error".to_string(),
            }],
            page: page(),
        }
        .render()
        .unwrap();
        HttpResponse::BadRequest().content_type("text/html").body(s)
    }
}

#[post("/users/signup")]
async fn create_user(
    form: Form<RegisterUserForm>,
    state: Data<AppState>,
) -> Result<HttpResponse, Error> {
    let form = form.into_inner();
    User::create(form.handle.clone(), form.password, &state.db)
        .await
        .map_err(|error| RegistrationError {
            message: format!("There was a problem: {}", error),
        })?;
    Ok(redirect_to(format!("/{}/queues", form.handle)))
}
