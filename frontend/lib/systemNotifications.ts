/**
 * Native OS notification system for Qbit.
 *
 * Responsibilities:
 * - Read opt-in setting (notifications.native_enabled)
 * - Permission management (isPermissionGranted, requestPermission)
 * - Gating logic (only send when native_enabled AND inactive tab OR app not focused/visible)
 * - Sending notifications with minimal body
 * - Click routing (focus/show app and activate associated tab)
 */

import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted as checkPermission,
  requestPermission as doRequestPermission,
  sendNotification as doSendNotification,
  type Options,
  onAction,
} from "@tauri-apps/plugin-notification";
import { logger } from "./logger";
import { getSettings } from "./settings";
import { type NotificationSoundType, playNotificationSound } from "./sound";

// =============================================================================
// Types
// =============================================================================

/** Minimal store interface for notification gating */
interface NotificationStoreApi {
  getState: () => {
    activeSessionId: string | null;
    appIsFocused: boolean;
    appIsVisible: boolean;
    setActiveSession: (sessionId: string) => void;
  };
}

// =============================================================================
// Configuration
// =============================================================================

/** Time-to-live for notification-to-tab mappings (5 minutes) */
export const NOTIFICATION_TTL_MS = 5 * 60 * 1000;

/** Maximum number of pending notifications to track */
export const MAX_PENDING_NOTIFICATIONS = 100;

/** Interval for periodic cleanup (1 minute) */
const CLEANUP_INTERVAL_MS = 60_000;

// =============================================================================
// State
// =============================================================================

/** Cached permission state to avoid repeated prompts */
let permissionGranted: boolean | null = null;

/** Entry in the notification-to-tab map with timestamp for TTL */
interface NotificationEntry {
  tabId: string;
  createdAt: number;
}

/** In-memory map of notification identifiers to tab IDs for click routing */
const notificationToTabMap = new Map<number, NotificationEntry>();

/** Counter for generating notification IDs (must be 32-bit integer) */
let notificationIdCounter = 1;

/** Store instance (set during initialization) */
let storeApi: NotificationStoreApi | null = null;

/** Cleanup interval handle */
let cleanupIntervalId: ReturnType<typeof setInterval> | null = null;

// =============================================================================
// Memory Management
// =============================================================================

/**
 * Clean up expired notifications from the map.
 * Entries older than NOTIFICATION_TTL_MS are removed.
 */
export function cleanupExpiredNotifications(): void {
  const now = Date.now();
  for (const [id, entry] of notificationToTabMap) {
    if (now - entry.createdAt > NOTIFICATION_TTL_MS) {
      notificationToTabMap.delete(id);
    }
  }
}

/**
 * Evict oldest entries if the map exceeds MAX_PENDING_NOTIFICATIONS.
 * This ensures bounded memory usage even if cleanup doesn't run.
 */
function evictOldestIfNeeded(): void {
  if (notificationToTabMap.size <= MAX_PENDING_NOTIFICATIONS) {
    return;
  }

  // Convert to array and sort by createdAt (oldest first)
  const entries = Array.from(notificationToTabMap.entries()).sort(
    ([, a], [, b]) => a.createdAt - b.createdAt
  );

  // Remove oldest entries until we're at the limit
  const toRemove = entries.slice(0, notificationToTabMap.size - MAX_PENDING_NOTIFICATIONS);
  for (const [id] of toRemove) {
    notificationToTabMap.delete(id);
  }
}

/**
 * Clear all entries from the notification map.
 * Useful for testing and cleanup.
 */
export function clearNotificationMap(): void {
  notificationToTabMap.clear();
}

/**
 * Get the current size of the notification map.
 * Useful for testing and debugging.
 */
export function getNotificationMapSize(): number {
  return notificationToTabMap.size;
}

/**
 * Start periodic cleanup of expired notifications.
 */
function startCleanupInterval(): void {
  if (cleanupIntervalId !== null) {
    return; // Already running
  }
  cleanupIntervalId = setInterval(cleanupExpiredNotifications, CLEANUP_INTERVAL_MS);
}

/**
 * Stop periodic cleanup of expired notifications.
 * Useful for testing.
 */
export function stopCleanupInterval(): void {
  if (cleanupIntervalId !== null) {
    clearInterval(cleanupIntervalId);
    cleanupIntervalId = null;
  }
}

// =============================================================================
// Permission Management
// =============================================================================

/**
 * Check if notification permission is granted.
 * Uses cached value to avoid repeated prompts.
 */
export async function isPermissionGranted(): Promise<boolean> {
  if (permissionGranted !== null) {
    return permissionGranted;
  }

  try {
    permissionGranted = await checkPermission();
    return permissionGranted;
  } catch (error) {
    logger.error("Failed to check notification permission:", error);
    return false;
  }
}

/**
 * Request notification permission from the user.
 * Caches the result to avoid repeated prompts.
 */
export async function requestPermission(): Promise<boolean> {
  try {
    const result = await doRequestPermission();
    // NotificationPermission is "granted" | "denied" | "default"
    permissionGranted = result === "granted";
    return permissionGranted;
  } catch (error) {
    logger.error("Failed to request notification permission:", error);
    return false;
  }
}

// =============================================================================
// Initialization
// =============================================================================

/**
 * Initialize the notification system.
 * Should be called once at app startup.
 */
export async function initSystemNotifications(store: NotificationStoreApi): Promise<void> {
  storeApi = store;

  // Start periodic cleanup of expired notifications
  startCleanupInterval();

  // Register click action listener for notification routing.
  // Note: onAction uses addPluginListener which requires `register_listener` —
  // this command is not implemented by the notification plugin on desktop,
  // so we silently ignore the failure.
  try {
    await onAction((notification: Options) => {
      handleNotificationClick(notification);
    });
  } catch {
    // Expected on desktop — notification click routing is mobile-only
  }

  logger.debug("System notifications initialized");
}

