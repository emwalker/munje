use chrono;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    models::{Creatable, UpsertResult},
    prelude::*,
    questions::{Question, QuestionRow},
    queues::{
        choosers,
        choosers::{Choice, ChoiceRow, Strategy},
    },
    types::{DateTime, Markdown, Pool},
};

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct CreateQueue {
    pub description: String,
    pub starting_question_external_id: String,
    pub title: String,
    pub user_id: i64,
}

#[derive(Debug, FromRow)]
pub struct QueueRow {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
    pub external_id: String,
    pub id: i64,
    pub starting_question_id: i64,
    pub title: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_id: i64,
}

#[derive(Debug, Serialize)]
pub struct Queue {
    pub created_at: DateTime,
    pub description: Option<Markdown>,
    pub external_id: String,
    pub id: i64,
    pub starting_question_id: i64,
    pub title: String,
    pub updated_at: DateTime,
    pub user_id: i64,
}

pub struct NextQuestion {
    pub question: Option<Question>,
    next_available_at: DateTime,
}

#[derive(Debug, FromRow)]
pub struct AnswerRow {
    pub answered_at: chrono::DateTime<chrono::Utc>,
    pub consecutive_correct: i32,
    pub external_id: String,
    pub id: i64,
    pub question_id: i64,
    pub queue_id: i64,
    pub state: String,
    pub user_id: i64,
}

#[derive(Debug, Serialize)]
pub struct Answer {
    pub answered_at: DateTime,
    pub consecutive_correct: i32,
    pub external_id: String,
    pub id: i64,
    pub question_id: i64,
    pub queue_id: i64,
    pub state: String,
    pub user_id: i64,
}

pub struct QueueAnswer {
    pub question_id: String,
    pub queue_id: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WideAnswer {
    pub answer_id: i64,
    pub answer_state: String,
    pub answer_answered_at: chrono::DateTime<chrono::Utc>,
    pub answer_consecutive_correct: i32,
    pub question_title: String,
    pub question_text: String,
    pub question_link: Option<String>,
    pub question_id: i64,
    pub queue_id: i64,
}

#[derive(Debug)]
pub struct AnswerQuestion {
    pub question_external_id: String,
    pub queue_id: i64,
    pub state: String,
    pub user_id: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LastAnswer {
    pub answer_id: i64,
    pub answer_state: String,
    pub answer_answered_at: chrono::DateTime<chrono::Utc>,
    pub answer_consecutive_correct: i32,
    pub created_at: chrono::DateTime<Utc>,
    pub id: i64,
    pub question_id: i64,
    pub queue_id: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_id: i64,
}

pub struct UpsertLastAnswer {
    pub answer_answered_at: DateTime,
    pub answer_consecutive_correct: i32,
    pub answer_id: i64,
    pub answer_state: String,
    pub question_id: i64,
    pub queue_id: i64,
    pub user_id: i64,
}

impl AnswerRow {
    pub fn to_answer(&self) -> Answer {
        Answer {
            answered_at: DateTime(self.answered_at),
            consecutive_correct: self.consecutive_correct,
            external_id: self.external_id.clone(),
            id: self.id,
            question_id: self.question_id,
            queue_id: self.queue_id,
            state: self.state.clone(),
            user_id: self.user_id,
        }
    }
}

impl AnswerQuestion {
    fn consecutive_correct(&self) -> i32 {
        match self.state.as_ref() {
            "correct" => 1,
            _ => 0,
        }
    }
}

impl QueueRow {
    pub fn to_queue(&self) -> Queue {
        Queue {
            created_at: DateTime(self.created_at),
            description: self.description.clone().map(|s| Markdown::from(s)),
            external_id: self.external_id.to_string(),
            id: self.id.clone(),
            starting_question_id: self.starting_question_id.clone(),
            title: self.title.clone(),
            updated_at: DateTime(self.updated_at),
            user_id: self.user_id.clone(),
        }
    }
}

impl Creatable for Queue {}

impl Queue {
    pub async fn create(queue: CreateQueue, db: &Pool) -> Result<Self, Error> {
        let question = Question::find(&queue.starting_question_external_id, db).await?;
        let id = Self::next_id("queues_id_seq", db).await?;

        let title = "Algorithms and data structures";
        let description = "A queue of problems related to this question";

        let row = sqlx::query_as!(
            QueueRow,
            "insert into queues
                (id, external_id, user_id, title, description, starting_question_id)
                values ($1, $2, $3, $4, $5, $6)
                returning *",
            id.internal_id(),
            id.external_id(),
            queue.user_id,
            title,
            description,
            question.id,
        )
        .fetch_one(db)
        .await?;

        Ok(row.to_queue())
    }

