import { describe, expect, it } from "vitest";
import { getThemeCompatibility } from "./compatibility";
import type { QbitTheme } from "./types";

function makeTheme(partial?: Partial<QbitTheme>): QbitTheme {
  return {
    schemaVersion: "2.0.0",
    name: "Test Theme",
    colors: {
      ui: {
        background: "#000000",
        foreground: "#ffffff",
        card: "#111111",
        cardForeground: "#ffffff",
        popover: "#111111",
        popoverForeground: "#ffffff",
        primary: "#0066ff",
        primaryForeground: "#ffffff",
        secondary: "#333333",
        secondaryForeground: "#ffffff",
        muted: "#222222",
        mutedForeground: "#888888",
        accent: "#0066ff",
        accentForeground: "#ffffff",
        destructive: "#ff0000",
        border: "#333333",
        input: "#222222",
        ring: "#0066ff",
        sidebar: "#111111",
        sidebarForeground: "#ffffff",
        sidebarPrimary: "#0066ff",
        sidebarPrimaryForeground: "#ffffff",
        sidebarAccent: "#222222",
        sidebarAccentForeground: "#ffffff",
        sidebarBorder: "#333333",
        sidebarRing: "#0066ff",
      },
      ansi: {
        black: "#000000",
        red: "#ff0000",
        green: "#00ff00",
        yellow: "#ffff00",
        blue: "#0000ff",
        magenta: "#ff00ff",
        cyan: "#00ffff",
        white: "#ffffff",
        brightBlack: "#666666",
        brightRed: "#ff6666",
        brightGreen: "#66ff66",
        brightYellow: "#ffff66",
        brightBlue: "#6666ff",
        brightMagenta: "#ff66ff",
        brightCyan: "#66ffff",
        brightWhite: "#ffffff",
        defaultFg: "#ffffff",
        defaultBg: "#000000",
      },
    },
    ...partial,
  };
}

describe("getThemeCompatibility", () => {
  it("treats themes with no constraints as compatible", () => {
    const theme = makeTheme();
    expect(getThemeCompatibility(theme, "0.2.9")).toEqual({ compatible: true });
  });

  it("enforces minAppVersion (incompatible when app < min)", () => {
    const theme = makeTheme({ minAppVersion: "1.0.0" });
    expect(getThemeCompatibility(theme, "0.2.9")).toEqual({
      compatible: false,
      message: "Requires Qbit >= 1.0.0 (you are on 0.2.9)",
    });
  });

  it("enforces minAppVersion (compatible when app == min)", () => {
    const theme = makeTheme({ minAppVersion: "1.0.0" });
    expect(getThemeCompatibility(theme, "1.0.0")).toEqual({ compatible: true });
  });

  it("enforces minAppVersion (compatible when app > min)", () => {
    const theme = makeTheme({ minAppVersion: "1.0.0" });
    expect(getThemeCompatibility(theme, "1.2.0")).toEqual({ compatible: true });
  });

  it("enforces maxAppVersion (incompatible when app > max)", () => {
    const theme = makeTheme({ maxAppVersion: "0.2.9" });
    expect(getThemeCompatibility(theme, "0.3.0")).toEqual({
      compatible: false,
      message: "Requires Qbit <= 0.2.9 (you are on 0.3.0)",
    });
  });

  it("enforces maxAppVersion (compatible when app == max)", () => {
    const theme = makeTheme({ maxAppVersion: "0.2.9" });
    expect(getThemeCompatibility(theme, "0.2.9")).toEqual({ compatible: true });
  });

  it("enforces maxAppVersion (compatible when app < max)", () => {
    const theme = makeTheme({ maxAppVersion: "0.2.9" });
    expect(getThemeCompatibility(theme, "0.2.8")).toEqual({ compatible: true });
  });

  it("enforces both min and max (compatible within range)", () => {
    const theme = makeTheme({ minAppVersion: "0.2.0", maxAppVersion: "0.2.9" });
    expect(getThemeCompatibility(theme, "0.2.5")).toEqual({ compatible: true });
  });

  it("enforces both min and max (incompatible below min)", () => {
    const theme = makeTheme({ minAppVersion: "0.2.0", maxAppVersion: "0.2.9" });
    expect(getThemeCompatibility(theme, "0.1.9")).toEqual({
      compatible: false,
      message: "Requires Qbit >= 0.2.0 (you are on 0.1.9)",
    });
  });

  it("enforces both min and max (incompatible above max)", () => {
    const theme = makeTheme({ minAppVersion: "0.2.0", maxAppVersion: "0.2.9" });
    expect(getThemeCompatibility(theme, "0.3.0")).toEqual({
      compatible: false,
      message: "Requires Qbit <= 0.2.9 (you are on 0.3.0)",
    });
  });

  it("does not warn when semver parsing fails (defensive behavior)", () => {
    const theme = makeTheme({
      // Not semver; schema validation should catch this, but compatibility logic should remain safe.
      // biome-ignore lint/suspicious/noExplicitAny: intentionally invalid shape for defensive test
      minAppVersion: ">=0.2.0" as any,
      // biome-ignore lint/suspicious/noExplicitAny: intentionally invalid shape for defensive test
      maxAppVersion: "latest" as any,
    });

    expect(getThemeCompatibility(theme, "dev")).toEqual({ compatible: true });
    expect(getThemeCompatibility(theme, "0.2.9")).toEqual({ compatible: true });
  });
});
