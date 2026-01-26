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
import { useEffect, useRef } from "react";
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
import type { PathCompletion } from "@/lib/tauri";
import { cn } from "@/lib/utils";

interface PathCompletionPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  completions: PathCompletion[];
  totalCount: number;
  selectedIndex: number;
  onSelect: (completion: PathCompletion) => void;
  children: React.ReactNode;
}

const iconClass = "h-4 w-4 shrink-0";

/** Get appropriate icon based on file extension */
function getFileIcon(name: string) {
  const lowerName = name.toLowerCase();
  const ext = lowerName.split(".").pop() ?? "";

  // Special filenames
  if (lowerName === "dockerfile" || lowerName.startsWith("dockerfile.")) {
    return <SiDocker className={cn(iconClass, "text-[#2496ED]")} />;
  }
  if (lowerName === "makefile" || lowerName === "justfile") {
    return <Cog className={cn(iconClass, "text-orange-500")} />;
  }
  if (lowerName.includes("lock") || lowerName.endsWith("-lock.yaml")) {
    return <Lock className={cn(iconClass, "text-yellow-600")} />;
  }
  if (lowerName.startsWith(".git") || lowerName === ".gitignore") {
    return <SiGit className={cn(iconClass, "text-[#F05032]")} />;
  }
  if (lowerName.startsWith(".env")) {
    return <Cog className={cn(iconClass, "text-yellow-500")} />;
  }
  if (lowerName === "cargo.toml" || lowerName === "cargo.lock") {
    return <SiRust className={cn(iconClass, "text-[#CE422B]")} />;
  }
  if (lowerName === "package.json" || lowerName === "package-lock.json") {
    return <FileJson className={cn(iconClass, "text-[#CB3837]")} />;
  }
  if (lowerName === "tsconfig.json" || lowerName.startsWith("tsconfig.")) {
    return <SiTypescript className={cn(iconClass, "text-[#3178C6]")} />;
  }

  // By extension
  switch (ext) {
    // JavaScript/TypeScript
    case "js":
    case "mjs":
    case "cjs":
      return <SiJavascript className={cn(iconClass, "text-[#F7DF1E]")} />;
    case "jsx":
      return <SiReact className={cn(iconClass, "text-[#61DAFB]")} />;
    case "ts":
    case "mts":
    case "cts":
      return <SiTypescript className={cn(iconClass, "text-[#3178C6]")} />;
    case "tsx":
      return <SiReact className={cn(iconClass, "text-[#61DAFB]")} />;

    // Web
    case "html":
    case "htm":
      return <SiHtml5 className={cn(iconClass, "text-[#E34F26]")} />;
    case "css":
      return <SiCss3 className={cn(iconClass, "text-[#1572B6]")} />;
    case "scss":
    case "sass":
      return <SiSass className={cn(iconClass, "text-[#CC6699]")} />;
    case "less":
      return <SiCss3 className={cn(iconClass, "text-[#1D365D]")} />;

    // Data formats
    case "json":
    case "jsonc":
      return <FileJson className={cn(iconClass, "text-yellow-500")} />;
    case "yaml":
    case "yml":
      return <SiYaml className={cn(iconClass, "text-[#CB171E]")} />;
    case "toml":
      return <SiToml className={cn(iconClass, "text-[#9C4121]")} />;
    case "xml":
      return <FileCode className={cn(iconClass, "text-orange-400")} />;
    case "csv":
      return <Database className={cn(iconClass, "text-green-500")} />;
    case "sql":
      return <Database className={cn(iconClass, "text-blue-400")} />;

    // Documentation
    case "md":
    case "mdx":
      return <SiMarkdown className={cn(iconClass, "text-foreground")} />;
    case "txt":
      return <FileText className={cn(iconClass, "text-muted-foreground")} />;
    case "pdf":
      return <FileType className={cn(iconClass, "text-red-500")} />;

    // Images
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
    case "ico":
    case "bmp":
      return <FileImage className={cn(iconClass, "text-purple-500")} />;
    case "svg":
      return <SiSvg className={cn(iconClass, "text-[#FFB13B]")} />;

    // Programming languages
    case "py":
    case "pyw":
      return <SiPython className={cn(iconClass, "text-[#3776AB]")} />;
    case "rs":
      return <SiRust className={cn(iconClass, "text-[#CE422B]")} />;
    case "go":
      return <SiGo className={cn(iconClass, "text-[#00ADD8]")} />;
    case "rb":
      return <SiRuby className={cn(iconClass, "text-[#CC342D]")} />;
    case "java":
      return <FaJava className={cn(iconClass, "text-[#ED8B00]")} />;
    case "c":
    case "h":
      return <SiC className={cn(iconClass, "text-[#A8B9CC]")} />;
    case "cpp":
    case "cc":
    case "cxx":
    case "hpp":
      return <SiCplusplus className={cn(iconClass, "text-[#00599C]")} />;
    case "php":
      return <SiPhp className={cn(iconClass, "text-[#777BB4]")} />;
    case "swift":
      return <SiSwift className={cn(iconClass, "text-[#F05138]")} />;
    case "kt":
    case "kts":
      return <SiKotlin className={cn(iconClass, "text-[#7F52FF]")} />;
    case "lua":
      return <SiLua className={cn(iconClass, "text-[#2C2D72]")} />;
    case "zig":
      return <SiZig className={cn(iconClass, "text-[#F7A41D]")} />;

    // Shell/Scripts
    case "sh":
    case "bash":
    case "zsh":
    case "fish":
      return <SiGnubash className={cn(iconClass, "text-[#4EAA25]")} />;
    case "ps1":
    case "bat":
    case "cmd":
      return <Terminal className={cn(iconClass, "text-[#5391FE]")} />;

    // Config
    case "ini":
    case "conf":
    case "cfg":
      return <Cog className={cn(iconClass, "text-gray-500")} />;

    // Archives
    case "zip":
    case "tar":
    case "gz":
    case "rar":
    case "7z":
      return <Archive className={cn(iconClass, "text-yellow-600")} />;

    default:
      return <File className={cn(iconClass, "text-muted-foreground")} />;
  }
}

