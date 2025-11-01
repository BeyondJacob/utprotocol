pub mod models;
pub mod repository;
pub mod vector_models;
pub mod vector_repository;

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

    // Add network latency columns
    sqlx::query(
        r#"
        ALTER TABLE messages
        ADD COLUMN IF NOT EXISTS network_send_ms INTEGER,
        ADD COLUMN IF NOT EXISTS network_receive_ms INTEGER,
        ADD COLUMN IF NOT EXISTS network_total_ms INTEGER
        "#,
    )
    .execute(&pool)
    .await?;

    // Add UTP columns to conversations table
    sqlx::query(
        r#"
        ALTER TABLE conversations
        ADD COLUMN IF NOT EXISTS utp_enabled BOOLEAN NOT NULL DEFAULT FALSE
        "#,
    )
    .execute(&pool)
    .await?;

    // Add UTP metadata column to messages table
    sqlx::query(
        r#"
        ALTER TABLE messages
        ADD COLUMN IF NOT EXISTS utp_metadata JSONB
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

    // Create index for UTP metadata queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_utp_used
        ON messages ((utp_metadata->>'used_utp'))
        WHERE utp_metadata IS NOT NULL
        "#,
    )
    .execute(&pool)
    .await?;

    // Create agents table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id SERIAL PRIMARY KEY,
            label TEXT NOT NULL,
            agent_type TEXT NOT NULL CHECK (agent_type IN ('responder', 'analyzer', 'router', 'aggregator')),
            status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'running', 'completed', 'error')),
            model TEXT,
            utp_enabled BOOLEAN NOT NULL DEFAULT FALSE,
            config JSONB DEFAULT '{}',
            position_x DOUBLE PRECISION NOT NULL,
            position_y DOUBLE PRECISION NOT NULL,
            last_output TEXT,
            last_error TEXT,
            execution_time_ms INTEGER,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create agent flows table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agent_flows (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create agent edges table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agent_edges (
            id SERIAL PRIMARY KEY,
            flow_id INTEGER NOT NULL REFERENCES agent_flows(id) ON DELETE CASCADE,
            source_agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            target_agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            edge_data JSONB DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(flow_id, source_agent_id, target_agent_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create agent messages table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agent_messages (
            id SERIAL PRIMARY KEY,
            source_agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            target_agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            content TEXT NOT NULL,
            metadata JSONB DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create agent flow memberships table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agent_flow_memberships (
            id SERIAL PRIMARY KEY,
            flow_id INTEGER NOT NULL REFERENCES agent_flows(id) ON DELETE CASCADE,
            agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(flow_id, agent_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Add utp_enabled column to agents table if it doesn't exist (for existing databases)
    sqlx::query(
        r#"
        ALTER TABLE agents
        ADD COLUMN IF NOT EXISTS utp_enabled BOOLEAN NOT NULL DEFAULT FALSE
        "#,
    )
    .execute(&pool)
    .await?;

    // Create indexes for agent tables (must be separate queries for prepared statements)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agents_agent_type ON agents(agent_type)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_edges_flow_id ON agent_edges(flow_id)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_edges_source ON agent_edges(source_agent_id)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_edges_target ON agent_edges(target_agent_id)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_messages_source ON agent_messages(source_agent_id)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_messages_target ON agent_messages(target_agent_id)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_messages_created_at ON agent_messages(created_at)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_flow_memberships_flow ON agent_flow_memberships(flow_id)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_flow_memberships_agent ON agent_flow_memberships(agent_id)")
        .execute(&pool)
        .await?;

    // Run vector storage migration (for RAG comparison)
    // Note: Due to multiple commands, this needs to be run manually:
    // psql utprotocol < migrations/007_create_vector_storage.sql
    // TODO: Split into individual statements or use a migration runner
    tracing::info!("Vector tables migration skipped - run manually if needed");

    Ok(pool)
}
