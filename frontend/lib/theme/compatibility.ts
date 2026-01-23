import type { QbitTheme } from "./types";

export type ThemeCompatibility =
  | { compatible: true; message?: undefined }
  | { compatible: false; message: string };

function parseSemver(version: string): [number, number, number] | null {
  const match = version.trim().match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function compareSemver(a: string, b: string): number {
  const pa = parseSemver(a);
  const pb = parseSemver(b);

  // If parsing fails, treat as equal so we don't incorrectly warn.
  if (!pa || !pb) return 0;

  for (let i = 0; i < 3; i++) {
    if (pa[i] < pb[i]) return -1;
    if (pa[i] > pb[i]) return 1;
  }
  return 0;
}

export function getThemeCompatibility(theme: QbitTheme, appVersion: string): ThemeCompatibility {
  const min = theme.minAppVersion;
  const max = theme.maxAppVersion;

  // No declared constraints => assume compatible.
  if (!min && !max) return { compatible: true };

  if (min && compareSemver(appVersion, min) < 0) {
    return {
      compatible: false,
      message: `Requires Qbit >= ${min} (you are on ${appVersion})`,
    };
  }

  if (max && compareSemver(appVersion, max) > 0) {
    return {
      compatible: false,
      message: `Requires Qbit <= ${max} (you are on ${appVersion})`,
    };
  }

  return { compatible: true };
}
