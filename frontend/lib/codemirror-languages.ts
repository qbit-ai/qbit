/**
 * Dynamic CodeMirror language loader.
 *
 * This module provides lazy loading of CodeMirror language packages.
 * Languages are loaded on-demand based on file extension, which prevents
 * all 13+ language packages from being included in the initial bundle.
 *
 * Usage:
 *   const extension = await getLanguageExtension("typescript");
 *   if (extension) {
 *     // Add to CodeMirror extensions
 *   }
 */

import type { Extension } from "@codemirror/state";

/**
 * Dynamically loads and returns the CodeMirror language extension
 * for the specified language. Returns null for unsupported languages.
 *
 * @param language - The language identifier (e.g., "typescript", "python", "rust")
 * @returns A promise that resolves to the language extension, or null if unsupported
 */
export async function getLanguageExtension(language?: string): Promise<Extension | null> {
  if (!language) return null;

  switch (language) {
    case "typescript":
    case "tsx": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true, typescript: true });
    }

    case "javascript":
    case "jsx": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true, typescript: false });
    }

    case "json": {
      const { json } = await import("@codemirror/lang-json");
      return json();
    }

    case "markdown":
    case "md": {
      const { markdown } = await import("@codemirror/lang-markdown");
      return markdown();
    }

    case "python":
    case "py": {
      const { python } = await import("@codemirror/lang-python");
      return python();
    }

    case "rust":
    case "rs": {
      const { rust } = await import("@codemirror/lang-rust");
      return rust();
    }

    case "go": {
      const { go } = await import("@codemirror/lang-go");
      return go();
    }

    case "yaml":
    case "yml": {
      const { yaml } = await import("@codemirror/lang-yaml");
      return yaml();
    }

    case "html": {
      const { html } = await import("@codemirror/lang-html");
      return html();
    }

    case "css": {
      const { css } = await import("@codemirror/lang-css");
      return css();
    }

    case "sql": {
      const { sql } = await import("@codemirror/lang-sql");
      return sql();
    }

    case "xml": {
      const { xml } = await import("@codemirror/lang-xml");
      return xml();
    }

    case "java": {
      const { java } = await import("@codemirror/lang-java");
      return java();
    }

    case "cpp":
    case "c":
    case "h":
    case "hpp": {
      const { cpp } = await import("@codemirror/lang-cpp");
      return cpp();
    }

    // TOML: no official @codemirror/lang-toml package; fall back to no highlighting.
    case "toml":
      return null;

    default:
      return null;
  }
}

/**
 * Maps file extensions to language identifiers.
 * Used for determining which language extension to load based on filename.
 */
export function getLanguageFromExtension(extension: string): string | null {
  const normalized = extension.toLowerCase().replace(/^\./, "");

  const extensionMap: Record<string, string> = {
    // JavaScript/TypeScript
    ts: "typescript",
    tsx: "tsx",
    js: "javascript",
    jsx: "jsx",
    mjs: "javascript",
    cjs: "javascript",

    // Markup
    json: "json",
    md: "markdown",
    markdown: "markdown",
    html: "html",
    htm: "html",
    xml: "xml",
    svg: "xml",

    // Styles
    css: "css",

    // Python
    py: "python",
    pyw: "python",
    pyi: "python",

    // Rust
    rs: "rust",

    // Go
    go: "go",

    // Java
    java: "java",

    // C/C++
    c: "c",
    h: "h",
    cpp: "cpp",
    hpp: "hpp",
    cc: "cpp",
    cxx: "cpp",

    // Data/Config
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",

    // SQL
    sql: "sql",
  };

  return extensionMap[normalized] ?? null;
}
