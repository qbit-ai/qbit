import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import type { Extension } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { Vim, vim } from "@replit/codemirror-vim";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { FolderOpen, Save, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useFileEditorSidebar } from "@/hooks/useFileEditorSidebar";
import { qbitTheme } from "@/lib/codemirror-theme";
import { cn } from "@/lib/utils";
import { useFileEditorSidebarStore } from "@/store/file-editor-sidebar";

// Custom vim command callbacks (set by component)
let vimSaveCallback: (() => void) | null = null;
let vimCloseCallback: (() => void) | null = null;
let vimForceCloseCallback: (() => void) | null = null;
let vimReloadCallback: (() => void) | null = null;

export function setVimCallbacks(callbacks: {
  save: (() => void) | null;
  close: (() => void) | null;
  forceClose: (() => void) | null;
  reload: (() => void) | null;
}) {
  vimSaveCallback = callbacks.save;
  vimCloseCallback = callbacks.close;
  vimForceCloseCallback = callbacks.forceClose;
  vimReloadCallback = callbacks.reload;
}

// Register custom vim ex commands (only runs once at module load)
let vimCommandsRegistered = false;
function registerVimCommands() {
  if (vimCommandsRegistered) return;
  vimCommandsRegistered = true;

  // biome-ignore lint/suspicious/noExplicitAny: Vim.defineEx not fully typed
  const defineEx = (Vim as any).defineEx;
  if (!defineEx) return;

  // :set <option> / :set no<option>
  defineEx("set", "", (_cm: unknown, params: { args?: string[] }) => {
    const args = params.args || [];
    const arg = args[0]?.toLowerCase();

    // Get current session ID from the store (find the one with vimMode enabled)
    const state = useFileEditorSidebarStore.getState();
    const sessionId = Object.keys(state.sessions).find((id) => state.sessions[id]?.vimMode);
    if (!sessionId) return;

    switch (arg) {
      case "wrap":
        state.setWrap(sessionId, true);
        break;
      case "nowrap":
        state.setWrap(sessionId, false);
        break;
      case "number":
      case "nu":
        state.setLineNumbers(sessionId, true);
        break;
      case "nonumber":
      case "nonu":
        state.setLineNumbers(sessionId, false);
        break;
      case "relativenumber":
      case "rnu":
        state.setRelativeLineNumbers(sessionId, true);
        break;
      case "norelativenumber":
      case "nornu":
        state.setRelativeLineNumbers(sessionId, false);
        break;
    }
  });

  // :w / :write - save
  defineEx("write", "w", () => {
    vimSaveCallback?.();
  });

  // :q / :quit - close (respects dirty state via callback)
  defineEx("quit", "q", () => {
    vimCloseCallback?.();
  });

  // :q! - force close (ignores dirty state)
  defineEx("q!", "q!", () => {
    vimForceCloseCallback?.();
  });

  // :wq - save and close
  defineEx("wq", "wq", () => {
    vimSaveCallback?.();
    // Small delay to let save complete before closing
    setTimeout(() => vimCloseCallback?.(), 100);
  });

  // :e! - reload file (discard changes)
  defineEx("e!", "e!", () => {
    vimReloadCallback?.();
  });
}

interface FileEditorSidebarPanelProps {
  sessionId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workingDirectory?: string | null;
}

const MIN_WIDTH = 320;
const MAX_WIDTH = 1200;
const DEFAULT_WIDTH = 420;

function languageExtension(language?: string): Extension | null {
  switch (language) {
    case "typescript":
      return javascript({ jsx: true, typescript: true });
    case "javascript":
      return javascript({ jsx: true, typescript: false });
    case "json":
      return json();
    case "markdown":
      return markdown();
    case "python":
      return python();
    case "rust":
      return rust();
    default:
      return null;
  }
}

