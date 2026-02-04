import { Loader2, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { createGitWorktree, listGitBranches } from "@/lib/indexer";
import { logger } from "@/lib/logger";

export interface NewWorktreeModalProps {
  isOpen: boolean;
  projectPath: string;
  projectName: string;
  onClose: () => void;
  onSuccess: (worktreePath: string) => void;
}

export function NewWorktreeModal({
  isOpen,
  projectPath,
  projectName,
  onClose,
  onSuccess,
}: NewWorktreeModalProps) {
  const [branchName, setBranchName] = useState("");
  const [baseBranch, setBaseBranch] = useState("");
  const [availableBranches, setAvailableBranches] = useState<string[]>([]);
  const [isLoadingBranches, setIsLoadingBranches] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load branches when modal opens
  useEffect(() => {
    if (isOpen && projectPath) {
      setIsLoadingBranches(true);
      setError(null);
      listGitBranches(projectPath)
        .then((branches) => {
          setAvailableBranches(branches);
          // Default to main or master if available
          const defaultBranch = branches.find((b) => b === "main" || b === "master");
          if (defaultBranch) {
            setBaseBranch(defaultBranch);
          } else if (branches.length > 0) {
            setBaseBranch(branches[0]);
          }
        })
        .catch((err) => {
          logger.error("Failed to load branches:", err);
          setError("Failed to load branches");
        })
        .finally(() => {
          setIsLoadingBranches(false);
        });
    }
  }, [isOpen, projectPath]);

  // Reset form when modal closes
  useEffect(() => {
    if (!isOpen) {
      setBranchName("");
      setBaseBranch("");
      setError(null);
    }
  }, [isOpen]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();

      if (!branchName.trim()) {
        setError("Branch name is required");
        return;
      }

      if (!baseBranch) {
        setError("Base branch is required");
        return;
      }

      setIsCreating(true);
      setError(null);

      try {
        const result = await createGitWorktree(projectPath, branchName.trim(), baseBranch);
        onSuccess(result.path);
        onClose();
      } catch (err) {
        logger.error("Failed to create worktree:", err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsCreating(false);
      }
    },
    [projectPath, branchName, baseBranch, onSuccess, onClose]
  );

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: backdrop dismiss */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: backdrop dismiss */}
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-[#161b22] border border-[#30363d] rounded-lg shadow-xl w-full max-w-md mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[#30363d]">
          <h2 className="text-lg font-semibold text-gray-200">New Worktree</h2>
          <button
            type="button"
            onClick={onClose}
            className="p-1 hover:bg-[#30363d] rounded transition-colors"
            aria-label="Close"
          >
            <X size={18} className="text-gray-400" />
          </button>
        </div>

        {/* Body */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          <div className="text-sm text-gray-400 mb-4">
            Create a new worktree for{" "}
            <span className="text-gray-200 font-medium">{projectName}</span>
          </div>

          {/* Branch name */}
          <label className="block">
            <span className="block text-sm font-medium text-gray-300 mb-1">Branch Name</span>
            <input
              type="text"
              value={branchName}
              onChange={(e) => setBranchName(e.target.value)}
              placeholder="feature/my-new-feature"
              className="w-full px-3 py-2 bg-[#0d1117] border border-[#30363d] rounded-md text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-[#58a6ff] focus:border-transparent"
              disabled={isCreating}
            />
          </label>

          {/* Base branch */}
          <div className="block">
            <span className="block text-sm font-medium text-gray-300 mb-1">Base Branch</span>
            {isLoadingBranches ? (
              <div className="flex items-center text-gray-500 text-sm py-2">
                <Loader2 size={14} className="animate-spin mr-2" />
                Loading branches...
              </div>
            ) : (
              <select
                value={baseBranch}
                onChange={(e) => setBaseBranch(e.target.value)}
                className="w-full px-3 py-2 bg-[#0d1117] border border-[#30363d] rounded-md text-gray-200 focus:outline-none focus:ring-2 focus:ring-[#58a6ff] focus:border-transparent"
                disabled={isCreating}
                aria-label="Base Branch"
              >
                {availableBranches.map((branch) => (
                  <option key={branch} value={branch}>
                    {branch}
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* Error */}
          {error && (
            <div className="text-sm text-[#f85149] bg-[#f8514922] border border-[#f8514966] rounded-md p-3">
              {error}
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={isCreating}
              className="px-4 py-2 text-sm text-gray-300 hover:text-white hover:bg-[#30363d] rounded-md transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isCreating || isLoadingBranches || !branchName.trim() || !baseBranch}
              className="px-4 py-2 text-sm bg-[#238636] text-white rounded-md hover:bg-[#2ea043] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
            >
              {isCreating ? (
                <>
                  <Loader2 size={14} className="animate-spin mr-2" />
                  Creating...
                </>
              ) : (
                "Create Worktree"
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
