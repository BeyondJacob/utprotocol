"use client";

import { useState, KeyboardEvent } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";

interface ChatInputProps {
  onSendMessage: (message: string) => void;
  isLoading: boolean;
}

export function ChatInput({ onSendMessage, isLoading }: ChatInputProps) {
  const [input, setInput] = useState("");

  const handleSend = () => {
    if (input.trim() && !isLoading) {
      onSendMessage(input.trim());
      setInput("");
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex gap-2 px-2 sm:px-3 md:px-4 pb-2 sm:pb-3 md:pb-4">
      <Input
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Type your message..."
        disabled={isLoading}
        className="flex-1 text-sm md:text-base"
      />
      <Button
        onClick={handleSend}
        disabled={isLoading || !input.trim()}
        className="px-3 sm:px-4 text-sm md:text-base"
      >
        {isLoading ? <span className="hidden sm:inline">Sending...</span> : <span className="hidden sm:inline">Send</span>}
        <span className="sm:hidden">{isLoading ? "..." : "→"}</span>
      </Button>
    </div>
  );
}
