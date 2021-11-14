use actix_web::{get, post, web, web::Form};
use anyhow::Result;
use askama::Template;

use crate::{
    error::Error,
    mutations::{AuthenticateUser, DestroyUserSession, RegisterUser},
    prelude::*,
    request::Authentication,
    types::{CurrentPage, Message},
};

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(signup)
        .service(create_user)
        .service(login)
        .service(create_session)
        .service(destroy_session);
}

#[derive(Template)]
#[template(path = "users/signup.jinja")]
struct Signup {
    messages: Vec<Message>,
    form: RegisterUser,
    page: CurrentPage,
}

#[get("/users/signup")]
async fn signup(request: HttpRequest) -> Result<HttpResponse, Error> {
    if request.is_authenticated()? {
        return request.redirect_home();
    }

    let s = Signup {
        messages: Message::none(),
        form: RegisterUser::default(),
        page: CurrentPage::from("/users", request.user()?),
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[post("/users/signup")]
async fn create_user(
    form: Form<RegisterUser>,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    if request.is_authenticated()? {
        return request.redirect_home();
    }

    let mut mutation = form.into_inner();
    if !mutation.validate() {
        let s = Signup {
            messages: Message::none(),
            form: mutation,
            page: CurrentPage::from("/users", request.user()?),
        }
        .render()
        .unwrap();
        return Ok(HttpResponse::BadRequest().content_type("text/html").body(s));
    }

    mutation.call(request.db()?).await?;
    request.redirect_home()
}

#[derive(Template)]
#[template(path = "users/login.jinja")]
struct Login {
    messages: Vec<Message>,
    form: AuthenticateUser,
    page: CurrentPage,
}

#[get("/users/login")]
async fn login(request: HttpRequest) -> Result<HttpResponse, Error> {
    if request.is_authenticated()? {
        return request.redirect_home();
    }

    let s = Login {
        messages: Message::none(),
        form: AuthenticateUser::default(),
        page: CurrentPage::from("/users", request.user()?),
    }
    .render()
    .unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[post("/users/login")]
async fn create_session(
    form: Form<AuthenticateUser>,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    if request.is_authenticated()? {
        return request.redirect_home();
    }

    let mut mutation = form.into_inner();
    if !mutation.validate() {
        let s = Login {
            messages: Message::none(),
            form: mutation,
            page: CurrentPage::from("/users", request.user()?),
        }
        .render()
        .unwrap();
        return Ok(HttpResponse::BadRequest().content_type("text/html").body(s));
    }

    let db = request.db()?;
    match mutation.call(&request, db).await {
        Ok(()) => request.redirect_home(),

        Err(Error::InvalidPassword) | Err(Error::Database(sqlx::Error::RowNotFound)) => {
            mutation
                .password
                .errors
                .push("Username or password is invalid".to_string());
            let s = Login {
                messages: Message::none(),
                form: mutation,
                page: CurrentPage::from("/users", request.user()?),
            }
            .render()
            .unwrap();
            return Ok(HttpResponse::BadRequest().content_type("text/html").body(s));
        }

        Err(error) => Err(error),
    }
}

#[post("/users/logout")]
async fn destroy_session(request: HttpRequest) -> Result<HttpResponse, Error> {
    DestroyUserSession {}.call(&request).await?;
    request.redirect("/")
}
