use actix_web::{HttpResponse, ResponseError};
use argon2;
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    ActixWeb(actix_web::error::Error),
    Anyhow(anyhow::Error),
    Database(sqlx::Error),
    Generic(String),
    HashPasswordError(argon2::Error),
    Json(serde_json::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ActixWeb(e) => Some(e),
            Error::Anyhow(e) => Some(e.root_cause()),
            Error::Database(e) => Some(e),
            Error::Generic(_) => None,
            Error::HashPasswordError(e) => Some(e),
            Error::Json(e) => Some(e),
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

// TODO: Set up a nice error page
impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError()
            .content_type("text/html; charset=utf-8")
            .body(format!("There was a problem: {:?}", self))
    }
}