function FileOpenPrompt({
  workingDirectory,
  onOpen,
  recentFiles,
}: {
  workingDirectory?: string | null;
  onOpen: (path: string) => void;
  recentFiles: string[];
}) {
  const handleBrowse = async () => {
    const selected = await openFileDialog({
      directory: false,
      multiple: false,
      defaultPath: workingDirectory ?? undefined,
    });
    if (selected) {
      onOpen(selected);
    }
  };

  return (
    <div className="h-full flex flex-col items-center justify-center gap-6 px-6 text-center">
      <div className="space-y-2 max-w-xl">
        <p className="text-sm text-muted-foreground">Open a file to start editing</p>
        {workingDirectory && (
          <p className="text-xs text-muted-foreground/70 font-mono truncate">
            Base: {workingDirectory}
          </p>
        )}
        <p className="text-xs text-muted-foreground/80">
          Browse for a file or pick from your recent list.
        </p>
      </div>
      <div className="w-full max-w-xl flex flex-col items-stretch gap-3">
        <Button onClick={handleBrowse} variant="default" className="w-full justify-center gap-2">
          <FolderOpen className="h-4 w-4" />
          Browse files
        </Button>
        {recentFiles.length > 0 && (
          <div className="w-full text-left">
            <p className="text-xs text-muted-foreground mb-2">Recent</p>
            <div className="grid gap-2">
              {recentFiles.slice(0, 5).map((file) => (
                <button
                  key={file}
                  type="button"
                  onClick={() => onOpen(file)}
                  className="text-left text-sm font-mono px-3 py-2 rounded-md border border-border hover:border-primary/60 hover:bg-muted transition"
                >
                  {file}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export function FileEditorSidebarPanel({
  sessionId,
  open,
  onOpenChange,
  workingDirectory,
}: FileEditorSidebarPanelProps) {
  const {
    session,
    openFile,
    saveActiveFile,
    reloadActiveFile,
    setOpen,
    setWidth,
    updateContent,
    setVimMode,
    setVimModeState,
    closeFile,
  } = useFileEditorSidebar(sessionId, workingDirectory || undefined);

  const [containerWidth, setContainerWidth] = useState(DEFAULT_WIDTH);
  const isResizing = useRef(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<ReactCodeMirrorRef>(null);

  useEffect(() => {
    if (session?.width) {
      setContainerWidth(session.width);
    }
  }, [session?.width]);

  useEffect(() => {
    if (sessionId) {
      setOpen(open);
    }
  }, [open, setOpen, sessionId]);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current) return;
      const newWidth = window.innerWidth - e.clientX;
      if (newWidth >= MIN_WIDTH && newWidth <= MAX_WIDTH) {
        setContainerWidth(newWidth);
        if (sessionId) setWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      if (isResizing.current) {
        isResizing.current = false;
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      }
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [setWidth, sessionId]);

  const onStartResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isResizing.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  // Register custom vim commands when vim mode is first enabled
  useEffect(() => {
    if (session?.vimMode) {
      registerVimCommands();
      setVimCallbacks({
        save: () => void saveActiveFile(),
        close: () => onOpenChange(false),
        forceClose: () => {
          closeFile();
          onOpenChange(false);
        },
        reload: () => void reloadActiveFile(),
      });
    } else {
      setVimCallbacks({ save: null, close: null, forceClose: null, reload: null });
    }
    return () => {
      setVimCallbacks({ save: null, close: null, forceClose: null, reload: null });
    };
  }, [session?.vimMode, saveActiveFile, reloadActiveFile, closeFile, onOpenChange]);

  useEffect(() => {
    if (!session?.vimMode || !editorRef.current?.view) return;

    // biome-ignore lint/suspicious/noExplicitAny: CodeMirror vim internals not fully typed
    const cm = (editorRef.current.view as any).cm;
    if (!cm) return;

    const handler = (event: { mode: string }) => {
      const mode = event.mode.toLowerCase();
      if (mode === "normal" || mode === "insert" || mode === "visual") {
        setVimModeState(mode);
      }
    };

    // CM5-style event listener on the cm object (not Vim.on)
    cm.on("vim-mode-change", handler);
    return () => {
      cm.off("vim-mode-change", handler);
    };
  }, [session?.vimMode, setVimModeState]);

  const extensions = useMemo(() => {
    const ext: Extension[] = [];

    const lang = languageExtension(session?.activeFile?.language);
    if (lang) ext.push(lang);
    if (session?.wrap) {
      ext.push(EditorView.lineWrapping);
    }
    if (session?.vimMode) {
      ext.push(vim());
    }
    // Keymap for save shortcut inside the editor
    ext.push(
      keymap.of([
        {
          key: "Mod-s",
          preventDefault: true,
          run: () => {
            void saveActiveFile();
            return true;
          },
        },
      ])
    );

    return ext;
  }, [saveActiveFile, session?.activeFile?.language, session?.vimMode, session?.wrap]);

  // Memoize basicSetup to react to line number settings changes
  const basicSetup = useMemo(
    () => ({
      lineNumbers: session?.lineNumbers ?? true,
      foldGutter: true,
      highlightActiveLine: true,
    }),
    [session?.lineNumbers]
  );

  if (!open || !sessionId) return null;

  const activeFile = session?.activeFile;

  return (
    <div
      ref={panelRef}
      className="bg-card border-l border-border flex flex-col relative"
      style={{
        width: `${containerWidth}px`,
        minWidth: `${MIN_WIDTH}px`,
        maxWidth: `${MAX_WIDTH}px`,
      }}
    >
      {/* Resize handle */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: resize handle is mouse-only */}
      <div
        className="absolute top-0 left-0 w-1 h-full cursor-col-resize hover:bg-primary/50 transition-colors z-10 group"
        onMouseDown={onStartResize}
      />

      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">File Editor</span>
          {activeFile && (
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={() => void saveActiveFile()}
              title="Save file (Ctrl+S)"
            >
              <Save className="w-3.5 h-3.5" />
            </Button>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => onOpenChange(false)}
          title="Close file editor"
        >
          <X className="w-4 h-4" />
        </Button>
      </div>

      {/* Body */}
      <div className="flex-1 min-h-0 flex flex-col">
        {activeFile ? (
          <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
            <CodeMirror
              ref={editorRef}
              value={activeFile.content}
              height="100%"
              theme={qbitTheme}
              extensions={extensions}
              basicSetup={basicSetup}
              onChange={(value) => updateContent(value)}
              className="h-full [&_.cm-editor]:h-full [&_.cm-scroller]:overflow-auto"
            />
          </div>
        ) : (
          <FileOpenPrompt
            workingDirectory={workingDirectory ?? undefined}
            onOpen={(path) => openFile(path)}
            recentFiles={session?.recentFiles ?? []}
          />
        )}
      </div>

      {/* Footer */}
      <div className="px-3 py-2 border-t border-border text-xs text-muted-foreground flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          {session?.vimMode && (
            <Badge variant="outline" className="text-[11px] font-mono uppercase">
              {session?.vimModeState ?? "normal"}
            </Badge>
          )}
          {activeFile?.path && (
            <span className="font-mono text-[11px] truncate">{activeFile.path}</span>
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <button
            type="button"
            onClick={() => setVimMode(!session?.vimMode)}
            className={cn(
              "text-[11px] px-1.5 py-0.5 rounded transition-colors",
              session?.vimMode
                ? "bg-primary/20 text-primary hover:bg-primary/30"
                : "text-muted-foreground hover:text-foreground hover:bg-muted"
            )}
            title={session?.vimMode ? "Disable Vim mode" : "Enable Vim mode"}
          >
            Vim
          </button>
          {activeFile && (
            <Badge
              variant={activeFile.dirty ? "destructive" : "secondary"}
              className="text-[11px] uppercase"
            >
              {activeFile.dirty ? "Dirty" : "Clean"}
            </Badge>
          )}
        </div>
      </div>
    </div>
  );
}
