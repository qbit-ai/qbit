import { ChevronDown } from "lucide-react";
import React from "react";
import { cn } from "@/lib/utils";

interface SettingsAccordionProps {
  /** Always-visible header row content */
  header: React.ReactNode;
  /** Sub-items rendered inside the collapsible body */
  children: React.ReactNode;
  /**
   * When `false`, the accordion will auto-collapse and stay closed.
   * When `true` the user controls open/close freely.
   */
  parentEnabled: boolean;
  className?: string;
}

/**
 * A small accordion used in settings panels.
 *
 * - The chevron button lets users manually collapse/expand at any time.
 * - When `parentEnabled` transitions from true → false the body auto-collapses.
 * - The chevron is always clickable regardless of `parentEnabled`.
 */
export function SettingsAccordion({ header, children, parentEnabled, className }: SettingsAccordionProps) {
  // Start open only when the parent is initially enabled.
  const [expanded, setExpanded] = React.useState(parentEnabled);

  // Auto-collapse when the parent is disabled; don't auto-expand when re-enabled.
  const prevParentEnabled = React.useRef(parentEnabled);
  React.useEffect(() => {
    if (prevParentEnabled.current && !parentEnabled) {
      setExpanded(false);
    }
    prevParentEnabled.current = parentEnabled;
  }, [parentEnabled]);

  const isOpen = expanded;
  const toggle = () => setExpanded((v) => !v);

  return (
    <div className={cn("space-y-0", className)}>
      {/* Header row — always visible */}
      <div className="flex items-center gap-1.5">
        {/* Chevron toggle — left-anchored, always interactive */}
        <button
          type="button"
          onClick={toggle}
          aria-label={isOpen ? "Collapse section" : "Expand section"}
          className="h-5 w-5 flex items-center justify-center rounded transition-colors shrink-0 text-muted-foreground hover:text-foreground hover:bg-muted/70 cursor-pointer"
        >
          <ChevronDown
            className={cn(
              "w-4 h-4 transition-transform duration-300 ease-in-out",
              isOpen ? "rotate-0" : "-rotate-90",
            )}
          />
        </button>

        <div className="flex-1">{header}</div>
      </div>

      {/* Collapsible body */}
      <div
        className={cn(
          "overflow-hidden transition-all duration-300 ease-in-out",
          isOpen ? "max-h-[600px] opacity-100 mt-3" : "max-h-0 opacity-0 mt-0",
        )}
      >
        {/* Indent children to align under the header label (past the chevron) */}
        <div className="pl-6">{children}</div>
      </div>
    </div>
  );
}
