/**
 * Shared file/folder icon utilities.
 * Used by PathCompletionPopup, FileBrowser, and other components that display file entries.
 */

import {
  Archive,
  Cog,
  Database,
  File,
  FileCode,
  FileImage,
  FileJson,
  FileText,
  FileType,
  Folder,
  Link2,
  Lock,
  Terminal,
} from "lucide-react";
import { FaJava } from "react-icons/fa";
import {
  SiC,
  SiCplusplus,
  SiCss3,
  SiDocker,
  SiGit,
  SiGnubash,
  SiGo,
  SiHtml5,
  SiJavascript,
  SiKotlin,
  SiLua,
  SiMarkdown,
  SiPhp,
  SiPython,
  SiReact,
  SiRuby,
  SiRust,
  SiSass,
  SiSvg,
  SiSwift,
  SiToml,
  SiTypescript,
  SiYaml,
  SiZig,
} from "react-icons/si";
import { cn } from "@/lib/utils";

const iconClass = "h-4 w-4 shrink-0";

/** Get appropriate icon based on file extension */
export function getFileIcon(name: string, className?: string) {
  const cls = className ?? iconClass;
  const lowerName = name.toLowerCase();
  const ext = lowerName.split(".").pop() ?? "";

  // Special filenames
  if (lowerName === "dockerfile" || lowerName.startsWith("dockerfile.")) {
    return <SiDocker className={cn(cls, "text-[#2496ED]")} />;
  }
  if (lowerName === "makefile" || lowerName === "justfile") {
    return <Cog className={cn(cls, "text-orange-500")} />;
  }
  if (lowerName.includes("lock") || lowerName.endsWith("-lock.yaml")) {
    return <Lock className={cn(cls, "text-yellow-600")} />;
  }
  if (lowerName.startsWith(".git") || lowerName === ".gitignore") {
    return <SiGit className={cn(cls, "text-[#F05032]")} />;
  }
  if (lowerName.startsWith(".env")) {
    return <Cog className={cn(cls, "text-yellow-500")} />;
  }
  if (lowerName === "cargo.toml" || lowerName === "cargo.lock") {
    return <SiRust className={cn(cls, "text-[#CE422B]")} />;
  }
  if (lowerName === "package.json" || lowerName === "package-lock.json") {
    return <FileJson className={cn(cls, "text-[#CB3837]")} />;
  }
  if (lowerName === "tsconfig.json" || lowerName.startsWith("tsconfig.")) {
    return <SiTypescript className={cn(cls, "text-[#3178C6]")} />;
  }

  // By extension
  switch (ext) {
    // JavaScript/TypeScript
    case "js":
    case "mjs":
    case "cjs":
      return <SiJavascript className={cn(cls, "text-[#F7DF1E]")} />;
    case "jsx":
      return <SiReact className={cn(cls, "text-[#61DAFB]")} />;
    case "ts":
    case "mts":
    case "cts":
      return <SiTypescript className={cn(cls, "text-[#3178C6]")} />;
    case "tsx":
      return <SiReact className={cn(cls, "text-[#61DAFB]")} />;

    // Web
    case "html":
    case "htm":
      return <SiHtml5 className={cn(cls, "text-[#E34F26]")} />;
    case "css":
      return <SiCss3 className={cn(cls, "text-[#1572B6]")} />;
    case "scss":
    case "sass":
      return <SiSass className={cn(cls, "text-[#CC6699]")} />;
    case "less":
      return <SiCss3 className={cn(cls, "text-[#1D365D]")} />;

    // Data formats
    case "json":
    case "jsonc":
      return <FileJson className={cn(cls, "text-yellow-500")} />;
    case "yaml":
    case "yml":
      return <SiYaml className={cn(cls, "text-[#CB171E]")} />;
    case "toml":
      return <SiToml className={cn(cls, "text-[#9C4121]")} />;
    case "xml":
      return <FileCode className={cn(cls, "text-orange-400")} />;
    case "csv":
      return <Database className={cn(cls, "text-green-500")} />;
    case "sql":
      return <Database className={cn(cls, "text-blue-400")} />;

    // Documentation
    case "md":
    case "mdx":
      return <SiMarkdown className={cn(cls, "text-foreground")} />;
    case "txt":
      return <FileText className={cn(cls, "text-muted-foreground")} />;
    case "pdf":
      return <FileType className={cn(cls, "text-red-500")} />;

    // Images
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
    case "ico":
    case "bmp":
      return <FileImage className={cn(cls, "text-purple-500")} />;
    case "svg":
      return <SiSvg className={cn(cls, "text-[#FFB13B]")} />;

    // Programming languages
    case "py":
    case "pyw":
      return <SiPython className={cn(cls, "text-[#3776AB]")} />;
    case "rs":
      return <SiRust className={cn(cls, "text-[#CE422B]")} />;
    case "go":
      return <SiGo className={cn(cls, "text-[#00ADD8]")} />;
    case "rb":
      return <SiRuby className={cn(cls, "text-[#CC342D]")} />;
    case "java":
      return <FaJava className={cn(cls, "text-[#ED8B00]")} />;
    case "c":
    case "h":
      return <SiC className={cn(cls, "text-[#A8B9CC]")} />;
    case "cpp":
    case "cc":
    case "cxx":
    case "hpp":
      return <SiCplusplus className={cn(cls, "text-[#00599C]")} />;
    case "php":
      return <SiPhp className={cn(cls, "text-[#777BB4]")} />;
    case "swift":
      return <SiSwift className={cn(cls, "text-[#F05138]")} />;
    case "kt":
    case "kts":
      return <SiKotlin className={cn(cls, "text-[#7F52FF]")} />;
    case "lua":
      return <SiLua className={cn(cls, "text-[#2C2D72]")} />;
    case "zig":
      return <SiZig className={cn(cls, "text-[#F7A41D]")} />;

    // Shell/Scripts
    case "sh":
    case "bash":
    case "zsh":
    case "fish":
      return <SiGnubash className={cn(cls, "text-[#4EAA25]")} />;
    case "ps1":
    case "bat":
    case "cmd":
      return <Terminal className={cn(cls, "text-[#5391FE]")} />;

    // Config
    case "ini":
    case "conf":
    case "cfg":
      return <Cog className={cn(cls, "text-gray-500")} />;

    // Archives
    case "zip":
    case "tar":
    case "gz":
    case "rar":
    case "7z":
      return <Archive className={cn(cls, "text-yellow-600")} />;

    default:
      return <File className={cn(cls, "text-muted-foreground")} />;
  }
}

export type EntryType = "directory" | "file" | "symlink";

/** Get icon for a file system entry based on type and name */
export function getEntryIcon(entryType: EntryType, name: string, className?: string) {
  const cls = className ?? iconClass;
  switch (entryType) {
    case "directory":
      return <Folder className={cn(cls, "text-blue-500")} />;
    case "symlink":
      return <Link2 className={cn(cls, "text-cyan-500")} />;
    default:
      return getFileIcon(name, cls);
  }
}
