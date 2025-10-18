use sqlx::{PgPool, Result};
use super::models::{Conversation, Message};

pub struct ConversationRepository {
    pool: PgPool,
}

impl ConversationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_conversation(&self, title: &str, model: &str) -> Result<Conversation> {
        let conversation = sqlx::query_as::<_, Conversation>(
            r#"
            INSERT INTO conversations (title, model, created_at, updated_at)
            VALUES ($1, $2, NOW(), NOW())
            RETURNING id, title, model, created_at, updated_at
            "#,
        )
        .bind(title)
        .bind(model)
        .fetch_one(&self.pool)
        .await?;

        Ok(conversation)
    }

    pub async fn get_all_conversations(&self) -> Result<Vec<Conversation>> {
        let conversations = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT id, title, model, created_at, updated_at
            FROM conversations
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(conversations)
    }

    pub async fn get_conversation(&self, id: i32) -> Result<Option<Conversation>> {
        let conversation = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT id, title, model, created_at, updated_at
            FROM conversations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conversation)
    }

    pub async fn update_conversation_timestamp(&self, id: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE conversations
            SET updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_conversation(&self, id: i32) -> Result<()> {
        // Delete messages first (foreign key constraint)
        sqlx::query("DELETE FROM messages WHERE conversation_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        // Delete conversation
        sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

pub struct MessageRepository {
    pool: PgPool,
}

impl MessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_message(
        &self,
        conversation_id: i32,
        role: &str,
        content: &str,
        model: Option<&str>,
        latency_ms: Option<i32>,
        prompt_tokens: Option<i32>,
        completion_tokens: Option<i32>,
        tokens_per_second: Option<f64>,
        total_tokens: Option<i32>,
    ) -> Result<Message> {
        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (conversation_id, role, content, model, latency_ms, prompt_tokens, completion_tokens, tokens_per_second, total_tokens, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            RETURNING id, conversation_id, role, content, model, latency_ms, prompt_tokens, completion_tokens, tokens_per_second, total_tokens, created_at
            "#,
        )
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(model)
        .bind(latency_ms)
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .bind(tokens_per_second)
        .bind(total_tokens)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    pub async fn get_messages_by_conversation(&self, conversation_id: i32) -> Result<Vec<Message>> {
        let messages = sqlx::query_as::<_, Message>(
            r#"
            SELECT id, conversation_id, role, content, model, latency_ms, prompt_tokens, completion_tokens, tokens_per_second, total_tokens, created_at
            FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }
}
