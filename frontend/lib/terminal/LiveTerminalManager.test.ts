import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

const { mockFit, mockSerialize, mockThemeApply, mockThemeOnChange } = vi.hoisted(() => ({
  mockFit: vi.fn(),
  mockSerialize: vi.fn(() => ""),
  mockThemeApply: vi.fn(),
  mockThemeOnChange: vi.fn(() => () => {}),
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit = mockFit;
  },
}));

vi.mock("@xterm/addon-serialize", () => ({
  SerializeAddon: class {
    serialize = mockSerialize;
  },
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    element: HTMLElement | null = null;
    options: Record<string, unknown>;

    constructor(options: Record<string, unknown>) {
      this.options = { ...options, theme: {} };
    }

    loadAddon() {}

    open(container: HTMLElement) {
      if (!this.element) {
        const el = document.createElement("div");
        el.className = "xterm";
        this.element = el;
      }
      container.appendChild(this.element);
    }

    write(_data: string, callback?: () => void) {
      callback?.();
    }

    scrollToBottom() {}

    dispose() {
      this.element?.remove();
    }
  },
}));

vi.mock("@/lib/theme", () => ({
  ThemeManager: {
    applyToTerminal: mockThemeApply,
    onChange: mockThemeOnChange,
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    debug: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  },
}));

import { liveTerminalManager } from "./LiveTerminalManager";

describe("LiveTerminalManager", () => {
  beforeAll(() => {
    if (typeof globalThis.requestAnimationFrame !== "function") {
      vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
        cb(0);
        return 1;
      });
    }

    if (typeof globalThis.cancelAnimationFrame !== "function") {
      vi.stubGlobal("cancelAnimationFrame", (_id: number) => {});
    }
  });

  beforeEach(() => {
    vi.clearAllMocks();
    liveTerminalManager.disposeAll();
    document.getElementById("qbit-live-xterm-parking-lot")?.remove();
  });

  it("does not throw when fit runs before renderer is ready", () => {
    mockFit.mockImplementation(() => {
      throw new TypeError("undefined is not an object (evaluating 'this._renderer.value')");
    });

    const sessionId = "live-session-1";
    const container = document.createElement("div");
    document.body.appendChild(container);

    liveTerminalManager.getOrCreate(sessionId);
    expect(() => liveTerminalManager.attachToContainer(sessionId, container)).not.toThrow();
  });

  it("keeps terminal element mounted in parking lot on detach", () => {
    const sessionId = "live-session-2";
    const container = document.createElement("div");
    document.body.appendChild(container);

    liveTerminalManager.getOrCreate(sessionId);
    expect(liveTerminalManager.attachToContainer(sessionId, container)).toBe(true);

    const terminalEl = container.querySelector(".xterm");
    expect(terminalEl).toBeTruthy();

    liveTerminalManager.detach(sessionId);

    const parkingLot = document.getElementById("qbit-live-xterm-parking-lot");
    expect(parkingLot).toBeTruthy();
    expect(parkingLot?.contains(terminalEl as HTMLElement)).toBe(true);
  });
});
