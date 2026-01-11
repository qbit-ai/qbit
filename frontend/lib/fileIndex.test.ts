import { describe, expect, it } from "vitest";
import { buildFileIndex } from "./fileIndex";

describe("buildFileIndex", () => {
  it("should create empty index from empty array", () => {
    const index = buildFileIndex([], "/workspace");

    expect(index.absolutePaths.size).toBe(0);
    expect(index.byFilename.size).toBe(0);
    expect(index.byRelativePath.size).toBe(0);
  });

  it("should index absolute paths", () => {
    const files = ["/workspace/src/main.ts", "/workspace/src/utils.ts"];
    const index = buildFileIndex(files, "/workspace");

    expect(index.absolutePaths.has("/workspace/src/main.ts")).toBe(true);
    expect(index.absolutePaths.has("/workspace/src/utils.ts")).toBe(true);
  });

  it("should build filename lookup map", () => {
    const files = [
      "/workspace/src/main.ts",
      "/workspace/lib/main.ts", // same filename, different path
      "/workspace/src/utils.ts",
    ];
    const index = buildFileIndex(files, "/workspace");

    expect(index.byFilename.get("main.ts")).toEqual([
      "/workspace/src/main.ts",
      "/workspace/lib/main.ts",
    ]);
    expect(index.byFilename.get("utils.ts")).toEqual(["/workspace/src/utils.ts"]);
  });

  it("should build relative path lookup map", () => {
    const files = ["/workspace/src/lib/utils.ts"];
    const index = buildFileIndex(files, "/workspace");

    expect(index.byRelativePath.get("src/lib/utils.ts")).toBe("/workspace/src/lib/utils.ts");
  });

  it("should handle files outside workspace root", () => {
    const files = ["/other/path/file.ts"];
    const index = buildFileIndex(files, "/workspace");

    expect(index.absolutePaths.has("/other/path/file.ts")).toBe(true);
    // Relative path should be the full absolute path
    expect(index.byRelativePath.has("/other/path/file.ts")).toBe(true);
  });

  it("should handle files at workspace root", () => {
    const files = ["/workspace/README.md"];
    const index = buildFileIndex(files, "/workspace");

    expect(index.byRelativePath.get("README.md")).toBe("/workspace/README.md");
    expect(index.byFilename.get("README.md")).toEqual(["/workspace/README.md"]);
  });

  it("should handle hidden files (dotfiles)", () => {
    const files = ["/workspace/.gitignore", "/workspace/.env"];
    const index = buildFileIndex(files, "/workspace");

    expect(index.byFilename.get(".gitignore")).toEqual(["/workspace/.gitignore"]);
    expect(index.byFilename.get(".env")).toEqual(["/workspace/.env"]);
    expect(index.byRelativePath.get(".gitignore")).toBe("/workspace/.gitignore");
  });

  it("should handle extensionless files", () => {
    const files = ["/workspace/Makefile", "/workspace/Dockerfile"];
    const index = buildFileIndex(files, "/workspace");

    expect(index.byFilename.get("Makefile")).toEqual(["/workspace/Makefile"]);
    expect(index.byFilename.get("Dockerfile")).toEqual(["/workspace/Dockerfile"]);
  });
});
