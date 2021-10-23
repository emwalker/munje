use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages, Level};
use scraper::{Html, Selector};
use sqlx::SqlitePool;

pub type Pool = SqlitePool;

pub struct AppState {
    pub pool: Pool,
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

#[derive(Clone)]
pub struct Document {
    inner: Html,
}

impl Document {
    pub fn from(html: &str) -> Self {
        Self {
            inner: Html::parse_document(html),
        }
    }

    pub fn select_text(&self, selector: &str) -> Option<String> {
        let sel = Selector::parse(selector).unwrap();
        Some(self.inner.select(&sel).next().unwrap().inner_html())
    }
}
