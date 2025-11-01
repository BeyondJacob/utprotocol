"use client";

import { Plus, MessageSquare, Trash2, Clock } from "lucide-react";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";

export type Conversation = {
  id: number;
  title: string;
  model: string;
  created_at: string;
  updated_at: string;
};

interface ChatSidebarProps {
  conversations: Conversation[];
  activeConversationId: number | null;
  onSelectConversation: (id: number) => void;
  onNewConversation: () => void;
  onDeleteConversation: (id: number) => void;
}

export function ChatSidebar({
  conversations,
  activeConversationId,
  onSelectConversation,
  onNewConversation,
  onDeleteConversation,
}: ChatSidebarProps) {
  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const minutes = Math.floor(diff / (1000 * 60));

    if (minutes < 1) return "Just now";
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days === 0) return "Today";
    if (days === 1) return "Yesterday";
    if (days < 7) return `${days}d ago`;
    if (days < 30) return `${Math.floor(days / 7)}w ago`;
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  };

  // Group conversations by time period
  const groupConversations = () => {
    const now = new Date();
    const groups: { [key: string]: Conversation[] } = {
      Today: [],
      Yesterday: [],
      "Previous 7 Days": [],
      "Previous 30 Days": [],
      Older: [],
    };

    conversations.forEach((conv) => {
      const date = new Date(conv.updated_at);
      const diff = now.getTime() - date.getTime();
      const days = Math.floor(diff / (1000 * 60 * 60 * 24));

      if (days === 0) groups.Today.push(conv);
      else if (days === 1) groups.Yesterday.push(conv);
      else if (days < 7) groups["Previous 7 Days"].push(conv);
      else if (days < 30) groups["Previous 30 Days"].push(conv);
      else groups.Older.push(conv);
    });

    return groups;
  };

  const groupedConversations = groupConversations();

  return (
    <div className="w-full lg:w-72 flex-shrink-0 border-r bg-background flex flex-col h-full">
      {/* Header */}
      <div className="p-2.5 sm:p-3 border-b bg-muted/20">
        <Button
          onClick={onNewConversation}
          className="w-full justify-start gap-2 h-9 sm:h-10 font-medium shadow-sm hover:shadow-md transition-all text-sm"
          size="default"
        >
          <Plus className="h-4 w-4" />
          New Conversation
        </Button>
      </div>

      {/* Conversations List */}
      <ScrollArea className="flex-1">
        {conversations.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 px-6 text-center">
            <div className="rounded-full bg-muted/50 p-4 mb-4">
              <MessageSquare className="h-8 w-8 text-muted-foreground" />
            </div>
            <p className="text-sm font-medium text-muted-foreground mb-1">
              No conversations yet
            </p>
            <p className="text-xs text-muted-foreground/70">
              Start a new chat to begin
            </p>
          </div>
        ) : (
          <div className="py-2">
            {Object.entries(groupedConversations).map(([group, convs]) => {
              if (convs.length === 0) return null;

              return (
                <div key={group} className="mb-4">
                  {/* Group Header */}
                  <div className="px-3 sm:px-4 py-1.5 sm:py-2 mb-1">
                    <h3 className="text-[10px] sm:text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                      {group}
                    </h3>
                  </div>

                  {/* Group Items */}
                  <div className="space-y-0.5 px-1.5 sm:px-2">
                    {convs.map((conversation) => {
                      const isActive = activeConversationId === conversation.id;

                      return (
                        <div
                          key={conversation.id}
                          onClick={() => onSelectConversation(conversation.id)}
                          className={`
                            group relative flex items-start gap-2 sm:gap-3 p-2 sm:p-3 rounded-lg cursor-pointer
                            transition-all duration-200
                            ${
                              isActive
                                ? "bg-primary/10 shadow-sm border border-primary/20"
                                : "hover:bg-muted/50 border border-transparent"
                            }
                          `}
                        >
                          {/* Icon */}
                          <div className={`
                            flex-shrink-0 mt-0.5
                            ${isActive ? "text-primary" : "text-muted-foreground"}
                          `}>
                            <MessageSquare className="h-3.5 w-3.5 sm:h-4 sm:w-4" />
                          </div>

                          {/* Content */}
                          <div className="flex-1 min-w-0 overflow-hidden">
                            <p className={`
                              text-xs sm:text-sm font-medium truncate leading-tight mb-0.5 sm:mb-1
                              ${isActive ? "text-foreground" : "text-foreground/90"}
                            `}>
                              {conversation.title}
                            </p>
                            <div className="flex items-center gap-1 sm:gap-2 text-[10px] sm:text-xs text-muted-foreground">
                              <Clock className="h-2.5 w-2.5 sm:h-3 sm:w-3" />
                              <span>{formatDate(conversation.updated_at)}</span>
                            </div>
                          </div>

                          {/* Delete Button */}
                          <Button
                            variant="ghost"
                            size="icon"
                            className={`
                              absolute right-1 sm:right-2 top-1 sm:top-2 h-6 w-6 sm:h-7 sm:w-7
                              opacity-0 group-hover:opacity-100
                              transition-opacity duration-200
                              hover:bg-destructive/10 hover:text-destructive
                              flex-shrink-0
                            `}
                            onClick={(e) => {
                              e.stopPropagation();
                              onDeleteConversation(conversation.id);
                            }}
                          >
                            <Trash2 className="h-3 w-3 sm:h-3.5 sm:w-3.5" />
                          </Button>
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </ScrollArea>

      {/* Footer - Optional */}
      <div className="border-t p-3 bg-muted/10">
        <div className="text-xs text-muted-foreground text-center">
          {conversations.length} conversation{conversations.length !== 1 ? "s" : ""}
        </div>
      </div>
    </div>
  );
}
