"use client";

import { Button } from "./ui/button";
import { MessageSquare, Network } from "lucide-react";

export type InterfaceMode = "conversation" | "agentic";

interface ModeToggleProps {
  mode: InterfaceMode;
  onModeChange: (mode: InterfaceMode) => void;
}

export function ModeToggle({ mode, onModeChange }: ModeToggleProps) {
  return (
    <div className="flex items-center gap-1 border rounded-md p-1 bg-muted/50">
      <Button
        variant={mode === "conversation" ? "default" : "ghost"}
        size="sm"
        className="h-8 gap-1.5 text-xs md:text-sm transition-all duration-200"
        onClick={() => onModeChange("conversation")}
      >
        <MessageSquare className="h-3.5 w-3.5 transition-transform duration-200" />
        <span className="hidden sm:inline">Conversation</span>
      </Button>
      <Button
        variant={mode === "agentic" ? "default" : "ghost"}
        size="sm"
        className="h-8 gap-1.5 text-xs md:text-sm transition-all duration-200"
        onClick={() => onModeChange("agentic")}
      >
        <Network className="h-3.5 w-3.5 transition-transform duration-200" />
        <span className="hidden sm:inline">Agentic</span>
      </Button>
    </div>
  );
}
