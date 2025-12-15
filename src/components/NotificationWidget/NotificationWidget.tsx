import { Bell, Check, Info, AlertTriangle, XCircle, CheckCircle2, X, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import {
  useNotifications,
  useUnreadNotificationCount,
  useNotificationsExpanded,
  useStore,
  type Notification,
  type NotificationType,
} from "@/store";

const PREVIEW_DURATION_MS = 10000; // 10 seconds

const NOTIFICATION_ICONS: Record<NotificationType, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};

const NOTIFICATION_COLORS: Record<NotificationType, string> = {
  info: "var(--ansi-blue)",
  success: "var(--ansi-green)",
  warning: "var(--ansi-yellow)",
  error: "var(--ansi-red)",
};

function formatRelativeTime(timestamp: string): string {
  const now = new Date();
  const then = new Date(timestamp);
  const diffMs = now.getTime() - then.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  return then.toLocaleDateString();
}

function NotificationItem({ notification }: { notification: Notification }) {
  const Icon = NOTIFICATION_ICONS[notification.type];
  const color = NOTIFICATION_COLORS[notification.type];
  const removeNotification = useStore((state) => state.removeNotification);
  const markNotificationRead = useStore((state) => state.markNotificationRead);

  const handleClick = useCallback(() => {
    if (!notification.read) {
      markNotificationRead(notification.id);
    }
  }, [notification.id, notification.read, markNotificationRead]);

  const handleRemove = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      removeNotification(notification.id);
    },
    [notification.id, removeNotification]
  );

  return (
    <div
      data-testid={`notification-item-${notification.type}`}
      onClick={handleClick}
      className={cn(
        "group relative flex items-start gap-3 px-3 py-2.5 cursor-pointer transition-all duration-200",
        "border-b border-border/30 last:border-b-0",
        "hover:bg-card/80",
        !notification.read && "bg-card/40"
      )}
    >
      {/* Unread indicator */}
      {!notification.read && (
        <div
          className="absolute left-1 top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full animate-pulse"
          style={{ backgroundColor: color }}
        />
      )}

      {/* Icon */}
      <div
        className="flex-shrink-0 mt-0.5 p-1.5 rounded-md"
        style={{ backgroundColor: `${color}15` }}
      >
        <Icon className="w-3.5 h-3.5" style={{ color }} />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <p
          className={cn(
            "text-xs font-medium leading-tight",
            notification.read ? "text-muted-foreground" : "text-foreground"
          )}
        >
          {notification.title}
        </p>
        {notification.message && (
          <p className="text-[11px] text-muted-foreground/70 mt-0.5 leading-snug line-clamp-2">
            {notification.message}
          </p>
        )}
        <p className="text-[10px] text-muted-foreground/50 mt-1 font-mono">
          {formatRelativeTime(notification.timestamp)}
        </p>
      </div>

      {/* Remove button */}
      <button
        type="button"
        onClick={handleRemove}
        className="flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-[var(--ansi-red)]/10"
      >
        <X className="w-3 h-3 text-muted-foreground hover:text-[var(--ansi-red)]" />
      </button>
    </div>
  );
}

