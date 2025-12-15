/**
 * Notification helper - drop-in replacement for toast notifications.
 *
 * Usage:
 *   import { notify } from "@/lib/notify";
 *
 *   notify.success("Task completed");
 *   notify.error("Something went wrong");
 *   notify.info("Processing...");
 *   notify.warning("Low disk space");
 *
 *   // With message
 *   notify.success("Saved", { message: "Your changes have been saved" });
 */

import { useStore, type NotificationType } from "@/store";

interface NotifyOptions {
  message?: string;
}

function addNotification(type: NotificationType, title: string, options?: NotifyOptions) {
  useStore.getState().addNotification({
    type,
    title,
    message: options?.message,
  });
}

export const notify = {
  success: (title: string, options?: NotifyOptions) => addNotification("success", title, options),
  error: (title: string, options?: NotifyOptions) => addNotification("error", title, options),
  info: (title: string, options?: NotifyOptions) => addNotification("info", title, options),
  warning: (title: string, options?: NotifyOptions) => addNotification("warning", title, options),
};
