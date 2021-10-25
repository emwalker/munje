use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages, Level};
use sqlx::SqlitePool;

pub type Pool = SqlitePool;

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
