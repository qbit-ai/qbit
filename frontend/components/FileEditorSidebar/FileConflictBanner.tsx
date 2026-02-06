import { AlertTriangle, Download, X } from "lucide-react";
import { useCallback } from "react";
import { Button } from "@/components/ui/button";
import { readWorkspaceFile } from "@/lib/file-editor";
import { notify } from "@/lib/notify";
import { useFileEditorSidebarStore } from "@/store/file-editor-sidebar";

interface FileConflictBannerProps {
  tabId: string;
  filePath: string;
}

export function FileConflictBanner({ tabId, filePath }: FileConflictBannerProps) {
  const handleReload = useCallback(async () => {
    try {
      const result = await readWorkspaceFile(filePath);
      useFileEditorSidebarStore.getState().acceptExternalChange(tabId, result.content, result.modifiedAt);
    } catch (error) {
      notify.error(`Failed to reload file: ${error}`);
    }
  }, [tabId, filePath]);

  const handleKeep = useCallback(() => {
    useFileEditorSidebarStore.getState().keepLocalVersion(tabId);
  }, [tabId]);

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 bg-yellow-500/10 border-b border-yellow-500/30 text-xs">
      <AlertTriangle className="w-3.5 h-3.5 text-yellow-500 shrink-0" />
      <span className="text-yellow-200/90 truncate">
        This file has been modified externally.
      </span>
      <div className="flex items-center gap-1 shrink-0 ml-auto">
        <Button
          variant="ghost"
          size="sm"
          className="h-5 px-1.5 text-[11px] gap-1 text-yellow-200/90 hover:text-yellow-100"
          onClick={handleReload}
        >
          <Download className="w-3 h-3" />
          Reload
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="h-5 px-1.5 text-[11px] gap-1 text-yellow-200/90 hover:text-yellow-100"
          onClick={handleKeep}
        >
          <X className="w-3 h-3" />
          Keep mine
        </Button>
      </div>
    </div>
  );
}
