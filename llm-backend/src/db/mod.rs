pub mod models;
pub mod repository;

use sqlx::{PgPool, Result};

pub async fn init_db(database_url: &str) -> Result<PgPool> {
    let pool = PgPool::connect(database_url).await?;

    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            model TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id SERIAL PRIMARY KEY,
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            latency_ms INTEGER,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Add new columns if they don't exist (for existing databases)
    sqlx::query(
        r#"
        ALTER TABLE messages
        ADD COLUMN IF NOT EXISTS tokens_per_second DOUBLE PRECISION,
        ADD COLUMN IF NOT EXISTS total_tokens INTEGER,
        ADD COLUMN IF NOT EXISTS model TEXT,
        ADD COLUMN IF NOT EXISTS prompt_tokens INTEGER,
        ADD COLUMN IF NOT EXISTS completion_tokens INTEGER
        "#,
    )
    .execute(&pool)
    .await?;

    // Create index for faster queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_conversation_id
        ON messages(conversation_id)
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
