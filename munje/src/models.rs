use anyhow::Result;
use async_trait::async_trait;
use sqlx::postgres::PgRow;

use crate::types::{Id, Pool};

pub struct UpsertResult<T> {
    pub record: T,
    pub created: bool,
}

#[async_trait]
pub trait Creatable {
    async fn next_id(sequence_name: &str, db: &Pool) -> Result<Id> {
        use sqlx::Row;

        let query = format!("select nextval('{}') id", sequence_name);
        sqlx::query(&query)
            .map(|row: PgRow| {
                let value = row.try_get("id")?;
                Ok(Id(value))
            })
            .fetch_one(db)
            .await?
    }
}
