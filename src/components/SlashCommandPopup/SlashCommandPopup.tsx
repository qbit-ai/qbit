import { useEffect, useRef } from "react";
import { Badge } from "@/components/ui/badge";
import { Popover, PopoverAnchor, PopoverContent } from "@/components/ui/popover";
import type { PromptInfo } from "@/lib/tauri";
import { cn } from "@/lib/utils";

interface SlashCommandPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Already-filtered prompts to display */
  prompts: PromptInfo[];
  selectedIndex: number;
  onSelect: (prompt: PromptInfo) => void;
  children: React.ReactNode;
}

export function SlashCommandPopup({
  open,
  onOpenChange,
  prompts,
  selectedIndex,
  onSelect,
  children,
}: SlashCommandPopupProps) {
  const listRef = useRef<HTMLDivElement>(null);

  // Scroll selected item into view
  useEffect(() => {
    if (open && listRef.current) {
      const selectedElement = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
      selectedElement?.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex, open]);

  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverAnchor asChild>{children}</PopoverAnchor>
      <PopoverContent
        className="w-[300px] p-0"
        side="top"
        align="start"
        sideOffset={8}
        onOpenAutoFocus={(e) => e.preventDefault()}
      >
        <div ref={listRef} className="bg-popover border border-border rounded-md overflow-hidden">
          {prompts.length === 0 ? (
            <div className="py-3 text-center text-sm text-muted-foreground">No prompts found</div>
          ) : (
            <div className="max-h-[200px] overflow-y-auto py-1" role="listbox">
              {prompts.map((prompt, index) => (
                <div
                  key={prompt.path}
                  role="option"
                  aria-selected={index === selectedIndex}
                  tabIndex={0}
                  data-index={index}
                  onClick={() => onSelect(prompt)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelect(prompt);
                    }
                  }}
                  className={cn(
                    "flex items-center justify-between gap-2 px-3 py-2",
                    "cursor-pointer transition-colors",
                    index === selectedIndex ? "bg-primary/10" : "hover:bg-card"
                  )}
                >
                  <span className="font-mono text-sm text-foreground">/{prompt.name}</span>
                  <Badge
                    variant="outline"
                    className={cn(
                      "text-xs",
                      prompt.source === "local"
                        ? "border-[var(--ansi-green)] text-[var(--ansi-green)]"
                        : "border-[var(--ansi-blue)] text-[var(--ansi-blue)]"
                    )}
                  >
                    {prompt.source}
                  </Badge>
                </div>
              ))}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

// Export helper to get filtered prompts (for use in parent component)
export function filterPrompts(prompts: PromptInfo[], query: string): PromptInfo[] {
  return prompts.filter((prompt) => prompt.name.toLowerCase().includes(query.toLowerCase()));
}
