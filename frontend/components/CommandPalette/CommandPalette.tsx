import {
  ArrowLeftRight,
  Clock,
  Columns,
  Database,
  FilePenLine,
  FileSearch,
  FileText,
  FolderTree,
  Keyboard,
  ListTodo,
  Monitor,
  Palette,
  Plus,
  RefreshCw,
  Rows,
  Search,
  Settings,
  Terminal,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useState } from "react";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import { indexDirectory, isIndexerInitialized, searchCode, searchFiles } from "@/lib/indexer";
import { notify } from "@/lib/notify";

export type PageRoute = "main" | "testbed";

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentPage: PageRoute;
  onNavigate: (page: PageRoute) => void;
  activeSessionId: string | null;
  onNewTab: () => void;
  onToggleMode: () => void;
  onClearConversation: () => void;
  onToggleSidebar?: () => void;
  onToggleFullTerminal?: () => void;
  workingDirectory?: string;
  onShowSearchResults?: (results: SearchResult[]) => void;
  onOpenSessionBrowser?: () => void;
  onToggleFileEditorPanel?: () => void;
  onOpenContextPanel?: () => void;
  onOpenTaskPlanner?: () => void;
  onOpenSettings?: () => void;
  // Pane management
  onSplitPaneRight?: () => void;
  onSplitPaneDown?: () => void;
  onClosePane?: () => void;
}

// Types for search results
export interface SearchResult {
  file_path: string;
  line_number: number;
  line_content: string;
  matches: string[];
}

export interface SymbolResult {
  name: string;
  kind: string;
  line: number;
  column: number;
  scope: string | null;
  signature: string | null;
  documentation: string | null;
}

