use argon2;
use chrono;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    mutations::{AuthenticateUser, RegisterUser},
    queues::{Queue, QueueRow},
    types::{DateTime, Pool},
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct User {
    pub created_at: DateTime,
    pub handle: String,
    pub id: i64,
    pub is_admin: bool,
    pub is_anonymous: bool,
    pub last_login: Option<DateTime>,
    #[serde(skip_serializing, skip_deserializing)]
    pub hashed_password: String,
    pub updated_at: DateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserRow {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub handle: String,
    pub hashed_password: String,
    pub id: i64,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
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

    fn verify(&self, password: &str) -> Result<bool, Error> {
        Ok(argon2::verify_encoded(&self.0, password.as_bytes())?)
    }
}

impl UserRow {
    fn to_user(&self) -> User {
        User {
            created_at: DateTime(self.created_at),
            handle: self.handle.clone(),
            hashed_password: self.hashed_password.clone(),
            id: self.id,
            is_admin: false,
            is_anonymous: false,
            last_login: self.last_login.map(|dt| DateTime(dt)),
            updated_at: DateTime(self.updated_at),
        }
    }
}

impl User {
    pub fn guest() -> Self {
        Self {
            is_anonymous: true,
            ..Self::default()
        }
    }

    pub fn is_authenticated(&self) -> bool {
        !self.is_anonymous
    }

    pub async fn find_by_handle(handle: &str, db: &Pool) -> Result<Self, Error> {
        let row = sqlx::query_as!(UserRow, "select * from users where handle = $1", handle)
            .fetch_one(db)
            .await?;
        Ok(row.to_user())
    }

    pub async fn register(mutation: &RegisterUser, db: &Pool) -> Result<Self, Error> {
        let password = mutation.password.value.clone();
        let hashed_password = Password(password.to_string()).to_hash().unwrap();

        let row = sqlx::query_as!(
            UserRow,
            "insert into users (handle, hashed_password, last_login) values ($1, $2, $3)
             returning *",
            mutation.handle.value.clone(),
            hashed_password,
            chrono::Utc::now(),
        )
        .fetch_one(db)
        .await?;

        Ok(row.to_user())
    }

    pub async fn update_last_login(id: i64, db: &Pool) -> Result<(), Error> {
        sqlx::query!("update users set last_login = now() where id = $1", id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn authenticate(mutation: &AuthenticateUser, db: &Pool) -> Result<User, Error> {
        let user = Self::find_by_handle(&mutation.handle.value, db).await?;
        let password = Password(user.hashed_password.clone());
        if !password.verify(&mutation.password.value)? {
            return Err(Error::InvalidPassword);
        }
        Ok(user)
    }

    pub async fn queues(&self, db: &Pool) -> Result<Vec<Queue>, Error> {
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
