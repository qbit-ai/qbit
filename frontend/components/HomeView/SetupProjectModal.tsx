import { open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, GitBranch, Play, Search, Terminal, Wrench, X } from "lucide-react";
import { useCallback, useState } from "react";

interface SetupProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (projectData: ProjectFormData) => void;
}

export interface ProjectFormData {
  name: string;
  rootPath: string;
  worktreesDir: string;
  testCommand: string;
  lintCommand: string;
  buildCommand: string;
  startCommand: string;
  worktreeInitScript: string;
}

export function SetupProjectModal({ isOpen, onClose, onSubmit }: SetupProjectModalProps) {
  const [formData, setFormData] = useState<ProjectFormData>({
    name: "",
    rootPath: "",
    worktreesDir: "",
    testCommand: "",
    lintCommand: "",
    buildCommand: "",
    startCommand: "",
    worktreeInitScript: "",
  });

  const handleChange = useCallback((field: keyof ProjectFormData, value: string) => {
    setFormData((prev) => ({
      ...prev,
      [field]: value,
    }));
  }, []);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(formData);
    onClose();
  };

  const handlePickFolder = useCallback(
    async (field: "rootPath" | "worktreesDir") => {
      const selected = await openFolderDialog({
        directory: true,
        multiple: false,
        title: field === "rootPath" ? "Select project root folder" : "Select worktrees directory",
      });

      if (selected) {
        handleChange(field, selected);
      }
    },
    [handleChange]
  );

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: backdrop dismiss */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: backdrop dismiss */}
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[#30363d]">
          <h2 className="text-lg font-semibold text-white">Setup New Project</h2>
          <button
            type="button"
            onClick={onClose}
            className="p-1 hover:bg-[#30363d] rounded transition-colors"
          >
            <X size={20} className="text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <form
          onSubmit={handleSubmit}
          className="p-6 overflow-y-auto max-h-[calc(90vh-140px)] custom-scrollbar"
        >
          <div className="space-y-6">
            {/* Basic Info Section */}
            <div className="space-y-4">
              <div className="flex items-center space-x-2 text-sm font-medium text-gray-300">
                <FolderOpen size={16} className="text-[#58a6ff]" />
                <span>Basic Information</span>
              </div>

              <div className="space-y-3 pl-6">
                <div>
                  <label className="block">
                    <span className="block text-xs text-gray-400 mb-1.5">Project Name</span>
                    <input
                      type="text"
                      value={formData.name}
                      onChange={(e) => handleChange("name", e.target.value)}
                      placeholder="my-awesome-project"
                      className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                    />
                  </label>
                </div>
              </div>
            </div>

            {/* Paths Section */}
            <div className="space-y-4">
              <div className="flex items-center space-x-2 text-sm font-medium text-gray-300">
                <Search size={16} className="text-[#58a6ff]" />
                <span>Project Paths</span>
              </div>

              <div className="space-y-3 pl-6">
                <div>
                  <span className="block text-xs text-gray-400 mb-1.5">Root Project Path</span>
                  <div className="flex items-center space-x-2">
                    <label className="flex-1">
                      <input
                        type="text"
                        value={formData.rootPath}
                        onChange={(e) => handleChange("rootPath", e.target.value)}
                        placeholder="/Users/username/Code/project"
                        className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                      />
                    </label>
                    <button
                      type="button"
                      onClick={() => handlePickFolder("rootPath")}
                      className="h-[38px] px-3 bg-[#21262d] border border-[#30363d] rounded-md hover:bg-[#30363d] transition-colors"
                    >
                      <FolderOpen size={16} className="text-gray-400" />
                    </button>
                  </div>
                </div>

                <div>
                  <span className="block text-xs text-gray-400 mb-1.5">Worktrees Directory</span>
                  <div className="flex items-center space-x-2">
                    <label className="flex-1">
                      <input
                        type="text"
                        value={formData.worktreesDir}
                        onChange={(e) => handleChange("worktreesDir", e.target.value)}
                        placeholder="/Users/username/Code/project-worktrees"
                        className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                      />
                    </label>
                    <button
                      type="button"
                      onClick={() => handlePickFolder("worktreesDir")}
                      className="h-[38px] px-3 bg-[#21262d] border border-[#30363d] rounded-md hover:bg-[#30363d] transition-colors"
                    >
                      <FolderOpen size={16} className="text-gray-400" />
                    </button>
                  </div>
                  <p className="text-xs text-gray-600 mt-1.5">
                    Directory where git worktrees will be created
                  </p>
                </div>
              </div>
            </div>

            {/* Commands Section */}
            <div className="space-y-4">
              <div className="flex items-center space-x-2 text-sm font-medium text-gray-300">
                <Terminal size={16} className="text-[#58a6ff]" />
                <span>Commands</span>
              </div>

              <div className="space-y-3 pl-6">
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block">
                      <span className="flex items-center space-x-1 text-xs text-gray-400 mb-1.5">
                        <Play size={12} className="text-[#3fb950]" />
                        <span>Test Command</span>
                      </span>
                      <input
                        type="text"
                        value={formData.testCommand}
                        onChange={(e) => handleChange("testCommand", e.target.value)}
                        className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                      />
                    </label>
                  </div>

                  <div>
                    <label className="block">
                      <span className="flex items-center space-x-1 text-xs text-gray-400 mb-1.5">
                        <Search size={12} className="text-[#f0883e]" />
                        <span>Lint Command</span>
                      </span>
                      <input
                        type="text"
                        value={formData.lintCommand}
                        onChange={(e) => handleChange("lintCommand", e.target.value)}
                        className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                      />
                    </label>
                  </div>

                  <div>
                    <label className="block">
                      <span className="flex items-center space-x-1 text-xs text-gray-400 mb-1.5">
                        <Wrench size={12} className="text-[#a371f7]" />
                        <span>Build Command</span>
                      </span>
                      <input
                        type="text"
                        value={formData.buildCommand}
                        onChange={(e) => handleChange("buildCommand", e.target.value)}
                        className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                      />
                    </label>
                  </div>

                  <div>
                    <label className="block">
                      <span className="flex items-center space-x-1 text-xs text-gray-400 mb-1.5">
                        <Play size={12} className="text-[#58a6ff]" />
                        <span>Start Command</span>
                      </span>
                      <input
                        type="text"
                        value={formData.startCommand}
                        onChange={(e) => handleChange("startCommand", e.target.value)}
                        className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors"
                      />
                    </label>
                  </div>
                </div>
              </div>
            </div>

            {/* Worktree Initialization Section */}
            <div className="space-y-4">
              <div className="flex items-center space-x-2 text-sm font-medium text-gray-300">
                <GitBranch size={16} className="text-[#58a6ff]" />
                <span>New Worktree Initialization</span>
              </div>

              <div className="space-y-3 pl-6">
                <div>
                  <label className="block">
                    <span className="block text-xs text-gray-400 mb-1.5">
                      Initialization Script
                    </span>
                    <textarea
                      value={formData.worktreeInitScript}
                      onChange={(e) => handleChange("worktreeInitScript", e.target.value)}
                      rows={5}
                      className="w-full bg-[#0d1117] border border-[#30363d] rounded-md px-3 py-2 text-sm text-gray-200 placeholder-gray-600 font-mono focus:outline-none focus:border-[#58a6ff] focus:ring-1 focus:ring-[#58a6ff] transition-colors resize-none"
                    />
                  </label>
                  <p className="text-xs text-gray-600 mt-1.5">
                    Commands to run when a new worktree is created (one per line)
                  </p>
                </div>
              </div>
            </div>
          </div>
        </form>

        {/* Footer */}
        <div className="flex items-center justify-end space-x-3 p-4 border-t border-[#30363d] bg-[#161b22]">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-300 bg-[#21262d] border border-[#30363d] rounded-md hover:bg-[#30363d] transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            onClick={handleSubmit}
            className="px-4 py-2 text-sm font-medium text-white bg-[#238636] rounded-md hover:bg-[#2ea043] transition-colors"
          >
            Create Project
          </button>
        </div>
      </div>
    </div>
  );
}
