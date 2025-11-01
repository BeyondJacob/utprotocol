"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import { X, CheckCircle2, Clock, Zap, AlertCircle, ChevronDown, ChevronUp } from "lucide-react";

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
}

interface ExecutionResult {
  flow_id: number;
  final_output: string;
  execution_trace: ExecutionTrace[];
  total_execution_time_ms: number;
  success: boolean;
  error?: string;
}

interface ExecutionOutputPanelProps {
  result: ExecutionResult | null;
  onClose: () => void;
}

export function ExecutionOutputPanel({
  result,
  onClose,
}: ExecutionOutputPanelProps) {
  const [expandedTraces, setExpandedTraces] = useState<Set<number>>(new Set());

  if (!result) return null;

  const toggleTrace = (agentId: number) => {
    setExpandedTraces(prev => {
      const newSet = new Set(prev);
      if (newSet.has(agentId)) {
        newSet.delete(agentId);
      } else {
        newSet.add(agentId);
      }
      return newSet;
    });
  };

  const agentTypeColors: Record<string, string> = {
    input: "bg-cyan-500/10 text-cyan-600 border-cyan-500/20",
    responder: "bg-blue-500/10 text-blue-600 border-blue-500/20",
    analyzer: "bg-purple-500/10 text-purple-600 border-purple-500/20",
    router: "bg-orange-500/10 text-orange-600 border-orange-500/20",
    aggregator: "bg-green-500/10 text-green-600 border-green-500/20",
  };

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <Card className="w-full max-w-4xl max-h-[90vh] flex flex-col shadow-2xl border-2">
        <CardHeader className="pb-3 flex flex-row items-center justify-between space-y-0 border-b">
          <div className="flex items-center gap-3">
            <CheckCircle2 className="h-5 w-5 text-green-500" />
            <CardTitle className="text-lg font-semibold">
              Flow Execution Results
            </CardTitle>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={onClose}
          >
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>

        <CardContent className="flex-1 overflow-y-auto p-6 space-y-6">
              {/* Summary Stats */}
              <div className="grid grid-cols-3 gap-4">
                <div className="bg-muted rounded-lg p-3">
                  <div className="flex items-center gap-2 text-muted-foreground text-xs mb-1">
                    <Clock className="h-3 w-3" />
                    <span>Total Time</span>
                  </div>
                  <div className="text-xl font-semibold">
                    {result.total_execution_time_ms}ms
                  </div>
                </div>
                <div className="bg-muted rounded-lg p-3">
                  <div className="flex items-center gap-2 text-muted-foreground text-xs mb-1">
                    <CheckCircle2 className="h-3 w-3" />
                    <span>Agents Executed</span>
                  </div>
                  <div className="text-xl font-semibold">
                    {result.execution_trace.length}
                  </div>
                </div>
                <div className="bg-muted rounded-lg p-3">
                  <div className="flex items-center gap-2 text-muted-foreground text-xs mb-1">
                    <Zap className="h-3 w-3" />
                    <span>UTP Enabled</span>
                  </div>
                  <div className="text-xl font-semibold">
                    {result.execution_trace.filter((t) => t.utp_enabled).length}
                  </div>
                </div>
              </div>

              {/* Final Output */}
              <div className="space-y-2">
                <h3 className="text-sm font-semibold flex items-center gap-2">
                  <CheckCircle2 className="h-4 w-4 text-green-500" />
                  Final Output
                </h3>
                <div className="bg-muted rounded-lg p-4 border border-green-500/20 max-h-[300px] overflow-y-auto">
                  <pre className="text-sm whitespace-pre-wrap font-mono">
                    {result.final_output}
                  </pre>
                </div>
              </div>

              {/* Execution Trace */}
              <div className="space-y-3">
                <h3 className="text-sm font-semibold">Step-by-Step Execution Trace</h3>
                <p className="text-xs text-muted-foreground">
                  Click any step to expand and see full input/output details
                </p>
                <div className="space-y-2">
                  {result.execution_trace
                    .sort((a, b) => a.order - b.order)
                    .map((trace, idx) => {
                      const isExpanded = expandedTraces.has(trace.agent_id);
                      return (
                        <div
                          key={`${trace.agent_id}-${trace.order}`}
                          className={`rounded-lg border transition-all ${
                            agentTypeColors[trace.agent_type] || "bg-muted"
                          }`}
                        >
                          {/* Trace Header - Always Visible */}
                          <button
                            onClick={() => toggleTrace(trace.agent_id)}
                            className="w-full p-3 flex items-center justify-between hover:bg-background/50 transition-colors"
                          >
                            <div className="flex items-center gap-2">
                              <Badge variant="outline" className="text-xs">
                                Step {idx + 1}
                              </Badge>
                              <span className="font-semibold text-sm">
                                {trace.agent_label}
                              </span>
                              <Badge variant="secondary" className="text-xs capitalize">
                                {trace.agent_type}
                              </Badge>
                              {trace.error && (
                                <Badge variant="destructive" className="text-xs">
                                  Error
                                </Badge>
                              )}
                            </div>
                            <div className="flex items-center gap-2">
                              {trace.utp_enabled && (
                                <Badge className="gap-1 h-5 text-xs">
                                  <Zap className="h-3 w-3" />
                                  UTP
                                </Badge>
                              )}
                              <span className="text-xs text-muted-foreground">
                                {trace.execution_time_ms}ms
                              </span>
                              {isExpanded ? (
                                <ChevronUp className="h-4 w-4" />
                              ) : (
                                <ChevronDown className="h-4 w-4" />
                              )}
                            </div>
                          </button>

                          {/* Trace Content - Collapsible */}
                          {isExpanded && (
                            <div className="px-3 pb-3 space-y-3 border-t">
                              {/* Input Section */}
                              <div className="pt-3">
                                <div className="flex items-center justify-between mb-2">
                                  <div className="text-xs font-semibold text-muted-foreground">
                                    📥 Input Received:
                                  </div>
                                  <span className="text-xs text-muted-foreground">
                                    {trace.input.length} chars
                                  </span>
                                </div>
                                <div className="text-xs bg-background/70 rounded p-3 border max-h-[200px] overflow-y-auto">
                                  <pre className="whitespace-pre-wrap font-mono">
                                    {trace.input}
                                  </pre>
                                </div>
                              </div>

                              {/* Output Section */}
                              <div>
                                <div className="flex items-center justify-between mb-2">
                                  <div className="text-xs font-semibold text-muted-foreground">
                                    📤 Output Generated:
                                  </div>
                                  <span className="text-xs text-muted-foreground">
                                    {trace.output.length} chars
                                  </span>
                                </div>
                                <div className="text-xs bg-background/70 rounded p-3 border max-h-[200px] overflow-y-auto">
                                  {trace.error ? (
                                    <div className="flex items-start gap-2 text-red-600">
                                      <AlertCircle className="h-4 w-4 flex-shrink-0 mt-0.5" />
                                      <pre className="whitespace-pre-wrap">{trace.error}</pre>
                                    </div>
                                  ) : (
                                    <pre className="whitespace-pre-wrap font-mono">
                                      {trace.output}
                                    </pre>
                                  )}
                                </div>
                              </div>

                              {/* Decision Path Indicator */}
                              {idx < result.execution_trace.length - 1 && (
                                <div className="flex items-center gap-2 text-xs text-muted-foreground pt-2">
                                  <div className="h-px bg-border flex-1" />
                                  <span>⬇️ Passed to next agent</span>
                                  <div className="h-px bg-border flex-1" />
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      );
                    })}
                </div>
              </div>
        </CardContent>
      </Card>
    </div>
  );
}
