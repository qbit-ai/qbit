import {
  ChevronDown,
  ChevronRight,
  File,
  FolderGit2,
  FolderOpen,
  GitBranch,
  Minus,
  Plus,
  RefreshCw,
  TreePine,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useCreateTerminalTab } from "@/hooks/useCreateTerminalTab";
import {
  listProjectsForHome,
  listRecentDirectories,
  type ProjectInfo,
  type RecentDirectory,
} from "@/lib/indexer";
import { type ProjectFormData, saveProject } from "@/lib/projects";
import { NewWorktreeModal } from "./NewWorktreeModal";
import { type ProjectFormData as ModalFormData, SetupProjectModal } from "./SetupProjectModal";

/** Context menu state */
interface ContextMenuState {
  x: number;
  y: number;
  projectPath: string;
  projectName: string;
}

/** Stats badge showing file count, insertions, and deletions */
function StatsBadge({
  fileCount,
  insertions,
  deletions,
}: {
  fileCount: number;
  insertions: number;
  deletions: number;
}) {
  if (fileCount === 0 && insertions === 0 && deletions === 0) {
    return null;
  }

  return (
    <div className="flex items-center bg-[#0d1117] px-2 py-1 rounded-full border border-[#30363d] space-x-2 text-xs text-gray-500">
      {fileCount > 0 && (
        <div className="flex items-center">
          <File size={12} className="mr-0.5 text-gray-500" />
          <span>{fileCount}</span>
        </div>
      )}
      {insertions > 0 && (
        <div className="flex items-center">
          <Plus size={12} className="mr-0.5 text-[#3fb950]" />
          <span>{insertions}</span>
        </div>
      )}
      {deletions > 0 && (
        <div className="flex items-center">
          <Minus size={12} className="mr-0.5 text-[#f85149]" />
          <span>{deletions}</span>
        </div>
      )}
    </div>
  );
}

/** Worktree count badge */
function WorktreeBadge({ count }: { count: number }) {
  return (
    <div className="flex items-center bg-[#0d1117] px-2 py-1 rounded-full border border-[#30363d] text-xs text-gray-500">
      <TreePine size={14} className="mr-1 text-[#238636]" />
      {count}
    </div>
  );
}

