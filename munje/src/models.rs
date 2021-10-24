use crate::types::Pool;
use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct QuestionData {
    pub link: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Question {
    pub id: String,
    pub link: Option<String>,
    pub link_logo: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Question {
    pub async fn find_all(pool: &Pool) -> Result<Vec<Question>> {
        let records = sqlx::query!(
            r#"
            select id, link, link_logo, created_at, updated_at
            from questions
            order by created_at desc
            "#
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|record| Question {
            id: record.id,
            link: record.link,
            link_logo: record.link_logo,
            created_at: record.created_at,
            updated_at: record.updated_at
        })
        .collect();

        Ok(records)
    }

    pub async fn find_by_id(id: String, pool: &Pool) -> Result<Option<Question>> {
        let record = sqlx::query!(
            r#"
            select id, link, link_logo, created_at, updated_at
            from questions
            where id = $1
            "#,
            id
        )
        .fetch_optional(&*pool)
        .await?;

        Ok(record.map(|record| Question {
            id: record.id,
            link_logo: record.link_logo,
            link: record.link,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }))
    }

    pub async fn create(
        item: &QuestionData,
        link_logo: Option<String>,
        pool: &Pool,
    ) -> Result<Question> {
        let mut tx = pool.acquire().await?;

        let uuid = Uuid::new_v4().to_hyphenated().to_string();
        let created_at = Utc::now().to_rfc3339();
        sqlx::query!(
            r#"
            insert into questions (id, author_id, link, link_logo, created_at, updated_at)
                values ($1, $2, $3, $4, $5, $6)
            "#,
            uuid,
            "21546b43-dcde-43b2-a251-e736194de0a0",
            item.link,
            link_logo,
            created_at,
            created_at
        )
        .execute(&mut tx)
        .await?;

        Ok(Question {
            id: uuid,
            link: Some(item.link.to_string()),
            link_logo: link_logo,
            created_at: created_at.clone(),
            updated_at: created_at,
        })
    }

    // pub async fn update(id: i32, todo: TodoRequest, pool: &SqlitePool) -> Result<Option<Todo>> {
    //     let mut tx = pool.begin().await.unwrap();

    //     let n = sqlx::query!(
    //         r#"
    //       UPDATE todos
    //       SET description = $1, done = $2
    //       WHERE id = $3
    //       "#,
    //         todo.description,
    //         todo.done,
    //         id,
    //     )
    //     .execute(&mut tx)
    //     .await?;

    //     if n == 0 {
    //         return Ok(None);
    //     }

    //     // TODO: this can be replaced with RETURNING with sqlite v3.35+ and/or sqlx v0.5+
    //     let todo = sqlx::query!(
    //         r#"
    //       SELECT id, description, done
    //       FROM todos
    //       WHERE id = $1
    //       "#,
    //         id,
    //     )
    //     .fetch_one(&mut tx)
    //     .await
    //     .map(|record| Todo {
    //         id: record.id,
    //         description: record.description,
    //         done: record.done,
    //     })?;

    //     tx.commit().await.unwrap();
    //     Ok(Some(todo))
    // }

    // pub async fn delete(id: i32, pool: &SqlitePool) -> Result<u64> {
    //     let mut tx = pool.begin().await?;

    //     let n_deleted = sqlx::query!(
    //         r#"
    //       DELETE FROM todos
    //       WHERE id = $1
    //       "#,
    //         id,
    //     )
    //     .execute(&mut tx)
    //     .await?;

    //     tx.commit().await?;
    //     Ok(n_deleted)
    // }
}
