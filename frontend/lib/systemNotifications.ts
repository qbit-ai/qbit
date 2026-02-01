import {
  isPermissionGranted,
  onAction,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getCurrent } from "@tauri-apps/api/window";
import { logger } from "@/lib/logger";
import type { QbitSettings } from "@/lib/settings";
import { getSettings } from "@/lib/settings";
import { useStore } from "@/store";

type PermissionState = "unknown" | "granted" | "denied";

const notificationTabMap = new Map<string, string>();
const handledNotifications = new Set<string>();

let nativeEnabled = false;
let permissionState: PermissionState = "unknown";
let listenersInitialized = false;

function updateNativeEnabled(settings?: QbitSettings | null) {
  nativeEnabled = settings?.notifications?.native_enabled ?? false;
}

async function ensurePermissionGranted(): Promise<boolean> {
  if (permissionState === "granted") {
    return true;
  }

  if (permissionState === "denied") {
    return false;
  }

  try {
    const alreadyGranted = await isPermissionGranted();
    if (alreadyGranted) {
      permissionState = "granted";
      return true;
    }

    const result = await requestPermission();
    permissionState = result === "granted" ? "granted" : "denied";
    return permissionState === "granted";
  } catch (error) {
    logger.warn("Failed to request notification permission:", error);
    return false;
  }
}

function shouldSendNotification(tabId: string | null): boolean {
  if (!nativeEnabled || !tabId) {
    return false;
  }

  const state = useStore.getState();
  const isInactiveTab = state.activeSessionId !== tabId;
  const appNotFocused = !state.appIsFocused;
  const appNotVisible = !state.appIsVisible;

  return isInactiveTab || appNotFocused || appNotVisible;
}

async function focusAndActivateTab(tabId: string) {
  try {
    const window = getCurrent();
    await window.show();
    await window.setFocus();
  } catch (error) {
    logger.debug("Failed to focus notification window:", error);
  }

  useStore.getState().setActiveSession(tabId);
}

function resolveNotificationId(event: unknown): string | null {
  const payload = event as {
    id?: string;
    tag?: string;
    notification?: { id?: string; tag?: string };
  };

  return payload?.notification?.tag ?? payload?.notification?.id ?? payload?.tag ?? payload?.id ?? null;
}

export async function initSystemNotificationListeners() {
  if (listenersInitialized) {
    return;
  }

  listenersInitialized = true;

  try {
    const settings = await getSettings();
    updateNativeEnabled(settings);
  } catch (error) {
    logger.debug("Failed to load settings for notifications:", error);
  }

  window.addEventListener("settings-updated", (event) => {
    const detail = (event as CustomEvent<QbitSettings>).detail;
    updateNativeEnabled(detail);
  });

  onAction((event) => {
    const notificationId = resolveNotificationId(event);
    if (!notificationId) {
      return;
    }

    if (handledNotifications.has(notificationId)) {
      return;
    }

    handledNotifications.add(notificationId);
    const tabId = notificationTabMap.get(notificationId);
    if (!tabId) {
      return;
    }

    notificationTabMap.delete(notificationId);
    void focusAndActivateTab(tabId);
  });
}

export async function requestNativeNotificationPermission(): Promise<boolean> {
  try {
    const alreadyGranted = await isPermissionGranted();
    if (alreadyGranted) {
      permissionState = "granted";
      return true;
    }

    const result = await requestPermission();
    permissionState = result === "granted" ? "granted" : "denied";
    return permissionState === "granted";
  } catch (error) {
    logger.warn("Failed to request notification permission:", error);
    return false;
  }
}

export async function sendNativeNotification(payload: {
  title: string;
  body?: string;
  tabId: string | null;
}) {
  const { title, body, tabId } = payload;
  if (!shouldSendNotification(tabId)) {
    return;
  }

  const permissionGranted = await ensurePermissionGranted();
  if (!permissionGranted) {
    return;
  }

  const notificationId = `qbit-${tabId}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  notificationTabMap.set(notificationId, tabId as string);

  try {
    sendNotification({ title, body, tag: notificationId });
  } catch (error) {
    logger.warn("Failed to send native notification:", error);
  }
}
