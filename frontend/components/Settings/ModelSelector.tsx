import { Check, ChevronsUpDown } from "lucide-react";
import { useState } from "react";
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
import { PROVIDER_GROUPS } from "@/lib/models";
import type { AiProvider, AiSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";

interface ModelSelectorProps {
  provider: AiProvider;
  model: string;
  settings: AiSettings;
  onChange: (provider: AiProvider, model: string) => void;
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
    default:
      return false;
  }
}

export function ModelSelector({ provider, model, settings, onChange }: ModelSelectorProps) {
  const [open, setOpen] = useState(false);

  // Filter to only configured providers with show_in_selector enabled
  const availableProviders = PROVIDER_GROUPS.filter((g) =>
    isProviderAvailable(settings, g.provider)
  );

  // Find current selection display info
  const currentGroup = PROVIDER_GROUPS.find((g) => g.provider === provider);
  const currentModel = currentGroup?.models.find((m) => m.id === model);

  return (
    <Popover open={open} onOpenChange={setOpen} modal={false}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between h-10 bg-background border-[var(--border-medium)] hover:bg-[var(--bg-hover)]"
        >
          <span className="flex items-center gap-2 truncate">
            {currentGroup && (
              <>
                <span className="text-base">{currentGroup.icon}</span>
                <span className="text-muted-foreground">{currentGroup.providerName}</span>
                <span className="text-muted-foreground">/</span>
                <span className="text-foreground">{currentModel?.name || model}</span>
              </>
            )}
            {!currentGroup && <span className="text-muted-foreground">Select a model...</span>}
          </span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[400px] p-0" align="start">
        <Command>
          <CommandInput placeholder="Search models..." />
          <CommandList
            style={{
              maxHeight: "300px",
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
                {group.models.map((m) => {
                  const isSelected = provider === group.provider && model === m.id;
                  return (
                    <CommandItem
                      key={`${group.provider}:${m.id}`}
                      value={`${group.providerName} ${m.name}`}
                      onSelect={() => {
                        onChange(group.provider, m.id);
                        setOpen(false);
                      }}
                      className="flex items-center justify-between"
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
                        <span>{m.name}</span>
                      </span>
                    </CommandItem>
                  );
                })}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
