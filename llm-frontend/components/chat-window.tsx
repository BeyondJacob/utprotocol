"use client";

import { useState, useEffect, useMemo } from "react";
import { MessageList } from "./message-list";
import { ChatInput } from "./chat-input";
import { ModelSelector } from "./model-selector";
import { Card, CardHeader, CardTitle, CardContent } from "./ui/card";
import { Button } from "./ui/button";
import { X, Maximize2, Minimize2 } from "lucide-react";

export type Message = {
  id: string;
  role: "user" | "assistant";
  content: string;
  model?: string;
  latency?: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  tokens_per_second?: number;
  total_tokens?: number;
};

type DbMessage = {
  id: number;
  conversation_id: number;
  role: string;
  content: string;
  model: string | null;
  latency_ms: number | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  tokens_per_second: number | null;
  total_tokens: number | null;
  created_at: string;
};

export type ModelPricing = {
  input_per_million: number;
  output_per_million: number;
};

export type ModelInfo = {
  name: string;
  display_name: string;
  provider: string;
  is_local: boolean;
  downloaded: boolean;
  pricing?: ModelPricing;
};

// Utility function to calculate cost
export function calculateCost(
  tokens: number,
  costPerMillion: number
): number {
  return (tokens / 1_000_000) * costPerMillion;
}

// Utility function to format cost
export function formatCost(cost: number): string {
  if (cost < 0.0001) {
    return `$${cost.toFixed(6)}`;
  } else if (cost < 0.01) {
    return `$${cost.toFixed(4)}`;
  }
  return `$${cost.toFixed(2)}`;
}

interface ChatWindowProps {
  windowId: string;
  conversationId: number | null;
  onClose: () => void;
  availableModels: ModelInfo[];
  onConversationChange?: (id: number) => void;
  isMaximized?: boolean;
  onToggleMaximize?: () => void;
}

