import { describe, expect, it } from "vitest";
import { buildFileIndex } from "./fileIndex";
import { detectFilePathsWithIndex } from "./pathDetection";

describe("detectFilePathsWithIndex", () => {
  const workspaceRoot = "/Users/dev/project";

  function createIndex(relativePaths: string[]) {
    return buildFileIndex(
      relativePaths.map((f) => `${workspaceRoot}/${f}`),
      workspaceRoot
    );
  }

  describe("absolute paths", () => {
    it("should detect and validate absolute paths that exist", () => {
      const index = createIndex(["src/main.ts"]);
      const text = `Check the file ${workspaceRoot}/src/main.ts for details`;

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].path).toBe(`${workspaceRoot}/src/main.ts`);
      expect(results[0].absolutePath).toBe(`${workspaceRoot}/src/main.ts`);
      expect(results[0].validated).toBe(true);
    });

    it("should NOT detect absolute paths that do not exist in index", () => {
      const index = createIndex(["src/main.ts"]);
      const text = `Check the file ${workspaceRoot}/src/missing.ts`;

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(0);
    });
  });

  describe("relative paths", () => {
    it("should detect and resolve relative paths", () => {
      const index = createIndex(["src/utils/helper.ts"]);
      const text = "Look at src/utils/helper.ts";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].path).toBe("src/utils/helper.ts");
      expect(results[0].absolutePath).toBe(`${workspaceRoot}/src/utils/helper.ts`);
      expect(results[0].validated).toBe(true);
    });

    it("should detect ./relative paths", () => {
      const index = createIndex(["lib/utils.ts"]);
      const text = "Import from ./lib/utils.ts";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].absolutePath).toBe(`${workspaceRoot}/lib/utils.ts`);
    });
  });

  describe("bare filenames", () => {
    it("should detect bare filename with single match", () => {
      const index = createIndex(["src/components/Button.tsx"]);
      const text = "Update the Button.tsx component";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].path).toBe("Button.tsx");
      expect(results[0].absolutePath).toBe(`${workspaceRoot}/src/components/Button.tsx`);
    });

    it("should detect bare filename with multiple matches (no absolutePath)", () => {
      const index = createIndex(["src/components/Button.tsx", "lib/ui/Button.tsx"]);
      const text = "Update Button.tsx";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].path).toBe("Button.tsx");
      expect(results[0].absolutePath).toBeUndefined(); // ambiguous
      expect(results[0].validated).toBe(true);
    });

    it("should NOT detect bare filename that does not exist", () => {
      const index = createIndex(["src/main.ts"]);
      const text = "Check NonExistent.tsx";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(0);
    });
  });

  describe("bare word filename matching", () => {
    it("should detect filenames without extensions when they match index", () => {
      const index = createIndex(["Makefile", "Dockerfile", "README"]);
      const text = "Check the Makefile and Dockerfile";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(2);
      expect(results.map((r) => r.path)).toContain("Makefile");
      expect(results.map((r) => r.path)).toContain("Dockerfile");
    });

    it("should detect config files like .gitignore", () => {
      const index = createIndex([".gitignore", ".env"]);
      const text = "Add it to .gitignore";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].path).toBe(".gitignore");
    });
  });

  describe("line and column numbers", () => {
    it("should preserve line numbers in detection", () => {
      const index = createIndex(["src/main.ts"]);
      const text = "Error at src/main.ts:42";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].line).toBe(42);
      expect(results[0].absolutePath).toBe(`${workspaceRoot}/src/main.ts`);
    });

    it("should preserve line:column numbers", () => {
      const index = createIndex(["src/main.ts"]);
      const text = "Error at src/main.ts:42:10";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
      expect(results[0].line).toBe(42);
      expect(results[0].column).toBe(10);
    });
  });

  describe("edge cases", () => {
    it("should not match URLs", () => {
      const index = createIndex(["https/file.ts"]); // unlikely but possible
      const text = "Visit https://example.com/file.ts";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(0);
    });

    it("should not match version numbers", () => {
      const index = createIndex(["1.2.3.ts"]); // unlikely
      const text = "Upgrade to version 1.2.3";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(0);
    });

    it("should handle empty text", () => {
      const index = createIndex(["src/main.ts"]);
      const results = detectFilePathsWithIndex("", index);

      expect(results).toHaveLength(0);
    });

    it("should handle text with no paths", () => {
      const index = createIndex(["src/main.ts"]);
      const results = detectFilePathsWithIndex("Hello world!", index);

      expect(results).toHaveLength(0);
    });

    it("should not double-detect the same path", () => {
      const index = createIndex(["src/utils.ts"]);
      const text = "src/utils.ts";

      const results = detectFilePathsWithIndex(text, index);

      expect(results).toHaveLength(1);
    });
  });
});
