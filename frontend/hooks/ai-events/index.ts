/**
 * AI Events module.
 *
 * Provides a registry-based pattern for handling AI events from the backend.
 * Event handlers are organized by domain (core, tool, workflow, etc.) and
 * combined into a single registry for dispatch.
 */

export { dispatchEvent, eventHandlerRegistry } from "./registry";
export type { EventHandler, EventHandlerContext, EventHandlerRegistry } from "./types";