export function CommandPalette({
  open,
  onOpenChange,
  currentPage,
  onNavigate,
  activeSessionId,
  onNewTab,
  onToggleMode,
  onClearConversation,
  onToggleSidebar,
  onToggleFullTerminal,
  workingDirectory,
  onShowSearchResults,
  onOpenSessionBrowser,
  onToggleFileEditorPanel,
  onOpenContextPanel,
  onOpenTaskPlanner,
  onOpenSettings,
  onSplitPaneRight,
  onSplitPaneDown,
  onClosePane,
}: CommandPaletteProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [isSearching, setIsSearching] = useState(false);

  // Handle command selection
  const runCommand = useCallback(
    (command: () => void) => {
      onOpenChange(false);
      command();
    },
    [onOpenChange]
  );

  // Re-index workspace
  const handleReindex = useCallback(async () => {
    if (!workingDirectory) {
      notify.error("No workspace directory available");
      return;
    }
    try {
      const initialized = await isIndexerInitialized();
      if (!initialized) {
        notify.error("Indexer not initialized");
        return;
      }
      notify.info("Re-indexing workspace...");
      await indexDirectory(workingDirectory);
      notify.success("Workspace re-indexed successfully");
    } catch (error) {
      notify.error(`Failed to re-index: ${error}`);
    }
  }, [workingDirectory]);

  // Search code in workspace
  const handleSearchCode = useCallback(async () => {
    if (!searchQuery.trim()) {
      notify.error("Enter a search query first");
      return;
    }
    try {
      setIsSearching(true);
      const results = await searchCode(searchQuery);
      if (results.length === 0) {
        notify.info("No matches found");
      } else {
        notify.success(`Found ${results.length} matches`);
        onShowSearchResults?.(results);
      }
    } catch (error) {
      notify.error(`Search failed: ${error}`);
    } finally {
      setIsSearching(false);
    }
  }, [searchQuery, onShowSearchResults]);

  // Search files by name
  const handleSearchFiles = useCallback(async () => {
    if (!searchQuery.trim()) {
      notify.error("Enter a file name pattern first");
      return;
    }
    try {
      setIsSearching(true);
      const files = await searchFiles(searchQuery);
      if (files.length === 0) {
        notify.info("No files found");
      } else {
        notify.success(`Found ${files.length} files`);
        // Convert to search results format for display
        const results: SearchResult[] = files.map((f) => ({
          file_path: f,
          line_number: 0,
          line_content: "",
          matches: [],
        }));
        onShowSearchResults?.(results);
      }
    } catch (error) {
      notify.error(`File search failed: ${error}`);
    } finally {
      setIsSearching(false);
    }
  }, [searchQuery, onShowSearchResults]);

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange}>
      <CommandInput
        placeholder="Type a command or search..."
        value={searchQuery}
        onValueChange={setSearchQuery}
      />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        {/* Navigation */}
        <CommandGroup heading="Navigation">
          <CommandItem
            onSelect={() => runCommand(() => onNavigate("main"))}
            disabled={currentPage === "main"}
          >
            <Terminal className="mr-2 h-4 w-4" />
            <span>Main App</span>
            {currentPage === "main" && (
              <span className="ml-auto text-xs text-muted-foreground">Current</span>
            )}
          </CommandItem>
          <CommandItem
            onSelect={() => runCommand(() => onNavigate("testbed"))}
            disabled={currentPage === "testbed"}
          >
            <Palette className="mr-2 h-4 w-4" />
            <span>Component Testbed</span>
            {currentPage === "testbed" && (
              <span className="ml-auto text-xs text-muted-foreground">Current</span>
            )}
          </CommandItem>
          {onToggleSidebar && (
            <CommandItem onSelect={() => runCommand(onToggleSidebar)}>
              <FolderTree className="mr-2 h-4 w-4" />
              <span>Toggle Sidebar</span>
              <CommandShortcut>⌘B</CommandShortcut>
            </CommandItem>
          )}
        </CommandGroup>

        <CommandSeparator />

        {/* Session Actions */}
        <CommandGroup heading="Session">
          <CommandItem onSelect={() => runCommand(onNewTab)}>
            <Plus className="mr-2 h-4 w-4" />
            <span>New Tab</span>
            <CommandShortcut>⌘T</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(onToggleMode)}>
            <ArrowLeftRight className="mr-2 h-4 w-4" />
            <span>Toggle Mode</span>
            <CommandShortcut>⌘I</CommandShortcut>
          </CommandItem>
          {onToggleFullTerminal && activeSessionId && (
            <CommandItem onSelect={() => runCommand(onToggleFullTerminal)}>
              <Monitor className="mr-2 h-4 w-4" />
              <span>Toggle Full Terminal</span>
              <CommandShortcut>⌘⇧F</CommandShortcut>
            </CommandItem>
          )}
          {activeSessionId && (
            <CommandItem onSelect={() => runCommand(onClearConversation)}>
              <Trash2 className="mr-2 h-4 w-4" />
              <span>Clear Conversation</span>
              <CommandShortcut>⌘K</CommandShortcut>
            </CommandItem>
          )}
          {onOpenSessionBrowser && (
            <CommandItem onSelect={() => runCommand(onOpenSessionBrowser)}>
              <Clock className="mr-2 h-4 w-4" />
              <span>Browse Session History</span>
              <CommandShortcut>⌘H</CommandShortcut>
            </CommandItem>
          )}
          {onToggleFileEditorPanel && (
            <CommandItem onSelect={() => runCommand(onToggleFileEditorPanel)}>
              <FilePenLine className="mr-2 h-4 w-4" />
              <span>File Editor Panel</span>
              <CommandShortcut>⌘⇧E</CommandShortcut>
            </CommandItem>
          )}
          {onOpenContextPanel && (
            <CommandItem onSelect={() => runCommand(onOpenContextPanel)}>
              <Database className="mr-2 h-4 w-4" />
              <span>Context Capture</span>
              <CommandShortcut>⌘⇧C</CommandShortcut>
            </CommandItem>
          )}
          {onOpenTaskPlanner && (
            <CommandItem onSelect={() => runCommand(onOpenTaskPlanner)}>
              <ListTodo className="mr-2 h-4 w-4" />
              <span>Task Planner</span>
              <CommandShortcut>⌘⇧T</CommandShortcut>
            </CommandItem>
          )}
        </CommandGroup>

        <CommandSeparator />

        {/* Pane Management */}
        {(onSplitPaneRight || onSplitPaneDown || onClosePane) && (
          <>
            <CommandGroup heading="Panes">
              {onSplitPaneRight && (
                <CommandItem onSelect={() => runCommand(onSplitPaneRight)}>
                  <Columns className="mr-2 h-4 w-4" />
                  <span>Split Pane Right</span>
                  <CommandShortcut>⌘D</CommandShortcut>
                </CommandItem>
              )}
              {onSplitPaneDown && (
                <CommandItem onSelect={() => runCommand(onSplitPaneDown)}>
                  <Rows className="mr-2 h-4 w-4" />
                  <span>Split Pane Down</span>
                  <CommandShortcut>⌘⇧D</CommandShortcut>
                </CommandItem>
              )}
              {onClosePane && (
                <CommandItem onSelect={() => runCommand(onClosePane)}>
                  <X className="mr-2 h-4 w-4" />
                  <span>Close Pane</span>
                  <CommandShortcut>⌘W</CommandShortcut>
                </CommandItem>
              )}
            </CommandGroup>
            <CommandSeparator />
          </>
        )}

        {/* Code Search & Analysis */}
        <CommandGroup heading="Code Search">
          <CommandItem onSelect={() => runCommand(handleSearchCode)} disabled={isSearching}>
            <Search className="mr-2 h-4 w-4" />
            <span>Search Code</span>
            <span className="ml-auto text-xs text-muted-foreground">regex</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(handleSearchFiles)} disabled={isSearching}>
            <FileSearch className="mr-2 h-4 w-4" />
            <span>Find Files</span>
            <span className="ml-auto text-xs text-muted-foreground">pattern</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(handleReindex)} disabled={!workingDirectory}>
            <RefreshCw className="mr-2 h-4 w-4" />
            <span>Re-index Workspace</span>
          </CommandItem>
        </CommandGroup>

        <CommandSeparator />

        {/* Help */}
        <CommandGroup heading="Help">
          <CommandItem disabled>
            <Keyboard className="mr-2 h-4 w-4" />
            <span>Keyboard Shortcuts</span>
          </CommandItem>
          <CommandItem disabled>
            <FileText className="mr-2 h-4 w-4" />
            <span>Documentation</span>
          </CommandItem>
          {onOpenSettings && (
            <CommandItem onSelect={() => runCommand(onOpenSettings)}>
              <Settings className="mr-2 h-4 w-4" />
              <span>Settings</span>
              <CommandShortcut>⌘,</CommandShortcut>
            </CommandItem>
          )}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}

// Hook to manage command palette state
export function useCommandPalette() {
  return {
    // Can be extended with more functionality
  };
}
