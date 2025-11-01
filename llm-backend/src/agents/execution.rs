use std::sync::Arc;
use std::time::Instant;
use crate::agents::models::*;
use crate::agents::registry::AgentRegistry;
use crate::providers::{ModelProvider, GenerateResponse};

pub struct AgentExecutor {
    registry: Arc<AgentRegistry>,
}

impl AgentExecutor {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }

    /// Execute an agent with the given input
    pub async fn execute_agent(
        &self,
        agent_id: i32,
        input: String,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<ExecuteAgentResponse, String> {
        // Get agent details
        let agent = self
            .registry
            .get_agent(agent_id)
            .await
            .map_err(|e| format!("Failed to get agent: {}", e))?
            .ok_or_else(|| format!("Agent {} not found", agent_id))?;

        // Update agent status to running
        self.registry
            .update_agent(
                agent_id,
                UpdateAgentRequest {
                    status: Some(AgentStatus::Running.as_str().to_string()),
                    label: None,
                    model: None,
                    utp_enabled: None,
                    config: None,
                    position: None,
                    last_output: None,
                    last_error: None,
                    execution_time_ms: None,
                },
            )
            .await
            .map_err(|e| format!("Failed to update agent status: {}", e))?;

        let start = Instant::now();

        // Execute based on agent type and capture the full provider response
        let result = match AgentType::from_str(&agent.agent_type) {
            Ok(AgentType::Input) => {
                self.execute_input(&agent, &input, provider.clone()).await
            }
            Ok(AgentType::Responder) => {
                self.execute_responder(&agent, &input, provider.clone()).await
            }
            Ok(AgentType::Analyzer) => {
                self.execute_analyzer(&agent, &input, provider).await
            }
            Ok(AgentType::Router) => {
                self.execute_router(&agent, &input, provider).await
            }
            Ok(AgentType::Aggregator) => {
                self.execute_aggregator(&agent, &input, provider).await
            }
            Err(e) => Err(e),
        };

        let execution_time_ms = start.elapsed().as_millis() as i32;

        // Update agent with result
        match result {
            Ok(generate_response) => {
                let output = generate_response.content.clone();

                self.registry
                    .update_agent(
                        agent_id,
                        UpdateAgentRequest {
                            status: Some(AgentStatus::Completed.as_str().to_string()),
                            last_output: Some(output.clone()),
                            last_error: None,
                            execution_time_ms: Some(execution_time_ms),
                            label: None,
                            model: None,
                            utp_enabled: None,
                            config: None,
                            position: None,
                        },
                    )
                    .await
                    .map_err(|e| format!("Failed to update agent: {}", e))?;

                Ok(ExecuteAgentResponse {
                    agent_id,
                    output,
                    status: AgentStatus::Completed.as_str().to_string(),
                    execution_time_ms,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "agent_type": agent.agent_type,
                        "model": agent.model,
                        "prompt_tokens": generate_response.prompt_tokens,
                        "completion_tokens": generate_response.completion_tokens,
                        "total_tokens": generate_response.total_tokens,
                    })),
                })
            }
            Err(error) => {
                self.registry
                    .update_agent(
                        agent_id,
                        UpdateAgentRequest {
                            status: Some(AgentStatus::Error.as_str().to_string()),
                            last_error: Some(error.clone()),
                            execution_time_ms: Some(execution_time_ms),
                            label: None,
                            model: None,
                            utp_enabled: None,
                            config: None,
                            position: None,
                            last_output: None,
                        },
                    )
                    .await
                    .map_err(|e| format!("Failed to update agent: {}", e))?;

                Ok(ExecuteAgentResponse {
                    agent_id,
                    output: String::new(),
                    status: AgentStatus::Error.as_str().to_string(),
                    execution_time_ms,
                    error: Some(error),
                    metadata: Some(serde_json::json!({
                        "agent_type": agent.agent_type,
                        "model": agent.model,
                    })),
                })
            }
        }
    }

    /// Execute an input agent (receives initial user input and passes it through)
    async fn execute_input(
        &self,
        agent: &Agent,
        input: &str,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<GenerateResponse, String> {
        let model = agent
            .model
            .as_ref()
            .ok_or("Agent has no model configured")?;

        // Extract custom instructions from config if available
        let custom_instructions = agent
            .config
            .get("instructions")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let system_prompt = if !custom_instructions.is_empty() {
            format!(
                "You are an Input Agent - the starting point of an agentic workflow that receives user input.\n\n\
                Your role:\n\
                - Receive and understand user input\n\
                - Extract key information and intent\n\
                - Format the input appropriately for downstream agents\n\
                - Provide clear, structured output that other agents can process\n\n\
                Custom Instructions:\n{}\n\n\
                User Input: {}",
                custom_instructions, input
            )
        } else {
            format!(
                "You are an Input Agent - the starting point of an agentic workflow that receives user input.\n\n\
                Your role:\n\
                - Receive and understand user input\n\
                - Extract key information and intent\n\
                - Format the input appropriately for downstream agents\n\
                - Provide clear, structured output that other agents can process\n\n\
                User Input: {}",
                input
            )
        };

        let response = provider
            .generate(model, &system_prompt)
            .await
            .map_err(|e| format!("Model generation failed: {}", e))?;

        Ok(response)
    }

    /// Execute a responder agent (conversational Q&A)
    async fn execute_responder(
        &self,
        agent: &Agent,
        input: &str,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<GenerateResponse, String> {
        let model = agent
            .model
            .as_ref()
            .ok_or("Agent has no model configured")?;

        // Extract custom instructions from config if available
        let custom_instructions = agent
            .config
            .get("instructions")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let system_prompt = if !custom_instructions.is_empty() {
            format!(
                "You are a Responder Agent - a conversational assistant designed to provide clear, helpful, and direct answers to user questions.\n\n\
                Your role:\n\
                - Provide accurate and relevant responses to questions\n\
                - Maintain context from previous interactions when available\n\
                - Be concise yet comprehensive in your answers\n\
                - Ask clarifying questions if the request is ambiguous\n\
                - Acknowledge when you don't have enough information\n\n\
                Custom Instructions:\n{}\n\n\
                User Request: {}",
                custom_instructions, input
            )
        } else {
            format!(
                "You are a Responder Agent - a conversational assistant designed to provide clear, helpful, and direct answers to user questions.\n\n\
                Your role:\n\
                - Provide accurate and relevant responses to questions\n\
                - Maintain context from previous interactions when available\n\
                - Be concise yet comprehensive in your answers\n\
                - Ask clarifying questions if the request is ambiguous\n\
                - Acknowledge when you don't have enough information\n\n\
                User Request: {}",
                input
            )
        };

        let response = provider
            .generate(model, &system_prompt)
            .await
            .map_err(|e| format!("Model generation failed: {}", e))?;

        Ok(response)
    }

    /// Execute an analyzer agent (deep analysis and insight extraction)
    async fn execute_analyzer(
        &self,
        agent: &Agent,
        input: &str,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<GenerateResponse, String> {
        let model = agent
            .model
            .as_ref()
            .ok_or("Agent has no model configured")?;

        // Extract custom instructions from config if available
        let custom_instructions = agent
            .config
            .get("instructions")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let analysis_prompt = if !custom_instructions.is_empty() {
            format!(
                "You are an Analyzer Agent - a specialized assistant designed to perform deep analysis and extract actionable insights.\n\n\
                Your role:\n\
                - Identify patterns, trends, and key themes in the provided information\n\
                - Extract the most important insights and highlight critical points\n\
                - Provide structured analysis with clear reasoning\n\
                - Offer recommendations based on your analysis when appropriate\n\
                - Consider multiple perspectives and potential implications\n\n\
                Analysis Framework:\n\
                1. Summary: Brief overview of what you're analyzing\n\
                2. Key Findings: Main points and patterns discovered\n\
                3. Insights: Deeper understanding and implications\n\
                4. Recommendations: Actionable next steps (if applicable)\n\n\
                Custom Instructions:\n{}\n\n\
                Content to Analyze:\n{}",
                custom_instructions, input
            )
        } else {
            format!(
                "You are an Analyzer Agent - a specialized assistant designed to perform deep analysis and extract actionable insights.\n\n\
                Your role:\n\
                - Identify patterns, trends, and key themes in the provided information\n\
                - Extract the most important insights and highlight critical points\n\
                - Provide structured analysis with clear reasoning\n\
                - Offer recommendations based on your analysis when appropriate\n\
                - Consider multiple perspectives and potential implications\n\n\
                Analysis Framework:\n\
                1. Summary: Brief overview of what you're analyzing\n\
                2. Key Findings: Main points and patterns discovered\n\
                3. Insights: Deeper understanding and implications\n\
                4. Recommendations: Actionable next steps (if applicable)\n\n\
                Content to Analyze:\n{}",
                input
            )
        };

        let response = provider
            .generate(model, &analysis_prompt)
            .await
            .map_err(|e| format!("Model generation failed: {}", e))?;

        Ok(response)
    }

    /// Execute a router agent (intelligent request routing and categorization)
    async fn execute_router(
        &self,
        agent: &Agent,
        input: &str,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<GenerateResponse, String> {
        let model = agent
            .model
            .as_ref()
            .ok_or("Agent has no model configured")?;

        // Extract custom instructions from config if available
        let custom_instructions = agent
            .config
            .get("instructions")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let routing_prompt = if !custom_instructions.is_empty() {
            format!(
                "You are a Router Agent - a decision-making assistant that analyzes requests and determines the most appropriate routing path.\n\n\
                Your role:\n\
                - Analyze incoming requests to understand intent, complexity, and domain\n\
                - Categorize requests by type (e.g., question, task, analysis, synthesis)\n\
                - Determine which agent type(s) would best handle the request\n\
                - Consider the capabilities and strengths of different agent types\n\
                - Provide clear reasoning for your routing decision\n\n\
                Available Agent Types:\n\
                - Responder: Best for direct questions and conversational interactions\n\
                - Analyzer: Best for deep analysis, pattern recognition, and insight extraction\n\
                - Aggregator: Best for combining multiple inputs or synthesizing information\n\
                - Router: Best for complex multi-step requests that need orchestration\n\n\
                Output Format:\n\
                Target Agent: [agent type]\n\
                Reasoning: [why this agent is the best choice]\n\
                Context to Pass: [key information to provide to the target agent]\n\
                Priority: [low/medium/high]\n\n\
                Custom Instructions:\n{}\n\n\
                Request to Route:\n{}",
                custom_instructions, input
            )
        } else {
            format!(
                "You are a Router Agent - a decision-making assistant that analyzes requests and determines the most appropriate routing path.\n\n\
                Your role:\n\
                - Analyze incoming requests to understand intent, complexity, and domain\n\
                - Categorize requests by type (e.g., question, task, analysis, synthesis)\n\
                - Determine which agent type(s) would best handle the request\n\
                - Consider the capabilities and strengths of different agent types\n\
                - Provide clear reasoning for your routing decision\n\n\
                Available Agent Types:\n\
                - Responder: Best for direct questions and conversational interactions\n\
                - Analyzer: Best for deep analysis, pattern recognition, and insight extraction\n\
                - Aggregator: Best for combining multiple inputs or synthesizing information\n\
                - Router: Best for complex multi-step requests that need orchestration\n\n\
                Output Format:\n\
                Target Agent: [agent type]\n\
                Reasoning: [why this agent is the best choice]\n\
                Context to Pass: [key information to provide to the target agent]\n\
                Priority: [low/medium/high]\n\n\
                Request to Route:\n{}",
                input
            )
        };

        let response = provider
            .generate(model, &routing_prompt)
            .await
            .map_err(|e| format!("Model generation failed: {}", e))?;

        Ok(response)
    }

    /// Execute an aggregator agent (synthesis and information combination)
    async fn execute_aggregator(
        &self,
        agent: &Agent,
        input: &str,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<GenerateResponse, String> {
        let model = agent
            .model
            .as_ref()
            .ok_or("Agent has no model configured")?;

        // Extract custom instructions from config if available
        let custom_instructions = agent
            .config
            .get("instructions")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let aggregation_prompt = if !custom_instructions.is_empty() {
            format!(
                "You are an Aggregator Agent - a synthesis specialist that combines multiple inputs into coherent, unified outputs.\n\n\
                Your role:\n\
                - Synthesize information from multiple sources or perspectives\n\
                - Identify common themes, agreements, and contradictions\n\
                - Resolve conflicts between different inputs when possible\n\
                - Create a unified narrative that preserves important details\n\
                - Attribute information to sources when relevant\n\
                - Highlight areas where inputs disagree or are incomplete\n\n\
                Synthesis Framework:\n\
                1. Overview: High-level summary of all inputs\n\
                2. Common Themes: Points of agreement and shared insights\n\
                3. Key Differences: Contradictions or varying perspectives\n\
                4. Unified Conclusion: Synthesized understanding incorporating all inputs\n\
                5. Confidence Level: How well the inputs can be reconciled\n\
                6. Sources: Attribution of key points to their origins\n\n\
                Custom Instructions:\n{}\n\n\
                Information to Aggregate:\n{}",
                custom_instructions, input
            )
        } else {
            format!(
                "You are an Aggregator Agent - a synthesis specialist that combines multiple inputs into coherent, unified outputs.\n\n\
                Your role:\n\
                - Synthesize information from multiple sources or perspectives\n\
                - Identify common themes, agreements, and contradictions\n\
                - Resolve conflicts between different inputs when possible\n\
                - Create a unified narrative that preserves important details\n\
                - Attribute information to sources when relevant\n\
                - Highlight areas where inputs disagree or are incomplete\n\n\
                Synthesis Framework:\n\
                1. Overview: High-level summary of all inputs\n\
                2. Common Themes: Points of agreement and shared insights\n\
                3. Key Differences: Contradictions or varying perspectives\n\
                4. Unified Conclusion: Synthesized understanding incorporating all inputs\n\
                5. Confidence Level: How well the inputs can be reconciled\n\
                6. Sources: Attribution of key points to their origins\n\n\
                Information to Aggregate:\n{}",
                input
            )
        };

        let response = provider
            .generate(model, &aggregation_prompt)
            .await
            .map_err(|e| format!("Model generation failed: {}", e))?;

        Ok(response)
    }
}
