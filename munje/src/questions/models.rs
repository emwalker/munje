use anyhow::Result;
use chrono::prelude::*;
use comrak::{markdown_to_html, ComrakOptions};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::types::Pool;

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct CreateQuestion {
    pub author_id: String,
    pub title: String,
    pub link: String,
    pub link_logo: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Question {
    pub id: String,
    pub author_id: String,
    pub title: String,
    pub text: String,
    pub link: Option<String>,
    pub link_logo: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Question {
    pub async fn find_all(db: &Pool) -> Result<Vec<Self>> {
        let questions = sqlx::query_as!(
            Self,
            r#"
            select id, author_id, title, text, link, link_logo, created_at, updated_at
                from questions
                order by created_at desc
            "#
        )
        .fetch_all(db)
        .await?;

        Ok(questions)
    }

    pub async fn find_by_id(id: String, db: &Pool) -> Result<Option<Self>> {
        let question = sqlx::query_as!(Self, "select * from questions where id = $1", id)
            .fetch_optional(db)
            .await?;
        Ok(question)
    }

    pub async fn create(question: CreateQuestion, db: &Pool) -> Result<Self> {
        let uuid = Uuid::new_v4().to_hyphenated().to_string();
        let created_at = Utc::now().to_rfc3339();

        let s = format!(
            "Complete the challenge at [this link]({}). When you're done, come back
             to this question and indicate whether you solved the problem.",
            question.link,
        );
        let text = Regex::new(r"\s+").unwrap().replace_all(&s, " ").to_string();

        sqlx::query!(
            "insert into questions
                (id, author_id, title, text, link, link_logo, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $7, $8)",
            uuid,
            question.author_id,
            question.title,
            text,
            question.link,
            question.link_logo,
            created_at,
            created_at
        )
        .execute(db)
        .await?;

        Ok(Self {
            id: uuid,
            author_id: question.author_id.to_string(),
            title: question.title,
            text,
            link: Some(question.link.to_string()),
            link_logo: question.link_logo,
            created_at: created_at.clone(),
            updated_at: created_at,
        })
    }

    pub fn markdown(&self) -> String {
        markdown_to_html(&self.text, &ComrakOptions::default())
    }
}
