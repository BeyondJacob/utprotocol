"use client";

import { Handle, Position } from "@xyflow/react";
import { Card } from "../ui/card";
import { Badge } from "../ui/badge";
import { Brain, GitBranch, Sparkles, Combine, Zap, Settings2, Terminal, Play } from "lucide-react";
import { type AgentNodeData, type AgentType } from "./types";
import { Button } from "../ui/button";

interface AgentNodeProps {
  data: AgentNodeData;
  selected?: boolean;
  id?: string;
  onConfigure?: (nodeId: string) => void;
  onExecute?: (nodeId: string) => void;
}

// Icon mapping for agent types
const agentIcons: Record<AgentType, React.ReactNode> = {
  input: <Terminal className="h-4 w-4" />,
  responder: <Brain className="h-4 w-4" />,
  analyzer: <Sparkles className="h-4 w-4" />,
  router: <GitBranch className="h-4 w-4" />,
  aggregator: <Combine className="h-4 w-4" />,
};

// Status color mapping
const statusColors: Record<AgentNodeData["status"], string> = {
  idle: "bg-muted text-muted-foreground",
  pending: "bg-yellow-500 text-white",
  waiting: "bg-orange-500 text-white animate-pulse",
  running: "bg-blue-500 text-white animate-pulse",
  completed: "bg-green-500 text-white",
  error: "bg-red-500 text-white",
};

// Agent type colors
const typeColors: Record<AgentType, string> = {
  input: "border-cyan-500",
  responder: "border-blue-500",
  analyzer: "border-purple-500",
  router: "border-orange-500",
  aggregator: "border-green-500",
};

// Agent type tooltips/descriptions
const agentDescriptions: Record<AgentType, string> = {
  input: "Input agent - the starting point that receives user input and initiates the workflow",
  responder: "Conversational agent providing direct Q&A responses and interactive dialogue",
  analyzer: "Deep analysis specialist extracting insights, patterns, and structured recommendations",
  router: "Decision-making agent that routes requests to appropriate handlers based on intent",
  aggregator: "Synthesis specialist combining multiple inputs into coherent, unified outputs",
};

export function AgentNode({ data, selected, id, onConfigure, onExecute }: AgentNodeProps) {
  return (
    <Card
      className={`min-w-[220px] max-w-[300px] border-2 transition-all ${
        typeColors[data.type]
      } ${selected ? "shadow-lg ring-2 ring-primary ring-offset-2" : "shadow-sm"}`}
      title={agentDescriptions[data.type]}
    >
      {/* Input Handle */}
      <Handle
        type="target"
        position={Position.Top}
        className="!bg-primary !w-3 !h-3 !border-2 !border-background"
      />

      {/* Node Content */}
      <div className="p-3 space-y-2">
        {/* Header */}
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <div className="flex-shrink-0 text-muted-foreground">
              {agentIcons[data.type]}
            </div>
            <div className="min-w-0 flex-1">
              <h3 className="font-semibold text-sm truncate">{data.label}</h3>
              <p className="text-xs text-muted-foreground capitalize">{data.type}</p>
            </div>
          </div>
          <div className="flex items-center gap-1 flex-shrink-0">
            <Badge
              variant="secondary"
              className={`text-xs ${statusColors[data.status]}`}
            >
              {data.status}
            </Badge>
            {id && data.type === "input" && onExecute && (
              <Button
                variant="default"
                size="icon"
                className="h-6 w-6 p-0"
                onClick={(e) => {
                  e.stopPropagation();
                  onExecute(id);
                }}
                title="Execute flow from this start node"
              >
                <Play className="h-3 w-3" />
              </Button>
            )}
            {id && onConfigure && (
              <Button
                variant="ghost"
                size="icon"
                className="h-5 w-5 p-0"
                onClick={(e) => {
                  e.stopPropagation();
                  onConfigure(id);
                }}
              >
                <Settings2 className="h-3 w-3" />
              </Button>
            )}
          </div>
        </div>

        {/* Aggregator Waiting State */}
        {data.type === "aggregator" && data.status === "waiting" && data.waitingForInputs && (
          <div className="bg-orange-50 border border-orange-200 rounded p-2 text-xs">
            <div className="flex items-center gap-1 text-orange-700 font-medium mb-1">
              <Combine className="h-3 w-3" />
              <span>Waiting for Inputs</span>
            </div>
            <div className="text-orange-600">
              {data.receivedInputs || 0} / {data.waitingForInputs} received
            </div>
          </div>
        )}

        {/* UTP Status Chip */}
        {data.utpEnabled !== undefined && (
          <div className="flex items-center gap-1.5">
            <Badge
              variant={data.utpEnabled ? "default" : "outline"}
              className="text-xs gap-1 px-2 py-0.5"
              title={data.utpEnabled ? "UTP compression enabled - all downstream nodes will inherit this setting" : "Standard mode - no compression"}
            >
              <Zap className="h-3 w-3" />
              {data.utpEnabled ? "UTP Active" : "Standard"}
            </Badge>
          </div>
        )}

        {/* Model Info */}
        {data.model && (
          <div className="text-xs text-muted-foreground">
            <span className="font-medium">Model:</span> {data.model}
          </div>
        )}

        {/* Execution Stats */}
        {(data.executionTime || data.promptTokens || data.completionTokens) && (
          <div className="grid grid-cols-3 gap-1 text-xs text-muted-foreground">
            {data.executionTime && (
              <div className="text-center bg-muted/50 rounded p-1">
                <div className="font-medium">{data.executionTime}ms</div>
                <div className="text-[10px]">Time</div>
              </div>
            )}
            {data.promptTokens !== undefined && (
              <div className="text-center bg-muted/50 rounded p-1">
                <div className="font-medium">{data.promptTokens}</div>
                <div className="text-[10px]">In</div>
              </div>
            )}
            {data.completionTokens !== undefined && (
              <div className="text-center bg-muted/50 rounded p-1">
                <div className="font-medium">{data.completionTokens}</div>
                <div className="text-[10px]">Out</div>
              </div>
            )}
          </div>
        )}

        {/* Last Output Preview */}
        {data.lastOutput && (
          <div className="text-xs bg-muted rounded p-2 max-h-20 overflow-y-auto">
            <p className="line-clamp-3">{data.lastOutput}</p>
          </div>
        )}

        {/* Error Display */}
        {data.lastError && (
          <div className="text-xs bg-red-50 border border-red-200 rounded p-2">
            <p className="text-red-600 line-clamp-2">{data.lastError}</p>
          </div>
        )}
      </div>

      {/* Output Handle */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!bg-primary !w-3 !h-3 !border-2 !border-background"
      />
    </Card>
  );
}
