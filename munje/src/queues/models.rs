use anyhow::Result;
use chrono::{prelude::*, DateTime};
use comrak::{markdown_to_html, ComrakOptions};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use timeago::Formatter;
use uuid::Uuid;

use crate::questions::Question;
use crate::queues::{
    choosers,
    choosers::{Choice, Strategy},
};
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
    pub answered_at: Option<String>,
}

pub struct CreateAnswer {
    pub user_id: String,
    pub queue_id: String,
    pub question_id: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WideAnswer {
    pub answer_id: String,
    pub answer_state: String,
    pub answer_answered_at: Option<String>,
    pub question_title: String,
    pub question_text: String,
    pub question_link: Option<String>,
    pub question_id: String,
    pub queue_id: String,
}

pub struct AnswerQuestion {
    pub user_id: String,
    pub answer_id: String,
    pub queue_id: String,
    pub state: String,
}

pub struct LastAnswer {
    pub user_id: String,
    pub question_id: String,
    pub queue_id: String,
    pub answer_id: String,
    pub answer_state: String,
    pub answered_at: String,
    pub consecutive_correct_answers: u16,
    pub answer_stage: u32,
    pub created_at: String,
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

    pub async fn next_answer(&self, db: &Pool) -> Result<Option<WideAnswer>> {
        let result = sqlx::query_as!(
            WideAnswer,
            r#"
            select
                a.id answer_id, a.state answer_state, a.question_id, q.title question_title,
                q.text question_text, q.link question_link,
                a.queue_id, a.answered_at answer_answered_at
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
        Answer::update_state(answer.answer_id.clone(), answer.state, db).await?;
        let answer = Answer::find_by_id(answer.answer_id, db)
            .await?
            .expect("expected an answer");

        let possible_choices = sqlx::query_as!(
            Choice,
            "select distinct q.id question_id, a.state, a.answered_at
             from questions q
             left join answers a on q.id = a.question_id
             where q.id <> $1
             limit 20",
            answer.question_id,
        )
        .fetch_all(db)
        .await?;

        for choice in choosers::Random::new(possible_choices).take(1) {
            Answer::create(
                CreateAnswer {
                    user_id: answer.user_id.clone(),
                    queue_id: answer.queue_id.clone(),
                    question_id: choice.question_id.to_string(),
                },
                db,
            )
            .await?;
        }

        Ok(())
    }
}

impl Creatable for Answer {}

impl Answer {
    pub async fn find_by_id(id: String, db: &Pool) -> Result<Option<Self>> {
        let answer = sqlx::query_as!(Self, "select * from answers where id = $1", id)
            .fetch_optional(db)
            .await?;
        Ok(answer)
    }

    async fn create(answer: CreateAnswer, db: &Pool) -> Result<Answer> {
        let (id, timestamp) = Self::id_and_timestamp();
        let state = "unstarted";

        sqlx::query!(
            r#"
            insert into answers
                (id, user_id, queue_id, question_id, state, created_at)
                values ($1, $2, $3, $4, $5, $6)
            "#,
            id,
            answer.user_id,
            answer.queue_id,
            answer.question_id,
            state,
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
            answered_at: None,
        })
    }

    pub async fn update_state(answer_id: String, state: String, db: &Pool) -> Result<()> {
        let (_id, timestamp) = Self::id_and_timestamp();
        sqlx::query!(
            "update answers set state = $1, answered_at = $2 where id = $3",
            state,
            timestamp,
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

impl WideAnswer {
    pub fn markdown(&self) -> String {
        markdown_to_html(&self.question_text, &ComrakOptions::default())
    }

    pub async fn recent_answers(&self, db: &Pool) -> Result<Vec<WideAnswer>> {
        let answers = sqlx::query_as!(
            WideAnswer,
            "select
                a.id answer_id, a.state answer_state, a.question_id, q.title question_title,
                q.text question_text, q.link question_link, a.queue_id,
                a.answered_at answer_answered_at
             from answers a
             join questions q on a.question_id = q.id
             where a.queue_id = $1 order by a.created_at desc limit 6",
            self.queue_id
        )
        .fetch_all(db)
        .await?;
        Ok(answers)
    }

    pub fn tag_class(&self) -> String {
        match self.answer_state.as_ref() {
            "unsure" => "is-info",
            "incorrect" => "is-danger",
            "correct" => "is-success",
            _ => "",
        }
        .to_string()
    }

    pub fn tag_text(&self) -> String {
        match self.answer_state.as_ref() {
            "unsure" => "Too hard",
            "incorrect" => "Incorrect",
            "correct" => "Correct",
            "unstarted" => "Not answered",
            _ => "",
        }
        .to_string()
    }

    pub fn timeago(&self) -> Result<String> {
        let s = match &self.answer_answered_at {
            Some(answered_at) => {
                let formatter = Formatter::new();
                let dt1 = DateTime::parse_from_rfc3339(&answered_at)?;
                let dt2 = Utc::now();
                let duration = dt2.signed_duration_since(dt1).to_std()?;
                formatter.convert(duration)
            }
            None => "coming up".to_string(),
        };
        Ok(s)
    }
}
