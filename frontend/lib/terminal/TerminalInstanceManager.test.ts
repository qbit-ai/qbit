import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { TerminalInstanceManager } from "@/lib/terminal/TerminalInstanceManager";

describe("TerminalInstanceManager", () => {
  beforeAll(() => {
    // JSDOM may not provide requestAnimationFrame; manager uses it for safeFit.
    // Use setTimeout(0) so callbacks fire asynchronously, which is required for
    // the deferred-fit test to correctly observe that fit() is NOT called synchronously.
    if (typeof globalThis.requestAnimationFrame !== "function") {
      let rafId = 0;
      vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
        const id = ++rafId;
        setTimeout(() => cb(performance.now()), 0);
        return id;
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

    // biome-ignore lint/suspicious/noExplicitAny: Test mocks don't need full type implementation
    TerminalInstanceManager.register(sessionId, terminal as any, fitAddon as any);
    expect(() => TerminalInstanceManager.detach(sessionId)).not.toThrow();
  });

  it("defers fit() call during reattachment to avoid race condition", async () => {
    const sessionId = "session-3";

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

    // Initial attach
    TerminalInstanceManager.attachToContainer(sessionId, containerA);

    // Clear previous fit calls
    fitAddon.fit.mockClear();

    // Detach and reattach (simulating pane restructuring)
    TerminalInstanceManager.detach(sessionId);
    TerminalInstanceManager.attachToContainer(sessionId, containerB);

    // fit() should NOT be called synchronously during reattachment
    // It should be deferred to the next animation frame
    expect(fitAddon.fit).not.toHaveBeenCalled();

    // Wait for RAF to fire
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => resolve());
    });

    // Now fit() should have been called
    expect(fitAddon.fit).toHaveBeenCalled();

    // Cleanup
    containerA.remove();
    containerB.remove();
  });
});
