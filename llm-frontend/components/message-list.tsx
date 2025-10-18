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
    <div className="flex-1 overflow-y-auto mb-4 px-4 min-w-0">
      <div className="space-y-4 py-2 max-w-full">
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${
              message.role === "user" ? "justify-end" : "justify-start"
            }`}
          >
            <Card
              className={`max-w-[85%] p-4 break-words ${
                message.role === "user"
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted"
              }`}
            >
              <div className="space-y-2">
                <p className="text-sm whitespace-pre-wrap">{message.content}</p>
                {message.role === "assistant" && (message.model || message.latency) && (
                  <div className="text-xs pt-2 border-t border-muted-foreground/20 text-muted-foreground/70">
                    <div className="flex items-center gap-3 flex-wrap">
                      {message.model && <span className="font-medium">{message.model}</span>}
                      {message.latency && (
                        <>
                          {message.model && <span>•</span>}
                          <span>{message.latency}ms</span>
                        </>
                      )}
                      {message.tokens_per_second && (
                        <>
                          <span>•</span>
                          <span>{message.tokens_per_second.toFixed(1)} tokens/s</span>
                        </>
                      )}
                      {message.total_tokens && (
                        <>
                          <span>•</span>
                          <span>{message.total_tokens} tokens</span>
                        </>
                      )}
                    </div>
                  </div>
                )}
                {message.role === "user" && message.model && (
                  <div className="text-xs pt-2 border-t border-primary-foreground/20 opacity-70">
                    <span className="font-medium">Sent to: {message.model}</span>
                  </div>
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
