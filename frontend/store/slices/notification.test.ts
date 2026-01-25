import { beforeEach, describe, expect, it } from "vitest";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import {
  createNotificationSlice,
  initialNotificationState,
  type NotificationSlice,
  selectNotifications,
  selectNotificationsExpanded,
  selectUnreadNotificationCount,
} from "./notification";

describe("Notification Slice", () => {
  // Create a test store with just the notification slice
  const createTestStore = () =>
    create<NotificationSlice>()(immer((set, get) => createNotificationSlice(set, get)));

  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    store = createTestStore();
  });

  describe("initial state", () => {
    it("should have empty notifications array", () => {
      expect(store.getState().notifications).toEqual([]);
    });

    it("should have notificationsExpanded set to false", () => {
      expect(store.getState().notificationsExpanded).toBe(false);
    });
  });

  describe("addNotification", () => {
    it("should add a notification with generated id and timestamp", () => {
      store.getState().addNotification({
        type: "info",
        title: "Test notification",
      });

      const notifications = store.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].type).toBe("info");
      expect(notifications[0].title).toBe("Test notification");
      expect(notifications[0].id).toBeDefined();
      expect(notifications[0].timestamp).toBeDefined();
      expect(notifications[0].read).toBe(false);
    });

    it("should add notification with optional message", () => {
      store.getState().addNotification({
        type: "error",
        title: "Error occurred",
        message: "Something went wrong",
      });

      const notifications = store.getState().notifications;
      expect(notifications[0].message).toBe("Something went wrong");
    });

    it("should prepend new notifications (most recent first)", () => {
      store.getState().addNotification({ type: "info", title: "First" });
      store.getState().addNotification({ type: "info", title: "Second" });

      const notifications = store.getState().notifications;
      expect(notifications[0].title).toBe("Second");
      expect(notifications[1].title).toBe("First");
    });

    it("should support all notification types", () => {
      const types = ["info", "success", "warning", "error"] as const;

      for (const type of types) {
        store.getState().addNotification({ type, title: `${type} notification` });
      }

      const notifications = store.getState().notifications;
      expect(notifications).toHaveLength(4);
    });
  });

  describe("markNotificationRead", () => {
    it("should mark a notification as read", () => {
      store.getState().addNotification({ type: "info", title: "Test" });
      const notificationId = store.getState().notifications[0].id;

      store.getState().markNotificationRead(notificationId);

      expect(store.getState().notifications[0].read).toBe(true);
    });

    it("should not throw for non-existent notification id", () => {
      expect(() => {
        store.getState().markNotificationRead("non-existent");
      }).not.toThrow();
    });

    it("should only mark the specified notification", () => {
      store.getState().addNotification({ type: "info", title: "First" });
      store.getState().addNotification({ type: "info", title: "Second" });
      const firstId = store.getState().notifications[1].id;

      store.getState().markNotificationRead(firstId);

      const notifications = store.getState().notifications;
      expect(notifications[0].read).toBe(false);
      expect(notifications[1].read).toBe(true);
    });
  });

  describe("markAllNotificationsRead", () => {
    it("should mark all notifications as read", () => {
      store.getState().addNotification({ type: "info", title: "First" });
      store.getState().addNotification({ type: "info", title: "Second" });
      store.getState().addNotification({ type: "info", title: "Third" });

      store.getState().markAllNotificationsRead();

      const notifications = store.getState().notifications;
      expect(notifications.every((n) => n.read)).toBe(true);
    });

    it("should work with empty notifications", () => {
      expect(() => {
        store.getState().markAllNotificationsRead();
      }).not.toThrow();
    });
  });

  describe("removeNotification", () => {
    it("should remove a notification by id", () => {
      store.getState().addNotification({ type: "info", title: "Test" });
      const notificationId = store.getState().notifications[0].id;

      store.getState().removeNotification(notificationId);

      expect(store.getState().notifications).toHaveLength(0);
    });

    it("should not throw for non-existent notification id", () => {
      expect(() => {
        store.getState().removeNotification("non-existent");
      }).not.toThrow();
    });

    it("should only remove the specified notification", () => {
      store.getState().addNotification({ type: "info", title: "First" });
      store.getState().addNotification({ type: "info", title: "Second" });
      const firstId = store.getState().notifications[1].id;

      store.getState().removeNotification(firstId);

      const notifications = store.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].title).toBe("Second");
    });
  });

  describe("clearNotifications", () => {
    it("should remove all notifications", () => {
      store.getState().addNotification({ type: "info", title: "First" });
      store.getState().addNotification({ type: "info", title: "Second" });

      store.getState().clearNotifications();

      expect(store.getState().notifications).toHaveLength(0);
    });

    it("should work with empty notifications", () => {
      expect(() => {
        store.getState().clearNotifications();
      }).not.toThrow();
    });
  });

  describe("setNotificationsExpanded", () => {
    it("should set notifications expanded to true", () => {
      store.getState().setNotificationsExpanded(true);
      expect(store.getState().notificationsExpanded).toBe(true);
    });

    it("should set notifications expanded to false", () => {
      store.getState().setNotificationsExpanded(true);
      store.getState().setNotificationsExpanded(false);
      expect(store.getState().notificationsExpanded).toBe(false);
    });
  });

  describe("selectors", () => {
    it("selectNotifications should return notifications", () => {
      store.getState().addNotification({ type: "info", title: "Test" });
      expect(selectNotifications(store.getState())).toHaveLength(1);
    });

    it("selectNotifications should return stable empty array for empty state", () => {
      const result1 = selectNotifications(initialNotificationState);
      const result2 = selectNotifications(initialNotificationState);
      expect(result1).toBe(result2); // Same reference
    });

    it("selectUnreadNotificationCount should count unread notifications", () => {
      store.getState().addNotification({ type: "info", title: "First" });
      store.getState().addNotification({ type: "info", title: "Second" });
      const firstId = store.getState().notifications[1].id;
      store.getState().markNotificationRead(firstId);

      expect(selectUnreadNotificationCount(store.getState())).toBe(1);
    });

    it("selectNotificationsExpanded should return expanded state", () => {
      expect(selectNotificationsExpanded(store.getState())).toBe(false);
      store.getState().setNotificationsExpanded(true);
      expect(selectNotificationsExpanded(store.getState())).toBe(true);
    });
  });
});
