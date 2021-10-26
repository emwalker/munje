use anyhow::Result;
use chrono::prelude::*;
use comrak::{markdown_to_html, ComrakOptions};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::questions::Question;
use crate::types::Pool;

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct CreateQueue {
    pub user_id: String,
    pub starting_question_id: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Queue {
    pub id: String,
    pub user_id: String,
    pub starting_question_id: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CreateResult {
    pub queue: Queue,
    pub created: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Answer {
    pub id: String,
    pub user_id: String,
    pub queue_id: String,
    pub question_id: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CreateAnswer {
    pub user_id: String,
    pub queue_id: String,
    pub question_id: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct NextAnswer {
    pub answer_id: String,
    pub question_text: String,
    pub question_link: Option<String>,
    pub question_id: String,
}

pub struct AnswerQuestion {
    pub answer_id: String,
    pub state: String,
}

trait Creatable {
    fn id_and_timestamp() -> (String, String) {
        let id = Uuid::new_v4().to_hyphenated().to_string();
        let timestamp = Utc::now().to_rfc3339();
        (id, timestamp)
    }
}

impl Creatable for Queue {}

impl Queue {
    pub async fn create(queue: CreateQueue, db: &Pool) -> Result<Self> {
        let (id, timestamp) = Self::id_and_timestamp();

        sqlx::query!(
            r#"
            insert into queues (id, user_id, starting_question_id, created_at, updated_at)
                values ($1, $2, $3, $4, $5)
            "#,
            id,
            queue.user_id,
            queue.starting_question_id,
            timestamp,
            timestamp,
        )
        .execute(db)
        .await?;

        Answer::create(
            CreateAnswer {
                user_id: queue.user_id.clone(),
                queue_id: id.clone(),
                question_id: queue.starting_question_id.clone(),
            },
            db,
        )
        .await?;

        Ok(Self {
            id,
            user_id: queue.user_id.clone(),
            starting_question_id: queue.starting_question_id.clone(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        })
    }

    pub async fn find_by_id(id: &str, db: &Pool) -> Result<Option<Self>> {
        let queue = sqlx::query_as!(Self, "select * from queues where id = $1", id)
            .fetch_optional(db)
            .await?;
        Ok(queue)
    }

    pub async fn find_or_create(queue: CreateQueue, db: &Pool) -> Result<CreateResult> {
        let result = sqlx::query_as!(
            Self,
            r#"
            select * from queues where user_id = $1 and starting_question_id = $2
            "#,
            queue.user_id,
            queue.starting_question_id
        )
        .fetch_optional(db)
        .await?;

        let create_result = match result {
            Some(queue) => CreateResult {
                queue: queue,
                created: false,
            },
            None => CreateResult {
                queue: Self::create(queue, db).await?,
                created: true,
            },
        };

        Ok(create_result)
    }

    pub async fn answers(&self, db: &Pool) -> Result<Vec<Answer>> {
        let answers = sqlx::query_as!(Answer, "select * from answers where queue_id = $1", self.id)
            .fetch_all(db)
            .await?;
        Ok(answers)
    }

    pub async fn next_answer(&self, db: &Pool) -> Result<Option<NextAnswer>> {
        let result = sqlx::query_as!(
            NextAnswer,
            r#"
            select a.id answer_id, a.question_id, q.text question_text, q.link question_link
            from answers a
            join questions q on a.question_id = q.id
            where a.queue_id = $1 and a.state = 'unstarted'
            limit 1
            "#,
            self.id
        )
        .fetch_optional(db)
        .await?;
        Ok(result)
    }

    pub async fn answer_question(&self, answer: AnswerQuestion, db: &Pool) -> Result<()> {
        Answer::update_state(answer.answer_id, answer.state, db).await?;

        // Add 1-5 questions to the queue

        Ok(())
    }
}

impl Creatable for Answer {}

impl Answer {
    async fn create(answer: CreateAnswer, db: &Pool) -> Result<Answer> {
        let (id, timestamp) = Self::id_and_timestamp();
        let state = "unstarted";

        sqlx::query!(
            r#"
            insert into answers
                (id, user_id, queue_id, question_id, state, created_at, updated_at)
                values ($1, $2, $3, $4, $5, $6, $7)
            "#,
            id,
            answer.user_id,
            answer.queue_id,
            answer.question_id,
            state,
            timestamp,
            timestamp,
        )
        .execute(db)
        .await?;

        Ok(Self {
            id,
            user_id: answer.user_id,
            queue_id: answer.queue_id,
            question_id: answer.question_id,
            state: state.to_string(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        })
    }

    pub async fn update_state(answer_id: String, state: String, db: &Pool) -> Result<()> {
        sqlx::query!(
            "update answers set state = $1 where id = $2",
            state,
            answer_id,
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn question(&self, db: &Pool) -> Result<Question> {
        let question = sqlx::query_as!(
            Question,
            "select * from questions where id = $1",
            self.question_id
        )
        .fetch_one(db)
        .await?;
        Ok(question)
    }
}

impl NextAnswer {
    pub fn markdown(&self) -> String {
        markdown_to_html(&self.question_text, &ComrakOptions::default())
    }
}
