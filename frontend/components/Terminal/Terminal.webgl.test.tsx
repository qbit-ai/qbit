import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock logger
vi.mock("@/lib/logger", () => ({
  logger: {
    warn: vi.fn(),
    debug: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

// Use vi.hoisted to ensure mock functions are available during vi.mock hoisting
const {
  mockLoadAddon,
  mockOpen,
  mockOnData,
  mockOnResize,
  mockFocus,
  mockWebglDispose,
  mockOnContextLoss,
} = vi.hoisted(() => ({
  mockLoadAddon: vi.fn(),
  mockOpen: vi.fn(),
  mockOnData: vi.fn(() => ({ dispose: vi.fn() })),
  mockOnResize: vi.fn(() => ({ dispose: vi.fn() })),
  mockFocus: vi.fn(),
  mockWebglDispose: vi.fn(),
  mockOnContextLoss: vi.fn(),
}));

vi.mock("@xterm/xterm", () => {
  return {
    Terminal: class {
      loadAddon = mockLoadAddon;
      open = mockOpen;
      onData = mockOnData;
      onResize = mockOnResize;
      focus = mockFocus;
      rows = 24;
      cols = 80;
      modes = {};
    },
  };
});

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit = vi.fn();
  },
}));

vi.mock("@xterm/addon-web-links", () => ({
  WebLinksAddon: class {},
}));

// Track WebGL addon instance for testing context loss
let webglAddonInstance: { dispose: () => void; onContextLoss: (cb: () => void) => void } | null =
  null;

vi.mock("@xterm/addon-webgl", () => ({
  WebglAddon: class {
    dispose = mockWebglDispose;
    private contextLossCallback: (() => void) | null = null;

    constructor() {
      webglAddonInstance = this;
    }

    onContextLoss(callback: () => void) {
      this.contextLossCallback = callback;
      mockOnContextLoss(callback);
    }

    // Helper to simulate context loss
    simulateContextLoss() {
      if (this.contextLossCallback) {
        this.contextLossCallback();
      }
    }
  },
}));

// Mock Tauri API
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({
  ptyWrite: vi.fn().mockResolvedValue(undefined),
  ptyResize: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/lib/terminal/TerminalInstanceManager", () => ({
  TerminalInstanceManager: {
    get: vi.fn().mockReturnValue(undefined),
    register: vi.fn(),
    attachToContainer: vi.fn(),
    detach: vi.fn(),
    dispose: vi.fn(),
  },
}));

vi.mock("@/lib/theme", () => ({
  ThemeManager: {
    applyToTerminal: vi.fn(),
    onChange: vi.fn(() => () => {}),
  },
}));

vi.mock("@/store", () => ({
  useRenderMode: vi.fn(() => "timeline"),
  useTerminalClearRequest: vi.fn(() => 0),
}));

import { logger } from "@/lib/logger";
import { Terminal } from "./Terminal";

// Store original createElement for restoration
const originalCreateElement = document.createElement.bind(document);

// Mock WebGL context
const mockWebGLContext = {
  getExtension: vi.fn(),
  getParameter: vi.fn(),
};

// Helper to set up WebGL-available mock
function setupWebGLAvailable() {
  vi.spyOn(document, "createElement").mockImplementation((tagName: string) => {
    const element = originalCreateElement(tagName);
    if (tagName === "canvas") {
      vi.spyOn(element as HTMLCanvasElement, "getContext").mockImplementation(
        (contextId: string) => {
          if (contextId === "webgl2" || contextId === "webgl") {
            return mockWebGLContext as unknown as WebGLRenderingContext;
          }
          return null;
        }
      );
    }
    return element;
  });
}

// Helper to set up WebGL-unavailable mock
function setupWebGLUnavailable() {
  vi.spyOn(document, "createElement").mockImplementation((tagName: string) => {
    const element = originalCreateElement(tagName);
    if (tagName === "canvas") {
      vi.spyOn(element as HTMLCanvasElement, "getContext").mockReturnValue(null);
    }
    return element;
  });
}

