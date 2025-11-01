"use client";

import { Card } from "../ui/card";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import { Clock, Zap, Activity, CheckCircle2, Loader2, AlertCircle } from "lucide-react";

interface AgentExecution {
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

interface RealtimeExecutionPanelProps {
  executions: AgentExecution[];
  isExecuting: boolean;
}

export function RealtimeExecutionPanel({
  executions,
  isExecuting,
}: RealtimeExecutionPanelProps) {
  if (executions.length === 0 && !isExecuting) return null;

  const getStatusIcon = (status: AgentExecution["status"]) => {
    switch (status) {
      case "pending":
        return <Clock className="h-4 w-4 text-muted-foreground" />;
      case "running":
        return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />;
      case "completed":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case "error":
        return <AlertCircle className="h-4 w-4 text-red-500" />;
    }
  };

  const getStatusColor = (status: AgentExecution["status"]) => {
    switch (status) {
      case "pending":
        return "bg-muted text-muted-foreground";
      case "running":
        return "bg-blue-500/10 text-blue-600 border-blue-500/20";
      case "completed":
        return "bg-green-500/10 text-green-600 border-green-500/20";
      case "error":
        return "bg-red-500/10 text-red-600 border-red-500/20";
    }
  };

  return (
    <Card className="absolute bottom-4 right-4 w-96 shadow-xl border-2 z-10">
      <div className="p-3 border-b flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4" />
          <span className="font-semibold text-sm">Live Execution</span>
        </div>
        {isExecuting && (
          <Badge variant="secondary" className="gap-1">
            <Loader2 className="h-3 w-3 animate-spin" />
            Running
          </Badge>
        )}
      </div>
      <ScrollArea className="h-[400px]">
        <div className="p-3 space-y-2">
          {executions.map((exec, idx) => (
            <div
              key={`${exec.agent_id}-${idx}`}
              className={`rounded-lg border p-3 transition-all ${getStatusColor(
                exec.status
              )}`}
            >
              {/* Header */}
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  {getStatusIcon(exec.status)}
                  <span className="font-semibold text-sm">
                    {exec.agent_label}
                  </span>
                  <Badge variant="outline" className="text-xs capitalize">
                    {exec.agent_type}
                  </Badge>
                </div>
                {exec.utp_enabled && (
                  <Badge className="gap-1 h-5 text-xs">
                    <Zap className="h-3 w-3" />
                    UTP
                  </Badge>
                )}
              </div>

              {/* Stats */}
              {exec.status === "completed" && (
                <div className="grid grid-cols-3 gap-2 mb-2">
                  {exec.execution_time_ms !== undefined && (
                    <div className="text-xs">
                      <div className="text-muted-foreground">Time</div>
                      <div className="font-semibold">{exec.execution_time_ms}ms</div>
                    </div>
                  )}
                  {exec.prompt_tokens !== undefined && (
                    <div className="text-xs">
                      <div className="text-muted-foreground">In</div>
                      <div className="font-semibold">{exec.prompt_tokens}t</div>
                    </div>
                  )}
                  {exec.completion_tokens !== undefined && (
                    <div className="text-xs">
                      <div className="text-muted-foreground">Out</div>
                      <div className="font-semibold">{exec.completion_tokens}t</div>
                    </div>
                  )}
                </div>
              )}

              {/* Output Preview */}
              {exec.status === "completed" && exec.output_preview && (
                <div className="text-xs bg-background/50 rounded p-2 border">
                  <div className="line-clamp-2">{exec.output_preview}</div>
                </div>
              )}

              {/* Error */}
              {exec.status === "error" && exec.error && (
                <div className="text-xs text-red-600">
                  {exec.error}
                </div>
              )}

              {/* Running Status */}
              {exec.status === "running" && (
                <div className="text-xs text-muted-foreground flex items-center gap-2">
                  <Loader2 className="h-3 w-3 animate-spin" />
                  Processing...
                </div>
              )}
            </div>
          ))}
        </div>
      </ScrollArea>
    </Card>
  );
}
