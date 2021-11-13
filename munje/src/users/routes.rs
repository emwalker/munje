use actix_web::{
    get, post, web,
    web::{Data, Form},
    HttpResponse,
};
use anyhow::Result;
use askama::Template;

use crate::{
    errors::Error,
    routes::redirect_to,
    types::{AppState, CurrentPage, Message},
    users::mutations::RegisterUser,
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
) -> Result<HttpResponse, Error> {
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
    Ok(redirect_to(format!("/{}/queues", mutation.handle.value)))
}
