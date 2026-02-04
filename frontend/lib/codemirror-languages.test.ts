import { describe, expect, it } from "vitest";
import { getLanguageExtension, getLanguageFromExtension } from "./codemirror-languages";

describe("codemirror-languages", () => {
  describe("getLanguageExtension", () => {
    it("should return null for undefined language", async () => {
      const result = await getLanguageExtension(undefined);
      expect(result).toBeNull();
    });

    it("should return null for unsupported language", async () => {
      const result = await getLanguageExtension("brainfuck");
      expect(result).toBeNull();
    });

    it("should return null for toml (no official package)", async () => {
      const result = await getLanguageExtension("toml");
      expect(result).toBeNull();
    });

    it("should dynamically load typescript extension", async () => {
      const result = await getLanguageExtension("typescript");
      expect(result).not.toBeNull();
    });

    it("should dynamically load javascript extension", async () => {
      const result = await getLanguageExtension("javascript");
      expect(result).not.toBeNull();
    });

    it("should dynamically load json extension", async () => {
      const result = await getLanguageExtension("json");
      expect(result).not.toBeNull();
    });

    it("should dynamically load markdown extension", async () => {
      const result = await getLanguageExtension("markdown");
      expect(result).not.toBeNull();
    });

    it("should dynamically load python extension", async () => {
      const result = await getLanguageExtension("python");
      expect(result).not.toBeNull();
    });

    it("should dynamically load rust extension", async () => {
      const result = await getLanguageExtension("rust");
      expect(result).not.toBeNull();
    });

    it("should dynamically load go extension", async () => {
      const result = await getLanguageExtension("go");
      expect(result).not.toBeNull();
    });

    it("should dynamically load yaml extension", async () => {
      const result = await getLanguageExtension("yaml");
      expect(result).not.toBeNull();
    });

    it("should dynamically load html extension", async () => {
      const result = await getLanguageExtension("html");
      expect(result).not.toBeNull();
    });

    it("should dynamically load css extension", async () => {
      const result = await getLanguageExtension("css");
      expect(result).not.toBeNull();
    });

    it("should dynamically load sql extension", async () => {
      const result = await getLanguageExtension("sql");
      expect(result).not.toBeNull();
    });

    it("should dynamically load xml extension", async () => {
      const result = await getLanguageExtension("xml");
      expect(result).not.toBeNull();
    });

    it("should dynamically load java extension", async () => {
      const result = await getLanguageExtension("java");
      expect(result).not.toBeNull();
    });

    it("should dynamically load cpp extension", async () => {
      const result = await getLanguageExtension("cpp");
      expect(result).not.toBeNull();
    });

    // Test aliases
    it("should handle tsx alias for typescript", async () => {
      const result = await getLanguageExtension("tsx");
      expect(result).not.toBeNull();
    });

    it("should handle jsx alias for javascript", async () => {
      const result = await getLanguageExtension("jsx");
      expect(result).not.toBeNull();
    });

    it("should handle md alias for markdown", async () => {
      const result = await getLanguageExtension("md");
      expect(result).not.toBeNull();
    });

    it("should handle py alias for python", async () => {
      const result = await getLanguageExtension("py");
      expect(result).not.toBeNull();
    });

    it("should handle rs alias for rust", async () => {
      const result = await getLanguageExtension("rs");
      expect(result).not.toBeNull();
    });

    it("should handle yml alias for yaml", async () => {
      const result = await getLanguageExtension("yml");
      expect(result).not.toBeNull();
    });

    it("should handle c/h aliases for cpp", async () => {
      const resultC = await getLanguageExtension("c");
      const resultH = await getLanguageExtension("h");
      const resultHpp = await getLanguageExtension("hpp");

      expect(resultC).not.toBeNull();
      expect(resultH).not.toBeNull();
      expect(resultHpp).not.toBeNull();
    });
  });

  describe("getLanguageFromExtension", () => {
    it("should return null for unknown extension", () => {
      expect(getLanguageFromExtension("xyz")).toBeNull();
      expect(getLanguageFromExtension(".unknown")).toBeNull();
    });

    it("should handle extension with or without leading dot", () => {
      expect(getLanguageFromExtension(".ts")).toBe("typescript");
      expect(getLanguageFromExtension("ts")).toBe("typescript");
    });

    it("should be case-insensitive", () => {
      expect(getLanguageFromExtension("TS")).toBe("typescript");
      expect(getLanguageFromExtension(".PY")).toBe("python");
    });

    // TypeScript/JavaScript
    it("should map TypeScript extensions", () => {
      expect(getLanguageFromExtension("ts")).toBe("typescript");
      expect(getLanguageFromExtension("tsx")).toBe("tsx");
    });

    it("should map JavaScript extensions", () => {
      expect(getLanguageFromExtension("js")).toBe("javascript");
      expect(getLanguageFromExtension("jsx")).toBe("jsx");
      expect(getLanguageFromExtension("mjs")).toBe("javascript");
      expect(getLanguageFromExtension("cjs")).toBe("javascript");
    });

    // Markup
    it("should map markup extensions", () => {
      expect(getLanguageFromExtension("json")).toBe("json");
      expect(getLanguageFromExtension("md")).toBe("markdown");
      expect(getLanguageFromExtension("markdown")).toBe("markdown");
      expect(getLanguageFromExtension("html")).toBe("html");
      expect(getLanguageFromExtension("htm")).toBe("html");
      expect(getLanguageFromExtension("xml")).toBe("xml");
      expect(getLanguageFromExtension("svg")).toBe("xml");
    });

    // Styles
    it("should map style extensions", () => {
      expect(getLanguageFromExtension("css")).toBe("css");
    });

    // Python
    it("should map Python extensions", () => {
      expect(getLanguageFromExtension("py")).toBe("python");
      expect(getLanguageFromExtension("pyw")).toBe("python");
      expect(getLanguageFromExtension("pyi")).toBe("python");
    });

    // Rust
    it("should map Rust extensions", () => {
      expect(getLanguageFromExtension("rs")).toBe("rust");
    });

    // Go
    it("should map Go extensions", () => {
      expect(getLanguageFromExtension("go")).toBe("go");
    });

    // Java
    it("should map Java extensions", () => {
      expect(getLanguageFromExtension("java")).toBe("java");
    });

    // C/C++
    it("should map C/C++ extensions", () => {
      expect(getLanguageFromExtension("c")).toBe("c");
      expect(getLanguageFromExtension("h")).toBe("h");
      expect(getLanguageFromExtension("cpp")).toBe("cpp");
      expect(getLanguageFromExtension("hpp")).toBe("hpp");
      expect(getLanguageFromExtension("cc")).toBe("cpp");
      expect(getLanguageFromExtension("cxx")).toBe("cpp");
    });

    // Config
    it("should map config file extensions", () => {
      expect(getLanguageFromExtension("yaml")).toBe("yaml");
      expect(getLanguageFromExtension("yml")).toBe("yaml");
      expect(getLanguageFromExtension("toml")).toBe("toml");
    });

    // SQL
    it("should map SQL extensions", () => {
      expect(getLanguageFromExtension("sql")).toBe("sql");
    });
  });
});
