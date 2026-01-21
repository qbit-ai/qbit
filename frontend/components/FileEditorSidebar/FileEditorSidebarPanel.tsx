import { cpp } from "@codemirror/lang-cpp";
import { css } from "@codemirror/lang-css";
import { go } from "@codemirror/lang-go";
import { html } from "@codemirror/lang-html";
import { java } from "@codemirror/lang-java";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { sql } from "@codemirror/lang-sql";
import { xml } from "@codemirror/lang-xml";
import { yaml } from "@codemirror/lang-yaml";
import type { Extension } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { Vim, vim } from "@replit/codemirror-vim";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { Eye, FileText, FolderOpen, Plus, Save, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Markdown } from "@/components/Markdown/Markdown";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useFileEditorSidebar } from "@/hooks/useFileEditorSidebar";
import { qbitTheme } from "@/lib/codemirror-theme";
import { cn } from "@/lib/utils";
import { useFileEditorSidebarStore } from "@/store/file-editor-sidebar";
import { FileBrowser } from "./FileBrowser";
import { TabBar } from "./TabBar";

// Custom vim command callbacks (set by component)
let vimSaveCallback: (() => void) | null = null;
let vimCloseCallback: (() => void) | null = null;
let vimForceCloseCallback: (() => void) | null = null;
let vimCloseAllCallback: (() => void) | null = null;
let vimReloadCallback: (() => void) | null = null;
let vimNextTabCallback: (() => void) | null = null;
let vimPrevTabCallback: (() => void) | null = null;

