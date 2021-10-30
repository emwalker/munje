use anyhow::{bail, Error, Result};
use chrono;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::convert::TryFrom;
use uuid::Uuid;

use crate::questions::Question;
use crate::queues::{
    choosers,
    choosers::{ChoiceRow, Strategy},
};
use crate::types::{DateTime, Pool};

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

pub struct UpsertResult<T> {
    pub record: T,
    pub created: bool,
}

pub struct NextQuestion {
    pub question: Option<Question>,
    next_available_at: DateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Answer {
    pub consecutive_correct: Option<i64>,
    pub answered_at: Option<String>,
    pub created_at: String,
    pub id: String,
    pub question_id: String,
    pub queue_id: String,
    pub stage: Option<i64>,
    pub state: String,
    pub user_id: String,
}

pub struct QueueAnswer {
    pub question_id: String,
    pub queue_id: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WideAnswer {
    pub answer_id: String,
    pub answer_state: String,
    pub answer_answered_at: Option<String>,
    pub answer_stage: Option<i64>,
    pub answer_consecutive_correct: Option<i64>,
    pub question_title: String,
    pub question_text: String,
    pub question_link: Option<String>,
    pub question_id: String,
    pub queue_id: String,
}

pub struct AnswerQuestion {
    pub question_id: String,
    pub queue_id: String,
    pub state: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LastAnswer {
    pub answer_id: String,
    pub answer_stage: i64,
    pub answer_state: String,
    pub answer_answered_at: String,
    pub answer_consecutive_correct: i64,
    pub created_at: String,
    pub id: String,
    pub question_id: String,
    pub queue_id: String,
    pub updated_at: String,
    pub user_id: String,
}

pub struct UpsertLastAnswer {
    pub answer_answered_at: String,
    pub answer_consecutive_correct: i64,
    pub answer_id: String,
    pub answer_stage: i64,
    pub answer_state: String,
    pub question_id: String,
    pub queue_id: String,
    pub user_id: String,
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
            "insert into queues (id, user_id, starting_question_id, created_at, updated_at)
                values ($1, $2, $3, $4, $5)",
            id,
            queue.user_id,
            queue.starting_question_id,
            timestamp,
            timestamp,
        )
        .execute(db)
        .await?;

        Ok(Self {
            id,
            user_id: queue.user_id.clone(),
            starting_question_id: queue.starting_question_id.clone(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        })
    }

    pub async fn find_by_id(id: &str, db: &Pool) -> Result<Self> {
        let queue = sqlx::query_as!(Self, "select * from queues where id = $1", id)
            .fetch_one(db)
            .await?;
        Ok(queue)
    }

    pub async fn find_or_create(queue: CreateQueue, db: &Pool) -> Result<UpsertResult<Self>> {
        let result = sqlx::query_as!(
            Self,
            "select * from queues where user_id = $1 and starting_question_id = $2",
            queue.user_id,
            queue.starting_question_id,
        )
        .fetch_optional(db)
        .await?;

        let upsert_result = match result {
            Some(queue) => UpsertResult {
                record: queue,
                created: false,
            },
            None => UpsertResult {
                record: Self::create(queue, db).await?,
                created: true,
            },
        };

        Ok(upsert_result)
    }

    pub async fn answers(&self, db: &Pool) -> Result<Vec<Answer>> {
        let answers = sqlx::query_as!(Answer, "select * from answers where queue_id = $1", self.id)
            .fetch_all(db)
            .await?;
        Ok(answers)
    }

    pub async fn next_question(&self, db: &Pool) -> Result<NextQuestion> {
        info!("Selecting next question");

        let choices = sqlx::query_as!(
            ChoiceRow,
            "select q.id question_id, la.answer_stage, la.answer_state, la.answer_answered_at
             from questions q
             left join last_answers la on q.id = la.question_id
             where (la.user_id = $1 or la.user_id is null)
             limit 100",
            self.user_id,
        )
        .fetch_all(db)
        .await?;

        if choices.len() < 1 {
            bail!("No possible choices found for queue {:?}", self);
        }

        info!("Choosing from choices: {:?}", choices);
        let (next_choice, next_available_at) =
            choosers::SpacedRepetition::from_rows(choices, choosers::TimeUnit::Minutes)
                .next_question()?;
        // let (result, next_available_at) =
        //     choosers::Random::from_rows(choices).next_question()?;

        let next_question = match next_choice {
            Some(choice) => {
                let question = Question::find(choice.question_id.to_string(), db).await?;
                info!("Found a next question: {:?}", question);
                NextQuestion {
                    question: Some(question),
                    next_available_at,
                }
            }
            None => {
                info!("No question ready to work on");
                NextQuestion {
                    question: None,
                    next_available_at,
                }
            }
        };

        Ok(next_question)
    }

    pub async fn answer_question(
        &self,
        answer_question: AnswerQuestion,
        db: &Pool,
    ) -> Result<(), Error> {
        let (_id, timestamp) = Self::id_and_timestamp();

        let answer = Answer::create_from(&answer_question, db).await?;
        let last_answer = LastAnswer::find_or_create(&answer, db).await?.record;
        let consecutive_correct = match answer_question.state.as_ref() {
            "correct" => last_answer.answer_consecutive_correct + 1,
            _ => 0,
        };
        let base: i64 = 2;
        let stage = base.pow(u32::try_from(consecutive_correct)?);

        let answer = answer
            .finalize(
                answer_question.state.clone(),
                timestamp,
                consecutive_correct,
                stage,
                db,
            )
            .await?;
        last_answer.update(&answer, db).await?;

        Ok(())
    }

