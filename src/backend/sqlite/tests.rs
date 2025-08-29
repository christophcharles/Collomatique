use super::*;

async fn prepare_empty_db(pool: sqlx::SqlitePool) -> Store {
    Store::fill_empty_db(&pool).await.unwrap();
    Store { pool }
}

mod week_patterns;