/**
 * Handle notification click events.
 * Focuses/shows the app window and activates the associated tab.
 */
async function handleNotificationClick(notification: Options): Promise<void> {
  if (!storeApi) {
    logger.warn("Store API not available for notification click handling");
    return;
  }

  try {
    // Extract notification identifier from the notification object
    const notificationId = notification.id;
    if (notificationId === undefined) {
      logger.debug("No notification ID in click event");
      return;
    }

    // Get the associated entry from our map
    const entry = notificationToTabMap.get(notificationId);
    if (!entry) {
      logger.debug(`No tab ID found for notification: ${notificationId}`);
      return;
    }

    // Focus and show the main window
    const appWindow = getCurrentWindow();
    await appWindow.setFocus();
    await appWindow.show();

    // Activate the associated tab
    storeApi.getState().setActiveSession(entry.tabId);

    // Clean up the mapping
    notificationToTabMap.delete(notificationId);

    logger.debug(`Activated tab ${entry.tabId} from notification click`);
  } catch (error) {
    logger.error("Failed to handle notification click:", error);
  }
}

// =============================================================================
// Notification Sending
// =============================================================================

export interface SendNotificationOptions {
  title: string;
  body: string;
  tabId: string;
  /** Sound type for in-app audio (default: "agent") */
  soundType?: NotificationSoundType;
}

/**
 * Send a notification with optional native OS notification and in-app sound.
 *
 * Gating logic (only when inactive tab OR app not focused/visible):
 * - Plays in-app sound if notifications.sound_enabled is true
 * - Sends native OS notification if notifications.native_enabled is true
 *
 * @param options - Notification options including title, body, and tab ID
 */
export async function sendNotification(options: SendNotificationOptions): Promise<void> {
  const { title, body, tabId, soundType = "agent" } = options;

  // Get notification settings
  const settings = await getSettings();
  const soundEnabled = settings?.notifications?.sound_enabled ?? true;
  const nativeEnabled = settings?.notifications?.native_enabled ?? false;

  // Early exit if neither sound nor native notifications are enabled
  if (!soundEnabled && !nativeEnabled) {
    return;
  }

  // Check gating condition: inactive tab OR app not focused/visible
  if (!storeApi) {
    logger.warn("Store API not available for notification gating");
    return;
  }

  const state = storeApi.getState();
  const { activeSessionId, appIsFocused, appIsVisible } = state;

  // Only send notifications if:
  // - The tab is NOT active, OR
  // - The app is NOT focused/visible
  const isActiveTab = activeSessionId === tabId;
  const isAppFocusedAndVisible = appIsFocused && appIsVisible;

  if (isActiveTab && isAppFocusedAndVisible) {
    // User is looking at this tab in a focused app - no notification needed
    return;
  }

  // Play in-app sound if enabled (independent of native notifications)
  if (soundEnabled) {
    playNotificationSound(soundType);
  }

  // Send native OS notification if enabled
  if (nativeEnabled) {
    // Check permission for native notifications
    const granted = await isPermissionGranted();
    if (!granted) {
      logger.debug("Native notification permission not granted, skipping");
      return;
    }

    // Generate a stable identifier for this notification (must be 32-bit integer)
    const notificationId = notificationIdCounter++;
    // Reset counter if it gets too large
    if (notificationIdCounter > 2147483647) {
      notificationIdCounter = 1;
    }

    // Store the mapping for click routing with timestamp for TTL-based cleanup
    notificationToTabMap.set(notificationId, {
      tabId,
      createdAt: Date.now(),
    });

    // Enforce max size limit to prevent unbounded growth
    evictOldestIfNeeded();

    // Use configured sound if provided and non-empty, otherwise fall back to platform default
    let sound: string | undefined;
    if (settings?.notifications?.sound && settings.notifications.sound.trim() !== "") {
      sound = settings.notifications.sound;
    } else {
      // Default to "Blow" on macOS (system sound name)
      // On other platforms, avoid setting `sound` as it may be interpreted as a file path
      const isMacOS = navigator.platform.toLowerCase().includes("mac");
      sound = isMacOS ? "Blow" : undefined;
    }

    try {
      doSendNotification({
        title,
        body,
        // Use id for click routing (must be a 32-bit integer)
        id: notificationId,
        sound,
      });

      logger.debug(`Sent native notification for tab ${tabId}: ${title}`);
    } catch (error) {
      logger.error("Failed to send native notification:", error);
      // Clean up the mapping on error
      notificationToTabMap.delete(notificationId);
    }
  }
}

// =============================================================================
// Settings Event Listener
// =============================================================================

/**
 * Listen for settings updates to reactively update notification state.
 * Returns a cleanup function that removes the event listener.
 */
export function listenForSettingsUpdates(): () => void {
  const handleSettingsUpdated = (event: Event) => {
    const customEvent = event as CustomEvent<{ notifications?: { native_enabled?: boolean } }>;
    const settings = customEvent.detail;
    if (settings?.notifications?.native_enabled) {
      // When enabling, check permission
      isPermissionGranted().then((granted) => {
        if (!granted) {
          logger.info("Notifications enabled but permission not granted");
        }
      });
    }
  };

  window.addEventListener("settings-updated", handleSettingsUpdated);

  // Return cleanup function to remove listener and prevent memory leak
  return () => {
    window.removeEventListener("settings-updated", handleSettingsUpdated);
  };
}
