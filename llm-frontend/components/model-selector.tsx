"use client";

import { Check, X, Download, Trash2 } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";
import type { ModelInfo } from "./chat-interface";

interface ModelSelectorProps {
  models: ModelInfo[];
  currentModel: string;
  onModelChange: (model: string) => void;
  onDownloadModel: (model: string) => void;
  onDeleteModel: (model: string) => void;
}

export function ModelSelector({
  models,
  currentModel,
  onModelChange,
  onDownloadModel,
  onDeleteModel,
}: ModelSelectorProps) {
  return (
    <Select value={currentModel} onValueChange={onModelChange}>
      <SelectTrigger className="w-[280px]">
        <SelectValue placeholder="Select a model" />
      </SelectTrigger>
      <SelectContent>
        {models.map((model) => (
          <SelectItem
            key={model.name}
            value={model.name}
            className="group relative cursor-pointer [&>span:first-child]:hidden"
          >
            <div className="flex items-center justify-between w-full pr-8">
              <span>{model.name}</span>
              <div className="absolute right-2 flex items-center gap-1 z-10">
                {model.downloaded ? (
                  <button
                    className="p-1 rounded hover:bg-accent transition-colors"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteModel(model.name);
                    }}
                    title="Delete model"
                  >
                    <Check className="h-4 w-4 text-green-500 group-hover:hidden" />
                    <Trash2 className="h-4 w-4 text-red-500 hidden group-hover:block" />
                  </button>
                ) : (
                  <button
                    className="p-1 rounded hover:bg-accent transition-colors"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDownloadModel(model.name);
                    }}
                    title="Download model"
                  >
                    <X className="h-4 w-4 text-red-500 group-hover:hidden" />
                    <Download className="h-4 w-4 text-blue-500 hidden group-hover:block" />
                  </button>
                )}
              </div>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
