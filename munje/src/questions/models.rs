use anyhow::Result;
use chrono;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    models::Creatable,
    types::{DateTime, Markdown, Pool},
};

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct CreateQuestion {
    pub author_id: i64,
    pub title: String,
    pub link: String,
    pub link_logo: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct QuestionRow {
    pub author_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub external_id: String,
    pub id: i64,
    pub link_logo: Option<String>,
    pub link: Option<String>,
    pub text: String,
    pub title: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct Question {
    pub author_id: i64,
    pub created_at: DateTime,
    pub external_id: String,
    pub id: i64,
    pub link_logo: Option<String>,
    pub link: Option<String>,
    pub text: Markdown,
    pub title: String,
    pub updated_at: DateTime,
}

impl QuestionRow {
    pub fn to_question(&self) -> Question {
        Question {
            author_id: self.author_id,
            created_at: DateTime(self.created_at),
            external_id: self.external_id.clone(),
            id: self.id,
            link_logo: self.link_logo.clone(),
            link: self.link.clone(),
            text: Markdown::from(self.text.clone()),
            title: self.title.clone(),
            updated_at: DateTime(self.updated_at),
        }
    }
}

impl Question {
    pub async fn find_all(db: &Pool) -> Result<Vec<Self>> {
        let questions = sqlx::query_as!(
            QuestionRow,
            "select id, external_id, author_id, title, text, link, link_logo, created_at,
                updated_at
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

    pub async fn find(external_id: &str, db: &Pool) -> Result<Self> {
        let row = sqlx::query_as!(
            QuestionRow,
            "select * from questions where external_id = $1",
            external_id
        )
        .fetch_one(db)
        .await?;
        Ok(row.to_question())
    }

    pub async fn find_by_id(id: i64, db: &Pool) -> Result<Self> {
        let row = sqlx::query_as!(QuestionRow, "select * from questions where id = $1", id)
            .fetch_one(db)
            .await?;
        Ok(row.to_question())
    }

    pub async fn create(question: CreateQuestion, db: &Pool) -> Result<Self> {
        let s = format!(
            "Complete the challenge at [this link]({}). When you're done, come back
             to this question and indicate whether you solved the problem.",
            question.link,
        );
        let text = Regex::new(r"\s+").unwrap().replace_all(&s, " ").to_string();
        let id = Self::next_id("questions_id_seq", db).await?;

        let row = sqlx::query_as!(
            QuestionRow,
            "insert into questions
                (id, external_id, author_id, title, text, link, link_logo)
             values ($1, $2, $3, $4, $5, $6, $7)
             returning *",
            id.internal_id(),
            id.external_id(),
            question.author_id,
            question.title,
            text,
            question.link,
            question.link_logo,
        )
        .fetch_one(db)
        .await?;

        Ok(row.to_question())
    }
}

impl Creatable for Question {}
