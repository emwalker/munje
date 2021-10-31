use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages, Level};
use chrono;
use chrono_humanize::HumanTime;
use comrak::{markdown_to_html, ComrakOptions};
use envy;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::ops::{Add, Sub};

pub type Pool = PgPool;

pub struct AppState {
    pub db: Pool,
}

pub struct Message {
    pub content: String,
    pub level: String,
}

impl Message {
    pub fn to_messages<'a>(messages: &'a IncomingFlashMessages) -> Vec<Message> {
        messages.iter().map(|message| Self::new(message)).collect()
    }

    pub fn new(message: &FlashMessage) -> Self {
        Self {
            content: message.content().to_string(),
            level: Self::level_str(message),
        }
    }

    fn level_str(message: &FlashMessage) -> String {
        match message.level() {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Success => "success",
            Level::Warning => "warning",
            Level::Error => "danger",
        }
        .to_string()
    }
}

#[derive(Default)]
pub struct CurrentPage {
    pub path: String,
}

impl CurrentPage {
    pub fn at(&self, path: &str) -> bool {
        self.path == path
    }

    pub fn active(&self, path: &str) -> &'static str {
        if self.at(path) {
            "is-active"
        } else {
            ""
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize)]
pub struct DateTime(pub chrono::DateTime<chrono::Utc>);

impl DateTime {
    pub fn from(string: &str) -> Self {
        let dt = chrono::DateTime::parse_from_rfc3339(string)
            .map(|dt| chrono::DateTime::from(dt))
            .unwrap_or(chrono::Utc::now());
        Self(dt)
    }

    pub fn now() -> Self {
        Self(chrono::Utc::now())
    }

    pub fn to_chrono(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.clone()
    }

    pub fn humanize(&self) -> String {
        format!("{}", HumanTime::from(self.0))
    }
}

impl Add<chrono::Duration> for DateTime {
    type Output = Self;

    fn add(self, rhs: chrono::Duration) -> Self {
        Self(self.0 + rhs)
    }
}

impl Sub<DateTime> for DateTime {
    type Output = chrono::Duration;

    fn sub(self, rhs: DateTime) -> chrono::Duration {
        self.0 - rhs.0
    }
}

#[derive(Debug, Serialize)]
pub struct Markdown {
    text: String,
}

impl Markdown {
    pub fn from(text: String) -> Self {
        Self { text }
    }

    pub fn markdown(&self) -> String {
        markdown_to_html(&self.text, &ComrakOptions::default())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub session_key: String,
    pub rust_log: String,
}

impl Config {
    pub fn load() -> Result<Self, envy::Error> {
        let profile = if cfg!(test) {
            "test"
        } else if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        };

        dotenv::from_filename(format!(".env.{}.local", profile)).ok();
        dotenv::from_filename(".env.local").ok();
        dotenv::from_filename(format!(".env.{}", profile)).ok();
        dotenv::dotenv().ok();

        envy::from_env::<Self>()
    }
}
