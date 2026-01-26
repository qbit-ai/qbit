/**
 * Store slices barrel export.
 *
 * This module exports all slice creators and their associated types
 * for composition in the main store.
 */

// Context slice
export {
  type ContextActions,
  type ContextMetrics,
  type ContextSlice,
  type ContextState,
  createContextSlice,
  initialContextState,
  selectCompactionCount,
  selectCompactionError,
  selectContextMetrics,
  selectIsCompacting,
  selectIsSessionDead,
  selectSessionTokenUsage,
} from "./context";

// Git slice
export {
  createGitSlice,
  type GitActions,
  type GitSlice,
  type GitState,
  initialGitState,
  selectGitCommitMessage,
  selectGitStatus,
  selectGitStatusLoading,
} from "./git";

// Notification slice
export {
  createNotificationSlice,
  initialNotificationState,
  type Notification,
  type NotificationActions,
  type NotificationSlice,
  type NotificationState,
  type NotificationType,
  selectNotifications,
  selectNotificationsExpanded,
  selectUnreadNotificationCount,
} from "./notification";

// Types
export type { ImmerSet, SliceCreator, StateGet } from "./types";