function getIcon(entryType: PathCompletion["entry_type"], name: string) {
  switch (entryType) {
    case "directory":
      return <Folder className={cn(iconClass, "text-blue-500")} />;
    case "symlink":
      return <Link2 className={cn(iconClass, "text-cyan-500")} />;
    default:
      return getFileIcon(name);
  }
}

/** Renders a name with matched characters highlighted */
function HighlightedName({ name, indices }: { name: string; indices: number[] }) {
  if (indices.length === 0) {
    return <span>{name}</span>;
  }

  const indexSet = new Set(indices);
  const chars = [...name];

  return (
    <span>
      {chars.map((char, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: Characters are static, never reordered
        <span key={i} className={indexSet.has(i) ? "text-primary font-semibold" : ""}>
          {char}
        </span>
      ))}
    </span>
  );
}

export function PathCompletionPopup({
  open,
  onOpenChange,
  completions,
  totalCount,
  selectedIndex,
  onSelect,
  children,
}: PathCompletionPopupProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Close popup when clicking outside
  useEffect(() => {
    if (!open) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onOpenChange(false);
      }
    };

    // Use capture phase to catch clicks before they're handled
    document.addEventListener("mousedown", handleClickOutside, true);
    return () => document.removeEventListener("mousedown", handleClickOutside, true);
  }, [open, onOpenChange]);

  // Close popup when window loses focus (e.g., switching tabs)
  useEffect(() => {
    if (!open) return;

    const handleBlur = () => onOpenChange(false);
    window.addEventListener("blur", handleBlur);
    return () => window.removeEventListener("blur", handleBlur);
  }, [open, onOpenChange]);

  // Scroll selected item into view
  useEffect(() => {
    if (open && listRef.current) {
      const selectedElement = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
      selectedElement?.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex, open]);

  return (
    <div ref={containerRef} className="relative flex-1 flex min-w-0">
      {children}
      {open && (
        <div
          data-testid="path-completion-popup"
          className="absolute bottom-full left-0 mb-2 min-w-[300px] max-w-[500px] z-50 bg-popover border border-border rounded-md shadow-md overflow-hidden"
        >
          {/* Result count badge - shown when there are more matches than displayed */}
          {totalCount > completions.length && (
            <div className="px-3 py-1 text-xs text-muted-foreground border-b border-border bg-muted/30">
              Showing {completions.length} of {totalCount} matches
            </div>
          )}

          {completions.length === 0 ? (
            <div className="py-3 text-center text-[13px] text-muted-foreground">
              No completions found
            </div>
          ) : (
            <div
              ref={listRef}
              className="max-h-[530px] overflow-y-scroll py-1"
              style={{ scrollbarGutter: "stable" }}
              role="listbox"
            >
              {completions.map((completion, index) => (
                <div
                  key={completion.insert_text}
                  role="option"
                  aria-selected={index === selectedIndex}
                  tabIndex={0}
                  data-index={index}
                  onClick={() => onSelect(completion)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelect(completion);
                    }
                  }}
                  className={cn(
                    "flex items-center gap-2 px-3 py-1.5",
                    "cursor-pointer transition-colors",
                    index === selectedIndex ? "bg-primary/10" : "hover:bg-card"
                  )}
                >
                  {getIcon(completion.entry_type, completion.name)}
                  <span className="font-mono text-[13px] text-foreground truncate">
                    <HighlightedName name={completion.name} indices={completion.match_indices} />
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
