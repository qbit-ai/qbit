import { Switch } from "@/components/ui/switch";
import { useFileEditorSidebarStore } from "@/store/file-editor-sidebar";

export function EditorSettings() {
  const {
    vimMode,
    setVimMode,
    wrap,
    setWrap,
    lineNumbers,
    setLineNumbers,
    relativeLineNumbers,
    setRelativeLineNumbers,
  } = useFileEditorSidebarStore();

  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-foreground">General</h3>

        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <label
              htmlFor="editor-wrap"
              className="text-sm font-medium text-foreground cursor-pointer"
            >
              Word Wrap
            </label>
            <p className="text-xs text-muted-foreground">Wrap long lines to fit the editor width</p>
          </div>
          <Switch id="editor-wrap" checked={wrap} onCheckedChange={setWrap} />
        </div>

        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <label
              htmlFor="editor-line-numbers"
              className="text-sm font-medium text-foreground cursor-pointer"
            >
              Line Numbers
            </label>
            <p className="text-xs text-muted-foreground">Show line numbers in the gutter</p>
          </div>
          <Switch id="editor-line-numbers" checked={lineNumbers} onCheckedChange={setLineNumbers} />
        </div>

        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <label
              htmlFor="editor-relative-line-numbers"
              className={`text-sm font-medium cursor-pointer ${!lineNumbers ? "text-muted-foreground" : "text-foreground"}`}
            >
              Relative Line Numbers
            </label>
            <p className="text-xs text-muted-foreground">
              Show line numbers relative to the cursor position (useful for Vim)
            </p>
          </div>
          <Switch
            id="editor-relative-line-numbers"
            checked={relativeLineNumbers}
            onCheckedChange={setRelativeLineNumbers}
            disabled={!lineNumbers}
          />
        </div>
      </div>

      <div className="border-t border-[var(--border-subtle)]" />

      <div className="space-y-4">
        <h3 className="text-sm font-medium text-foreground">Vim Mode</h3>

        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <label
              htmlFor="editor-vim-mode"
              className="text-sm font-medium text-foreground cursor-pointer"
            >
              Enable Vim Mode
            </label>
            <p className="text-xs text-muted-foreground">Use Vim keybindings for editing</p>
          </div>
          <Switch id="editor-vim-mode" checked={vimMode} onCheckedChange={setVimMode} />
        </div>
      </div>
    </div>
  );
}
