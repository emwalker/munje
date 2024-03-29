use actix_web::{HttpResponse, ResponseError};
use argon2;
use askama::Template;
use std::{error, fmt};

use crate::prelude::*;

#[derive(Debug)]
pub enum Error {
    ActixWeb(actix_web::error::Error),
    Anyhow(anyhow::Error),
    Config(envy::Error),
    Database(sqlx::Error),
    FetchPageError(reqwest::Error),
    Generic(String),
    HashPasswordError(argon2::Error),
    InvalidPassword,
    Json(serde_json::error::Error),
    MigrationError(sqlx::migrate::MigrateError),
    ParseUrlError(url::ParseError),
    Unauthorized,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::ActixWeb(e) => Some(e),
            Self::Anyhow(e) => Some(e.root_cause()),
            Self::Config(e) => Some(e),
            Self::Database(e) => Some(e),
            Self::FetchPageError(e) => Some(e),
            Self::Generic(_) => None,
            Self::HashPasswordError(e) => Some(e),
            Self::InvalidPassword => None,
            Self::Json(e) => Some(e),
            Self::MigrationError(e) => Some(e),
            Self::ParseUrlError(e) => Some(e),
            Self::Unauthorized => None,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}

impl From<actix_web::Error> for Error {
    fn from(e: actix_web::Error) -> Self {
        Self::ActixWeb(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<argon2::Error> for Error {
    fn from(e: argon2::Error) -> Self {
        Self::HashPasswordError(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Self::Json(e)
    }
}

impl From<envy::Error> for Error {
    fn from(e: envy::Error) -> Self {
        Self::Config(e)
    }
}

impl From<sqlx::migrate::MigrateError> for Error {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        Self::MigrationError(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::ParseUrlError(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::FetchPageError(e)
    }
}

#[derive(Template)]
#[template(path = "not-found.jinja")]
struct NotFound {
    messages: Vec<Message>,
    page: CurrentPage,
}

// TODO: Set up a nice error page
impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Database(sqlx::Error::RowNotFound) => {
                let s = NotFound {
                    messages: Vec::new(),
                    page: CurrentPage::from("/", User::guest()),
                }
                .render()
                .unwrap();
                HttpResponse::NotFound()
                    .content_type("text/html; charset=utf-8")
                    .body(s)
            }

            Self::Unauthorized => HttpResponse::Unauthorized()
                .content_type("text/html; charset=utf-8")
                .body("You don't have the necessary privileges"),

            _ => HttpResponse::InternalServerError()
                .content_type("text/html; charset=utf-8")
                .body(format!("There was a problem: {:?}", self)),
        }
    }
}
