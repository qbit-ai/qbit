/**
 * Tiny module for mock browser mode detection.
 *
 * This is extracted from mocks.ts to prevent the large mocks module
 * (1800+ lines) from being included in production builds.
 *
 * The mocks.ts file sets window.__MOCK_BROWSER_MODE__ = true when initialized.
 * Components should import isMockBrowserMode from this tiny module instead
 * of importing from mocks.ts.
 */

declare global {
  interface Window {
    __MOCK_BROWSER_MODE__?: boolean;
  }
}

/**
 * Check if we're running in mock browser mode.
 * This function is safe to call in production - it will return false
 * unless the mocks have been explicitly initialized.
 */
export function isMockBrowserMode(): boolean {
  return typeof window !== "undefined" && window.__MOCK_BROWSER_MODE__ === true;
}
