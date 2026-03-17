import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { QbitSettings } from "./settings";
import {
  getSettingsCached,
  invalidateSettingsCache,
  reloadSettings,
  resetSettings,
  SETTINGS_CACHE_TTL_MS,
  setSetting,
  updateSettings,
} from "./settings";

// The invoke mock is set up in vitest.config.ts
// We'll mock it per-test to control behavior

describe("Settings Cache", () => {
  beforeEach(() => {
    // Reset the cache before each test
    invalidateSettingsCache();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("getSettingsCached", () => {
    it("should call invoke on first call", async () => {
      await getSettingsCached();

      expect(invoke).toHaveBeenCalledWith("get_settings");
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it("should return cached settings on subsequent calls within TTL", async () => {
      // First call - should hit backend
      const settings1 = await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1);

      // Second call - should use cache
      const settings2 = await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1); // Still 1, no new call

      // Should return same reference (from cache)
      expect(settings1).toBe(settings2);
    });

    it("should refresh cache after TTL expires", async () => {
      vi.useFakeTimers();

      // First call
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1);

      // Advance time past TTL
      vi.advanceTimersByTime(SETTINGS_CACHE_TTL_MS + 100);

      // Second call should refresh cache
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(2);
    });

    it("should not refresh cache before TTL expires", async () => {
      vi.useFakeTimers();

      // First call
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1);

      // Advance time but stay within TTL
      vi.advanceTimersByTime(SETTINGS_CACHE_TTL_MS - 100);

      // Second call should still use cache
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1);
    });
  });

  describe("invalidateSettingsCache", () => {
    it("should force a refresh on next getSettingsCached call", async () => {
      // First call - populates cache
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1);

      // Second call - uses cache
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(1);

      // Invalidate cache
      invalidateSettingsCache();

      // Third call - should refresh
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledTimes(2);
    });
  });

  describe("cache invalidation after mutations", () => {
    // For each mutation test: prime the cache, call a mutator, then assert the next
    // getSettingsCached() triggers a fresh invoke("get_settings").
    // We also stub the mutation commands explicitly so the default mock doesn't emit
    // console.warn for unhandled commands, and so tests fail if a wrong command name is used.

    beforeEach(() => {
      vi.mocked(invoke).mockImplementation(async (command: string) => {
        switch (command) {
          case "get_settings":
            return { terminal: { fullterm_commands: [] }, ai: {} };
          case "update_settings":
          case "set_setting":
          case "reset_settings":
          case "reload_settings":
            return undefined;
          default:
            throw new Error(`Unexpected invoke command in mutation tests: ${command}`);
        }
      });
    });

    it("updateSettings should invalidate the cache", async () => {
      // Prime the cache
      await getSettingsCached();
      expect(invoke).toHaveBeenCalledWith("get_settings");
      const callsBefore = vi.mocked(invoke).mock.calls.length;

      // Mutate
      await updateSettings({} as QbitSettings);
      expect(invoke).toHaveBeenCalledWith("update_settings", { settings: {} });

      // Next read must bypass cache
      await getSettingsCached();
      const calls = vi.mocked(invoke).mock.calls;
      expect(calls.length).toBeGreaterThan(callsBefore);
      expect(calls[calls.length - 1]?.[0]).toBe("get_settings");
    });

    it("setSetting should invalidate the cache", async () => {
      await getSettingsCached();
      const callsBefore = vi.mocked(invoke).mock.calls.length;

      await setSetting("ui.theme", "dark");
      expect(invoke).toHaveBeenCalledWith("set_setting", { key: "ui.theme", value: "dark" });

      await getSettingsCached();
      const calls = vi.mocked(invoke).mock.calls;
      expect(calls.length).toBeGreaterThan(callsBefore);
      expect(calls[calls.length - 1]?.[0]).toBe("get_settings");
    });

    it("resetSettings should invalidate the cache", async () => {
      await getSettingsCached();
      const callsBefore = vi.mocked(invoke).mock.calls.length;

      await resetSettings();
      expect(invoke).toHaveBeenCalledWith("reset_settings");

      await getSettingsCached();
      const calls = vi.mocked(invoke).mock.calls;
      expect(calls.length).toBeGreaterThan(callsBefore);
      expect(calls[calls.length - 1]?.[0]).toBe("get_settings");
    });

    it("reloadSettings should invalidate the cache", async () => {
      await getSettingsCached();
      const callsBefore = vi.mocked(invoke).mock.calls.length;

      await reloadSettings();
      expect(invoke).toHaveBeenCalledWith("reload_settings");

      await getSettingsCached();
      const calls = vi.mocked(invoke).mock.calls;
      expect(calls.length).toBeGreaterThan(callsBefore);
      expect(calls[calls.length - 1]?.[0]).toBe("get_settings");
    });
  });
});
