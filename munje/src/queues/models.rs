use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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

impl Queue {
    pub async fn create(queue: &CreateQueue, db: &Pool) -> Result<Self> {
        let uuid = Uuid::new_v4().to_hyphenated().to_string();
        let created_at = Utc::now().to_rfc3339();

        sqlx::query!(
            r#"
            insert into queues (id, user_id, starting_question_id, created_at, updated_at)
                values ($1, $2, $3, $4, $5)
            "#,
            uuid,
            queue.user_id,
            queue.starting_question_id,
            created_at,
            created_at
        )
        .execute(db)
        .await?;

        Ok(Self {
            id: uuid,
            user_id: queue.user_id.clone(),
            starting_question_id: queue.starting_question_id.clone(),
            created_at: created_at.clone(),
            updated_at: created_at,
        })
    }

    pub async fn find_by_id(id: &str, db: &Pool) -> Result<Option<Self>> {
        let queue = sqlx::query_as!(Self, "select * from queues where id = $1", id)
            .fetch_optional(db)
            .await?;
        Ok(queue)
    }

    pub async fn find_or_create(queue: &CreateQueue, db: &Pool) -> Result<CreateResult> {
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
}