    pub async fn recent_answers(&self, db: &Pool) -> Result<Vec<WideAnswer>> {
        let answers = sqlx::query_as!(
            WideAnswer,
            "select
                a.id answer_id, a.state answer_state, a.question_id, q.title question_title,
                q.text question_text, q.link question_link, a.queue_id,
                a.answered_at answer_answered_at,
                a.consecutive_correct answer_consecutive_correct,
                a.stage answer_stage
             from answers a
             join questions q on a.question_id = q.id
             where a.queue_id = $1 order by a.created_at desc limit 6",
            self.id
        )
        .fetch_all(db)
        .await?;
        Ok(answers)
    }
}

impl Creatable for Answer {}

impl Answer {
    pub async fn find(answer_id: String, db: &Pool) -> Result<Self> {
        let answer = sqlx::query_as!(Self, "select * from answers where id = $1", answer_id)
            .fetch_one(db)
            .await?;
        Ok(answer)
    }

    pub async fn create_from(answer: &AnswerQuestion, db: &Pool) -> Result<Self> {
        let (id, timestamp) = Self::id_and_timestamp();

        sqlx::query!(
            "insert into answers
                (id, user_id, queue_id, question_id, state, created_at)
                values ($1, $2, $3, $4, $5, $6)",
            id,
            answer.user_id,
            answer.queue_id,
            answer.question_id,
            answer.state,
            timestamp,
        )
        .execute(db)
        .await?;

        Ok(Self {
            answered_at: None,
            consecutive_correct: None,
            created_at: timestamp.clone(),
            id,
            question_id: answer.question_id.clone(),
            queue_id: answer.queue_id.clone(),
            stage: None,
            state: answer.state.to_string(),
            user_id: answer.user_id.clone(),
        })
    }

    pub async fn finalize(
        &self,
        state: String,
        answered_at: String,
        consecutive_correct: i64,
        stage: i64,
        db: &Pool,
    ) -> Result<Answer> {
        sqlx::query!(
            "update answers set
                state = $1,
                answered_at = $2,
                consecutive_correct = $3,
                stage = $4
             where id = $5",
            state,
            answered_at,
            consecutive_correct,
            stage,
            self.id,
        )
        .execute(db)
        .await?;

        Ok(Self::find(self.id.clone(), db).await?)
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

impl NextQuestion {
    pub fn available_at(&self) -> String {
        self.next_available_at.humanize()
    }
}

impl WideAnswer {
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

    pub fn answered_at(&self) -> String {
        self.answer_answered_at
            .clone()
            .map(|s| DateTime::from(&s).humanize())
            .unwrap_or("now".to_string())
    }
}

impl Creatable for LastAnswer {}

impl LastAnswer {
    async fn find_or_create(answer: &Answer, db: &Pool) -> Result<UpsertResult<Self>, Error> {
        let result = sqlx::query_as!(
            Self,
            "select * from last_answers
                where user_id = $1
                  and question_id = $2
                  and queue_id = $3
             limit 1",
            answer.user_id,
            answer.question_id,
            answer.queue_id,
        )
        .fetch_optional(db)
        .await?;

        let upsert_result = match result {
            Some(last_answer) => UpsertResult {
                record: last_answer,
                created: false,
            },
            None => UpsertResult {
                record: Self::create_from(answer, db).await?,
                created: true,
            },
        };

        Ok(upsert_result)
    }

    async fn create_from(answer: &Answer, db: &Pool) -> Result<Self, Error> {
        let (id, timestamp) = Self::id_and_timestamp();

        let answered_at = answer.answered_at.clone().unwrap_or(timestamp.clone());
        let consecutive_correct = answer.consecutive_correct.unwrap_or(0);
        let stage = answer.stage.unwrap_or(0);

        sqlx::query!(
            "insert into last_answers
                (
                    answer_answered_at,
                    answer_id,
                    answer_stage,
                    answer_state,
                    answer_consecutive_correct,
                    created_at,
                    id,
                    question_id,
                    queue_id,
                    updated_at,
                    user_id
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            answered_at,
            answer.id,
            stage,
            answer.state,
            consecutive_correct,
            timestamp,
            id,
            answer.question_id,
            answer.queue_id,
            timestamp,
            answer.user_id,
        )
        .execute(db)
        .await?;

        Ok(Self {
            answer_id: answer.id.clone(),
            answer_stage: 0,
            answer_state: answer.state.clone(),
            answer_answered_at: answered_at,
            answer_consecutive_correct: 0,
            created_at: timestamp.clone(),
            id,
            question_id: answer.question_id.clone(),
            queue_id: answer.queue_id.clone(),
            updated_at: timestamp,
            user_id: answer.user_id.clone(),
        })
    }

    async fn update(&self, answer: &Answer, db: &Pool) -> Result<()> {
        let consecutive_correct = answer
            .consecutive_correct
            .clone()
            .unwrap_or(self.answer_consecutive_correct);
        let stage = answer.stage.clone().unwrap_or(self.answer_stage);
        let answered_at = answer
            .answered_at
            .clone()
            .unwrap_or(self.answer_answered_at.clone());

        sqlx::query!(
            "update last_answers set
                answer_id = $1,
                answer_consecutive_correct = $2,
                answer_stage = $3,
                answer_state = $4,
                answer_answered_at = $5
             where id = $6",
            answer.id,
            consecutive_correct,
            stage,
            answer.state,
            answered_at,
            self.id
        )
        .execute(db)
        .await?;
        Ok(())
    }
}