    pub async fn find(external_id: &str, db: &Pool) -> Result<Self, Error> {
        let row = sqlx::query_as!(
            QueueRow,
            "select * from queues where external_id = $1",
            external_id
        )
        .fetch_one(db)
        .await?;
        Ok(row.to_queue())
    }

    pub async fn find_or_create(
        queue: CreateQueue,
        db: &Pool,
    ) -> Result<UpsertResult<Self>, Error> {
        let result = sqlx::query_as!(
            QueueRow,
            "select qq.*
             from queues qq
             join questions q on q.id = qq.starting_question_id
             where qq.user_id = $1 and q.external_id = $2",
            queue.user_id,
            queue.starting_question_external_id,
        )
        .fetch_optional(db)
        .await?;

        let upsert_result = match result {
            Some(row) => UpsertResult {
                record: row.to_queue(),
                created: false,
            },
            None => UpsertResult {
                record: Self::create(queue, db).await?,
                created: true,
            },
        };

        Ok(upsert_result)
    }

    pub async fn answers(&self, db: &Pool) -> Result<Vec<Answer>, Error> {
        let answers = sqlx::query_as!(
            AnswerRow,
            "select * from answers where queue_id = $1",
            self.id
        )
        .fetch_all(db)
        .await?
        .iter()
        .map(AnswerRow::to_answer)
        .collect();
        Ok(answers)
    }

