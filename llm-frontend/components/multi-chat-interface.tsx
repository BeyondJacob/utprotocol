"use client";

import { useState, useEffect } from "react";
import { ChatWindow, type ModelInfo } from "./chat-window";
import { ChatSidebar, type Conversation } from "./chat-sidebar";
import { Button } from "./ui/button";
import { Sheet, SheetContent, SheetTrigger } from "./ui/sheet";
import { Menu, Plus, Columns2, Columns3, Square } from "lucide-react";

type ChatWindowState = {
  id: string;
  conversationId: number | null;
};

type LayoutMode = "single" | "dual" | "triple";

export function MultiChatInterface() {
  const [windows, setWindows] = useState<ChatWindowState[]>([
    { id: "window-1", conversationId: null },
  ]);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>("single");
  const [maximizedWindow, setMaximizedWindow] = useState<string | null>(null);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([]);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);

  useEffect(() => {
    fetchModels();
    fetchConversations();
  }, []);

  const fetchModels = async () => {
    try {
      const response = await fetch("http://localhost:3001/models");
      const data = await response.json();
      setAvailableModels(data.models);
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

  const addWindow = () => {
    if (windows.length >= 3) return;

    const newWindow: ChatWindowState = {
      id: `window-${Date.now()}`,
      conversationId: null,
    };

    setWindows([...windows, newWindow]);

    // Auto-adjust layout
    if (windows.length === 0) setLayoutMode("single");
    else if (windows.length === 1) setLayoutMode("dual");
    else if (windows.length === 2) setLayoutMode("triple");
  };

  const removeWindow = (windowId: string) => {
    const newWindows = windows.filter((w) => w.id !== windowId);

    // Ensure at least one window remains
    if (newWindows.length === 0) {
      setWindows([{ id: `window-${Date.now()}`, conversationId: null }]);
      setLayoutMode("single");
    } else {
      setWindows(newWindows);
      // Auto-adjust layout
      if (newWindows.length === 1) setLayoutMode("single");
      else if (newWindows.length === 2) setLayoutMode("dual");
    }

    // Clear maximized state if the maximized window was removed
    if (maximizedWindow === windowId) {
      setMaximizedWindow(null);
    }
  };

  const loadConversationInWindow = (conversationId: number, windowId?: string) => {
    // If no window specified, use first available or create new
    if (!windowId) {
      const availableWindow = windows.find((w) => w.conversationId === null);

      if (availableWindow) {
        setWindows(
          windows.map((w) =>
            w.id === availableWindow.id ? { ...w, conversationId } : w
          )
        );
      } else if (windows.length < 3) {
        const newWindow: ChatWindowState = {
          id: `window-${Date.now()}`,
          conversationId,
        };
        setWindows([...windows, newWindow]);
        if (windows.length === 1) setLayoutMode("dual");
        else if (windows.length === 2) setLayoutMode("triple");
      } else {
        // Replace last window
        setWindows(
          windows.map((w, i) =>
            i === windows.length - 1 ? { ...w, conversationId } : w
          )
        );
      }
    } else {
      setWindows(
        windows.map((w) => (w.id === windowId ? { ...w, conversationId } : w))
      );
    }

    setIsSidebarOpen(false);
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
          model: "gpt-oss:20b",
        }),
      });

      const newConversation = await response.json();
      setConversations([newConversation, ...conversations]);
      loadConversationInWindow(newConversation.id);
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

      // Clear windows that had this conversation
      setWindows(
        windows.map((w) =>
          w.conversationId === id ? { ...w, conversationId: null } : w
        )
      );
    } catch (error) {
      console.error("Failed to delete conversation:", error);
    }
  };

  const toggleMaximize = (windowId: string) => {
    setMaximizedWindow(maximizedWindow === windowId ? null : windowId);
  };

  const getLayoutClasses = () => {
    if (maximizedWindow) {
      return "grid grid-cols-1";
    }

    switch (layoutMode) {
      case "single":
        return "grid grid-cols-1";
      case "dual":
        return "grid grid-cols-1 lg:grid-cols-2 gap-2 md:gap-3";
      case "triple":
        return "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2 md:gap-3";
      default:
        return "grid grid-cols-1";
    }
  };

  const SidebarContent = () => (
    <ChatSidebar
      conversations={conversations}
      activeConversationId={null}
      onSelectConversation={(id) => loadConversationInWindow(id)}
      onNewConversation={createNewConversation}
      onDeleteConversation={deleteConversation}
    />
  );

  const visibleWindows = maximizedWindow
    ? windows.filter((w) => w.id === maximizedWindow)
    : windows;

  return (
    <div className="h-full flex flex-col bg-background">
      {/* Main App Header */}
      <div className="border-b bg-card px-3 py-2 md:px-4 md:py-3 flex-shrink-0">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 md:gap-3 min-w-0 flex-1">
            {/* Mobile Menu Button */}
            <Sheet open={isSidebarOpen} onOpenChange={setIsSidebarOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="lg:hidden flex-shrink-0 h-8 w-8">
                  <Menu className="h-4 w-4" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="p-0 w-[280px] sm:w-[320px] gap-0">
                <div className="h-full pt-12">
                  <SidebarContent />
                </div>
              </SheetContent>
            </Sheet>

            <h1 className="text-base sm:text-lg md:text-xl lg:text-2xl font-bold tracking-tight truncate">
              Universal Thought Protocol
            </h1>
          </div>

          {/* Layout Controls */}
          <div className="flex items-center gap-1 flex-shrink-0">
            {!maximizedWindow && (
              <>
                <Button
                  variant={layoutMode === "single" ? "default" : "ghost"}
                  size="icon"
                  className="h-8 w-8 hidden sm:flex"
                  onClick={() => {
                    setLayoutMode("single");
                    if (windows.length > 1) {
                      setWindows([windows[0]]);
                    }
                  }}
                  title="Single window"
                >
                  <Square className="h-4 w-4" />
                </Button>

                <Button
                  variant={layoutMode === "dual" ? "default" : "ghost"}
                  size="icon"
                  className="h-8 w-8 hidden sm:flex"
                  onClick={() => {
                    setLayoutMode("dual");
                    if (windows.length === 1) {
                      addWindow();
                    } else if (windows.length > 2) {
                      setWindows(windows.slice(0, 2));
                    }
                  }}
                  title="Dual windows"
                >
                  <Columns2 className="h-4 w-4" />
                </Button>

                <Button
                  variant={layoutMode === "triple" ? "default" : "ghost"}
                  size="icon"
                  className="h-8 w-8 hidden md:flex"
                  onClick={() => {
                    setLayoutMode("triple");
                    const currentLength = windows.length;
                    if (currentLength < 3) {
                      const newWindows = [...windows];
                      for (let i = currentLength; i < 3; i++) {
                        newWindows.push({
                          id: `window-${Date.now()}-${i}`,
                          conversationId: null,
                        });
                      }
                      setWindows(newWindows);
                    }
                  }}
                  title="Triple windows"
                >
                  <Columns3 className="h-4 w-4" />
                </Button>
              </>
            )}

            <Button
              variant="default"
              size="sm"
              className="h-8 gap-1.5 text-xs md:text-sm"
              onClick={addWindow}
              disabled={windows.length >= 3 || maximizedWindow !== null}
              title="Add new window"
            >
              <Plus className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">New Window</span>
            </Button>
          </div>
        </div>
      </div>

      {/* Chat Area */}
      <div className="flex-1 flex overflow-hidden w-full">
        {/* Desktop Sidebar */}
        <div className="hidden lg:block flex-shrink-0">
          <SidebarContent />
        </div>

        {/* Chat Windows Container */}
        <div className={`flex-1 overflow-auto p-2 md:p-3 ${getLayoutClasses()}`}>
          {visibleWindows.map((window) => (
            <div
              key={window.id}
              className="min-h-0 h-full animate-in fade-in-50 duration-300"
            >
              <ChatWindow
                windowId={window.id}
                conversationId={window.conversationId}
                onClose={() => removeWindow(window.id)}
                availableModels={availableModels}
                onConversationChange={(id) => {
                  setWindows(
                    windows.map((w) => (w.id === window.id ? { ...w, conversationId: id } : w))
                  );
                  fetchConversations();
                }}
                isMaximized={maximizedWindow === window.id}
                onToggleMaximize={() => toggleMaximize(window.id)}
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
