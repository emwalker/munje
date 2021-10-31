use anyhow::Result;
use chrono::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::types::{DateTime, Markdown, Pool};

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct CreateQuestion {
    pub author_id: String,
    pub title: String,
    pub link: String,
    pub link_logo: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct QuestionRow {
    pub id: String,
    pub author_id: String,
    pub title: String,
    pub text: String,
    pub link: Option<String>,
    pub link_logo: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct Question {
    pub author_id: String,
    pub created_at: DateTime,
    pub id: String,
    pub link_logo: Option<String>,
    pub link: Option<String>,
    pub text: Markdown,
    pub title: String,
    pub updated_at: DateTime,
}

impl QuestionRow {
    pub fn to_question(&self) -> Question {
        Question {
            author_id: self.author_id.clone(),
            created_at: DateTime::from(&self.created_at),
            id: self.id.clone(),
            link_logo: self.link_logo.clone(),
            link: self.link.clone(),
            text: Markdown::from(self.text.clone()),
            title: self.title.clone(),
            updated_at: DateTime::from(&self.updated_at),
        }
    }
}

impl Question {
    pub async fn find_all(db: &Pool) -> Result<Vec<Self>> {
        let questions = sqlx::query_as!(
            QuestionRow,
            "select id, author_id, title, text, link, link_logo, created_at, updated_at
                from questions
                order by created_at desc",
        )
        .fetch_all(db)
        .await?
        .iter()
        .map(|row| row.to_question())
        .collect();

        Ok(questions)
    }

    pub async fn find(id: String, db: &Pool) -> Result<Self> {
        let row = sqlx::query_as!(QuestionRow, "select * from questions where id = $1", id)
            .fetch_one(db)
            .await?;
        Ok(row.to_question())
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
            author_id: question.author_id.to_string(),
            created_at: DateTime::from(&created_at),
            id: uuid,
            link_logo: question.link_logo,
            link: Some(question.link.to_string()),
            text: Markdown::from(text),
            title: question.title,
            updated_at: DateTime::from(&created_at),
        })
    }
}
