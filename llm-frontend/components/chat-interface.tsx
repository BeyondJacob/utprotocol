"use client";

import { useState, useEffect } from "react";
import { MessageList } from "./message-list";
import { ChatInput } from "./chat-input";
import { ModelSelector } from "./model-selector";
import { Card, CardHeader, CardTitle, CardContent } from "./ui/card";

export type Message = {
  id: string;
  role: "user" | "assistant";
  content: string;
  model?: string;
  latency?: number;
};

export type ModelInfo = {
  name: string;
  downloaded: boolean;
};

export function ChatInterface() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [currentModel, setCurrentModel] = useState<string>("gpt-oss:20b");
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  // Fetch available models on mount
  useEffect(() => {
    fetchModels();
  }, []);

  const fetchModels = async () => {
    try {
      const response = await fetch("http://localhost:3001/models");
      const data = await response.json();
      setAvailableModels(data.models);
      setCurrentModel(data.active_model);
    } catch (error) {
      console.error("Failed to fetch models:", error);
    }
  };

  const sendMessage = async (text: string) => {
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
      // Refresh models list to update download status
      await fetchModels();
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
      // Refresh models list to update download status
      await fetchModels();
    } catch (error) {
      console.error("Failed to delete model:", error);
    }
  };

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
        <CardTitle className="text-2xl font-bold">
          Universal Thought Protocol
        </CardTitle>
        <ModelSelector
          models={availableModels}
          currentModel={currentModel}
          onModelChange={switchModel}
          onDownloadModel={downloadModel}
          onDeleteModel={deleteModel}
        />
      </CardHeader>
      <CardContent className="flex-1 flex flex-col overflow-hidden">
        <MessageList messages={messages} />
        <ChatInput onSendMessage={sendMessage} isLoading={isLoading} />
      </CardContent>
    </Card>
  );
}
