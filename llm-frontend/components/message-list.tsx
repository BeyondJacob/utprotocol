"use client";

import { useEffect, useRef } from "react";
import { Card } from "./ui/card";
import type { Message } from "./chat-interface";

export function MessageList({ messages }: { messages: Message[] }) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new message arrives
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

  if (messages.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground mb-4">
        <p>Start a conversation by typing a message below</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto pr-4 mb-4">
      <div className="space-y-4">
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${
              message.role === "user" ? "justify-end" : "justify-start"
            }`}
          >
            <Card
              className={`max-w-[80%] p-4 ${
                message.role === "user"
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted"
              }`}
            >
              <div className="space-y-2">
                <p className="text-sm whitespace-pre-wrap">{message.content}</p>
                {message.model && message.latency && (
                  <p className="text-xs opacity-70">
                    {message.model} • {message.latency}ms
                  </p>
                )}
              </div>
            </Card>
          </div>
        ))}
        <div ref={scrollRef} />
      </div>
    </div>
  );
}