export function ChatWindow({
  windowId,
  conversationId: initialConversationId,
  onClose,
  availableModels,
  onConversationChange,
  isMaximized = false,
  onToggleMaximize,
}: ChatWindowProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [currentModel, setCurrentModel] = useState<string>("gpt-oss:20b");
  const [isLoading, setIsLoading] = useState(false);
  const [activeConversationId, setActiveConversationId] = useState<number | null>(
    initialConversationId
  );
  const [conversationTitle, setConversationTitle] = useState<string>("New Conversation");

  useEffect(() => {
    if (initialConversationId && initialConversationId !== activeConversationId) {
      loadConversation(initialConversationId);
    }
  }, [initialConversationId]);

  const loadConversation = async (id: number) => {
    try {
      const response = await fetch(`http://localhost:3001/conversations/${id}`);
      const data = await response.json();

      const loadedMessages: Message[] = data.messages.map((msg: DbMessage) => ({
        id: msg.id.toString(),
        role: msg.role as "user" | "assistant",
        content: msg.content,
        model: msg.model ?? undefined,
        latency: msg.latency_ms ?? undefined,
        prompt_tokens: msg.prompt_tokens ?? undefined,
        completion_tokens: msg.completion_tokens ?? undefined,
        tokens_per_second: msg.tokens_per_second ?? undefined,
        total_tokens: msg.total_tokens ?? undefined,
      }));

      setMessages(loadedMessages);
      setActiveConversationId(id);
      setCurrentModel(data.conversation.model);
      setConversationTitle(data.conversation.title);
      onConversationChange?.(id);
    } catch (error) {
      console.error("Failed to load conversation:", error);
    }
  };

  const sendMessage = async (text: string) => {
    let conversationId = activeConversationId;
    const isFirstMessage = messages.length === 0;

    if (!conversationId) {
      try {
        const response = await fetch("http://localhost:3001/conversations", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            title: text.slice(0, 50),
            model: currentModel,
          }),
        });

        const newConversation = await response.json();
        conversationId = newConversation.id;
        setActiveConversationId(conversationId);
        setConversationTitle(newConversation.title);
        onConversationChange?.(conversationId);
      } catch (error) {
        console.error("Failed to create conversation:", error);
        return;
      }
    } else if (isFirstMessage) {
      setConversationTitle(text.slice(0, 50));
    }

    const userMessage: Message = {
      id: Date.now().toString(),
      role: "user",
      content: text,
    };

    setMessages((prev) => [...prev, userMessage]);
    setIsLoading(true);

    try {
      const response = await fetch("http://localhost:3001/chat", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: currentModel,
          message: text,
          conversation_id: conversationId,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();

      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content: data.response,
        model: data.model,
        latency: data.latency_ms,
        prompt_tokens: data.prompt_tokens,
        completion_tokens: data.completion_tokens,
        tokens_per_second: data.tokens_per_second,
        total_tokens: data.total_tokens,
      };

      setMessages((prev) => [...prev, assistantMessage]);
    } catch (error) {
      console.error("Failed to send message:", error);
      const errorMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content: "Sorry, I encountered an error processing your message.",
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
    }
  };

  const switchModel = async (modelName: string) => {
    try {
      await fetch("http://localhost:3001/switch-model", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: modelName,
        }),
      });
      setCurrentModel(modelName);
    } catch (error) {
      console.error("Failed to switch model:", error);
    }
  };

  const downloadModel = async (modelName: string) => {
    try {
      await fetch("http://localhost:3001/download-model", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: modelName,
        }),
      });
    } catch (error) {
      console.error("Failed to download model:", error);
    }
  };

  const deleteModel = async (modelName: string) => {
    try {
      await fetch("http://localhost:3001/delete-model", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: modelName,
        }),
      });
    } catch (error) {
      console.error("Failed to delete model:", error);
    }
  };

  // Calculate running totals for cloud models
  const runningTotals = useMemo(() => {
    const currentModelInfo = availableModels.find(m => m.name === currentModel);

    // Only show running totals for cloud models with pricing
    if (!currentModelInfo || currentModelInfo.is_local || !currentModelInfo.pricing) {
      return null;
    }

    let totalPromptTokens = 0;
    let totalCompletionTokens = 0;
    let totalInputCost = 0;
    let totalOutputCost = 0;

    messages.forEach(msg => {
      // Only count messages from the current model
      if (msg.model === currentModel && currentModelInfo.pricing) {
        if (msg.prompt_tokens) {
          totalPromptTokens += msg.prompt_tokens;
          totalInputCost += calculateCost(msg.prompt_tokens, currentModelInfo.pricing.input_per_million);
        }
        if (msg.completion_tokens) {
          totalCompletionTokens += msg.completion_tokens;
          totalOutputCost += calculateCost(msg.completion_tokens, currentModelInfo.pricing.output_per_million);
        }
      }
    });

    const totalCost = totalInputCost + totalOutputCost;

    // Only return if there are any tokens
    if (totalPromptTokens === 0 && totalCompletionTokens === 0) {
      return null;
    }

    return {
      promptTokens: totalPromptTokens,
      completionTokens: totalCompletionTokens,
      inputCost: totalInputCost,
      outputCost: totalOutputCost,
      totalCost,
    };
  }, [messages, currentModel, availableModels]);

  return (
    <Card className="flex-1 flex flex-col border min-w-0 overflow-hidden h-full shadow-lg">
      <CardHeader className="border-b px-3 py-2 md:px-4 md:py-3 bg-muted/20">
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-2 min-w-0">
            <CardTitle className="text-xs md:text-sm font-semibold truncate flex-1 min-w-0">
              {conversationTitle}
            </CardTitle>

            <div className="flex items-center gap-1 flex-shrink-0">
            <div className="min-w-[120px] sm:min-w-[140px] md:min-w-[160px] max-w-[180px]">
              <ModelSelector
                models={availableModels}
                currentModel={currentModel}
                onModelChange={switchModel}
                onDownloadModel={downloadModel}
                onDeleteModel={deleteModel}
              />
            </div>

            {onToggleMaximize && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 flex-shrink-0"
                onClick={onToggleMaximize}
                title={isMaximized ? "Restore" : "Maximize"}
              >
                {isMaximized ? (
                  <Minimize2 className="h-3.5 w-3.5" />
                ) : (
                  <Maximize2 className="h-3.5 w-3.5" />
                )}
              </Button>
            )}

            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 flex-shrink-0 hover:bg-destructive/10 hover:text-destructive"
              onClick={onClose}
              title="Close window"
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {/* Running totals for cloud models */}
        {runningTotals && (
          <div className="pt-2 border-t border-muted-foreground/10">
            <div className="text-[10px] sm:text-xs text-muted-foreground flex items-center gap-1.5 sm:gap-2 flex-wrap">
              <span className="whitespace-nowrap">
                In: {runningTotals.promptTokens.toLocaleString()} tok
                <span className="ml-1 opacity-80">({formatCost(runningTotals.inputCost)})</span>
              </span>
              <span>•</span>
              <span className="whitespace-nowrap">
                Out: {runningTotals.completionTokens.toLocaleString()} tok
                <span className="ml-1 opacity-80">({formatCost(runningTotals.outputCost)})</span>
              </span>
              <span>•</span>
              <span className="whitespace-nowrap font-medium">
                Total: {formatCost(runningTotals.totalCost)}
              </span>
            </div>
          </div>
        )}
        </div>
      </CardHeader>

      <CardContent className="flex-1 flex flex-col overflow-hidden p-0 min-w-0">
        <MessageList messages={messages} availableModels={availableModels} />
        <ChatInput onSendMessage={sendMessage} isLoading={isLoading} />
      </CardContent>
    </Card>
  );
}
