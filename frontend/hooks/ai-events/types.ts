/**
 * Types for the AI event handler registry.
 *
 * This module defines the interfaces for event handlers and their context.
 */

import type { AiEvent, ToolSource } from "@/lib/ai";
import type { ToolCallSource, useStore } from "@/store";

/** Store state type derived from the store */
type StoreState = ReturnType<typeof useStore.getState>;

/**
 * Context provided to all event handlers.
 * Contains utilities and state access needed for handling events.
 */
export interface EventHandlerContext {
  /** The session ID for this event */
  sessionId: string;
  /** Access to the store's getState function */
  getState: () => StoreState;
  /** Flush pending text deltas for a session (ensures correct ordering) */
  flushSessionDeltas: (sessionId: string) => void;
  /** Batch a text delta for throttled updates */
  batchTextDelta: (sessionId: string, delta: string) => void;
  /** Convert backend tool source to store tool source */
  convertToolSource: (source?: ToolSource) => ToolCallSource | undefined;
}

/**
 * Type for an event handler function.
 * Handlers receive the event and context, and perform store updates.
 */
export type EventHandler<T extends AiEvent = AiEvent> = (
  event: T,
  ctx: EventHandlerContext
) => void | Promise<void>;

/**
 * Registry mapping event types to their handlers.
 */
export type EventHandlerRegistry = Partial<{
  [K in AiEvent["type"]]: EventHandler<Extract<AiEvent, { type: K }>>;
}>;
