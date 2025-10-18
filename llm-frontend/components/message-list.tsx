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
    <div className="flex-1 overflow-y-auto mb-2 md:mb-4 px-2 sm:px-3 md:px-4 min-w-0">
      <div className="space-y-3 md:space-y-4 py-2 max-w-full">
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${
              message.role === "user" ? "justify-end" : "justify-start"
            }`}
          >
            <Card
              className={`max-w-[92%] sm:max-w-[88%] md:max-w-[85%] p-3 md:p-4 break-words ${
                message.role === "user"
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted"
              }`}
            >
              <div className="space-y-2">
                <p className="text-sm md:text-base whitespace-pre-wrap break-words">{message.content}</p>
                {message.role === "assistant" && (message.model || message.latency) && (
                  <div className="text-[10px] sm:text-xs pt-2 border-t border-muted-foreground/20 text-muted-foreground/70">
                    <div className="flex items-center gap-1.5 sm:gap-2 md:gap-3 flex-wrap">
                      {message.model && <span className="font-medium truncate max-w-[120px] sm:max-w-none">{message.model}</span>}
                      {message.latency && (
                        <>
                          {message.model && <span className="hidden sm:inline">•</span>}
                          <span className="whitespace-nowrap">{message.latency}ms</span>
                        </>
                      )}
                      {message.tokens_per_second && (
                        <>
                          <span className="hidden sm:inline">•</span>
                          <span className="whitespace-nowrap">{message.tokens_per_second.toFixed(1)} tok/s</span>
                        </>
                      )}
                      {message.total_tokens && (
                        <>
                          <span className="hidden sm:inline">•</span>
                          <span className="whitespace-nowrap">{message.total_tokens} tok</span>
                        </>
                      )}
                    </div>
                  </div>
                )}
                {message.role === "user" && message.model && (
                  <div className="text-[10px] sm:text-xs pt-2 border-t border-primary-foreground/20 opacity-70">
                    <span className="font-medium truncate max-w-full inline-block">Sent to: {message.model}</span>
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
