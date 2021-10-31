use anyhow::Result;

use crate::queues::{Queue, QueueRow};
use crate::types::Pool;

pub struct User {
    pub id: String,
    pub handle: String,
}

impl User {
    pub async fn find_by_handle(handle: String, _db: &Pool) -> Result<Self> {
        let user = User {
            handle: handle.clone(),
            id: "21546b43-dcde-43b2-a251-e736194de0a0".to_string(),
        };
        Ok(user)
    }

    pub async fn queues(&self, db: &Pool) -> Result<Vec<Queue>> {
        let queues = sqlx::query_as!(QueueRow, "select * from queues where user_id = $1", self.id)
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.to_queue())
            .collect();

        Ok(queues)
    }
}
