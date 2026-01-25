import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { TerminalInstanceManager } from "@/lib/terminal/TerminalInstanceManager";

describe("TerminalInstanceManager", () => {
  beforeAll(() => {
    // JSDOM may not provide requestAnimationFrame; manager uses it for safeFit.
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
    // Ensure singleton doesn't leak instances or DOM across tests.
    TerminalInstanceManager.disposeAll();
    document.getElementById("qbit-xterm-parking-lot")?.remove();
  });

  it("keeps the terminal element in the DOM on detach by parking it offscreen", () => {
    const sessionId = "session-1";

    const terminalEl = document.createElement("div");
    terminalEl.className = "xterm";

    const terminal = {
      element: terminalEl,
      open: vi.fn(),
      dispose: vi.fn(),
    };

    const fitAddon = {
      fit: vi.fn(),
    };

    TerminalInstanceManager.register(
      sessionId,
      terminal as unknown as import("@xterm/xterm").Terminal,
      fitAddon as unknown as import("@xterm/addon-fit").FitAddon
    );

    const containerA = document.createElement("div");
    const containerB = document.createElement("div");
    document.body.appendChild(containerA);
    document.body.appendChild(containerB);

    expect(TerminalInstanceManager.attachToContainer(sessionId, containerA)).toBe(true);
    expect(containerA.contains(terminalEl)).toBe(true);

    // Detach should move the element to the parking lot so removing containerA
    // (React unmount) doesn't remove the xterm element.
    TerminalInstanceManager.detach(sessionId);
    const parkingLot = document.getElementById("qbit-xterm-parking-lot");
    expect(parkingLot).toBeTruthy();
    expect(parkingLot?.contains(terminalEl)).toBe(true);

    containerA.remove();
    expect(document.body.contains(terminalEl)).toBe(true);

    // Reattach should move element from parking lot into new container.
    expect(TerminalInstanceManager.attachToContainer(sessionId, containerB)).toBe(true);
    expect(containerB.contains(terminalEl)).toBe(true);
  });

  it("does not throw on detach if terminal has no element", () => {
    const sessionId = "session-2";

    const terminal = {
      element: null,
      open: vi.fn(),
      dispose: vi.fn(),
    };

    const fitAddon = {
      fit: vi.fn(),
    };

    TerminalInstanceManager.register(sessionId, terminal as any, fitAddon as any);
    expect(() => TerminalInstanceManager.detach(sessionId)).not.toThrow();
  });
});
