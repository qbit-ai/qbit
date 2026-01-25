/**
 * Notification slice for the Zustand store.
 *
 * Manages notification state including adding, reading, and clearing notifications.
 */

import type { SliceCreator } from "./types";

// Types
export type NotificationType = "info" | "success" | "warning" | "error";

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message?: string;
  timestamp: string;
  read: boolean;
}

// State interface
export interface NotificationState {
  notifications: Notification[];
  notificationsExpanded: boolean;
}

// Actions interface
export interface NotificationActions {
  addNotification: (notification: Omit<Notification, "id" | "timestamp" | "read">) => void;
  markNotificationRead: (notificationId: string) => void;
  markAllNotificationsRead: () => void;
  removeNotification: (notificationId: string) => void;
  clearNotifications: () => void;
  setNotificationsExpanded: (expanded: boolean) => void;
}

// Combined slice interface
export interface NotificationSlice extends NotificationState, NotificationActions {}

// Initial state
export const initialNotificationState: NotificationState = {
  notifications: [],
  notificationsExpanded: false,
};

/**
 * Creates the notification slice.
 * This slice manages all notification-related state and actions.
 */
export const createNotificationSlice: SliceCreator<NotificationSlice> = (set) => ({
  // State
  ...initialNotificationState,

  // Actions
  addNotification: (notification) =>
    set((state) => {
      state.notifications.unshift({
        ...notification,
        id: crypto.randomUUID(),
        timestamp: new Date().toISOString(),
        read: false,
      });
    }),

  markNotificationRead: (notificationId) =>
    set((state) => {
      const notification = state.notifications.find((n) => n.id === notificationId);
      if (notification) {
        notification.read = true;
      }
    }),

  markAllNotificationsRead: () =>
    set((state) => {
      for (const notification of state.notifications) {
        notification.read = true;
      }
    }),

  removeNotification: (notificationId) =>
    set((state) => {
      state.notifications = state.notifications.filter((n) => n.id !== notificationId);
    }),

  clearNotifications: () =>
    set((state) => {
      state.notifications = [];
    }),

  setNotificationsExpanded: (expanded) =>
    set((state) => {
      state.notificationsExpanded = expanded;
    }),
});

// Stable empty array for selectors
const EMPTY_NOTIFICATIONS: Notification[] = [];

/**
 * Selector for notifications array.
 * Returns stable empty array when no notifications.
 */
export const selectNotifications = <T extends NotificationState>(state: T): Notification[] =>
  state.notifications ?? EMPTY_NOTIFICATIONS;

/**
 * Selector for unread notification count.
 */
export const selectUnreadNotificationCount = <T extends NotificationState>(state: T): number =>
  state.notifications.filter((n) => !n.read).length;

/**
 * Selector for notifications expanded state.
 */
export const selectNotificationsExpanded = <T extends NotificationState>(state: T): boolean =>
  state.notificationsExpanded;