    pub async fn next_question(&self, db: &Pool) -> Result<NextQuestion, Error> {
        info!("Selecting next question");

        let choices = sqlx::query_as!(
            ChoiceRow,
            r#"select
                q.id question_id,
                la.answer_state "answer_state?",
                la.answer_answered_at "answer_answered_at?",
                la.answer_consecutive_correct "answer_consecutive_correct?"
             from questions q
             left join last_answers la
                on  q.id = la.question_id
                and la.user_id = $1
                and la.queue_id = $2
             limit 100"#,
            self.user_id,
            self.id,
        )
        .fetch_all(db)
        .await?;

        if choices.len() < 1 {
            let error = Error::Generic(format!("No choices found for queue {:?}", self));
            return Err(error);
        }

        info!("Choosing from choices: {:?}", choices);
        let (next_choice, next_available_at) =
            choosers::SpacedRepetition::from_rows(choices, choosers::TimeUnit::Minutes)
                .next_question()?;
        // let (result, next_available_at) =
        //     choosers::Random::from_rows(choices).next_question()?;

        let next_question = match next_choice {
            Some(choice) => {
                let question = Question::find_by_id(choice.question_id, db).await?;
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
        let answer = Answer::create_from(&answer_question, db).await?;
        let last_answer = LastAnswer::find_or_create(&answer, db).await?.record;
        let consecutive_correct = match answer_question.state.as_ref() {
            "correct" => last_answer.answer_consecutive_correct + 1,
            _ => 0,
        };

        let answer = answer
            .finalize(
                answer_question.state.clone(),
                DateTime::now(),
                consecutive_correct,
                db,
            )
            .await?;
        last_answer.update(&answer, db).await?;

        Ok(())
    }

    pub async fn recent_answers(&self, db: &Pool) -> Result<Vec<WideAnswer>, Error> {
        let answers = sqlx::query_as!(
            WideAnswer,
            "select
                a.id answer_id,
                a.state answer_state,
                a.question_id,
                q.title question_title,
                q.text question_text,
                q.link question_link,
                a.queue_id,
                a.answered_at answer_answered_at,
                a.consecutive_correct answer_consecutive_correct
             from answers a
             join questions q on a.question_id = q.id
             where a.queue_id = $1 order by a.answered_at desc limit 6",
            self.id
        )
        .fetch_all(db)
        .await?;
        Ok(answers)
    }
}

impl Creatable for Answer {}

impl Answer {
    pub async fn find(external_id: String, db: &Pool) -> Result<Self, Error> {
        let row = sqlx::query_as!(
            AnswerRow,
            "select * from answers where external_id = $1",
            external_id,
        )
        .fetch_one(db)
        .await?;

        Ok(row.to_answer())
    }

    pub async fn create_from(answer: &AnswerQuestion, db: &Pool) -> Result<Self, Error> {
        let id = Self::next_id("last_answers_id_seq", db).await?;
        let question = Question::find(&answer.question_external_id, db).await?;

        let row = sqlx::query_as!(
            AnswerRow,
            "insert into answers
                (id, external_id, user_id, queue_id, question_id, state, answered_at,
                 consecutive_correct)
             values ($1, $2, $3, $4, $5, $6, $7, $8)
             returning *",
            id.internal_id(),
            id.external_id(),
            answer.user_id,
            answer.queue_id,
            question.id,
            answer.state,
            DateTime::now().to_chrono(),
            answer.consecutive_correct(),
        )
        .fetch_one(db)
        .await?;
        info!("Answer created: {:?}", id);

        Ok(row.to_answer())
    }

    pub async fn finalize(
        &self,
        state: String,
        answered_at: DateTime,
        consecutive_correct: i32,
        db: &Pool,
    ) -> Result<Self, Error> {
        let row = sqlx::query_as!(
            AnswerRow,
            "update answers set
                state = $1,
                answered_at = $2,
                consecutive_correct = $3
             where id = $4
             returning *",
            state,
            answered_at.to_chrono(),
            consecutive_correct,
            self.id,
        )
        .fetch_one(db)
        .await?;

        Ok(row.to_answer())
    }

    pub async fn question(&self, db: &Pool) -> Result<Question, Error> {
        let row = sqlx::query_as!(
            QuestionRow,
            "select * from questions where id = $1",
            self.question_id
        )
        .fetch_one(db)
        .await?;
        Ok(row.to_question())
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
        DateTime(self.answer_answered_at).humanize()
    }

    pub fn answer_stage(&self) -> i32 {
        Choice::stage_from(self.answer_consecutive_correct)
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
        let last_answer = sqlx::query_as!(
            Self,
            "insert into last_answers
                (
                    answer_answered_at,
                    answer_id,
                    answer_state,
                    answer_consecutive_correct,
                    question_id,
                    queue_id,
                    user_id
                )
                values ($1, $2, $3, $4, $5, $6, $7)
                returning *",
            answer.answered_at.to_chrono(),
            answer.id,
            answer.state,
            answer.consecutive_correct,
            answer.question_id,
            answer.queue_id,
            answer.user_id,
        )
        .fetch_one(db)
        .await?;

        Ok(last_answer)
    }

    async fn update(&self, answer: &Answer, db: &Pool) -> Result<(), Error> {
        sqlx::query!(
            "update last_answers set
                answer_id = $1,
                answer_consecutive_correct = $2,
                answer_state = $3,
                answer_answered_at = $4
             where id = $5",
            answer.id,
            answer.consecutive_correct,
            answer.state,
            answer.answered_at.to_chrono(),
            self.id
        )
        .execute(db)
        .await?;
        Ok(())
    }
}
