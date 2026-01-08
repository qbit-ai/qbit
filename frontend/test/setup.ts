import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { enableMapSet } from "immer";
import { afterEach, vi } from "vitest";

// Enable Immer MapSet plugin for Set/Map support in store
enableMapSet();

// Mock terminal managers for tests
vi.mock("@/lib/terminal", () => ({
  liveTerminalManager: {
    create: vi.fn(),
    getOrCreate: vi.fn(),
    attachToContainer: vi.fn(),
    write: vi.fn(),
    dispose: vi.fn(),
    scrollToBottom: vi.fn(),
    serializeAndDispose: vi.fn().mockResolvedValue(""),
  },
  virtualTerminalManager: {
    create: vi.fn(),
    write: vi.fn(),
    dispose: vi.fn(),
  },
}));

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Mock crypto.randomUUID for consistent test IDs
vi.stubGlobal("crypto", {
  randomUUID: vi.fn(() => `test-uuid-${Math.random().toString(36).slice(2, 9)}`),
});

// Mock scrollIntoView which is not implemented in jsdom
Element.prototype.scrollIntoView = vi.fn();

// Mock ResizeObserver which is not implemented in jsdom
class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
vi.stubGlobal("ResizeObserver", MockResizeObserver);