export function setVimCallbacks(callbacks: {
  save: (() => void) | null;
  close: (() => void) | null;
  forceClose: (() => void) | null;
  closeAll: (() => void) | null;
  reload: (() => void) | null;
  nextTab: (() => void) | null;
  prevTab: (() => void) | null;
}) {
  vimSaveCallback = callbacks.save;
  vimCloseCallback = callbacks.close;
  vimForceCloseCallback = callbacks.forceClose;
  vimCloseAllCallback = callbacks.closeAll;
  vimReloadCallback = callbacks.reload;
  vimNextTabCallback = callbacks.nextTab;
  vimPrevTabCallback = callbacks.prevTab;
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

  // :q / :quit - close current tab
  defineEx("quit", "q", () => {
    vimCloseCallback?.();
  });

  // :q! - force close current tab (ignores dirty state)
  defineEx("q!", "q!", () => {
    vimForceCloseCallback?.();
  });

  // :qa / :qall - close all tabs and panel
  defineEx("qall", "qa", () => {
    vimCloseAllCallback?.();
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

  // :bn / :bnext - next tab
  defineEx("bnext", "bn", () => {
    vimNextTabCallback?.();
  });

  // :bp / :bprev - previous tab
  defineEx("bprev", "bp", () => {
    vimPrevTabCallback?.();
  });

  // :tabn - next tab (alias)
  defineEx("tabnext", "tabn", () => {
    vimNextTabCallback?.();
  });

  // :tabp - previous tab (alias)
  defineEx("tabprev", "tabp", () => {
    vimPrevTabCallback?.();
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

function MarkdownPreview({ content }: { content: string }) {
  return (
    <div className="p-4 overflow-auto h-full">
      <Markdown content={content} />
    </div>
  );
}

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

    case "go":
      return go();
    case "yaml":
      return yaml();
    case "html":
      return html();
    case "css":
      return css();
    case "sql":
      return sql();
    case "xml":
      return xml();
    case "java":
      return java();
    case "cpp":
      return cpp();

    // TOML: no official @codemirror/lang-toml package; fall back to no highlighting.
    case "toml":
      return null;

    default:
      return null;
  }
}

function FileOpenPrompt({
  workingDirectory,
  onOpen,
  onOpenBrowser,
  recentFiles,
}: {
  workingDirectory?: string | null;
  onOpen: (path: string) => void;
  onOpenBrowser: () => void;
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
        <p className="text-xs text-muted-foreground">Open a file to start editing</p>
        <p className="text-xs text-muted-foreground/80">
          Browse for a file or use the file browser.
        </p>
      </div>
      <div className="w-full max-w-xl flex flex-col items-stretch gap-3">
        <div className="flex gap-2">
          <Button onClick={handleBrowse} variant="default" className="flex-1 justify-center gap-2">
            <Plus className="h-4 w-4" />
            Open File
          </Button>
          <Button onClick={onOpenBrowser} variant="outline" className="flex-1 justify-center gap-2">
            <FolderOpen className="h-4 w-4" />
            Browse Files
          </Button>
        </div>
        {recentFiles.length > 0 && (
          <div className="w-full text-left mt-4">
            <p className="text-xs text-muted-foreground mb-2">Recent files:</p>
            <div className="grid gap-2">
              {recentFiles.slice(0, 5).map((file) => {
                const fileName = file.split("/").pop() || file;
                const parentDir = file.split("/").slice(-2, -1)[0];
                const displayPath = parentDir ? `${parentDir}/${fileName}` : fileName;
                return (
                  <button
                    key={file}
                    type="button"
                    onClick={() => onOpen(file)}
                    className="text-left text-xs font-mono px-3 py-2 rounded-md border border-border hover:border-primary/60 hover:bg-muted transition truncate"
                    title={file}
                  >
                    {displayPath}
                  </button>
                );
              })}
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
    activeTab,
    activeFile,
    tabs,
    openFile,
    openBrowser,
    saveFile,
    reloadFile,
    setActiveTab,
    closeTab,
    closeAllTabs,
    closeOtherTabs,
    setOpen,
    setWidth,
    updateFileContent,
    setBrowserPath,
    setVimMode,
    setVimModeState,
    toggleMarkdownPreview,
    reorderTabs,
  } = useFileEditorSidebar(sessionId, workingDirectory || undefined);

  const [containerWidth, setContainerWidth] = useState(DEFAULT_WIDTH);
  const isResizing = useRef(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<ReactCodeMirrorRef>(null);

  // Navigate to next/previous tab
  const goToNextTab = useCallback(() => {
    if (!session || tabs.length <= 1) return;
    const currentIndex = session.activeTabId ? session.tabOrder.indexOf(session.activeTabId) : -1;
    const nextIndex = (currentIndex + 1) % tabs.length;
    const nextId = session.tabOrder[nextIndex];
    if (nextId) setActiveTab(nextId);
  }, [session, tabs.length, setActiveTab]);

  const goToPrevTab = useCallback(() => {
    if (!session || tabs.length <= 1) return;
    const currentIndex = session.activeTabId ? session.tabOrder.indexOf(session.activeTabId) : -1;
    const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    const prevId = session.tabOrder[prevIndex];
    if (prevId) setActiveTab(prevId);
  }, [session, tabs.length, setActiveTab]);

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
        save: () => void saveFile(),
        close: () => {
          // Close current tab; if no tabs left, close panel
          closeTab();
          // Check if we still have tabs after closing
          const state = useFileEditorSidebarStore.getState();
          const currentSession = sessionId ? state.sessions[sessionId] : null;
          if (!currentSession || currentSession.tabOrder.length === 0) {
            onOpenChange(false);
          }
        },
        forceClose: () => {
          closeTab();
          const state = useFileEditorSidebarStore.getState();
          const currentSession = sessionId ? state.sessions[sessionId] : null;
          if (!currentSession || currentSession.tabOrder.length === 0) {
            onOpenChange(false);
          }
        },
        closeAll: () => {
          closeAllTabs();
          onOpenChange(false);
        },
        reload: () => void reloadFile(),
        nextTab: goToNextTab,
        prevTab: goToPrevTab,
      });
    } else {
      setVimCallbacks({
        save: null,
        close: null,
        forceClose: null,
        closeAll: null,
        reload: null,
        nextTab: null,
        prevTab: null,
      });
    }
    return () => {
      setVimCallbacks({
        save: null,
        close: null,
        forceClose: null,
        closeAll: null,
        reload: null,
        nextTab: null,
        prevTab: null,
      });
    };
  }, [
    session?.vimMode,
    saveFile,
    reloadFile,
    closeTab,
    closeAllTabs,
    onOpenChange,
    sessionId,
    goToNextTab,
    goToPrevTab,
  ]);

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

    const lang = languageExtension(activeFile?.language);
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
            void saveFile();
            return true;
          },
        },
      ])
    );

    return ext;
  }, [saveFile, activeFile?.language, session?.vimMode, session?.wrap]);

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

  const hasTabs = tabs.length > 0;

  // Render the active tab content
  const renderTabContent = () => {
    if (!activeTab) {
      return (
        <FileOpenPrompt
          workingDirectory={workingDirectory ?? undefined}
          onOpen={(path) => openFile(path)}
          onOpenBrowser={() => openBrowser()}
          recentFiles={session?.recentFiles ?? []}
        />
      );
    }

    if (activeTab.type === "browser") {
      return (
        <FileBrowser
          currentPath={activeTab.browser.currentPath}
          workingDirectory={workingDirectory ?? undefined}
          onNavigate={(path) => {
            if (activeTab) {
              setBrowserPath(activeTab.id, path);
            }
          }}
          onOpenFile={(path) => openFile(path)}
        />
      );
    }

    if (activeTab.type === "file") {
      if (activeTab.file.language === "markdown" && activeTab.file.markdownPreview) {
        return (
          <div className="flex-1 min-h-0 overflow-auto p-4">
            <MarkdownPreview content={activeTab.file.content} />
          </div>
        );
      }

      return (
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <CodeMirror
            ref={editorRef}
            value={activeTab.file.content}
            height="100%"
            theme={qbitTheme}
            extensions={extensions}
            basicSetup={basicSetup}
            onChange={(value) => updateFileContent(activeTab.id, value)}
            className="h-full [&_.cm-editor]:h-full [&_.cm-scroller]:overflow-auto"
          />
        </div>
      );
    }

    return null;
  };

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
      <div className="flex items-center justify-between px-2 py-1.5 bg-muted border-b border-border">
        <div className="flex items-center gap-1">
          {/* Always visible: folder browser */}
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5 gap-1 text-[11px]"
            onClick={() => openBrowser()}
            title="Open file browser"
          >
            <FolderOpen className="w-3 h-3" />
            <span>Browse</span>
          </Button>
          {/* Always visible: save (disabled when no file) */}
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5 gap-1 text-[11px]"
            onClick={() => void saveFile()}
            disabled={!activeFile}
            title="Save file (Ctrl+S)"
          >
            <Save className="w-3 h-3" />
            <span>Save</span>
          </Button>
          {/* Separator before conditional icons */}
          <div className="w-px h-3 bg-border mx-1" />
          {/* Conditional: markdown preview toggle */}
          {activeTab?.type === "file" && activeTab.file.language === "markdown" && (
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-1.5 gap-1 text-[11px]"
              onClick={() => toggleMarkdownPreview(activeTab.id)}
              title={activeTab.file.markdownPreview ? "Switch to edit" : "Switch to preview"}
            >
              {activeTab.file.markdownPreview ? (
                <>
                  <FileText className="w-3 h-3" />
                  <span>Edit</span>
                </>
              ) : (
                <>
                  <Eye className="w-3 h-3" />
                  <span>Preview</span>
                </>
              )}
            </Button>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5"
          onClick={() => {
            closeAllTabs();
            onOpenChange(false);
          }}
          title="Close file editor"
        >
          <X className="w-3 h-3" />
        </Button>
      </div>

      {/* Tab Bar */}
      {hasTabs && (
        <TabBar
          tabs={tabs}
          activeTabId={session?.activeTabId ?? null}
          onSelectTab={setActiveTab}
          onCloseTab={(tabId) => {
            closeTab(tabId);
            // If no tabs left, close panel
            const state = useFileEditorSidebarStore.getState();
            const currentSession = sessionId ? state.sessions[sessionId] : null;
            if (!currentSession || currentSession.tabOrder.length === 0) {
              onOpenChange(false);
            }
          }}
          onCloseOtherTabs={closeOtherTabs}
          onReorderTabs={reorderTabs}
          onCloseAllTabs={() => {
            closeAllTabs();
            onOpenChange(false);
          }}
        />
      )}

      {/* Body */}
      <div className="flex-1 min-h-0 flex flex-col">{renderTabContent()}</div>

      {/* Footer */}
      <div className="px-3 py-2 border-t border-border text-xs text-muted-foreground flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          {session?.vimMode && activeTab?.type === "file" && (
            <Badge variant="outline" className="text-[11px] font-mono uppercase">
              {session?.vimModeState ?? "normal"}
            </Badge>
          )}
          {activeFile?.path && (
            <span className="font-mono text-[11px] truncate">{activeFile.path}</span>
          )}
          {activeTab?.type === "browser" && (
            <span className="font-mono text-[11px] truncate">
              {activeTab.browser.currentPath || workingDirectory || "Browser"}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {tabs.length > 1 && (
            <span className="text-[11px] text-muted-foreground/60">{tabs.length} tabs</span>
          )}
          {activeTab?.type === "file" && (
            <>
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
            </>
          )}
        </div>
      </div>
    </div>
  );
}
