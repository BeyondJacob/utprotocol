import { Node, Edge } from "@xyflow/react";

// Agent types that can be instantiated
export type AgentType = "input" | "responder" | "analyzer" | "router" | "aggregator";

// Agent status during execution
export type AgentStatus = "idle" | "pending" | "waiting" | "running" | "completed" | "error";

// Agent node data structure
export interface AgentNodeData extends Record<string, unknown> {
  label: string;
  type: AgentType;
  status: AgentStatus;
  model?: string;
  utpEnabled?: boolean;
  config?: Record<string, unknown>;
  lastOutput?: string;
  lastError?: string;
  executionTime?: number;
  promptTokens?: number;
  completionTokens?: number;
  waitingForInputs?: number; // Number of inputs aggregator is waiting for
  receivedInputs?: number; // Number of inputs received so far
}

// Custom agent node type
export type AgentNode = Node<AgentNodeData>;

// Message passed between agents
export interface AgentMessage {
  id: string;
  sourceAgentId: string;
  targetAgentId: string;
  content: string;
  timestamp: Date;
  metadata?: Record<string, unknown>;
}

// Edge data for agent connections
export interface AgentEdgeData extends Record<string, unknown> {
  messages: AgentMessage[];
  lastMessageAt?: Date;
}

// Custom agent edge type
export type AgentEdge = Edge<AgentEdgeData>;

// Flow configuration
export interface AgentFlow {
  id: string;
  name: string;
  nodes: AgentNode[];
  edges: AgentEdge[];
  createdAt: Date;
  updatedAt: Date;
}

// Agent execution request
export interface AgentExecutionRequest {
  agentId: string;
  input: string;
  context?: Record<string, unknown>;
}

// Agent execution response
export interface AgentExecutionResponse {
  agentId: string;
  output: string;
  status: AgentStatus;
  executionTime: number;
  error?: string;
  metadata?: Record<string, unknown>;
}

// Backend API types
export interface CreateAgentRequest {
  label: string;
  type: AgentType;
  model?: string;
  config?: Record<string, unknown>;
  position: { x: number; y: number };
}

export interface CreateFlowRequest {
  name: string;
  nodes: AgentNode[];
  edges: AgentEdge[];
}

export interface AgentFlowState {
  mode: "conversation" | "agentic";
  currentFlow: AgentFlow | null;
  selectedAgentId: string | null;
  isExecuting: boolean;
}
