/**
 * Shared types for store slices.
 *
 * This module defines types for creating Zustand slices that can be
 * composed in the main store with Immer middleware.
 */

import type { WritableDraft } from "immer";

/**
 * Type for the Immer-wrapped set function.
 * Allows mutations via a callback that receives a writable draft.
 */
export type ImmerSet<TState> = (fn: (state: WritableDraft<TState>) => void) => void;

/**
 * Type for the get function that returns the current state.
 */
export type StateGet<TState> = () => TState;

/**
 * Type for a slice creator function.
 * Takes set and get functions and returns the slice state and actions.
 *
 * Note: The TState generic defaults to TSlice for testing slices in isolation,
 * but can be widened to the full store state when composing slices.
 */
export type SliceCreator<TSlice, TState = TSlice> = (
  set: ImmerSet<TState>,
  get: StateGet<TState>
) => TSlice;