/** Single project row (expandable) */
function ProjectRow({
  project,
  isExpanded,
  onToggle,
  onOpenDirectory,
  onContextMenu,
}: {
  project: ProjectInfo;
  isExpanded: boolean;
  onToggle: () => void;
  onOpenDirectory: (path: string) => void;
  onContextMenu: (e: React.MouseEvent) => void;
}) {
  return (
    <div className="border-b border-[#30363d]/50 last:border-0">
      {/* Project header */}
      <button
        type="button"
        onClick={onToggle}
        onContextMenu={onContextMenu}
        className="w-full flex items-center justify-between p-3 hover:bg-[#1c2128] transition-colors group text-left"
      >
        <div className="flex items-center min-w-0 mr-4">
          <div className="mr-2 flex-shrink-0 hover:bg-[#30363d] rounded p-0.5 transition-colors">
            {isExpanded ? (
              <ChevronDown size={14} className="text-gray-500" />
            ) : (
              <ChevronRight size={14} className="text-gray-500" />
            )}
          </div>
          <FolderGit2
            size={16}
            className="text-gray-500 mr-3 flex-shrink-0 group-hover:text-[#58a6ff] transition-colors"
          />
          <div className="min-w-0">
            <div className="text-sm font-medium text-gray-300 truncate group-hover:text-white transition-colors">
              {project.name}
            </div>
          </div>
        </div>

        <div className="flex items-center text-xs text-gray-500 flex-shrink-0 space-x-3">
          <WorktreeBadge count={project.branches.length} />
          <span>{project.last_activity}</span>
          <ChevronRight
            size={14}
            className="opacity-0 group-hover:opacity-100 transition-opacity text-[#58a6ff]"
          />
        </div>
      </button>

      {/* Expanded branches */}
      {isExpanded && project.branches.length > 0 && (
        <div className="bg-[#0d1117] border-t border-[#30363d]/50 max-h-[420px] overflow-y-auto custom-scrollbar">
          {project.branches.map((branch) => (
            <button
              type="button"
              key={branch.name}
              onClick={() => onOpenDirectory(branch.path)}
              className="w-full flex items-center p-3 pl-12 hover:bg-[#161b22] transition-colors text-left border-b border-[#30363d]/30 last:border-0 group"
            >
              <div className="flex items-center min-w-0 w-[450px] mr-4">
                <div className="min-w-0">
                  <div className="flex items-center text-xs text-gray-500">
                    <GitBranch size={12} className="mr-1 text-[#58a6ff] flex-shrink-0" />
                    <span className="text-gray-300 truncate">{branch.name}</span>
                  </div>
                  <div className="text-xs text-gray-600 truncate font-mono mt-0.5">
                    {branch.path}
                  </div>
                </div>
              </div>

              <StatsBadge
                fileCount={branch.file_count}
                insertions={branch.insertions}
                deletions={branch.deletions}
              />

              <div className="flex items-center text-xs text-gray-500 flex-shrink-0 ml-auto space-x-2">
                <span>{branch.last_activity}</span>
                <ChevronRight
                  size={14}
                  className="opacity-0 group-hover:opacity-100 transition-opacity text-[#58a6ff]"
                />
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** Single recent directory row */
function RecentDirectoryRow({
  directory,
  onOpen,
}: {
  directory: RecentDirectory;
  onOpen: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="w-full flex items-center p-3 hover:bg-[#1c2128] transition-colors group text-left border-b border-[#30363d]/50 last:border-0"
    >
      <div className="flex items-center min-w-0 w-[500px] mr-4">
        <FolderOpen
          size={16}
          className="text-gray-500 mr-3 flex-shrink-0 group-hover:text-[#58a6ff] transition-colors"
        />
        <div className="min-w-0">
          <div className="text-sm font-medium text-gray-300 truncate group-hover:text-white transition-colors">
            {directory.name}
          </div>
          {directory.branch && (
            <div className="flex items-center text-xs text-gray-500 opacity-60">
              <GitBranch size={12} className="mr-1 text-[#58a6ff]" />
              <span className="text-gray-300">{directory.branch}</span>
            </div>
          )}
        </div>
      </div>

      <StatsBadge
        fileCount={directory.file_count}
        insertions={directory.insertions}
        deletions={directory.deletions}
      />

      <div className="flex items-center text-xs text-gray-500 flex-shrink-0 ml-auto space-x-2">
        <span>{directory.last_accessed}</span>
        <ChevronRight
          size={14}
          className="opacity-0 group-hover:opacity-100 transition-opacity text-[#58a6ff]"
        />
      </div>
    </button>
  );
}

/** Context menu component */
function ProjectContextMenu({
  x,
  y,
  onNewWorktree,
  onClose,
}: {
  x: number;
  y: number;
  onNewWorktree: () => void;
  onClose: () => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);

  // Close on click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  return (
    <div
      ref={menuRef}
      className="fixed z-50 bg-[#1c2128] border border-[#30363d] rounded-md shadow-xl py-1 min-w-[160px]"
      style={{ left: x, top: y }}
    >
      <button
        type="button"
        onClick={() => {
          onNewWorktree();
          onClose();
        }}
        className="w-full flex items-center px-3 py-2 text-sm text-gray-300 hover:bg-[#30363d] hover:text-white transition-colors text-left"
      >
        <TreePine size={14} className="mr-2 text-[#238636]" />
        New Worktree
      </button>
    </div>
  );
}

export function HomeView() {
  const { createTerminalTab } = useCreateTerminalTab();
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [recentDirectories, setRecentDirectories] = useState<RecentDirectory[]>([]);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isSetupModalOpen, setIsSetupModalOpen] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [worktreeModal, setWorktreeModal] = useState<{
    projectPath: string;
    projectName: string;
  } | null>(null);

  // Fetch data helper
  const fetchData = useCallback(async (showLoadingState = true) => {
    if (showLoadingState) setIsLoading(true);
    setIsRefreshing(true);
    try {
      const [projectsData, directoriesData] = await Promise.all([
        listProjectsForHome(),
        listRecentDirectories(10),
      ]);
      setProjects(projectsData);
      setRecentDirectories(directoriesData);
    } catch (error) {
      console.error("Failed to fetch home view data:", error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  // Fetch data on mount
  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Refresh on window focus
  useEffect(() => {
    const handleFocus = () => {
      fetchData(false);
    };
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, [fetchData]);

  const handleRefresh = useCallback(() => {
    fetchData(false);
  }, [fetchData]);

  const toggleProject = useCallback((path: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const handleOpenDirectory = useCallback(
    (path: string) => {
      createTerminalTab(path);
    },
    [createTerminalTab]
  );

  const handleSetupNewProject = useCallback(() => {
    setIsSetupModalOpen(true);
  }, []);

  const handleProjectContextMenu = useCallback((e: React.MouseEvent, project: ProjectInfo) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      projectPath: project.path,
      projectName: project.name,
    });
  }, []);

  const handleNewWorktree = useCallback(() => {
    if (contextMenu) {
      setWorktreeModal({
        projectPath: contextMenu.projectPath,
        projectName: contextMenu.projectName,
      });
    }
  }, [contextMenu]);

  const handleWorktreeCreated = useCallback(
    (worktreePath: string) => {
      // Refresh the project list to show the new worktree
      fetchData(false);
      // Optionally open the new worktree in a tab
      createTerminalTab(worktreePath);
    },
    [fetchData, createTerminalTab]
  );

  const handleProjectSubmit = useCallback(
    async (data: ModalFormData) => {
      try {
        // Convert modal form data to project API format
        const projectData: ProjectFormData = {
          name: data.name,
          rootPath: data.rootPath,
          worktreesDir: data.worktreesDir,
          testCommand: data.testCommand,
          lintCommand: data.lintCommand,
          buildCommand: data.buildCommand,
          startCommand: data.startCommand,
          worktreeInitScript: data.worktreeInitScript,
        };
        await saveProject(projectData);
        setIsSetupModalOpen(false);
        // Refresh the project list
        fetchData(false);
      } catch (error) {
        console.error("Failed to save project:", error);
        // TODO: Show error toast
      }
    },
    [fetchData]
  );

  if (isLoading) {
    return <div className="h-full flex items-center justify-center text-gray-500">Loading...</div>;
  }

  return (
    <>
      <SetupProjectModal
        isOpen={isSetupModalOpen}
        onClose={() => setIsSetupModalOpen(false)}
        onSubmit={handleProjectSubmit}
      />

      {/* New Worktree Modal */}
      {worktreeModal && (
        <NewWorktreeModal
          isOpen={true}
          projectPath={worktreeModal.projectPath}
          projectName={worktreeModal.projectName}
          onClose={() => setWorktreeModal(null)}
          onSuccess={handleWorktreeCreated}
        />
      )}

      {/* Context Menu */}
      {contextMenu && (
        <ProjectContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onNewWorktree={handleNewWorktree}
          onClose={() => setContextMenu(null)}
        />
      )}
      <div className="h-full overflow-auto bg-[#0d1117] p-8">
        <div className="max-w-3xl mx-auto w-full space-y-8">
          {/* Projects Section */}
          <section className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <h2 className="text-sm font-bold text-gray-400 uppercase tracking-wider">
                  Projects
                </h2>
                <button
                  type="button"
                  onClick={handleRefresh}
                  disabled={isRefreshing}
                  className="p-1 hover:bg-[#30363d] rounded transition-colors disabled:opacity-50"
                  title="Refresh"
                >
                  <RefreshCw
                    size={14}
                    className={`text-gray-500 ${isRefreshing ? "animate-spin" : ""}`}
                  />
                </button>
              </div>
              <button
                type="button"
                onClick={handleSetupNewProject}
                className="flex items-center space-x-2 px-3 py-1.5 bg-[#238636] hover:bg-[#2ea043] text-white text-xs font-medium rounded-md transition-colors"
              >
                <Plus size={14} />
                <span>Setup new project</span>
              </button>
            </div>
            <div className="bg-[#161b22] border border-[#30363d] rounded-lg overflow-hidden">
              {projects.length === 0 ? (
                <div className="p-8 text-center text-gray-500">
                  No projects configured.{" "}
                  <button
                    type="button"
                    className="text-[#238636] hover:underline"
                    onClick={handleSetupNewProject}
                  >
                    Add your first project
                  </button>
                </div>
              ) : (
                projects.map((project) => (
                  <ProjectRow
                    key={project.path}
                    project={project}
                    isExpanded={expandedProjects.has(project.path)}
                    onToggle={() => toggleProject(project.path)}
                    onOpenDirectory={handleOpenDirectory}
                    onContextMenu={(e) => handleProjectContextMenu(e, project)}
                  />
                ))
              )}
            </div>
          </section>

          {/* Recent Directories Section */}
          <section className="space-y-4">
            <h2 className="text-sm font-bold text-gray-400 uppercase tracking-wider">
              Recent Directories
            </h2>
            <div className="bg-[#161b22] border border-[#30363d] rounded-lg overflow-hidden">
              {recentDirectories.length === 0 ? (
                <div className="p-8 text-center text-gray-500">No recent directories</div>
              ) : (
                recentDirectories.map((directory) => (
                  <RecentDirectoryRow
                    key={directory.path}
                    directory={directory}
                    onOpen={() => handleOpenDirectory(directory.path)}
                  />
                ))
              )}
            </div>
          </section>
        </div>
      </div>
    </>
  );
}
