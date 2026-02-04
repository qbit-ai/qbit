import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getSettingsCached,
  invalidateSettingsCache,
  SETTINGS_CACHE_TTL_MS,
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
});
