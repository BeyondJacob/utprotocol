"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Input } from "../ui/input";
import { Badge } from "../ui/badge";
import { Switch } from "../ui/switch";
import { Combobox } from "../ui/combobox";
import { Label } from "../ui/label";
import { Button } from "../ui/button";
import { Textarea } from "../ui/textarea";
import { X, Zap, ZapOff } from "lucide-react";
import type { AgentNodeData, AgentType } from "./types";
import type { ModelInfo } from "../chat-window";

interface AgentConfigPanelProps {
  nodeId: string;
  data: AgentNodeData;
  availableModels: ModelInfo[];
  onUpdate: (nodeId: string, updates: Partial<AgentNodeData>) => void;
  onClose: () => void;
}

export function AgentConfigPanel({
  nodeId,
  data,
  availableModels,
  onUpdate,
  onClose,
}: AgentConfigPanelProps) {
  const [label, setLabel] = useState(data.label);
  const [model, setModel] = useState(data.model || "");
  const [utpEnabled, setUtpEnabled] = useState(data.utpEnabled || false);
  const [instructions, setInstructions] = useState(
    (data.config?.instructions as string) || ""
  );

  const modelOptions = availableModels.map((m) => ({
    value: m.name,
    label: `${m.display_name} (${m.provider})`,
  }));

  const handleSave = () => {
    onUpdate(nodeId, {
      label,
      model: model || undefined,
      utpEnabled,
      config: {
        ...data.config,
        instructions: instructions || undefined,
      },
    });
    onClose();
  };

  const agentTypeLabels: Record<AgentType, string> = {
    input: "Input Agent",
    responder: "Responder Agent",
    analyzer: "Analyzer Agent",
    router: "Router Agent",
    aggregator: "Aggregator Agent",
  };

  return (
    <Card className="absolute top-4 right-4 z-50 w-80 shadow-xl border-2">
      <CardHeader className="pb-3 flex flex-row items-center justify-between space-y-0">
        <CardTitle className="text-sm font-semibold">
          Configure Agent
        </CardTitle>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={onClose}
        >
          <X className="h-4 w-4" />
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Agent Type Badge */}
        <div>
          <Label className="text-xs text-muted-foreground">Type</Label>
          <Badge variant="secondary" className="mt-1 capitalize">
            {agentTypeLabels[data.type]}
          </Badge>
        </div>

        {/* Agent Label */}
        <div className="space-y-2">
          <Label htmlFor="agent-label" className="text-xs">
            Label
          </Label>
          <Input
            id="agent-label"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="Enter agent name"
            className="h-8 text-sm"
          />
        </div>

        {/* Model Selection */}
        <div className="space-y-2">
          <Label htmlFor="agent-model" className="text-xs">
            Model
          </Label>
          <Combobox
            options={modelOptions}
            value={model}
            onValueChange={setModel}
            placeholder="Select model..."
            searchPlaceholder="Search models..."
            emptyMessage="No models found."
            className="w-full h-8 text-sm"
          />
          {model && (
            <p className="text-xs text-muted-foreground">
              Selected: {availableModels.find((m) => m.name === model)?.display_name}
            </p>
          )}
        </div>

        {/* Custom Instructions */}
        <div className="space-y-2">
          <Label htmlFor="agent-instructions" className="text-xs">
            Custom Instructions (Optional)
          </Label>
          <Textarea
            id="agent-instructions"
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            placeholder="Add custom behavior instructions for this agent..."
            className="text-xs min-h-[80px] resize-none"
          />
          <p className="text-xs text-muted-foreground">
            Override default agent behavior with specific instructions.
          </p>
        </div>

        {/* UTP Mode Toggle */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label htmlFor="utp-toggle" className="text-xs">
              UTP Mode
            </Label>
            <div className="flex items-center gap-2">
              {utpEnabled ? (
                <Badge variant="default" className="gap-1 h-5 text-xs">
                  <Zap className="h-3 w-3" />
                  Enabled
                </Badge>
              ) : (
                <Badge variant="outline" className="gap-1 h-5 text-xs">
                  <ZapOff className="h-3 w-3" />
                  Disabled
                </Badge>
              )}
              <Switch
                id="utp-toggle"
                checked={utpEnabled}
                onCheckedChange={setUtpEnabled}
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            {utpEnabled
              ? "UTP compression active. Downstream nodes will inherit this setting."
              : "Standard mode. No compression applied."}
          </p>
        </div>

        {/* Status Info */}
        <div className="pt-2 border-t">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Status:</span>
            <Badge
              variant={data.status === "idle" ? "secondary" : "default"}
              className="capitalize text-xs"
            >
              {data.status}
            </Badge>
          </div>
          {data.executionTime && (
            <div className="flex items-center justify-between text-xs mt-1">
              <span className="text-muted-foreground">Last execution:</span>
              <span className="font-medium">{data.executionTime}ms</span>
            </div>
          )}
        </div>

        {/* Action Buttons */}
        <div className="flex gap-2 pt-2">
          <Button size="sm" className="flex-1 h-8 text-xs" onClick={handleSave}>
            Save Changes
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="flex-1 h-8 text-xs"
            onClick={onClose}
          >
            Cancel
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
