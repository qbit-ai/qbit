import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock the Tauri notification APIs
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    setFocus: vi.fn().mockResolvedValue(undefined),
    show: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue("granted"),
  sendNotification: vi.fn(),
  onAction: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("./logger", () => ({
  logger: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("./settings", () => ({
  getSettings: vi.fn().mockResolvedValue({
    notifications: {
      native_enabled: true,
      sound: "",
    },
  }),
}));

// Import after mocks are set up
import {
  sendNotification,
  initSystemNotifications,
  getNotificationMapSize,
  cleanupExpiredNotifications,
  clearNotificationMap,
  NOTIFICATION_TTL_MS,
  MAX_PENDING_NOTIFICATIONS,
} from "./systemNotifications";

describe("systemNotifications", () => {
  const createMockStore = () => ({
    getState: () => ({
      activeSessionId: "session-1",
      appIsFocused: false,
      appIsVisible: false,
      setActiveSession: vi.fn(),
    }),
  });

  beforeEach(async () => {
    vi.useFakeTimers();
    clearNotificationMap();
    await initSystemNotifications(createMockStore());
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  describe("notificationToTabMap memory management", () => {
    it("should expose configuration constants", () => {
      expect(NOTIFICATION_TTL_MS).toBe(5 * 60 * 1000); // 5 minutes
      expect(MAX_PENDING_NOTIFICATIONS).toBe(100);
    });

    it("should add entries to the map when sending notifications", async () => {
      expect(getNotificationMapSize()).toBe(0);

      await sendNotification({
        title: "Test",
        body: "Test body",
        tabId: "tab-1",
      });

      expect(getNotificationMapSize()).toBe(1);
    });

    it("should clean up entries after TTL expires", async () => {
      await sendNotification({
        title: "Test",
        body: "Test body",
        tabId: "tab-1",
      });

      expect(getNotificationMapSize()).toBe(1);

      // Advance time past the TTL
      vi.advanceTimersByTime(NOTIFICATION_TTL_MS + 1000);

      // Run cleanup
      cleanupExpiredNotifications();

      expect(getNotificationMapSize()).toBe(0);
    });

    it("should not clean up entries before TTL expires", async () => {
      await sendNotification({
        title: "Test",
        body: "Test body",
        tabId: "tab-1",
      });

      expect(getNotificationMapSize()).toBe(1);

      // Advance time but not past the TTL
      vi.advanceTimersByTime(NOTIFICATION_TTL_MS - 1000);

      // Run cleanup
      cleanupExpiredNotifications();

      expect(getNotificationMapSize()).toBe(1);
    });

    it("should evict oldest entries when max size is exceeded", async () => {
      // Send MAX_PENDING_NOTIFICATIONS + 5 notifications
      for (let i = 0; i < MAX_PENDING_NOTIFICATIONS + 5; i++) {
        await sendNotification({
          title: `Test ${i}`,
          body: `Test body ${i}`,
          tabId: `tab-${i}`,
        });
      }

      // Map should be capped at MAX_PENDING_NOTIFICATIONS
      expect(getNotificationMapSize()).toBeLessThanOrEqual(MAX_PENDING_NOTIFICATIONS);
    });

    it("should clean up expired notifications when cleanup function is called", async () => {
      // Send some notifications
      await sendNotification({
        title: "Test 1",
        body: "Test body",
        tabId: "tab-1",
      });

      expect(getNotificationMapSize()).toBe(1);

      // Advance time past TTL
      vi.advanceTimersByTime(NOTIFICATION_TTL_MS + 1000);

      // Manually call cleanup (simulating what the interval would do)
      cleanupExpiredNotifications();

      // The cleanup should have removed the expired notification
      expect(getNotificationMapSize()).toBe(0);
    });

    it("should handle multiple notifications with mixed expiry times", async () => {
      // Send first notification
      await sendNotification({
        title: "Test 1",
        body: "Test body",
        tabId: "tab-1",
      });

      // Advance time by 3 minutes
      vi.advanceTimersByTime(3 * 60 * 1000);

      // Send second notification
      await sendNotification({
        title: "Test 2",
        body: "Test body",
        tabId: "tab-2",
      });

      expect(getNotificationMapSize()).toBe(2);

      // Advance time by 2.5 more minutes (first should expire, second should not)
      vi.advanceTimersByTime(2.5 * 60 * 1000);
      cleanupExpiredNotifications();

      expect(getNotificationMapSize()).toBe(1);
    });

    it("should clear all entries with clearNotificationMap", async () => {
      await sendNotification({
        title: "Test 1",
        body: "Test body",
        tabId: "tab-1",
      });
      await sendNotification({
        title: "Test 2",
        body: "Test body",
        tabId: "tab-2",
      });

      expect(getNotificationMapSize()).toBe(2);

      clearNotificationMap();

      expect(getNotificationMapSize()).toBe(0);
    });
  });
});