describe("Terminal WebGL Error Recovery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    webglAddonInstance = null;
    // Default: WebGL is available
    setupWebGLAvailable();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  describe("WebGL availability check", () => {
    it("should check for WebGL availability before creating addon", async () => {
      render(<Terminal sessionId="test-session" />);

      await waitFor(() => {
        expect(mockLoadAddon).toHaveBeenCalled();
      });

      // WebGL addon should be loaded when WebGL is available
      expect(mockOnContextLoss).toHaveBeenCalled();
      expect(logger.debug).toHaveBeenCalledWith(
        expect.stringContaining("WebGL renderer active"),
        expect.anything()
      );
    });

    it("should skip WebGL addon when WebGL is not available", async () => {
      // Override mock to return null for WebGL context
      vi.restoreAllMocks();
      setupWebGLUnavailable();

      render(<Terminal sessionId="test-session-no-webgl" />);

      await waitFor(() => {
        // FitAddon and WebLinksAddon should still be loaded
        expect(mockLoadAddon).toHaveBeenCalled();
      });

      // WebGL context loss handler should NOT be registered
      expect(mockOnContextLoss).not.toHaveBeenCalled();

      // Should log that WebGL is not available
      expect(logger.debug).toHaveBeenCalledWith(
        expect.stringContaining("WebGL not available")
      );
    });

    it("should try webgl2 first, then fall back to webgl", async () => {
      vi.restoreAllMocks();

      const getContextSpy = vi.fn((contextId: string) => {
        if (contextId === "webgl2") {
          return null; // WebGL2 not available
        }
        if (contextId === "webgl") {
          return mockWebGLContext as unknown as WebGLRenderingContext;
        }
        return null;
      });

      vi.spyOn(document, "createElement").mockImplementation((tagName: string) => {
        const element = originalCreateElement(tagName);
        if (tagName === "canvas") {
          vi.spyOn(element as HTMLCanvasElement, "getContext").mockImplementation(getContextSpy);
        }
        return element;
      });

      render(<Terminal sessionId="test-session-webgl1" />);

      await waitFor(() => {
        expect(mockOnContextLoss).toHaveBeenCalled();
      });

      // Should have tried webgl2 first, then webgl
      expect(getContextSpy).toHaveBeenCalledWith("webgl2");
      expect(getContextSpy).toHaveBeenCalledWith("webgl");
    });
  });

  describe("WebGL context loss handling", () => {
    it("should register a context loss handler when loading WebGL addon", async () => {
      render(<Terminal sessionId="test-session-loss-1" />);

      await waitFor(() => {
        expect(mockLoadAddon).toHaveBeenCalled();
      });

      // Check that onContextLoss handler was registered
      expect(mockOnContextLoss).toHaveBeenCalled();
    });

    it("should dispose WebGL addon on context loss", async () => {
      render(<Terminal sessionId="test-session-loss-2" />);

      await waitFor(() => {
        expect(mockOnContextLoss).toHaveBeenCalled();
      });

      // Simulate context loss by calling the registered callback
      if (webglAddonInstance) {
        (webglAddonInstance as unknown as { simulateContextLoss: () => void }).simulateContextLoss();
      }

      expect(mockWebglDispose).toHaveBeenCalled();
    });

    it("should log warning on context loss", async () => {
      render(<Terminal sessionId="test-session-loss-3" />);

      await waitFor(() => {
        expect(mockOnContextLoss).toHaveBeenCalled();
      });

      // Simulate context loss
      if (webglAddonInstance) {
        (webglAddonInstance as unknown as { simulateContextLoss: () => void }).simulateContextLoss();
      }

      expect(logger.warn).toHaveBeenCalledWith(
        expect.stringContaining("WebGL context lost"),
        expect.anything()
      );
    });

    it("should handle errors during context loss disposal gracefully", async () => {
      // Make dispose throw an error
      mockWebglDispose.mockImplementation(() => {
        throw new Error("Disposal failed - context already lost");
      });

      render(<Terminal sessionId="test-session-loss-4" />);

      await waitFor(() => {
        expect(mockOnContextLoss).toHaveBeenCalled();
      });

      // Simulate context loss - should not throw
      if (webglAddonInstance) {
        expect(() => {
          (
            webglAddonInstance as unknown as { simulateContextLoss: () => void }
          ).simulateContextLoss();
        }).not.toThrow();
      }

      // Should log debug message about disposal error
      expect(logger.debug).toHaveBeenCalledWith(
        expect.stringContaining("WebGL addon disposal error"),
        expect.anything()
      );
    });

  });

  describe("WebGL cleanup on unmount", () => {
    it("should dispose WebGL addon on component unmount", async () => {
      const { unmount } = render(<Terminal sessionId="test-session-cleanup-1" />);

      await waitFor(() => {
        expect(mockOnContextLoss).toHaveBeenCalled();
      });

      // Clear previous calls
      mockWebglDispose.mockClear();

      // Unmount the component
      unmount();

      // WebGL addon should be disposed during cleanup
      expect(mockWebglDispose).toHaveBeenCalled();
    });

    it("should handle errors during cleanup disposal gracefully", async () => {
      const { unmount } = render(<Terminal sessionId="test-session-cleanup-2" />);

      await waitFor(() => {
        expect(mockOnContextLoss).toHaveBeenCalled();
      });

      // Make dispose throw (simulating addon already disposed due to context loss)
      mockWebglDispose.mockClear();
      mockWebglDispose.mockImplementation(() => {
        throw new Error("Already disposed");
      });

      // Unmount should not throw
      expect(() => unmount()).not.toThrow();

      // Should log debug message about cleanup disposal error
      expect(logger.debug).toHaveBeenCalledWith(
        expect.stringContaining("WebGL cleanup disposal error"),
        expect.anything()
      );
    });
  });

  // NOTE: This test must be last in the file because it modifies the WebglAddon mock
  // which cannot be properly restored
  describe("WebGL addon creation failure", () => {
    it("should gracefully handle WebGL addon creation failure", async () => {
      // Make WebglAddon constructor throw
      const originalWebglAddon = await import("@xterm/addon-webgl");
      const mockWebglAddonClass = vi.fn(() => {
        throw new Error("WebGL not supported");
      });
      vi.mocked(originalWebglAddon).WebglAddon =
        mockWebglAddonClass as unknown as typeof originalWebglAddon.WebglAddon;

      // Should not throw
      expect(() => render(<Terminal sessionId="test-session-loss-5" />)).not.toThrow();

      // Logger should warn about fallback
      await waitFor(() => {
        expect(logger.warn).toHaveBeenCalledWith(
          expect.stringContaining("WebGL addon failed"),
          expect.anything()
        );
      });
    });
  });
});
