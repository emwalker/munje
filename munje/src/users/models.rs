use anyhow::Result;

use crate::queues::{Queue, QueueRow};
use crate::types::Pool;

pub struct User {
    pub id: i64,
    pub handle: String,
}

impl User {
    pub async fn find_by_handle(handle: String, _db: &Pool) -> Result<Self> {
        let user = User {
            handle: handle.clone(),
            id: 1,
        };
        Ok(user)
    }

    pub async fn queues(&self, db: &Pool) -> Result<Vec<Queue>> {
        let queues = sqlx::query_as!(QueueRow, "select * from queues where user_id = $1", self.id,)
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.to_queue())
            .collect();

        Ok(queues)
    }
}
