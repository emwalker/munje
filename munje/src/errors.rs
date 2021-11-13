use actix_web::{HttpResponse, ResponseError};
use argon2;
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    Anyhow(anyhow::Error),
    Database(sqlx::Error),
    HashPasswordError(argon2::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Anyhow(e) => Some(e.root_cause()),
            Error::Database(e) => Some(e),
            Error::HashPasswordError(e) => Some(e),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
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

// TODO: Set up a nice error page
impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError()
            .content_type("text/html; charset=utf-8")
            .body(format!("There was a problem: {:?}", self))
    }
}
