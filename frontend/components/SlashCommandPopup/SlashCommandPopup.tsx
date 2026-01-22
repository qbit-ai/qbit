import { Puzzle } from "lucide-react";
import { useEffect, useRef } from "react";
import { Badge } from "@/components/ui/badge";
import type { SlashCommand } from "@/hooks/useSlashCommands";
import { cn } from "@/lib/utils";

interface SlashCommandPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Already-filtered commands to display */
  commands: SlashCommand[];
  selectedIndex: number;
  onSelect: (command: SlashCommand) => void;
  children: React.ReactNode;
}

export function SlashCommandPopup({
  open,
  onOpenChange,
  commands,
  selectedIndex,
  onSelect,
  children,
}: SlashCommandPopupProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Close popup when clicking outside
  useEffect(() => {
    if (!open) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onOpenChange(false);
      }
    };

    // Use capture phase to catch clicks before they're handled
    document.addEventListener("mousedown", handleClickOutside, true);
    return () => document.removeEventListener("mousedown", handleClickOutside, true);
  }, [open, onOpenChange]);

  // Close popup when window loses focus (e.g., switching tabs)
  useEffect(() => {
    if (!open) return;

    const handleBlur = () => onOpenChange(false);
    window.addEventListener("blur", handleBlur);
    return () => window.removeEventListener("blur", handleBlur);
  }, [open, onOpenChange]);

  // Scroll selected item into view
  useEffect(() => {
    if (open && listRef.current) {
      const selectedElement = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
      selectedElement?.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex, open]);

  return (
    <div ref={containerRef} className="relative flex-1 flex min-w-0">
      {children}
      {open && (
        <div
          ref={listRef}
          className="absolute bottom-full left-0 mb-2 w-[350px] z-50 bg-popover border border-border rounded-md shadow-md overflow-hidden"
        >
          {commands.length === 0 ? (
            <div className="py-3 text-center text-sm text-muted-foreground">No commands found</div>
          ) : (
            <div className="max-h-[250px] overflow-y-auto py-1" role="listbox">
              {commands.map((command, index) => (
                <div
                  key={command.path}
                  role="option"
                  aria-selected={index === selectedIndex}
                  tabIndex={0}
                  data-index={index}
                  onClick={() => onSelect(command)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelect(command);
                    }
                  }}
                  className={cn(
                    "flex items-start justify-between gap-2 px-3 py-2",
                    "cursor-pointer transition-colors",
                    index === selectedIndex ? "bg-primary/10" : "hover:bg-card"
                  )}
                >
                  <div className="flex flex-col gap-0.5 min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      {command.type === "skill" && (
                        <Puzzle className="w-3.5 h-3.5 text-[var(--color-ansi-magenta)] shrink-0" />
                      )}
                      <span className="font-mono text-sm text-foreground">/{command.name}</span>
                    </div>
                    {command.type === "skill" && command.description && (
                      <span className="text-xs text-muted-foreground truncate">
                        {command.description}
                      </span>
                    )}
                  </div>
                  <Badge
                    variant="outline"
                    className={cn(
                      "text-xs shrink-0",
                      command.type === "skill"
                        ? "border-[var(--color-ansi-magenta)] text-[var(--color-ansi-magenta)]"
                        : command.source === "local"
                          ? "border-[var(--color-ansi-green)] text-[var(--color-ansi-green)]"
                          : "border-[var(--color-ansi-blue)] text-[var(--color-ansi-blue)]"
                    )}
                  >
                    {command.type === "skill" ? "skill" : command.source}
                  </Badge>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// Export helper to get filtered commands (for use in parent component)
export function filterCommands(commands: SlashCommand[], query: string): SlashCommand[] {
  const lowerQuery = query.toLowerCase();
  return commands.filter(
    (command) =>
      command.name.toLowerCase().includes(lowerQuery) ||
      command.description?.toLowerCase().includes(lowerQuery)
  );
}
