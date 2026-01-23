export function getAppVersion(): string {
  // __APP_VERSION__ is injected at build time by Vite (see vite.config.ts)
  // Use typeof to avoid ReferenceError in non-Vite contexts (tests, etc).
  return typeof __APP_VERSION__ === "string" && __APP_VERSION__.length > 0 ? __APP_VERSION__ : "0.0.0";
}
