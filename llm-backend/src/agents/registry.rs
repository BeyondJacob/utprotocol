use sqlx::PgPool;
use crate::agents::models::*;

pub struct AgentRegistry {
    db_pool: PgPool,
}

impl AgentRegistry {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    // Agent CRUD operations
    pub async fn create_agent(&self, req: CreateAgentRequest) -> Result<Agent, sqlx::Error> {
        let agent = sqlx::query_as!(
            Agent,
            r#"
            INSERT INTO agents (label, agent_type, model, utp_enabled, config, position_x, position_y)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, label, agent_type as "agent_type: _", status, model, utp_enabled,
                      config, position_x, position_y, last_output, last_error, execution_time_ms,
                      created_at, updated_at
            "#,
            req.label,
            req.agent_type,
            req.model,
            req.utp_enabled,
            req.config.unwrap_or(serde_json::json!({})),
            req.position.x,
            req.position.y
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(agent)
    }

    pub async fn get_agent(&self, id: i32) -> Result<Option<Agent>, sqlx::Error> {
        let agent = sqlx::query_as!(
            Agent,
            r#"
            SELECT id, label, agent_type, status, model, utp_enabled, config,
                   position_x, position_y, last_output, last_error, execution_time_ms,
                   created_at, updated_at
            FROM agents
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(agent)
    }

    pub async fn get_all_agents(&self) -> Result<Vec<Agent>, sqlx::Error> {
        let agents = sqlx::query_as!(
            Agent,
            r#"
            SELECT id, label, agent_type, status, model, utp_enabled, config,
                   position_x, position_y, last_output, last_error, execution_time_ms,
                   created_at, updated_at
            FROM agents
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(agents)
    }

    pub async fn update_agent(&self, id: i32, req: UpdateAgentRequest) -> Result<Agent, sqlx::Error> {
        // Build dynamic update query
        let agent = sqlx::query_as!(
            Agent,
            r#"
            UPDATE agents
            SET label = COALESCE($2, label),
                status = COALESCE($3, status),
                model = COALESCE($4, model),
                utp_enabled = COALESCE($5, utp_enabled),
                config = COALESCE($6, config),
                position_x = COALESCE($7, position_x),
                position_y = COALESCE($8, position_y),
                last_output = COALESCE($9, last_output),
                last_error = COALESCE($10, last_error),
                execution_time_ms = COALESCE($11, execution_time_ms),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, label, agent_type, status, model, utp_enabled, config,
                      position_x, position_y, last_output, last_error, execution_time_ms,
                      created_at, updated_at
            "#,
            id,
            req.label,
            req.status,
            req.model,
            req.utp_enabled,
            req.config,
            req.position.as_ref().map(|p| p.x),
            req.position.as_ref().map(|p| p.y),
            req.last_output,
            req.last_error,
            req.execution_time_ms
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(agent)
    }

    pub async fn delete_agent(&self, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM agents WHERE id = $1", id)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    // Flow CRUD operations
    pub async fn create_flow(&self, req: CreateFlowRequest) -> Result<AgentFlow, sqlx::Error> {
        let flow = sqlx::query_as!(
            AgentFlow,
            r#"
            INSERT INTO agent_flows (name, description)
            VALUES ($1, $2)
            RETURNING id, name, description, created_at, updated_at
            "#,
            req.name,
            req.description
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(flow)
    }

    pub async fn get_flow(&self, id: i32) -> Result<Option<AgentFlow>, sqlx::Error> {
        let flow = sqlx::query_as!(
            AgentFlow,
            r#"
            SELECT id, name, description, created_at, updated_at
            FROM agent_flows
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(flow)
    }

    pub async fn get_all_flows(&self) -> Result<Vec<AgentFlow>, sqlx::Error> {
        let flows = sqlx::query_as!(
            AgentFlow,
            r#"
            SELECT id, name, description, created_at, updated_at
            FROM agent_flows
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(flows)
    }

    pub async fn delete_flow(&self, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM agent_flows WHERE id = $1", id)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    // Edge operations
    pub async fn create_edge(&self, req: CreateEdgeRequest) -> Result<AgentEdge, sqlx::Error> {
        let edge = sqlx::query_as!(
            AgentEdge,
            r#"
            INSERT INTO agent_edges (flow_id, source_agent_id, target_agent_id, edge_data)
            VALUES ($1, $2, $3, $4)
            RETURNING id, flow_id, source_agent_id, target_agent_id, edge_data, created_at
            "#,
            req.flow_id,
            req.source_agent_id,
            req.target_agent_id,
            req.edge_data.unwrap_or(serde_json::json!({}))
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(edge)
    }

    pub async fn get_flow_edges(&self, flow_id: i32) -> Result<Vec<AgentEdge>, sqlx::Error> {
        let edges = sqlx::query_as!(
            AgentEdge,
            r#"
            SELECT id, flow_id, source_agent_id, target_agent_id, edge_data, created_at
            FROM agent_edges
            WHERE flow_id = $1
            "#,
            flow_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(edges)
    }

    pub async fn get_flow_with_agents(&self, flow_id: i32) -> Result<Option<FlowWithAgents>, sqlx::Error> {
        let flow = self.get_flow(flow_id).await?;

        if let Some(flow) = flow {
            // Get all agents in this flow
            let agents = sqlx::query_as!(
                Agent,
                r#"
                SELECT a.id, a.label, a.agent_type, a.status, a.model, a.utp_enabled, a.config,
                       a.position_x, a.position_y, a.last_output, a.last_error, a.execution_time_ms,
                       a.created_at, a.updated_at
                FROM agents a
                INNER JOIN agent_flow_memberships afm ON a.id = afm.agent_id
                WHERE afm.flow_id = $1
                "#,
                flow_id
            )
            .fetch_all(&self.db_pool)
            .await?;

            let edges = self.get_flow_edges(flow_id).await?;

            Ok(Some(FlowWithAgents {
                flow,
                agents,
                edges,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn add_agent_to_flow(&self, flow_id: i32, agent_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO agent_flow_memberships (flow_id, agent_id)
            VALUES ($1, $2)
            ON CONFLICT (flow_id, agent_id) DO NOTHING
            "#,
            flow_id,
            agent_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    // Message operations
    #[allow(dead_code)]
    pub async fn create_message(
        &self,
        source_agent_id: i32,
        target_agent_id: i32,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<AgentMessage, sqlx::Error> {
        let message = sqlx::query_as!(
            AgentMessage,
            r#"
            INSERT INTO agent_messages (source_agent_id, target_agent_id, content, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING id, source_agent_id, target_agent_id, content, metadata, created_at
            "#,
            source_agent_id,
            target_agent_id,
            content,
            metadata.unwrap_or(serde_json::json!({}))
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(message)
    }

    #[allow(dead_code)]
    pub async fn get_messages_between_agents(
        &self,
        source_agent_id: i32,
        target_agent_id: i32,
    ) -> Result<Vec<AgentMessage>, sqlx::Error> {
        let messages = sqlx::query_as!(
            AgentMessage,
            r#"
            SELECT id, source_agent_id, target_agent_id, content, metadata, created_at
            FROM agent_messages
            WHERE source_agent_id = $1 AND target_agent_id = $2
            ORDER BY created_at ASC
            "#,
            source_agent_id,
            target_agent_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(messages)
    }
}
