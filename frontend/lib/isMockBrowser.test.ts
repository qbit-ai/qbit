import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { isMockBrowserMode } from "./isMockBrowser";

describe("isMockBrowser", () => {
  describe("isMockBrowserMode", () => {
    let originalFlag: boolean | undefined;

    beforeEach(() => {
      // Save original value
      originalFlag = window.__MOCK_BROWSER_MODE__;
    });

    afterEach(() => {
      // Restore original value
      if (originalFlag === undefined) {
        delete window.__MOCK_BROWSER_MODE__;
      } else {
        window.__MOCK_BROWSER_MODE__ = originalFlag;
      }
    });

    it("should return false when __MOCK_BROWSER_MODE__ is undefined", () => {
      delete window.__MOCK_BROWSER_MODE__;
      expect(isMockBrowserMode()).toBe(false);
    });

    it("should return false when __MOCK_BROWSER_MODE__ is false", () => {
      window.__MOCK_BROWSER_MODE__ = false;
      expect(isMockBrowserMode()).toBe(false);
    });

    it("should return true when __MOCK_BROWSER_MODE__ is true", () => {
      window.__MOCK_BROWSER_MODE__ = true;
      expect(isMockBrowserMode()).toBe(true);
    });
  });
});
