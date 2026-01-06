import { useCallback } from "react";
import { useSidecarEvents } from "@/hooks/useSidecarEvents";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import type { SidecarEventType } from "@/lib/sidecar";

/**
 * Component that subscribes to sidecar events and displays toast notifications.
 * This component renders nothing - it only handles event subscriptions.
 */
export function SidecarNotifications() {
  const handleEvent = useCallback((event: SidecarEventType) => {
    switch (event.event_type) {
      // Session events
      case "session_started":
        notify.info("Sidecar session started", {
          message: `Session: ${event.session_id.slice(0, 8)}...`,
        });
        break;

      case "session_ended":
        notify.info("Sidecar session ended", {
          message: `Session: ${event.session_id.slice(0, 8)}...`,
        });
        break;

      // Patch events
      case "patch_created":
        notify.success("Patch created", {
          message: event.subject,
        });
        break;

      case "patch_applied":
        notify.success("Patch applied", {
          message: `Commit: ${event.commit_sha.slice(0, 7)}`,
        });
        break;

      case "patch_discarded":
        notify.info("Patch discarded", {
          message: `Patch #${event.patch_id}`,
        });
        break;

      case "patch_message_updated":
        notify.info("Patch message updated", {
          message: event.new_subject,
        });
        break;

      // Artifact events
      case "artifact_created":
        notify.success("Artifact generated", {
          message: `${event.filename} → ${event.target}`,
        });
        break;

      case "artifact_applied":
        notify.success("Artifact applied", {
          message: `${event.filename} written to ${event.target}`,
        });
        break;

      case "artifact_discarded":
        notify.info("Artifact discarded", {
          message: event.filename,
        });
        break;

      // State events
      case "state_updated":
        notify.success("Session state updated", {
          message: `state.md synthesized via ${event.backend}`,
        });
        break;

      default: {
        // TypeScript exhaustiveness check
        const _exhaustive: never = event;
        logger.warn("Unknown sidecar event:", _exhaustive);
      }
    }
  }, []);

  useSidecarEvents(handleEvent);

  // This component renders nothing
  return null;
}
