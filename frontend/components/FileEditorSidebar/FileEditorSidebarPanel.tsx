import type { Extension } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { Vim, vim } from "@replit/codemirror-vim";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { basicSetup as uiwBasicSetup } from "@uiw/codemirror-extensions-basic-setup";
import { lineNumbersRelative } from "@uiw/codemirror-extensions-line-numbers-relative";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { Eye, FileText, FolderOpen, Plus, Save, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Markdown } from "@/components/Markdown/Markdown";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useFileEditorSidebar } from "@/hooks/useFileEditorSidebar";
import { useFileWatcher } from "@/hooks/useFileWatcher";
import { useThrottledResize } from "@/hooks/useThrottledResize";
import { getLanguageExtension } from "@/lib/codemirror-languages";
import { qbitTheme } from "@/lib/codemirror-theme";
import { cn } from "@/lib/utils";
import { useFileEditorSidebarStore } from "@/store/file-editor-sidebar";
import { FileConflictBanner } from "./FileConflictBanner";
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

    const state = useFileEditorSidebarStore.getState();

    switch (arg) {
      case "wrap":
        state.setWrap(true);
        break;
      case "nowrap":
        state.setWrap(false);
        break;
      case "number":
      case "nu":
        state.setLineNumbers(true);
        break;
      case "nonumber":
      case "nonu":
        state.setLineNumbers(false);
        break;
      case "relativenumber":
      case "rnu":
        state.setRelativeLineNumbers(true);
        break;
      case "norelativenumber":
      case "nornu":
        state.setRelativeLineNumbers(false);
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

function EditablePathBar({
  value,
  onNavigate,
}: {
  value: string;
  onNavigate: (path: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sync draft when value changes externally (while not editing)
  useEffect(() => {
    if (!editing) {
      setDraft(value);
    }
  }, [value, editing]);

  const startEditing = () => {
    setDraft(value);
    setEditing(true);
    // Focus the input after it renders
    requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.select();
    });
  };

  const commit = () => {
    setEditing(false);
    const trimmed = draft.trim();
    if (trimmed && trimmed !== value) {
      onNavigate(trimmed);
    } else {
      setDraft(value);
    }
  };

  const cancel = () => {
    setEditing(false);
    setDraft(value);
  };

  if (!editing) {
    return (
      <button
        type="button"
        onClick={startEditing}
        className="font-mono text-[11px] truncate text-left hover:text-foreground transition-colors cursor-text min-w-0"
        title={`${value}\nClick to edit path`}
      >
        {value || "Browser"}
      </button>
    );
  }

  return (
    <input
      ref={inputRef}
      type="text"
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          commit();
        } else if (e.key === "Escape") {
          e.preventDefault();
          cancel();
        }
      }}
      onBlur={commit}
      className="font-mono text-[11px] bg-transparent border-b border-primary/50 outline-none text-foreground min-w-0 w-full"
    />
  );
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
  open,
  onOpenChange,
  workingDirectory,
}: FileEditorSidebarPanelProps) {
  const {
    activeTabId,
    activeTab,
    activeFile,
    tabs,
    vimMode,
    vimModeState,
    wrap,
    lineNumbers,
    relativeLineNumbers,
    showHiddenFiles,
    recentFiles,
    width,
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
    setShowHiddenFiles,
    toggleMarkdownPreview,
    reorderTabs,
  } = useFileEditorSidebar(workingDirectory || undefined);

  // Watch open files for external changes
  useFileWatcher();

  const [containerWidth, setContainerWidth] = useState(DEFAULT_WIDTH);
  const [languageExtension, setLanguageExtension] = useState<Extension | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<ReactCodeMirrorRef>(null);

  // Throttled resize handling
  const handleWidthChange = useCallback(
    (newWidth: number) => {
      setContainerWidth(newWidth);
      setWidth(newWidth);
    },
    [setWidth]
  );

  const { startResizing: onStartResize } = useThrottledResize({
    minWidth: MIN_WIDTH,
    maxWidth: MAX_WIDTH,
    onWidthChange: handleWidthChange,
    calculateWidth: (e) => window.innerWidth - e.clientX,
  });

  // Navigate to next/previous tab
  const goToNextTab = useCallback(() => {
    if (tabs.length <= 1) return;
    const tabOrder = tabs.map((tab) => tab.id);
    const currentIndex = activeTabId ? tabOrder.indexOf(activeTabId) : -1;
    const nextIndex = (currentIndex + 1) % tabs.length;
    const nextId = tabOrder[nextIndex];
    if (nextId) setActiveTab(nextId);
  }, [activeTabId, tabs, setActiveTab]);

  const goToPrevTab = useCallback(() => {
    if (tabs.length <= 1) return;
    const tabOrder = tabs.map((tab) => tab.id);
    const currentIndex = activeTabId ? tabOrder.indexOf(activeTabId) : -1;
    const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    const prevId = tabOrder[prevIndex];
    if (prevId) setActiveTab(prevId);
  }, [activeTabId, tabs, setActiveTab]);

  useEffect(() => {
    if (width) {
      setContainerWidth(width);
    }
  }, [width]);

  useEffect(() => {
    setOpen(open);
  }, [open, setOpen]);

  // Load language extension dynamically when active file changes
  useEffect(() => {
    let cancelled = false;

    async function loadLanguage() {
      const lang = activeFile?.language;
      if (!lang) {
        setLanguageExtension(null);
        return;
      }

      const ext = await getLanguageExtension(lang);
      if (!cancelled) {
        setLanguageExtension(ext);
      }
    }

    void loadLanguage();

    return () => {
      cancelled = true;
    };
  }, [activeFile?.language]);

  // Register custom vim commands when vim mode is first enabled
  useEffect(() => {
    if (vimMode) {
      registerVimCommands();
      setVimCallbacks({
        save: () => void saveFile(),
        close: () => {
          // Close current tab; if no tabs left, close panel
          closeTab();
          // Check if we still have tabs after closing
          const state = useFileEditorSidebarStore.getState();
          if (state.tabOrder.length === 0) {
            onOpenChange(false);
          }
        },
        forceClose: () => {
          closeTab();
          const state = useFileEditorSidebarStore.getState();
          if (state.tabOrder.length === 0) {
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
    vimMode,
    saveFile,
    reloadFile,
    closeTab,
    closeAllTabs,
    onOpenChange,
    goToNextTab,
    goToPrevTab,
  ]);

  useEffect(() => {
    if (!vimMode || !editorRef.current?.view) return;

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
  }, [vimMode, setVimModeState]);

  const extensions = useMemo(() => {
    const ext: Extension[] = [];

    // Basic setup from package
    ext.push(
      uiwBasicSetup({
        lineNumbers: (lineNumbers ?? true) && !relativeLineNumbers,
        foldGutter: true,
        highlightActiveLine: true,
      })
    );

    if (relativeLineNumbers) {
      ext.push(lineNumbersRelative);
    }

    // Language extension is loaded asynchronously via useEffect
    if (languageExtension) {
      ext.push(languageExtension);
    }

    if (wrap) {
      ext.push(EditorView.lineWrapping);
    }
    if (vimMode) {
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
  }, [saveFile, languageExtension, vimMode, wrap, lineNumbers, relativeLineNumbers]);

  if (!open) return null;

  const hasTabs = tabs.length > 0;

  // Render the active tab content
  const renderTabContent = () => {
    if (!activeTab) {
      return (
        <FileOpenPrompt
          workingDirectory={workingDirectory ?? undefined}
          onOpen={(path) => openFile(path)}
          onOpenBrowser={() => openBrowser()}
          recentFiles={recentFiles}
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
          showHiddenFiles={showHiddenFiles}
          onToggleHiddenFiles={() => setShowHiddenFiles(!showHiddenFiles)}
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
          {activeTab.file.externallyModified && (
            <FileConflictBanner tabId={activeTab.id} filePath={activeTab.file.path} />
          )}
          <CodeMirror
            ref={editorRef}
            value={activeTab.file.content}
            height="100%"
            theme={qbitTheme}
            extensions={extensions}
            basicSetup={false}
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
          activeTabId={activeTabId}
          onSelectTab={setActiveTab}
          onCloseTab={(tabId) => {
            closeTab(tabId);
            // If no tabs left, close panel
            const state = useFileEditorSidebarStore.getState();
            if (state.tabOrder.length === 0) {
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
        <div className="flex-1 flex items-center gap-2 min-w-0">
          {vimMode && activeTab?.type === "file" && (
            <Badge variant="outline" className="text-[11px] font-mono uppercase">
              {vimModeState ?? "normal"}
            </Badge>
          )}
          {activeTab?.type === "browser" && (
            <EditablePathBar
              value={activeTab.browser.currentPath || workingDirectory || ""}
              onNavigate={(path) => setBrowserPath(activeTab.id, path)}
            />
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {activeTab?.type === "file" && (
            <button
              type="button"
              onClick={() => setVimMode(!vimMode)}
              className={cn(
                "text-[11px] px-1.5 py-0.5 rounded transition-colors",
                vimMode
                  ? "bg-primary/20 text-primary hover:bg-primary/30"
                  : "text-muted-foreground hover:text-foreground hover:bg-muted"
              )}
              title={vimMode ? "Disable Vim mode" : "Enable Vim mode"}
            >
              Vim
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
