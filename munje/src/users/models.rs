use anyhow::Result;
use argon2;
use chrono;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    models::UpsertResult,
    mutations::RegisterUser,
    queues::{Queue, QueueRow},
    types::{DateTime, Pool},
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct User {
    pub id: i64,
    pub handle: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserRow {
    pub id: i64,
    pub handle: String,
    pub hashed_password: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

struct Password(String);

#[derive(Debug, Display, Error)]
struct HashPasswordError {
    details: String,
}

impl Password {
    fn to_hash(&self) -> Result<String, Error> {
        use rand::Rng;
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = argon2::Config::default();
        let hash = argon2::hash_encoded(self.0.as_bytes(), &salt, &config)?;
        Ok(hash)
    }
}

impl UserRow {
    fn to_user(&self) -> User {
        User {
            id: self.id,
            handle: self.handle.clone(),
            created_at: DateTime(self.created_at),
            updated_at: DateTime(self.updated_at),
        }
    }
}

impl User {
    pub fn guest() -> Self {
        Self::default()
    }

    pub fn is_authenticated(&self) -> bool {
        self.id != 0
    }

    pub async fn find_by_handle(handle: String, _db: &Pool) -> Result<Self> {
        let user = User {
            handle: handle.clone(),
            id: 1,
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
        };
        Ok(user)
    }

    pub async fn register(form: &RegisterUser, db: &Pool) -> Result<UpsertResult<Self>> {
        let password = form.password.value.clone();
        let hashed_password = Password(password.to_string()).to_hash().unwrap();

        let row = sqlx::query_as!(
            UserRow,
            "insert into users (handle, hashed_password) values ($1, $2)
             returning *",
            form.handle.value.clone(),
            hashed_password,
        )
        .fetch_one(db)
        .await?;

        let result = UpsertResult {
            record: row.to_user(),
            created: true,
        };
        Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_password() {
        let pass = Password("keyboard cat".to_string());
        assert_eq!(116, pass.to_hash().unwrap().len());
    }
}
