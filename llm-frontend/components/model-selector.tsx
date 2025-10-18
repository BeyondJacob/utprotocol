"use client";

import { Check, X, Download, Trash2, Network, Cloud } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";
import type { ModelInfo } from "./chat-interface";
import Image from "next/image";

interface ModelSelectorProps {
  models: ModelInfo[];
  currentModel: string;
  onModelChange: (model: string) => void;
  onDownloadModel: (model: string) => void;
  onDeleteModel: (model: string) => void;
}

const ProviderIcon = ({ provider, isLocal }: { provider: string; isLocal: boolean }) => {
  if (provider === "groq") {
    return (
      <Image
        src="/groq-logo.svg"
        alt="Groq"
        width={16}
        height={16}
        className="opacity-70"
      />
    );
  }

  if (isLocal) {
    return <Network className="h-4 w-4 opacity-70" />;
  }

  return <Cloud className="h-4 w-4 opacity-70" />;
};

export function ModelSelector({
  models,
  currentModel,
  onModelChange,
  onDownloadModel,
  onDeleteModel,
}: ModelSelectorProps) {
  const getCurrentModelDisplayName = () => {
    const model = models.find((m) => m.name === currentModel);
    return model?.display_name || currentModel;
  };

  return (
    <Select value={currentModel} onValueChange={onModelChange}>
      <SelectTrigger className="w-full text-xs sm:text-sm">
        <SelectValue placeholder="Select a model">
          <div className="flex items-center gap-1.5 sm:gap-2">
            {models.find((m) => m.name === currentModel) && (
              <ProviderIcon
                provider={models.find((m) => m.name === currentModel)!.provider}
                isLocal={models.find((m) => m.name === currentModel)!.is_local}
              />
            )}
            <span className="truncate">{getCurrentModelDisplayName()}</span>
          </div>
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="max-w-[90vw] sm:max-w-none">
        {models.map((model) => (
          <SelectItem
            key={model.name}
            value={model.name}
            className="group relative cursor-pointer [&>span:first-child]:hidden text-xs sm:text-sm"
          >
            <div className="flex items-center justify-between w-full pr-8">
              <div className="flex items-center gap-1.5 sm:gap-2 min-w-0">
                <ProviderIcon provider={model.provider} isLocal={model.is_local} />
                <span className="font-medium truncate">{model.display_name}</span>
              </div>
              <div className="absolute right-2 flex items-center gap-1 z-10">
                {model.is_local ? (
                  model.downloaded ? (
                    <button
                      className="p-0.5 sm:p-1 rounded hover:bg-accent transition-colors"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteModel(model.name);
                      }}
                      title="Delete model"
                    >
                      <Check className="h-3.5 w-3.5 sm:h-4 sm:w-4 text-green-500 group-hover:hidden" />
                      <Trash2 className="h-3.5 w-3.5 sm:h-4 sm:w-4 text-red-500 hidden group-hover:block" />
                    </button>
                  ) : (
                    <button
                      className="p-0.5 sm:p-1 rounded hover:bg-accent transition-colors"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDownloadModel(model.name);
                      }}
                      title="Download model"
                    >
                      <X className="h-3.5 w-3.5 sm:h-4 sm:w-4 text-red-500 group-hover:hidden" />
                      <Download className="h-3.5 w-3.5 sm:h-4 sm:w-4 text-blue-500 hidden group-hover:block" />
                    </button>
                  )
                ) : (
                  // Cloud models show checkmark if API is configured
                  model.downloaded && (
                    <Check className="h-3.5 w-3.5 sm:h-4 sm:w-4 text-green-500" />
                  )
                )}
              </div>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
