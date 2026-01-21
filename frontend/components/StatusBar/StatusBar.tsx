/**
 * StatusBar - Global footer that only contains the notification widget.
 * Model selection, token usage, and other per-session elements have been
 * moved to InputStatusRow in the UnifiedInput component for multi-pane support.
 */

import { NotificationWidget } from "@/components/NotificationWidget";

export function StatusBar() {
  return (
    <div className="h-6 px-3 flex items-center justify-end border-t border-[var(--border-subtle)] bg-background/80">
      <NotificationWidget />
    </div>
  );
}
