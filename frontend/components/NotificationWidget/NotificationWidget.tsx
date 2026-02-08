import { AlertTriangle, Bell, Check, CheckCircle2, Info, Trash2, X, XCircle } from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useShallow } from "zustand/react/shallow";
import { cn } from "@/lib/utils";
import { type Notification, type NotificationType, useStore } from "@/store";
import {
  selectNotifications,
  selectNotificationsExpanded,
  selectUnreadNotificationCount,
} from "@/store/slices";

const PREVIEW_DURATION_MS = 5000;
const FADEOUT_DURATION_MS = 300;

// Static style constants extracted to avoid recreation on each render
const glowStyle = { backgroundColor: "var(--ansi-cyan)" } as const;

const panelStyle = {
  top: "38px",
  right: "8px",
  boxShadow: `
    0 0 0 1px rgba(0,0,0,0.1),
    0 4px 6px -1px rgba(0,0,0,0.2),
    0 10px 20px -2px rgba(0,0,0,0.25)
  `,
} as const;

const emptyStateIconStyle = { backgroundColor: "var(--ansi-cyan)10" } as const;

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

  // Memoize style objects to avoid recreation on each render
  const dotStyle = useMemo(() => ({ backgroundColor: color }), [color]);
  const iconBgStyle = useMemo(() => ({ backgroundColor: `${color}15` }), [color]);
  const iconColorStyle = useMemo(() => ({ color }), [color]);

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
    // biome-ignore lint/a11y/useSemanticElements: Cannot use button element as it contains a nested button for remove action
    <div
      role="button"
      tabIndex={0}
      data-testid={`notification-item-${notification.type}`}
      onClick={handleClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleClick();
        }
      }}
      className={cn(
        "group relative flex items-start gap-3 px-3 py-2.5 cursor-pointer transition-all duration-200 w-full text-left",
        "border-b border-border/30 last:border-b-0",
        "hover:bg-card/80",
        !notification.read && "bg-card/40"
      )}
    >
      {/* Left gutter: unread dot + icon (kept in normal flow for alignment) */}
      <div className="flex items-center gap-2 flex-shrink-0">
        <div className="w-2 flex items-center justify-center">
          {!notification.read && (
            <div className="w-1.5 h-1.5 rounded-full animate-pulse" style={dotStyle} />
          )}
        </div>

        <div className="p-1.5 rounded-md" style={iconBgStyle}>
          <Icon className="w-3.5 h-3.5" style={iconColorStyle} />
        </div>
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

export const NotificationWidget = React.memo(function NotificationWidget() {
  // Consolidated selector with useShallow — re-renders only when any property changes,
  // not on every store mutation.
  const { notifications, unreadCount, isExpanded } = useStore(
    useShallow((state) => ({
      notifications: selectNotifications(state),
      unreadCount: selectUnreadNotificationCount(state),
      isExpanded: selectNotificationsExpanded(state),
    }))
  );
  const setExpanded = useStore((state) => state.setNotificationsExpanded);
  const markAllRead = useStore((state) => state.markAllNotificationsRead);
  const clearAll = useStore((state) => state.clearNotifications);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Preview state - shows truncated notification text temporarily
  const [previewNotification, setPreviewNotification] = useState<Notification | null>(null);
  const [_isPreviewFadingOut, setIsPreviewFadingOut] = useState(false);
  const lastNotificationIdRef = useRef<string | null>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeOutTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
        // Clear any existing timers
        if (previewTimerRef.current) {
          clearTimeout(previewTimerRef.current);
        }
        if (fadeOutTimerRef.current) {
          clearTimeout(fadeOutTimerRef.current);
        }

        // Show the preview (reset fade state)
        setIsPreviewFadingOut(false);
        setPreviewNotification(latestNotification);

        // Set timer to start fade-out animation
        previewTimerRef.current = setTimeout(() => {
          setIsPreviewFadingOut(true);
          previewTimerRef.current = null;

          // After fade-out animation completes, hide the preview
          fadeOutTimerRef.current = setTimeout(() => {
            setPreviewNotification(null);
            setIsPreviewFadingOut(false);
            fadeOutTimerRef.current = null;
          }, FADEOUT_DURATION_MS);
        }, PREVIEW_DURATION_MS);
      }
    }
  }, [notifications, isExpanded]);

  // Clear preview when panel is expanded
  useEffect(() => {
    if (isExpanded && previewNotification) {
      setPreviewNotification(null);
      setIsPreviewFadingOut(false);
      if (previewTimerRef.current) {
        clearTimeout(previewTimerRef.current);
        previewTimerRef.current = null;
      }
      if (fadeOutTimerRef.current) {
        clearTimeout(fadeOutTimerRef.current);
        fadeOutTimerRef.current = null;
      }
    }
  }, [isExpanded, previewNotification]);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      if (previewTimerRef.current) {
        clearTimeout(previewTimerRef.current);
      }
      if (fadeOutTimerRef.current) {
        clearTimeout(fadeOutTimerRef.current);
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

  return (
    <div data-testid="notification-widget" className="relative flex items-center gap-2">
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
              style={glowStyle}
            />
          )}
        </div>
        {unreadCount > 0 ? (
          <span className="tabular-nums">{unreadCount}</span>
        ) : (
          <span className="text-[10px] opacity-70">0</span>
        )}
      </button>

      {/* Notification panel - rendered via portal to avoid stacking context issues */}
      {isExpanded &&
        createPortal(
          // biome-ignore lint/a11y/noStaticElementInteractions: Used for click-outside detection
          <div
            ref={panelRef}
            role="presentation"
            onMouseDown={(e) => e.stopPropagation()}
            className={cn(
              "fixed w-80 z-[9999]",
              "bg-background/95 backdrop-blur-xl",
              "border border-border/60 rounded-lg shadow-2xl",
              "animate-in fade-in-0 slide-in-from-top-2 zoom-in-95 duration-200",
              "origin-top-right"
            )}
            style={panelStyle}
          >
            {/* Header */}
            <div className="flex items-center justify-between px-3 py-2 border-b border-border/40">
              <div className="flex items-center gap-2">
                <Bell className="w-4 h-4 text-[var(--ansi-cyan)]" />
                <span className="text-sm font-semibold text-foreground">Notifications</span>
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
                  <div className="p-3 rounded-full mb-3" style={emptyStateIconStyle}>
                    <Bell className="w-6 h-6 text-muted-foreground/50" />
                  </div>
                  <p className="text-sm text-muted-foreground">No notifications</p>
                  <p className="text-xs text-muted-foreground/50 mt-1">You're all caught up!</p>
                </div>
              ) : (
                notifications.map((notification) => (
                  <NotificationItem key={notification.id} notification={notification} />
                ))
              )}
            </div>
          </div>,
          document.body
        )}
    </div>
  );
});
