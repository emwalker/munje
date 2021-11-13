use actix_web::{
    get, post, web,
    web::{Data, Form},
};
use anyhow::Result;
use askama::Template;

use crate::{
    error::Error,
    mutations::RegisterUser,
    prelude::*,
    request::Authentication,
    types::{AppState, CurrentPage, Message},
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
struct Signup {
    messages: Vec<Message>,
    form: RegisterUser,
    page: CurrentPage,
}

#[get("/users/signup")]
async fn signup() -> Result<HttpResponse, Error> {
    let s = Signup {
        messages: Message::none(),
        form: RegisterUser::default(),
        page: page(),
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[post("/users/signup")]
async fn create_user(
    form: Form<RegisterUser>,
    state: Data<AppState>,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    if request.is_authenticated()? {
        let user = request.user()?;
        return request.redirect(format!("/{}/queues", user.handle).as_ref());
    }

    let mut mutation = form.into_inner();
    if !mutation.validate() {
        let s = Signup {
            messages: Message::none(),
            form: mutation,
            page: page(),
        }
        .render()
        .unwrap();
        return Ok(HttpResponse::BadRequest().content_type("text/html").body(s));
    }

    mutation.call(&state.db).await?;
    request.redirect(format!("/{}/queues", mutation.handle.value).as_ref())
}
