"use client";

import { useCallback, useState, useEffect } from "react";
import {
  ReactFlow,
  Background,
  MiniMap,
  addEdge,
  useNodesState,
  useEdgesState,
  Connection,
  ReactFlowProvider,
  BackgroundVariant,
  Panel,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { AgentNode as AgentNodeComponent } from "./agent-node";
import { AgentControls } from "./agent-controls";
import { AgentConfigPanel } from "./agent-config-panel";
import { ExecutionOutputPanel } from "./execution-output-panel";
import { RealtimeExecutionPanel } from "./realtime-execution-panel";
import { AgentNode as AgentNodeType, AgentEdge, AgentType, AgentNodeData } from "./types";
import { Button } from "../ui/button";
import { Brain, GitBranch, Sparkles, Combine, Terminal, FileText } from "lucide-react";
import type { NodeTypes } from "@xyflow/react";
import type { ModelInfo } from "../chat-window";

// Define custom node types - cast to satisfy React Flow's type requirements
const nodeTypes: NodeTypes = {
  agent: AgentNodeComponent as never,
};

// Initial empty canvas - users build their own flows
const initialNodes: AgentNodeType[] = [];

const initialEdges: AgentEdge[] = [];

interface ExecutionTrace {
  agent_id: number;
  agent_label: string;
  agent_type: string;
  input: string;
  output: string;
  execution_time_ms: number;
  utp_enabled: boolean;
  error?: string;
  order: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  network_time_ms?: number;
}

interface EdgeTiming {
  source_agent_id: number;
  target_agent_id: number;
  transfer_time_ms: number;
  data_size_bytes: number;
  utp_enabled: boolean;
}

interface ExecutionResult {
  flow_id: number;
  final_output: string;
  execution_trace: ExecutionTrace[];
  edge_timings: EdgeTiming[];
  total_execution_time_ms: number;
  success: boolean;
  error?: string;
}

interface LiveAgentExecution {
  agent_id: number;
  agent_label: string;
  agent_type: string;
  status: "pending" | "running" | "completed" | "error";
  execution_time_ms?: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  utp_enabled: boolean;
  output_preview?: string;
  error?: string;
}

function AgentFlowCanvasInner() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [isExecuting, setIsExecuting] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([]);
  const [executionResult, setExecutionResult] = useState<ExecutionResult | null>(null);
  const [liveExecutions, setLiveExecutions] = useState<LiveAgentExecution[]>([]);
  const [showDetailedResults, setShowDetailedResults] = useState(false);
  const [clipboard, setClipboard] = useState<AgentNodeType[]>([]);

  // Fetch available models
  useEffect(() => {
    const fetchModels = async () => {
      try {
        const response = await fetch("http://localhost:3001/models");
        const data = await response.json();
        setAvailableModels(data.models);
      } catch (error) {
        console.error("Failed to fetch models:", error);
      }
    };
    fetchModels();
  }, []);

  const onConnect = useCallback(
    (params: Connection) => {
      const newEdge: AgentEdge = {
        ...params,
        id: `${params.source}-${params.target}`,
        data: { messages: [] },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,
          height: 20,
        },
        style: {
          strokeWidth: 2,
        },
      };
      setEdges((eds) => addEdge(newEdge, eds));
    },
    [setEdges]
  );

  const handleAddAgent = useCallback(
    (type: AgentType) => {
      const newId = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      const nodeCount = nodes.length + 1;
      const newNode: AgentNodeType = {
        id: newId,
        type: "agent",
        position: {
          x: Math.random() * 400 + 100,
          y: Math.random() * 300 + 100,
        },
        data: {
          label: `${type.charAt(0).toUpperCase() + type.slice(1)} ${nodeCount}`,
          type,
          status: "idle",
        },
      };
      setNodes((nds) => [...nds, newNode]);
    },
    [nodes.length, setNodes]
  );

  // Helper function to find all connected nodes in the graph
  const findConnectedNodes = useCallback((nodeId: string, currentNodes: AgentNodeType[], currentEdges: AgentEdge[]): Set<string> => {
    const connected = new Set<string>();
    const queue: string[] = [nodeId];

    while (queue.length > 0) {
      const current = queue.shift()!;
      if (connected.has(current)) continue;
      connected.add(current);

      // Find all neighbors (both directions)
      currentEdges.forEach((edge) => {
        if (edge.source === current && !connected.has(edge.target!)) {
          queue.push(edge.target!);
        }
        if (edge.target === current && !connected.has(edge.source!)) {
          queue.push(edge.source!);
        }
      });
    }

    return connected;
  }, []);

  const handleExecuteFlow = useCallback(async (startNodeId?: string) => {
    if (nodes.length === 0) {
      alert("Please add agents to the flow before executing.");
      return;
    }

    // Find all nodes connected to the start node (if provided)
    const connectedNodeIds = startNodeId
      ? findConnectedNodes(startNodeId, nodes, edges)
      : new Set(nodes.map(n => n.id));

    const connectedNodes = nodes.filter(n => connectedNodeIds.has(n.id));
    const connectedEdges = edges.filter(e =>
      connectedNodeIds.has(e.source!) && connectedNodeIds.has(e.target!)
    );

    if (connectedNodes.length === 0) {
      alert("No connected nodes to execute.");
      return;
    }

    setIsExecuting(true);
    setExecutionResult(null);
    setShowDetailedResults(false);

    // Initialize live execution tracking only for connected agents
    const initialLiveExecutions: LiveAgentExecution[] = connectedNodes.map((node) => ({
      agent_id: parseInt(node.id),
      agent_label: node.data.label,
      agent_type: node.data.type,
      status: "pending" as const,
      utp_enabled: node.data.utpEnabled || false,
    }));
    setLiveExecutions(initialLiveExecutions);

    // Update only connected nodes to pending status
    setNodes((nds) =>
      nds.map((node) => ({
        ...node,
        data: {
          ...node.data,
          status: connectedNodeIds.has(node.id) ? ("pending" as const) : node.data.status
        },
      }))
    );

    try {
      // Step 1: Create a flow
      const flowResponse = await fetch("http://localhost:3001/flows", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: `Flow ${Date.now()}`,
          description: "Canvas flow execution",
        }),
      });

      if (!flowResponse.ok) {
        throw new Error("Failed to create flow");
      }

      const flow = await flowResponse.json();
      const flowId = flow.id;

      // Step 2: Create only connected agents and build ID mapping
      const idMapping: Record<string, number> = {};

      for (const node of connectedNodes) {
        const agentResponse = await fetch("http://localhost:3001/agents", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            label: node.data.label,
            type: node.data.type,
            model: node.data.model || "llama3.2:1b",
            utp_enabled: node.data.utpEnabled || false,
            config: node.data.config || {},
            position: { x: node.position.x, y: node.position.y },
          }),
        });

        if (!agentResponse.ok) {
          throw new Error(`Failed to create agent ${node.data.label}`);
        }

        const agent = await agentResponse.json();
        idMapping[node.id] = agent.id;

        // Add agent to flow
        await fetch(`http://localhost:3001/flows/${flowId}/agents/${agent.id}`, {
          method: "POST",
        });
      }

      // Step 3: Create only connected edges
      for (const edge of connectedEdges) {
        if (!edge.source || !edge.target) continue;

        const sourceId = idMapping[edge.source];
        const targetId = idMapping[edge.target];

        if (sourceId && targetId) {
          await fetch("http://localhost:3001/edges", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              flow_id: flowId,
              source_agent_id: sourceId,
              target_agent_id: targetId,
              edge_data: edge.data || {},
            }),
          });
        }
      }

      // Step 4: Execute the flow from the specified start node
      const response = await fetch(`http://localhost:3001/flows/${flowId}/execute`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          input: "Execute flow based on configured instructions",
          start_node_id: startNodeId ? idMapping[startNodeId] : null,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Execution failed: ${errorText}`);
      }

      const result: ExecutionResult = await response.json();

      // Update live executions with completed results
      const completedExecutions: LiveAgentExecution[] = result.execution_trace.map((trace) => ({
        agent_id: trace.agent_id,
        agent_label: trace.agent_label,
        agent_type: trace.agent_type,
        status: trace.error ? ("error" as const) : ("completed" as const),
        execution_time_ms: trace.execution_time_ms,
        prompt_tokens: trace.prompt_tokens,
        completion_tokens: trace.completion_tokens,
        utp_enabled: trace.utp_enabled,
        output_preview: trace.output.substring(0, 100),
        error: trace.error,
      }));
      setLiveExecutions(completedExecutions);

      // Update nodes with execution results including token statistics
      setNodes((nds) =>
        nds.map((node) => {
          const trace = result.execution_trace.find(
            (t) => t.agent_id === idMapping[node.id]
          );
          if (trace) {
            return {
              ...node,
              data: {
                ...node.data,
                status: trace.error ? ("error" as const) : ("completed" as const),
                lastOutput: trace.output,
                lastError: trace.error,
                executionTime: trace.execution_time_ms,
                promptTokens: trace.prompt_tokens,
                completionTokens: trace.completion_tokens,
                utpEnabled: trace.utp_enabled,
              },
            };
          }
          return {
            ...node,
            data: { ...node.data, status: "idle" as const },
          };
        })
      );

      // Update edges with timing information
      setEdges((eds) =>
        eds.map((edge) => {
          const sourceId = idMapping[edge.source!];
          const targetId = idMapping[edge.target!];
          const timing = result.edge_timings.find(
            (t) => t.source_agent_id === sourceId && t.target_agent_id === targetId
          );

          if (timing) {
            const sizeKb = (timing.data_size_bytes / 1024).toFixed(1);
            return {
              ...edge,
              label: `${timing.transfer_time_ms}ms • ${sizeKb}KB`,
              labelStyle: {
                fill: timing.utp_enabled ? "#10b981" : "#6b7280",
                fontSize: 10,
                fontWeight: 600,
              },
              labelBgStyle: {
                fill: timing.utp_enabled ? "#d1fae5" : "#f3f4f6",
                fillOpacity: 0.9,
              },
              labelBgPadding: [4, 6] as [number, number],
              labelBgBorderRadius: 4,
              style: {
                ...edge.style,
                stroke: timing.utp_enabled ? "#10b981" : "#6b7280",
                strokeWidth: 2,
              },
            };
          }
          return edge;
        })
      );

      // Store execution result for detailed view
      setExecutionResult(result);
    } catch (error) {
      console.error("Flow execution failed:", error);
      // Update only connected nodes to error state
      setNodes((nds) =>
        nds.map((node) => ({
          ...node,
          data: {
            ...node.data,
            status: connectedNodeIds.has(node.id) ? ("error" as const) : node.data.status,
            lastError: connectedNodeIds.has(node.id)
              ? (error instanceof Error ? error.message : "Unknown error")
              : node.data.lastError,
          },
        }))
      );
      alert(
        `Execution failed: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    } finally {
      setIsExecuting(false);
    }
  }, [setNodes, nodes, edges, findConnectedNodes]);

  const handleClearFlow = useCallback(() => {
    setNodes(initialNodes);
    setEdges(initialEdges);
  }, [setNodes, setEdges]);

  const handleConfigureNode = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
  }, []);

  const handleUpdateNode = useCallback((nodeId: string, updates: Partial<AgentNodeData>) => {
    setNodes((nds) => {
      // Check if UTP is being toggled
      const isUtpChange = updates.utpEnabled !== undefined;

      if (isUtpChange) {
        // Find all connected nodes
        const connectedNodeIds = findConnectedNodes(nodeId, nds, edges);

        // First, apply the update to the current node
        const updatedNodes = nds.map((node) =>
          node.id === nodeId
            ? { ...node, data: { ...node.data, ...updates } }
            : node
        );

        // Then check if ANY node in the connected component still has UTP enabled AFTER the update
        const anyNodeHasUtp = Array.from(connectedNodeIds).some(id => {
          const node = updatedNodes.find(n => n.id === id);
          return node?.data.utpEnabled || false;
        });

        // Apply UTP propagation to all connected nodes based on whether ANY has it enabled
        return updatedNodes.map((node) => {
          if (connectedNodeIds.has(node.id)) {
            return {
              ...node,
              data: {
                ...node.data,
                utpEnabled: anyNodeHasUtp,
              },
            };
          }
          return node;
        });
      }

      // Non-UTP updates
      return nds.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, ...updates } }
          : node
      );
    });
  }, [setNodes, edges, findConnectedNodes]);

  const handleCloseConfig = useCallback(() => {
    setSelectedNodeId(null);
  }, []);

  // Enhanced node types with configuration and execute handlers
  const enhancedNodeTypes = useCallback(
    () => ({
      agent: (props: { data: AgentNodeData; selected?: boolean; id?: string }) => (
        <AgentNodeComponent
          {...props}
          onConfigure={handleConfigureNode}
          onExecute={handleExecuteFlow}
        />
      ),
    }),
    [handleConfigureNode, handleExecuteFlow]
  );

  // Copy/paste handlers
  const handleCopy = useCallback(() => {
    const selectedNodes = nodes.filter((node) => node.selected);
    if (selectedNodes.length > 0) {
      setClipboard(selectedNodes);
      console.log(`Copied ${selectedNodes.length} node(s)`);
    }
  }, [nodes]);

  const handlePaste = useCallback(() => {
    if (clipboard.length === 0) return;

    const newNodes = clipboard.map((node) => {
      const newId = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      return {
        ...node,
        id: newId,
        position: {
          x: node.position.x + 50,
          y: node.position.y + 50,
        },
        selected: false,
      };
    });

    setNodes((nds) => [...nds, ...newNodes]);
    console.log(`Pasted ${newNodes.length} node(s)`);
  }, [clipboard, setNodes]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Ignore if user is typing in input/textarea/contenteditable
      const target = event.target as HTMLElement;
      const isTyping =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      if (isTyping) {
        return; // Don't handle shortcuts when typing
      }

      // Copy: Cmd/Ctrl + C
      if ((event.metaKey || event.ctrlKey) && event.key === "c") {
        event.preventDefault();
        handleCopy();
      }
      // Paste: Cmd/Ctrl + V
      if ((event.metaKey || event.ctrlKey) && event.key === "v") {
        event.preventDefault();
        handlePaste();
      }
      // Delete: Backspace or Delete key
      if (event.key === "Backspace" || event.key === "Delete") {
        const selectedNodes = nodes.filter((node) => node.selected);
        if (selectedNodes.length > 0) {
          event.preventDefault(); // Prevent browser back navigation
          const selectedIds = new Set(selectedNodes.map((n) => n.id));
          setNodes((nds) => nds.filter((n) => !selectedIds.has(n.id)));
          setEdges((eds) =>
            eds.filter(
              (e) => !selectedIds.has(e.source!) && !selectedIds.has(e.target!)
            )
          );
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleCopy, handlePaste, nodes, setNodes, setEdges]);

  const selectedNode = selectedNodeId
    ? nodes.find((n) => n.id === selectedNodeId)
    : null;

  return (
    <div className="h-full w-full relative">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={enhancedNodeTypes() as never}
        fitView
        multiSelectionKeyCode="Shift"
        deleteKeyCode={null}
        className="bg-muted/30"
      >
        <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
        <MiniMap
          className="!bg-background !border !border-border"
          nodeColor={(node) => {
            const agentNode = node as AgentNodeType;
            const colors = {
              input: "#06b6d4",
              responder: "#3b82f6",
              analyzer: "#a855f7",
              router: "#f97316",
              aggregator: "#22c55e",
            };
            return colors[agentNode.data.type];
          }}
        />

        {/* Custom Controls */}
        <AgentControls
          onExecuteFlow={handleExecuteFlow}
          onClearFlow={handleClearFlow}
          isExecuting={isExecuting}
        />

        {/* Agent Type Palette */}
        <Panel position="top-left" className="space-y-2">
          <div className="bg-background border rounded-lg shadow-lg p-3 space-y-2">
            <h3 className="text-sm font-semibold mb-2">Add Agent</h3>
            <div className="grid grid-cols-2 gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-auto flex-col gap-1 py-2 col-span-2"
                onClick={() => handleAddAgent("input")}
                title="Input: Starting point agent that receives user input and initiates the workflow (REQUIRED)"
              >
                <Terminal className="h-4 w-4" />
                <span className="text-xs font-semibold">Input (Start)</span>
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-auto flex-col gap-1 py-2"
                onClick={() => handleAddAgent("responder")}
                title="Responder: Direct Q&A agent for conversational responses and interactive dialogues"
              >
                <Brain className="h-4 w-4" />
                <span className="text-xs">Responder</span>
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-auto flex-col gap-1 py-2"
                onClick={() => handleAddAgent("analyzer")}
                title="Analyzer: Deep analysis agent that extracts insights, identifies patterns, and provides structured recommendations"
              >
                <Sparkles className="h-4 w-4" />
                <span className="text-xs">Analyzer</span>
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-auto flex-col gap-1 py-2"
                onClick={() => handleAddAgent("router")}
                title="Router: Decision agent that analyzes requests and routes them to the most appropriate handlers based on intent and complexity"
              >
                <GitBranch className="h-4 w-4" />
                <span className="text-xs">Router</span>
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-auto flex-col gap-1 py-2"
                onClick={() => handleAddAgent("aggregator")}
                title="Aggregator: Synthesis agent that combines multiple inputs from different sources into a coherent, unified output"
              >
                <Combine className="h-4 w-4" />
                <span className="text-xs">Aggregator</span>
              </Button>
            </div>
          </div>
        </Panel>

      </ReactFlow>

      {/* Configuration Panel */}
      {selectedNode && (
        <AgentConfigPanel
          nodeId={selectedNode.id}
          data={selectedNode.data as AgentNodeData}
          availableModels={availableModels}
          onUpdate={handleUpdateNode}
          onClose={handleCloseConfig}
        />
      )}

      {/* Real-time Execution Panel */}
      {(isExecuting || liveExecutions.length > 0) && (
        <RealtimeExecutionPanel
          executions={liveExecutions}
          isExecuting={isExecuting}
        />
      )}

      {/* Detailed Results Chip - Bottom Left above real-time panel */}
      {executionResult && !showDetailedResults && (
        <Button
          variant="default"
          size="sm"
          className="fixed bottom-[420px] right-4 z-10 shadow-lg"
          onClick={() => setShowDetailedResults(true)}
        >
          <FileText className="h-4 w-4 mr-2" />
          View Detailed Results
        </Button>
      )}

      {/* Detailed Execution Output Panel */}
      {executionResult && showDetailedResults && (
        <ExecutionOutputPanel
          result={executionResult}
          onClose={() => setShowDetailedResults(false)}
        />
      )}
    </div>
  );
}

export function AgentFlowCanvas() {
  return (
    <ReactFlowProvider>
      <AgentFlowCanvasInner />
    </ReactFlowProvider>
  );
}