export function NotificationWidget() {
  const notifications = useNotifications();
  const unreadCount = useUnreadNotificationCount();
  const isExpanded = useNotificationsExpanded();
  const setExpanded = useStore((state) => state.setNotificationsExpanded);
  const markAllRead = useStore((state) => state.markAllNotificationsRead);
  const clearAll = useStore((state) => state.clearNotifications);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Preview state - shows truncated notification text temporarily
  const [previewNotification, setPreviewNotification] = useState<Notification | null>(null);
  const lastNotificationIdRef = useRef<string | null>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Watch for new notifications and show preview
  useEffect(() => {
    if (notifications.length === 0) {
      lastNotificationIdRef.current = null;
      return;
    }

    const latestNotification = notifications[0];

    // Check if this is a new notification
    if (latestNotification.id !== lastNotificationIdRef.current) {
      lastNotificationIdRef.current = latestNotification.id;

      // Don't show preview if panel is already expanded
      if (!isExpanded) {
        // Clear any existing timer
        if (previewTimerRef.current) {
          clearTimeout(previewTimerRef.current);
        }

        // Show the preview
        setPreviewNotification(latestNotification);

        // Set timer to hide preview after 10 seconds
        previewTimerRef.current = setTimeout(() => {
          setPreviewNotification(null);
          previewTimerRef.current = null;
        }, PREVIEW_DURATION_MS);
      }
    }
  }, [notifications, isExpanded]);

  // Clear preview when panel is expanded
  useEffect(() => {
    if (isExpanded && previewNotification) {
      setPreviewNotification(null);
      if (previewTimerRef.current) {
        clearTimeout(previewTimerRef.current);
        previewTimerRef.current = null;
      }
    }
  }, [isExpanded, previewNotification]);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (previewTimerRef.current) {
        clearTimeout(previewTimerRef.current);
      }
    };
  }, []);

  // Close on click outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        isExpanded &&
        panelRef.current &&
        triggerRef.current &&
        !panelRef.current.contains(event.target as Node) &&
        !triggerRef.current.contains(event.target as Node)
      ) {
        setExpanded(false);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isExpanded, setExpanded]);

  // Close on escape
  useEffect(() => {
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape" && isExpanded) {
        setExpanded(false);
      }
    }

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [isExpanded, setExpanded]);

  const toggleExpanded = useCallback(() => {
    setExpanded(!isExpanded);
  }, [isExpanded, setExpanded]);

  // Get color for preview notification
  const previewColor = previewNotification
    ? NOTIFICATION_COLORS[previewNotification.type]
    : null;
  const PreviewIcon = previewNotification
    ? NOTIFICATION_ICONS[previewNotification.type]
    : null;

  return (
    <div data-testid="notification-widget" className="relative flex items-center gap-2">
      {/* Preview notification text (shows temporarily for new notifications) */}
      {previewNotification && !isExpanded && (
        <div
          data-testid="notification-preview"
          className={cn(
            "flex items-center gap-2 h-6 px-2.5 rounded-md",
            "animate-in fade-in-0 slide-in-from-right-2 duration-300",
            "max-w-[200px]"
          )}
          style={{ backgroundColor: `${previewColor}15` }}
        >
          {PreviewIcon && (
            <PreviewIcon
              className="w-3.5 h-3.5 flex-shrink-0"
              style={{ color: previewColor ?? undefined }}
            />
          )}
          <span
            className="text-xs font-medium truncate"
            style={{ color: previewColor ?? undefined }}
          >
            {previewNotification.title}
          </span>
        </div>
      )}

      {/* Trigger button */}
      <button
        ref={triggerRef}
        type="button"
        onClick={toggleExpanded}
        className={cn(
          "flex items-center gap-1.5 h-6 px-2 rounded-md transition-all duration-200",
          "text-xs font-medium",
          isExpanded
            ? "bg-[var(--ansi-cyan)]/20 text-[var(--ansi-cyan)]"
            : unreadCount > 0
              ? "bg-[var(--ansi-cyan)]/10 text-[var(--ansi-cyan)] hover:bg-[var(--ansi-cyan)]/20"
              : "text-muted-foreground hover:text-foreground hover:bg-card/50"
        )}
      >
        <div className="relative">
          <Bell
            className={cn(
              "w-3.5 h-3.5 transition-transform duration-200",
              unreadCount > 0 && !isExpanded && "animate-[wiggle_1s_ease-in-out]"
            )}
          />
          {/* Glow effect for unread */}
          {unreadCount > 0 && (
            <div
              className="absolute inset-0 rounded-full blur-sm opacity-50 animate-pulse"
              style={{ backgroundColor: "var(--ansi-cyan)" }}
            />
          )}
        </div>
        {unreadCount > 0 ? (
          <span className="tabular-nums">{unreadCount}</span>
        ) : (
          <span className="text-[10px] opacity-70">0</span>
        )}
      </button>

      {/* Notification panel */}
      {isExpanded && (
        <div
          ref={panelRef}
          className={cn(
            "absolute bottom-full right-0 mb-2 w-80",
            "bg-background/95 backdrop-blur-xl",
            "border border-border/60 rounded-lg shadow-2xl",
            "animate-in fade-in-0 slide-in-from-bottom-2 zoom-in-95 duration-200",
            "origin-bottom-right"
          )}
          style={{
            boxShadow: `
              0 0 0 1px rgba(0,0,0,0.1),
              0 4px 6px -1px rgba(0,0,0,0.2),
              0 10px 20px -2px rgba(0,0,0,0.25),
              0 0 40px -10px var(--ansi-cyan)
            `,
          }}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-3 py-2 border-b border-border/40">
            <div className="flex items-center gap-2">
              <Bell className="w-4 h-4 text-[var(--ansi-cyan)]" />
              <span className="text-sm font-semibold text-foreground">Notifications</span>
              {unreadCount > 0 && (
                <span
                  className="px-1.5 py-0.5 text-[10px] font-bold rounded-full"
                  style={{
                    backgroundColor: "var(--ansi-cyan)",
                    color: "var(--background)",
                  }}
                >
                  {unreadCount}
                </span>
              )}
            </div>
            <div className="flex items-center gap-1">
              {unreadCount > 0 && (
                <button
                  type="button"
                  onClick={markAllRead}
                  className="p-1 rounded hover:bg-card/80 transition-colors"
                  title="Mark all as read"
                >
                  <Check className="w-3.5 h-3.5 text-muted-foreground hover:text-[var(--ansi-green)]" />
                </button>
              )}
              {notifications.length > 0 && (
                <button
                  type="button"
                  onClick={clearAll}
                  className="p-1 rounded hover:bg-card/80 transition-colors"
                  title="Clear all"
                >
                  <Trash2 className="w-3.5 h-3.5 text-muted-foreground hover:text-[var(--ansi-red)]" />
                </button>
              )}
            </div>
          </div>

          {/* Notification list */}
          <div className="max-h-80 overflow-y-auto overscroll-contain">
            {notifications.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-8 px-4 text-center">
                <div
                  className="p-3 rounded-full mb-3"
                  style={{ backgroundColor: "var(--ansi-cyan)10" }}
                >
                  <Bell className="w-6 h-6 text-muted-foreground/50" />
                </div>
                <p className="text-sm text-muted-foreground">No notifications</p>
                <p className="text-xs text-muted-foreground/50 mt-1">
                  You're all caught up!
                </p>
              </div>
            ) : (
              notifications.map((notification) => (
                <NotificationItem key={notification.id} notification={notification} />
              ))
            )}
          </div>

          {/* Footer - subtle branding */}
          {notifications.length > 0 && (
            <div className="px-3 py-1.5 border-t border-border/30 text-center">
              <span className="text-[10px] text-muted-foreground/40 font-mono">
                {notifications.length} notification{notifications.length !== 1 ? "s" : ""}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
