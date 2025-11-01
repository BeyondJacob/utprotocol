use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use crate::agents::models::*;
use crate::agents::registry::AgentRegistry;
use crate::agents::execution::AgentExecutor;
use crate::providers::ModelProvider;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowExecutionTrace {
    pub agent_id: i32,
    pub agent_label: String,
    pub agent_type: String,
    pub input: String,
    pub output: String,
    pub execution_time_ms: i32,
    pub utp_enabled: bool,
    pub error: Option<String>,
    pub order: i32,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub network_time_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTiming {
    pub source_agent_id: i32,
    pub target_agent_id: i32,
    pub transfer_time_ms: i32,
    pub data_size_bytes: usize,
    pub utp_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowExecutionResult {
    pub flow_id: i32,
    pub final_output: String,
    pub execution_trace: Vec<FlowExecutionTrace>,
    pub edge_timings: Vec<EdgeTiming>,
    pub total_execution_time_ms: i32,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteFlowRequest {
    pub input: String,
    pub start_node_id: Option<i32>,
}

pub struct FlowExecutor {
    registry: Arc<AgentRegistry>,
    agent_executor: Arc<AgentExecutor>,
}

impl FlowExecutor {
    pub fn new(registry: Arc<AgentRegistry>, agent_executor: Arc<AgentExecutor>) -> Self {
        Self {
            registry,
            agent_executor,
        }
    }

    /// Execute a complete agent flow from start to finish
    pub async fn execute_flow(
        &self,
        flow_id: i32,
        input: String,
        start_node_id: Option<i32>,
        provider_resolver: impl Fn(&str) -> Option<Arc<dyn ModelProvider>>,
    ) -> Result<FlowExecutionResult, String> {
        let start = std::time::Instant::now();

        // Get flow with agents and edges
        let flow_data = self
            .registry
            .get_flow_with_agents(flow_id)
            .await
            .map_err(|e| format!("Failed to get flow: {}", e))?
            .ok_or_else(|| format!("Flow {} not found", flow_id))?;

        if flow_data.agents.is_empty() {
            return Err("Flow has no agents".to_string());
        }

        // Build adjacency list for the flow graph
        let mut graph: HashMap<i32, Vec<i32>> = HashMap::new();
        let mut in_degree: HashMap<i32, usize> = HashMap::new();
        let mut agent_map: HashMap<i32, &Agent> = HashMap::new();

        // Initialize graph structures
        for agent in &flow_data.agents {
            agent_map.insert(agent.id, agent);
            graph.entry(agent.id).or_insert_with(Vec::new);
            in_degree.entry(agent.id).or_insert(0);
        }

        // Build edges
        for edge in &flow_data.edges {
            graph
                .entry(edge.source_agent_id)
                .or_insert_with(Vec::new)
                .push(edge.target_agent_id);
            *in_degree.entry(edge.target_agent_id).or_insert(0) += 1;
        }

        // Determine start nodes (nodes with in_degree 0 or specified start_node_id)
        let start_nodes: Vec<i32> = if let Some(start_id) = start_node_id {
            vec![start_id]
        } else {
            in_degree
                .iter()
                .filter(|(_, &degree)| degree == 0)
                .map(|(&id, _)| id)
                .collect()
        };

        if start_nodes.is_empty() {
            return Err("No start nodes found in flow (circular dependency?)".to_string());
        }

        // Check for UTP propagation - if ANY node in connected graph has UTP enabled, enable for ALL connected nodes
        let utp_propagation_map = self.build_utp_propagation_map(&flow_data.agents, &graph);

        // Execute flow using topological sort (BFS-based execution)
        let mut execution_trace: Vec<FlowExecutionTrace> = Vec::new();
        let mut edge_timings: Vec<EdgeTiming> = Vec::new();
        let mut agent_outputs: HashMap<i32, String> = HashMap::new();
        let mut agent_completion_times: HashMap<i32, Instant> = HashMap::new(); // Track when each agent completes
        let mut queue: VecDeque<i32> = start_nodes.iter().copied().collect();
        let mut visited: HashSet<i32> = HashSet::new();
        let mut current_in_degree = in_degree.clone();
        let mut execution_order = 0;

        // Set initial input for start nodes
        for &start_id in &start_nodes {
            agent_outputs.insert(start_id, input.clone());
        }

        while let Some(agent_id) = queue.pop_front() {
            if visited.contains(&agent_id) {
                continue;
            }
            visited.insert(agent_id);

            let agent = agent_map
                .get(&agent_id)
                .ok_or_else(|| format!("Agent {} not found in flow", agent_id))?;

            // Get input for this agent (either from start or from predecessors)
            let agent_input = agent_outputs
                .get(&agent_id)
                .cloned()
                .unwrap_or_else(|| {
                    // Aggregate inputs from all predecessors
                    let mut inputs = Vec::new();
                    for (&source_id, targets) in &graph {
                        if targets.contains(&agent_id) {
                            if let Some(output) = agent_outputs.get(&source_id) {
                                inputs.push(format!("[From {}]: {}", agent_map.get(&source_id).map(|a| a.label.as_str()).unwrap_or("unknown"), output));
                            }
                        }
                    }
                    inputs.join("\n\n")
                });

            // Check if UTP should be enabled for this agent
            let utp_enabled = *utp_propagation_map.get(&agent_id).unwrap_or(&agent.utp_enabled);

            // Get provider for this agent's model
            let model = agent
                .model
                .as_ref()
                .ok_or_else(|| format!("Agent {} has no model configured", agent_id))?;

            let provider = provider_resolver(model)
                .ok_or_else(|| format!("No provider found for model: {}", model))?;

            // Track when this agent starts execution (to measure transfer time from predecessors)
            let agent_start_time = Instant::now();

            // Execute the agent
            let exec_result = self
                .agent_executor
                .execute_agent(agent_id, agent_input.clone(), provider)
                .await?;

            // Track when this agent completes
            let agent_complete_time = Instant::now();
            agent_completion_times.insert(agent_id, agent_complete_time);

            // Store trace
            execution_trace.push(FlowExecutionTrace {
                agent_id,
                agent_label: agent.label.clone(),
                agent_type: agent.agent_type.clone(),
                input: agent_input.clone(),
                output: exec_result.output.clone(),
                execution_time_ms: exec_result.execution_time_ms,
                utp_enabled,
                error: exec_result.error.clone(),
                order: execution_order,
                prompt_tokens: exec_result.metadata
                    .as_ref()
                    .and_then(|m| m.get("prompt_tokens"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                completion_tokens: exec_result.metadata
                    .as_ref()
                    .and_then(|m| m.get("completion_tokens"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                network_time_ms: None, // Will be calculated for inter-agent communication
            });
            execution_order += 1;

            // Store output for downstream agents
            agent_outputs.insert(agent_id, exec_result.output.clone());

            // Calculate real edge timings: time from when source completes to when target starts
            // For each predecessor, calculate transfer time
            for (&source_id, targets) in &graph {
                if targets.contains(&agent_id) {
                    // This edge goes FROM source_id TO current agent_id
                    if let Some(source_complete_time) = agent_completion_times.get(&source_id) {
                        // Calculate actual transfer time: from source completion to this agent's start
                        let transfer_time_ms = agent_start_time
                            .duration_since(*source_complete_time)
                            .as_millis() as i32;

                        let source_output = agent_outputs.get(&source_id).map(|s| s.len()).unwrap_or(0);

                        edge_timings.push(EdgeTiming {
                            source_agent_id: source_id,
                            target_agent_id: agent_id,
                            transfer_time_ms,
                            data_size_bytes: source_output,
                            utp_enabled,
                        });
                    }
                }
            }

            // Update downstream agents
            if let Some(neighbors) = graph.get(&agent_id) {
                for &neighbor_id in neighbors {
                    if let Some(degree) = current_in_degree.get_mut(&neighbor_id) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 && !visited.contains(&neighbor_id) {
                            queue.push_back(neighbor_id);
                        }
                    }
                }
            }
        }

        // Determine final output (from terminal nodes - nodes with no outgoing edges)
        let terminal_nodes: Vec<i32> = flow_data
            .agents
            .iter()
            .map(|a| a.id)
            .filter(|id| {
                graph.get(id).map(|neighbors| neighbors.is_empty()).unwrap_or(true)
            })
            .collect();

        let final_output = if terminal_nodes.len() == 1 {
            agent_outputs
                .get(&terminal_nodes[0])
                .cloned()
                .unwrap_or_else(|| "No output generated".to_string())
        } else if !terminal_nodes.is_empty() {
            // Multiple terminal nodes - aggregate their outputs
            let mut outputs = Vec::new();
            for &node_id in &terminal_nodes {
                if let Some(output) = agent_outputs.get(&node_id) {
                    let agent = agent_map.get(&node_id);
                    outputs.push(format!(
                        "=== {} ===\n{}",
                        agent.map(|a| a.label.as_str()).unwrap_or("Unknown"),
                        output
                    ));
                }
            }
            outputs.join("\n\n")
        } else {
            "No terminal nodes found in flow".to_string()
        };

        let total_time = start.elapsed().as_millis() as i32;

        Ok(FlowExecutionResult {
            flow_id,
            final_output,
            execution_trace,
            edge_timings,
            total_execution_time_ms: total_time,
            success: true,
            error: None,
        })
    }

    /// Build a map of which agents should have UTP enabled based on propagation rules
    /// If ANY node in a connected component has UTP enabled, ALL nodes in that component get UTP enabled
    fn build_utp_propagation_map(
        &self,
        agents: &[Agent],
        graph: &HashMap<i32, Vec<i32>>,
    ) -> HashMap<i32, bool> {
        let mut utp_map: HashMap<i32, bool> = HashMap::new();
        let mut agent_map: HashMap<i32, &Agent> = HashMap::new();

        for agent in agents {
            agent_map.insert(agent.id, agent);
            utp_map.insert(agent.id, agent.utp_enabled);
        }

        // Build reverse graph (for traversing upstream)
        let mut reverse_graph: HashMap<i32, Vec<i32>> = HashMap::new();
        for (&source, targets) in graph {
            for &target in targets {
                reverse_graph.entry(target).or_insert_with(Vec::new).push(source);
            }
        }

        // Find all connected components and check if any node in each component has UTP enabled
        let mut visited: HashSet<i32> = HashSet::new();

        for &agent_id in agent_map.keys() {
            if visited.contains(&agent_id) {
                continue;
            }

            // Find all nodes in this connected component using bidirectional BFS
            let mut component: HashSet<i32> = HashSet::new();
            let mut queue: VecDeque<i32> = VecDeque::new();
            queue.push_back(agent_id);

            while let Some(current) = queue.pop_front() {
                if component.contains(&current) {
                    continue;
                }
                component.insert(current);

                // Add downstream neighbors
                if let Some(neighbors) = graph.get(&current) {
                    for &neighbor in neighbors {
                        if !component.contains(&neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }

                // Add upstream neighbors (reverse direction)
                if let Some(neighbors) = reverse_graph.get(&current) {
                    for &neighbor in neighbors {
                        if !component.contains(&neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }

            // Check if ANY node in this component has UTP enabled
            let component_has_utp = component
                .iter()
                .any(|&id| agent_map.get(&id).map(|a| a.utp_enabled).unwrap_or(false));

            // If any node has UTP, enable it for ALL nodes in the component
            if component_has_utp {
                for &node_id in &component {
                    utp_map.insert(node_id, true);
                }
            }

            // Mark all nodes in component as visited
            visited.extend(component);
        }

        utp_map
    }
}
