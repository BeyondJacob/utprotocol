"use client";

import { useReactFlow } from "@xyflow/react";
import { Button } from "../ui/button";
import {
  ZoomIn,
  ZoomOut,
  Maximize2,
  RefreshCw,
  Plus,
  Play,
  Trash2,
} from "lucide-react";

interface AgentControlsProps {
  onAddAgent?: () => void;
  onExecuteFlow?: () => void;
  onClearFlow?: () => void;
  isExecuting?: boolean;
}

export function AgentControls({
  onAddAgent,
  onExecuteFlow,
  onClearFlow,
  isExecuting = false,
}: AgentControlsProps) {
  const { zoomIn, zoomOut, fitView } = useReactFlow();

  return (
    <div className="absolute top-4 right-4 z-10 flex flex-col gap-2">
      {/* Flow Actions */}
      <div className="flex flex-col gap-1 bg-background border rounded-lg shadow-lg p-1">
        {onAddAgent && (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={onAddAgent}
            title="Add Agent"
          >
            <Plus className="h-4 w-4" />
          </Button>
        )}
        {onExecuteFlow && (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={onExecuteFlow}
            disabled={isExecuting}
            title={isExecuting ? "Executing..." : "Execute Flow"}
          >
            {isExecuting ? (
              <RefreshCw className="h-4 w-4 animate-spin" />
            ) : (
              <Play className="h-4 w-4" />
            )}
          </Button>
        )}
        {onClearFlow && (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={onClearFlow}
            title="Clear Flow"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        )}
      </div>

      {/* View Controls */}
      <div className="flex flex-col gap-1 bg-background border rounded-lg shadow-lg p-1">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => zoomIn()}
          title="Zoom In"
        >
          <ZoomIn className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => zoomOut()}
          title="Zoom Out"
        >
          <ZoomOut className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => fitView({ padding: 0.2 })}
          title="Fit View"
        >
          <Maximize2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
