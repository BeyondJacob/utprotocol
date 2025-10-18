"use client";

import { useState, useEffect } from "react";
import { MessageList } from "./message-list";
import { ChatInput } from "./chat-input";
import { ModelSelector } from "./model-selector";
import { ChatSidebar, type Conversation } from "./chat-sidebar";
import { Card, CardHeader, CardTitle, CardContent } from "./ui/card";

export type Message = {
  id: string;
  role: "user" | "assistant";
  content: string;
  model?: string;
  latency?: number;
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
  tokens_per_second: number | null;
  total_tokens: number | null;
  created_at: string;
};

export type ModelInfo = {
  name: string;
  display_name: string;
  provider: string;
  is_local: boolean;
  downloaded: boolean;
};

export function ChatInterface() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [currentModel, setCurrentModel] = useState<string>("gpt-oss:20b");
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<
    number | null
  >(null);

  // Fetch available models and conversations on mount
  useEffect(() => {
    fetchModels();
    fetchConversations();
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

  const fetchConversations = async () => {
    try {
      const response = await fetch("http://localhost:3001/conversations");
      const data = await response.json();
      setConversations(data);
    } catch (error) {
      console.error("Failed to fetch conversations:", error);
    }
  };

  const loadConversation = async (id: number) => {
    try {
      const response = await fetch(
        `http://localhost:3001/conversations/${id}`
      );
      const data = await response.json();

      const loadedMessages: Message[] = data.messages.map((msg: DbMessage) => ({
        id: msg.id.toString(),
        role: msg.role as "user" | "assistant",
        content: msg.content,
        model: msg.model ?? undefined,
        latency: msg.latency_ms ?? undefined,
        tokens_per_second: msg.tokens_per_second ?? undefined,
        total_tokens: msg.total_tokens ?? undefined,
      }));

      setMessages(loadedMessages);
      setActiveConversationId(id);
      setCurrentModel(data.conversation.model);
    } catch (error) {
      console.error("Failed to load conversation:", error);
    }
  };

  const createNewConversation = async () => {
    try {
      const response = await fetch("http://localhost:3001/conversations", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          title: "New Chat",
          model: currentModel,
        }),
      });

      const newConversation = await response.json();
      setConversations([newConversation, ...conversations]);
      setActiveConversationId(newConversation.id);
      setMessages([]);
    } catch (error) {
      console.error("Failed to create conversation:", error);
    }
  };

  const deleteConversation = async (id: number) => {
    try {
      await fetch(`http://localhost:3001/conversations/${id}`, {
        method: "DELETE",
      });

      setConversations(conversations.filter((c) => c.id !== id));

      if (activeConversationId === id) {
        setActiveConversationId(null);
        setMessages([]);
      }
    } catch (error) {
      console.error("Failed to delete conversation:", error);
    }
  };

  const updateConversationTitle = async (
    conversationId: number,
    firstMessage: string
  ) => {
    const title = firstMessage.slice(0, 50);
    // Update locally - title generation happens on first message
    setConversations((prev) =>
      prev.map((c) => (c.id === conversationId ? { ...c, title } : c))
    );
  };

  const sendMessage = async (text: string) => {
    // Create conversation if none is active
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
        setConversations([newConversation, ...conversations]);
        setActiveConversationId(conversationId);
      } catch (error) {
        console.error("Failed to create conversation:", error);
        return;
      }
    } else if (isFirstMessage) {
      await updateConversationTitle(conversationId, text);
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
        tokens_per_second: data.tokens_per_second,
        total_tokens: data.total_tokens,
      };

      setMessages((prev) => [...prev, assistantMessage]);

      // Refresh conversations to update timestamp
      await fetchConversations();
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

  const getCurrentConversationTitle = () => {
    if (activeConversationId) {
      const conversation = conversations.find((c) => c.id === activeConversationId);
      return conversation?.title || "New Conversation";
    }
    return "New Conversation";
  };

  return (
    <div className="h-full flex flex-col">
      {/* Main App Header */}
      <div className="border-b bg-card px-6 py-4">
        <h1 className="text-3xl font-bold tracking-tight">
          Universal Thought Protocol
        </h1>
      </div>

      {/* Chat Area */}
      <div className="flex-1 flex overflow-hidden w-full">
        <ChatSidebar
          conversations={conversations}
          activeConversationId={activeConversationId}
          onSelectConversation={loadConversation}
          onNewConversation={createNewConversation}
          onDeleteConversation={deleteConversation}
        />
        <Card className="flex-1 flex flex-col border-l-0 rounded-l-none border-t-0 min-w-0 overflow-hidden">
          <CardHeader className="border-b">
            <div className="flex items-center justify-between gap-4 min-w-0">
              <CardTitle className="text-lg font-semibold truncate flex-shrink min-w-0">
                {getCurrentConversationTitle()}
              </CardTitle>
              <div className="flex-shrink-0 min-w-[200px] max-w-[320px] w-auto">
                <ModelSelector
                  models={availableModels}
                  currentModel={currentModel}
                  onModelChange={switchModel}
                  onDownloadModel={downloadModel}
                  onDeleteModel={deleteModel}
                />
              </div>
            </div>
          </CardHeader>
          <CardContent className="flex-1 flex flex-col overflow-hidden p-0 min-w-0">
            <MessageList messages={messages} />
            <ChatInput onSendMessage={sendMessage} isLoading={isLoading} />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
