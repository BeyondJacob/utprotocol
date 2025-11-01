"use client";

import { useEffect, useRef } from "react";
import { Card } from "./ui/card";
import { Badge } from "./ui/badge";
import { ArrowRight } from "lucide-react";
import type { Message, ModelInfo, TraceStep } from "./chat-window";
import { calculateCost, formatCost } from "./chat-window";

export function MessageList({
  messages,
  availableModels
}: {
  messages: Message[];
  availableModels?: ModelInfo[];
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new message arrives
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [messages]);

  if (messages.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground mb-4">
        <p>Start a conversation by typing a message below</p>
      </div>
    );
  }

  // Helper function to get pricing for a model
  const getModelPricing = (modelName?: string) => {
    if (!modelName || !availableModels) return null;
    return availableModels.find(m => m.name === modelName)?.pricing;
  };

  return (
    <div ref={containerRef} className="flex-1 overflow-y-auto mb-2 md:mb-4 px-2 sm:px-3 md:px-4 min-w-0">
      <div className="space-y-3 md:space-y-4 py-2 max-w-full">
        {messages.map((message) => {
          const pricing = getModelPricing(message.model);
          const hasTokenCosts = pricing && (message.prompt_tokens || message.completion_tokens);

          let inputCost = 0;
          let outputCost = 0;
          let totalCost = 0;

          if (pricing) {
            if (message.prompt_tokens) {
              inputCost = calculateCost(message.prompt_tokens, pricing.input_per_million);
            }
            if (message.completion_tokens) {
              outputCost = calculateCost(message.completion_tokens, pricing.output_per_million);
            }
            totalCost = inputCost + outputCost;
          }

          return (
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
                    <div className="text-[10px] sm:text-xs pt-2 border-t border-muted-foreground/20 text-muted-foreground/70 space-y-1.5">
                      <div className="flex items-center gap-1.5 sm:gap-2 md:gap-3 flex-wrap">
                        {message.model && <span className="font-medium truncate max-w-[120px] sm:max-w-none">{message.model}</span>}
                        {message.latency && (
                          <>
                            {message.model && <span className="hidden sm:inline">•</span>}
                            <span className="whitespace-nowrap" title="Total latency">Total: {message.latency}ms</span>
                          </>
                        )}
                        {message.tokens_per_second && (
                          <>
                            <span className="hidden sm:inline">•</span>
                            <span className="whitespace-nowrap">{message.tokens_per_second.toFixed(1)} tok/s</span>
                          </>
                        )}
                      </div>

                      {/* Network latency breakdown */}
                      {(message.network_send_ms || message.network_receive_ms || message.network_total_ms) && (
                        <div className="flex items-center gap-1.5 sm:gap-2 flex-wrap">
                          <span className="opacity-80">Network:</span>
                          {message.network_send_ms && (
                            <span className="whitespace-nowrap" title="Time to send request">
                              Send {message.network_send_ms}ms
                            </span>
                          )}
                          {message.network_receive_ms && (
                            <>
                              {message.network_send_ms && <span>•</span>}
                              <span className="whitespace-nowrap" title="Time to receive response">
                                Receive {message.network_receive_ms}ms
                              </span>
                            </>
                          )}
                          {message.network_total_ms && (
                            <>
                              {(message.network_send_ms || message.network_receive_ms) && <span>•</span>}
                              <span className="whitespace-nowrap font-medium" title="Total network time">
                                Total {message.network_total_ms}ms
                              </span>
                            </>
                          )}
                        </div>
                      )}

                      {/* Token breakdown */}
                      {(message.prompt_tokens || message.completion_tokens) && (
                        <div className="flex items-center gap-1.5 sm:gap-2 flex-wrap">
                          {message.prompt_tokens && (
                            <span className="whitespace-nowrap">
                              In: {message.prompt_tokens.toLocaleString()} tok
                              {pricing && <span className="ml-1 opacity-80">({formatCost(inputCost)})</span>}
                            </span>
                          )}
                          {message.completion_tokens && (
                            <>
                              {message.prompt_tokens && <span>•</span>}
                              <span className="whitespace-nowrap">
                                Out: {message.completion_tokens.toLocaleString()} tok
                                {pricing && <span className="ml-1 opacity-80">({formatCost(outputCost)})</span>}
                              </span>
                            </>
                          )}
                          {hasTokenCosts && (
                            <>
                              <span>•</span>
                              <span className="whitespace-nowrap font-medium">
                                Total: {formatCost(totalCost)}
                              </span>
                            </>
                          )}
                        </div>
                      )}

                      {/* UTP Metadata Badges */}
                      {message.utp_metadata && (
                        <div className="mt-2 space-y-2">
                          {/* Trace Visualization */}
                          {message.utp_metadata.trace && message.utp_metadata.trace.length > 0 && (
                            <div className="flex items-center gap-1 text-[10px] text-muted-foreground font-mono overflow-x-auto pb-1">
                              {message.utp_metadata.trace.map((step: TraceStep, idx: number) => (
                                <div key={idx} className="flex items-center gap-1 flex-shrink-0">
                                  <div className="flex flex-col items-center">
                                    <span className={`font-semibold ${
                                      step.stage === 'Cache' && message.utp_metadata?.cache_hit
                                        ? 'text-green-600'
                                        : step.stage === 'Compress'
                                        ? 'text-blue-600'
                                        : step.stage === 'LLM'
                                        ? 'text-orange-600'
                                        : 'text-gray-600'
                                    }`}>
                                      {step.stage}
                                    </span>
                                    {step.size_bytes > 0 && (
                                      <span className="text-[9px] text-muted-foreground/70">
                                        {step.size_bytes}B
                                      </span>
                                    )}
                                    {step.duration_us > 0 && (
                                      <span className="text-[9px] text-muted-foreground/70">
                                        {step.duration_us >= 1000
                                          ? `${(step.duration_us / 1000).toFixed(1)}ms`
                                          : `${step.duration_us}µs`}
                                      </span>
                                    )}
                                  </div>
                                  {idx < (message.utp_metadata?.trace?.length ?? 0) - 1 && (
                                    <ArrowRight className="h-3 w-3 text-muted-foreground/50" />
                                  )}
                                </div>
                              ))}
                            </div>
                          )}

                          {/* Badges */}
                          <div className="flex flex-wrap gap-1.5">
                            {message.utp_metadata.used_utp ? (
                              <>
                                <Badge variant="outline" className="bg-blue-50 text-blue-700 border-blue-200 text-[10px] sm:text-xs">
                                  UTP Mode
                                </Badge>
                                {message.utp_metadata.cache_hit && (
                                  <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200 text-[10px] sm:text-xs">
                                    ⚡ Cache Hit
                                  </Badge>
                                )}
                                <Badge variant="outline" className="text-[10px] sm:text-xs">
                                  {message.utp_metadata.compression_ratio.toFixed(2)}x compression
                                </Badge>
                                <Badge variant="outline" className="text-[10px] sm:text-xs">
                                  {message.utp_metadata.compressed_size}B
                                  (saved {message.utp_metadata.original_size - message.utp_metadata.compressed_size}B)
                                </Badge>
                                <Badge variant="outline" className="text-[10px] sm:text-xs">
                                  {message.utp_metadata.precision_used}
                                </Badge>
                              </>
                            ) : (
                              <Badge variant="outline" className="bg-gray-50 text-gray-700 border-gray-200 text-[10px] sm:text-xs">
                                Traditional Mode
                              </Badge>
                            )}
                          </div>
                        </div>
                      )}
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
          );
        })}
        <div ref={scrollRef} />
      </div>
    </div>
  );
}
