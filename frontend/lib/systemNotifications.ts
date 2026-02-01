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
import * as notification from "@tauri-apps/plugin-notification";
import type { StoreApi } from "zustand";
import type { QbitStore } from "../store";
import { logger } from "./logger";
import { getSettings } from "./settings";

// =============================================================================
// State
// =============================================================================

/** Cached permission state to avoid repeated prompts */
let permissionGranted: boolean | null = null;

/** In-memory map of notification identifiers to tab IDs for click routing */
const notificationToTabMap = new Map<string, string>();

/** Store instance (set during initialization) */
let storeApi: StoreApi<QbitStore> | null = null;

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
    permissionGranted = await notification.isPermissionGranted();
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
    const granted = await notification.requestPermission();
    permissionGranted = granted;
    return granted;
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
export async function initSystemNotifications(store: StoreApi<QbitStore>): Promise<void> {
  storeApi = store;

  // Register click action listener for notification routing
  try {
    await notification.registerListener((event) => {
      handleNotificationClick(event);
    });
    logger.debug("System notifications initialized");
  } catch (error) {
    logger.error("Failed to register notification listener:", error);
  }
}

/**
 * Handle notification click events.
 * Focuses/shows the app window and activates the associated tab.
 */
async function handleNotificationClick(event: unknown): Promise<void> {
  if (!storeApi) {
    logger.warn("Store API not available for notification click handling");
    return;
  }

  try {
    // Extract notification identifier from event
    // The notification plugin should include the identifier in the event payload
    const notificationId = extractNotificationId(event);
    if (!notificationId) {
      logger.debug("No notification ID in click event");
      return;
    }

    // Get the associated tab ID from our map
    const tabId = notificationToTabMap.get(notificationId);
    if (!tabId) {
      logger.debug(`No tab ID found for notification: ${notificationId}`);
      return;
    }

    // Focus and show the main window
    const appWindow = getCurrentWindow();
    await appWindow.setFocus();
    await appWindow.show();

    // Activate the associated tab
    storeApi.getState().setActiveSession(tabId);

    // Clean up the mapping
    notificationToTabMap.delete(notificationId);

    logger.debug(`Activated tab ${tabId} from notification click`);
  } catch (error) {
    logger.error("Failed to handle notification click:", error);
  }
}

/**
 * Extract notification identifier from click event.
 * The exact structure depends on the notification plugin's event payload.
 */
function extractNotificationId(event: unknown): string | null {
  if (typeof event === "object" && event !== null) {
    const e = event as Record<string, unknown>;
    // Try common field names for notification identifier
    return (e.id as string | undefined) ?? (e.tag as string | undefined) ?? null;
  }
  return null;
}

// =============================================================================
// Notification Sending
// =============================================================================

export interface SendNotificationOptions {
  title: string;
  body: string;
  tabId: string;
}

/**
 * Send a native OS notification with gating logic.
 *
 * Only sends when:
 * - notifications.native_enabled is true
 * - AND (inactive tab OR app not focused/visible)
 *
 * @param options - Notification options including title, body, and tab ID
 */
export async function sendNotification(options: SendNotificationOptions): Promise<void> {
  const { title, body, tabId } = options;

  // Check if notifications are enabled
  const settings = await getSettings();
  if (!settings.notifications.native_enabled) {
    return;
  }

  // Check permission
  const granted = await isPermissionGranted();
  if (!granted) {
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

  // Generate a stable identifier for this notification
  const notificationId = `qbit-${Date.now()}-${crypto.randomUUID()}`;

  // Store the mapping for click routing
  notificationToTabMap.set(notificationId, tabId);

  try {
    await notification.sendNotification({
      title,
      body,
      // Use the identifier for click routing
      identifier: notificationId,
    });

    logger.debug(`Sent notification for tab ${tabId}: ${title}`);
  } catch (error) {
    logger.error("Failed to send notification:", error);
    // Clean up the mapping on error
    notificationToTabMap.delete(notificationId);
  }
}

// =============================================================================
// Settings Event Listener
// =============================================================================

/**
 * Listen for settings updates to reactively update notification state.
 * Call this once at app startup.
 */
export function listenForSettingsUpdates(): void {
  window.addEventListener("settings-updated", (event) => {
    const settings = event.detail as { notifications: { native_enabled: boolean } };
    if (settings.notifications?.native_enabled) {
      // When enabling, check permission
      isPermissionGranted().then((granted) => {
        if (!granted) {
          logger.info("Notifications enabled but permission not granted");
        }
      });
    }
  });
}
