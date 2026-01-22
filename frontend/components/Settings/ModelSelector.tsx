import { Check, ChevronRight, ChevronsUpDown } from "lucide-react";
import { type JSX, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import type { ReasoningEffort } from "@/lib/ai";
import { type ModelEntry, PROVIDER_GROUPS_NESTED, type ProviderGroupNested } from "@/lib/models";
import type { AiProvider, AiSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";

interface ModelSelectorProps {
  provider: AiProvider;
  model: string;
  reasoningEffort?: ReasoningEffort;
  settings: AiSettings;
  onChange: (provider: AiProvider, model: string, reasoningEffort?: ReasoningEffort) => void;
}

function isProviderAvailable(settings: AiSettings, providerId: AiProvider): boolean {
  // Check if provider is configured and show_in_selector is enabled (defaults to true)
  switch (providerId) {
    case "vertex_ai":
      return (
        settings.vertex_ai.show_in_selector !== false &&
        !!(settings.vertex_ai.credentials_path || settings.vertex_ai.project_id)
      );
    case "anthropic":
      return settings.anthropic.show_in_selector !== false && !!settings.anthropic.api_key;
    case "openai":
      return settings.openai.show_in_selector !== false && !!settings.openai.api_key;
    case "openrouter":
      return settings.openrouter.show_in_selector !== false && !!settings.openrouter.api_key;
    case "ollama":
      return settings.ollama.show_in_selector !== false && !!settings.ollama.base_url;
    case "gemini":
      return settings.gemini.show_in_selector !== false && !!settings.gemini.api_key;
    case "groq":
      return settings.groq.show_in_selector !== false && !!settings.groq.api_key;
    case "xai":
      return settings.xai.show_in_selector !== false && !!settings.xai.api_key;
    case "zai":
      return settings.zai.show_in_selector !== false && !!settings.zai.api_key;
    case "zai_anthropic":
      return settings.zai_anthropic.show_in_selector !== false && !!settings.zai_anthropic.api_key;
    default:
      return false;
  }
}

/** Find current model display info from nested structure */
function findCurrentModelDisplay(
  groups: ProviderGroupNested[],
  provider: AiProvider,
  modelId: string,
  reasoningEffort?: ReasoningEffort
): { groupName: string; modelName: string; icon: string } | null {
  const group = groups.find((g) => g.provider === provider);
  if (!group) return null;

  for (const entry of group.models) {
    // Simple model
    if (entry.id === modelId) {
      return { groupName: group.providerName, modelName: entry.name, icon: group.icon };
    }
    // Model with sub-options
    if (entry.subModels) {
      const subModel = entry.subModels.find(
        (s) => s.id === modelId && (!reasoningEffort || s.reasoningEffort === reasoningEffort)
      );
      if (subModel) {
        const effortLabel = subModel.reasoningEffort
          ? ` (${subModel.reasoningEffort.charAt(0).toUpperCase()}${subModel.reasoningEffort.slice(1)})`
          : "";
        return {
          groupName: group.providerName,
          modelName: `${entry.name}${effortLabel}`,
          icon: group.icon,
        };
      }
    }
  }
  return null;
}

export function ModelSelector({
  provider,
  model,
  reasoningEffort,
  settings,
  onChange,
}: ModelSelectorProps) {
  const [open, setOpen] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());

  // Filter to only configured providers with show_in_selector enabled
  const availableProviders = PROVIDER_GROUPS_NESTED.filter((g) =>
    isProviderAvailable(settings, g.provider)
  );

  // Find current selection display info
  const currentDisplay = findCurrentModelDisplay(
    PROVIDER_GROUPS_NESTED,
    provider,
    model,
    reasoningEffort
  );

  const toggleGroup = (groupKey: string) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupKey)) {
        next.delete(groupKey);
      } else {
        next.add(groupKey);
      }
      return next;
    });
  };

  const handleSelect = (
    selectedProvider: AiProvider,
    modelId: string,
    effort?: ReasoningEffort
  ) => {
    onChange(selectedProvider, modelId, effort);
    setOpen(false);
  };

  // Helper to check if any nested model is selected (recursive)
  const isAnyNestedSelected = (groupProvider: AiProvider, entries: ModelEntry[]): boolean => {
    return entries.some((e) => {
      if (e.id) {
        return (
          provider === groupProvider && model === e.id && reasoningEffort === e.reasoningEffort
        );
      }
      if (e.subModels) {
        return isAnyNestedSelected(groupProvider, e.subModels);
      }
      return false;
    });
  };

  // Recursive renderer for model entries with indentation support
  const renderSubEntry = (
    group: ProviderGroupNested,
    entry: ModelEntry,
    parentKey: string,
    depth: number
  ): JSX.Element | null => {
    const entryKey = `${parentKey}:${entry.name}`;

    // Entry with sub-options (expandable)
    if (entry.subModels && entry.subModels.length > 0) {
      const isExpanded = expandedGroups.has(entryKey);
      const isSubSelected = isAnyNestedSelected(group.provider, entry.subModels);

      return (
        <div key={entryKey}>
          <CommandItem
            value={`${group.providerName} ${entry.name}`}
            onSelect={() => toggleGroup(entryKey)}
            className="flex items-center justify-between cursor-pointer"
          >
            <span className="flex items-center gap-3">
              <span
                className={cn(
                  "w-4 h-4 flex items-center justify-center",
                  isSubSelected ? "opacity-100" : "opacity-0"
                )}
              >
                <Check className="h-4 w-4" />
              </span>
              <span>{entry.name}</span>
            </span>
            <ChevronRight
              className={cn("h-4 w-4 transition-transform", isExpanded && "rotate-90")}
            />
          </CommandItem>
          {isExpanded && (
            <div className="ml-6 border-l border-[var(--color-border-medium)] pl-2">
              {entry.subModels.map((sub) => renderSubEntry(group, sub, entryKey, depth + 1))}
            </div>
          )}
        </div>
      );
    }

    // Leaf model (selectable)
    if (!entry.id) return null;
    const entryId = entry.id;
    const isSelected =
      provider === group.provider && model === entryId && reasoningEffort === entry.reasoningEffort;

    return (
      <CommandItem
        key={`${entryKey}:${entryId}:${entry.reasoningEffort ?? "default"}`}
        value={`${group.providerName} ${entry.name}`}
        onSelect={() => handleSelect(group.provider, entryId, entry.reasoningEffort)}
        className="flex items-center justify-between py-1"
      >
        <span className="flex items-center gap-3">
          <span
            className={cn(
              "w-4 h-4 flex items-center justify-center",
              isSelected ? "opacity-100" : "opacity-0"
            )}
          >
            <Check className="h-4 w-4" />
          </span>
          <span className={depth > 0 ? "text-sm" : ""}>{entry.name}</span>
        </span>
      </CommandItem>
    );
  };

  const renderModelEntry = (group: ProviderGroupNested, entry: ModelEntry, index: number) => {
    return renderSubEntry(group, entry, `${group.provider}:${index}`, 0);
  };

  return (
    <Popover open={open} onOpenChange={setOpen} modal={false}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between h-10 bg-background border-[var(--color-border-medium)] hover:bg-[var(--color-bg-hover)]"
        >
          <span className="flex items-center gap-2 truncate">
            {currentDisplay && (
              <>
                <span className="text-base">{currentDisplay.icon}</span>
                <span className="text-muted-foreground">{currentDisplay.groupName}</span>
                <span className="text-muted-foreground">/</span>
                <span className="text-foreground">{currentDisplay.modelName}</span>
              </>
            )}
            {!currentDisplay && <span className="text-muted-foreground">Select a model...</span>}
          </span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[400px] p-0" align="start">
        <Command>
          <CommandInput placeholder="Search models..." />
          <CommandList
            style={{
              maxHeight: "400px",
              overflowY: "scroll",
              overscrollBehavior: "contain",
            }}
            onWheel={(e) => {
              e.stopPropagation();
              e.currentTarget.scrollTop += e.deltaY;
            }}
          >
            <CommandEmpty>
              {availableProviders.length === 0
                ? "No providers available. Configure a provider and enable 'Show in model selector' below."
                : "No model found."}
            </CommandEmpty>
            {availableProviders.map((group) => (
              <CommandGroup
                key={group.provider}
                heading={
                  <span className="flex items-center gap-2">
                    <span>{group.icon}</span>
                    <span>{group.providerName}</span>
                  </span>
                }
              >
                {group.models.map((entry, index) => renderModelEntry(group, entry, index))}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
