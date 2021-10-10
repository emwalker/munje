use sqlx::SqlitePool;

pub type Pool = SqlitePool;

pub struct AppState {
    pub pool: Pool,
}
