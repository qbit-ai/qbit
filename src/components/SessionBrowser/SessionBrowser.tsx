import { formatDistanceToNow } from "date-fns";
import {
  AlertCircle,
  Bot,
  Calendar,
  CheckCircle2,
  Clock,
  Download,
  FileText,
  Folder,
  Loader2,
  MessageSquare,
  Search,
  Wrench,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { notify } from "@/lib/notify";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  exportAiSessionTranscript,
  listAiSessions,
  loadAiSession,
  type SessionListingInfo,
  type SessionSnapshot,
} from "@/lib/ai";

interface SessionBrowserProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSessionRestore?: (identifier: string) => void;
}

const statusConfig = {
  active: {
    color: "text-[var(--ansi-blue)]",
    bgColor: "bg-[var(--ansi-blue)]/10",
    icon: Loader2,
    label: "In Progress",
    dotClass: "bg-[var(--ansi-blue)]",
  },
  completed: {
    color: "text-[var(--ansi-green)]",
    bgColor: "bg-[var(--ansi-green)]/10",
    icon: CheckCircle2,
    label: "Completed",
    dotClass: "bg-[var(--ansi-green)]",
  },
  abandoned: {
    color: "text-[var(--ansi-yellow)]",
    bgColor: "bg-[var(--ansi-yellow)]/10",
    icon: AlertCircle,
    label: "Interrupted",
    dotClass: "bg-[var(--ansi-yellow)]",
  },
} as const;

function StatusDot({ status }: { status?: string }) {
  if (!status || !(status in statusConfig)) return null;
  const config = statusConfig[status as keyof typeof statusConfig];
  return (
    <span className={`inline-block w-2 h-2 rounded-full ${config.dotClass}`} title={config.label} />
  );
}

function StatusBadge({ status }: { status?: string }) {
  if (!status || !(status in statusConfig)) return null;
  const config = statusConfig[status as keyof typeof statusConfig];
  const Icon = config.icon;
  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs ${config.color} ${config.bgColor}`}
    >
      <Icon className={`h-3 w-3 ${status === "active" ? "animate-spin" : ""}`} />
      {config.label}
    </span>
  );
}

export function SessionBrowser({ open, onOpenChange, onSessionRestore }: SessionBrowserProps) {
  const [sessions, setSessions] = useState<SessionListingInfo[]>([]);
  const [filteredSessions, setFilteredSessions] = useState<SessionListingInfo[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<SessionListingInfo | null>(null);
  const [sessionDetail, setSessionDetail] = useState<SessionSnapshot | null>(null);
  const [isLoadingDetail, setIsLoadingDetail] = useState(false);

  const loadSessions = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await listAiSessions(50);
      setSessions(result);
      setFilteredSessions(result);
    } catch (error) {
      notify.error(`Failed to load sessions: ${error}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Load sessions when dialog opens
  useEffect(() => {
    if (open) {
      loadSessions();
    } else {
      // Reset state when closing
      setSelectedSession(null);
      setSessionDetail(null);
      setSearchQuery("");
    }
  }, [open, loadSessions]);

  // Filter sessions based on search query
  useEffect(() => {
    if (!searchQuery.trim()) {
      setFilteredSessions(sessions);
      return;
    }

    const query = searchQuery.toLowerCase();
    const filtered = sessions.filter(
      (session) =>
        session.workspace_label.toLowerCase().includes(query) ||
        session.model.toLowerCase().includes(query) ||
        session.first_prompt_preview?.toLowerCase().includes(query) ||
        session.first_reply_preview?.toLowerCase().includes(query)
    );
    setFilteredSessions(filtered);
  }, [searchQuery, sessions]);

  const handleSelectSession = useCallback(async (session: SessionListingInfo) => {
    setSelectedSession(session);
    setIsLoadingDetail(true);
    try {
      const detail = await loadAiSession(session.identifier);
      setSessionDetail(detail);
    } catch (error) {
      notify.error(`Failed to load session details: ${error}`);
    } finally {
      setIsLoadingDetail(false);
    }
  }, []);

  const handleExportSession = useCallback(
    async (session: SessionListingInfo, e: React.MouseEvent) => {
      e.stopPropagation();
      try {
        // Use Downloads folder with session identifier
        const outputPath = `${session.workspace_path}/session-${session.identifier}.md`;
        await exportAiSessionTranscript(session.identifier, outputPath);
        notify.success(`Exported to ${outputPath}`);
      } catch (error) {
        notify.error(`Failed to export: ${error}`);
      }
    },
    []
  );

  const handleLoadSession = useCallback(() => {
    if (selectedSession && onSessionRestore) {
      onSessionRestore(selectedSession.identifier);
      onOpenChange(false);
    }
  }, [selectedSession, onSessionRestore, onOpenChange]);

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      const now = new Date();
      const diffMs = now.getTime() - date.getTime();
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMs / 3600000);
      const diffDays = Math.floor(diffMs / 86400000);

      if (diffMins < 1) return "just now";
      if (diffMins < 60) return `${diffMins}m ago`;
      if (diffHours < 24) return `${diffHours}h ago`;
      if (diffDays < 7) return `${diffDays}d ago`;
      if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
      return formatDistanceToNow(date, { addSuffix: true });
    } catch {
      return dateStr;
    }
  };

  const formatDuration = (startedAt: string, endedAt: string) => {
    try {
      const start = new Date(startedAt);
      const end = new Date(endedAt);
      const durationMs = end.getTime() - start.getTime();
      const minutes = Math.floor(durationMs / 60000);
      const seconds = Math.floor((durationMs % 60000) / 1000);
      if (minutes > 0) {
        return `${minutes}m ${seconds}s`;
      }
      return `${seconds}s`;
    } catch {
      return "—";
    }
  };

  const truncateAtWord = (text: string, maxLength: number): string => {
    if (text.length <= maxLength) return text;
    const truncated = text.slice(0, maxLength);
    const lastSpace = truncated.lastIndexOf(" ");
    // Only use word boundary if it's reasonably close to max length (within 30%)
    if (lastSpace > maxLength * 0.7) {
      return `${truncated.slice(0, lastSpace)}...`;
    }
    return `${truncated}...`;
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="h-[85vh] p-0 gap-0 bg-accent border-border flex flex-col"
        style={{ maxWidth: "90vw", width: "90vw" }}
      >
        <DialogHeader className="px-4 py-3 border-b border-border shrink-0">
          <DialogTitle className="text-foreground flex items-center gap-2">
            <Clock className="h-5 w-5 text-[var(--ansi-blue)]" />
            Session History
          </DialogTitle>
          <DialogDescription className="text-muted-foreground">
            Browse and restore previous AI conversations
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* Session List */}
          <div className="w-[380px] shrink-0 border-r border-border flex flex-col min-h-0">
            {/* Search */}
            <div className="p-3 border-b border-border shrink-0">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search sessions..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 bg-card border-border text-foreground placeholder:text-muted-foreground"
                />
              </div>
            </div>

            {/* Session List */}
            <ScrollArea className="flex-1 min-h-0">
              {isLoading ? (
                <div className="p-4 text-center text-muted-foreground">Loading sessions...</div>
              ) : filteredSessions.length === 0 ? (
                <div className="p-4 text-center text-muted-foreground">
                  {sessions.length === 0 ? "No sessions found" : "No matching sessions"}
                </div>
              ) : (
                <div className="p-2">
                  {filteredSessions.map((session) => (
                    <button
                      type="button"
                      key={session.identifier}
                      onClick={() => handleSelectSession(session)}
                      className={`w-full text-left p-3 rounded-lg mb-1 transition-colors ${
                        selectedSession?.identifier === session.identifier
                          ? "bg-primary/10 border border-[var(--ansi-blue)]"
                          : "hover:bg-card border border-transparent"
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <Folder className="h-3.5 w-3.5 text-[var(--ansi-blue)] shrink-0" />
                            <span className="text-sm font-medium text-foreground truncate">
                              {session.title || session.workspace_label}
                            </span>
                            {session.title && (
                              <span className="text-xs text-muted-foreground truncate">
                                {session.workspace_label}
                              </span>
                            )}
                          </div>
                          {session.first_prompt_preview && (
                            <p className="text-xs text-muted-foreground truncate mb-1">
                              {session.first_prompt_preview}
                            </p>
                          )}
                          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                            <StatusDot status={session.status} />
                            <MessageSquare className="h-3 w-3" />
                            <span>{session.total_messages} messages</span>
                            <span>•</span>
                            <span>{formatDate(session.ended_at)}</span>
                          </div>
                        </div>
                        <button
                          type="button"
                          onClick={(e) => handleExportSession(session, e)}
                          className="p-1.5 rounded hover:bg-card text-muted-foreground hover:text-[var(--ansi-blue)] transition-colors"
                          title="Export transcript"
                        >
                          <Download className="h-4 w-4" />
                        </button>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </ScrollArea>
          </div>

          {/* Session Detail */}
          <div className="flex-1 flex flex-col min-w-0 min-h-0">
            {selectedSession ? (
              <>
                {/* Session Header */}
                <div className="p-4 border-b border-border shrink-0">
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-lg font-medium text-foreground mb-1">
                        {selectedSession.title || selectedSession.workspace_label}
                      </h3>
                      {selectedSession.title && (
                        <p className="text-sm text-muted-foreground mb-2 flex items-center gap-1.5">
                          <Folder className="h-3.5 w-3.5" />
                          {selectedSession.workspace_label}
                        </p>
                      )}
                      <div className="flex flex-wrap items-center gap-4 text-sm text-[#a9b1d6]">
                        <StatusBadge status={selectedSession.status} />
                        <span className="flex items-center gap-1.5">
                          <Bot className="h-4 w-4 text-[#bb9af7]" />
                          {selectedSession.model}
                        </span>
                        <span className="flex items-center gap-1.5">
                          <Calendar className="h-4 w-4 text-[#7dcfff]" />
                          {formatDate(selectedSession.started_at)}
                        </span>
                        <span className="flex items-center gap-1.5">
                          <Clock className="h-4 w-4 text-[#9ece6a]" />
                          {formatDuration(selectedSession.started_at, selectedSession.ended_at)}
                        </span>
                      </div>
                      {selectedSession.distinct_tools.length > 0 && (
                        <div className="flex items-center gap-2 mt-2">
                          <Wrench className="h-4 w-4 text-[#e0af68]" />
                          <div className="flex flex-wrap gap-1">
                            {selectedSession.distinct_tools.slice(0, 5).map((tool) => (
                              <span
                                key={tool}
                                className="px-2 py-0.5 text-xs bg-[#1f2335] text-[#a9b1d6] rounded"
                              >
                                {tool}
                              </span>
                            ))}
                            {selectedSession.distinct_tools.length > 5 && (
                              <span className="px-2 py-0.5 text-xs bg-[#1f2335] text-[#565f89] rounded">
                                +{selectedSession.distinct_tools.length - 5} more
                              </span>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                    {onSessionRestore && (
                      <button
                        type="button"
                        onClick={handleLoadSession}
                        disabled={!selectedSession}
                        className="px-4 py-2 bg-[#7aa2f7] text-[#1a1b26] rounded-lg font-medium hover:bg-[#89b4fa] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                      >
                        Load Session
                      </button>
                    )}
                  </div>
                </div>

                {/* Messages Preview */}
                <ScrollArea className="flex-1 min-h-0">
                  {isLoadingDetail ? (
                    <div className="p-4 text-center text-[#565f89] py-8">Loading messages...</div>
                  ) : sessionDetail ? (
                    <div className="p-4 space-y-4">
                      {sessionDetail.messages.map((msg, index) => (
                        <div
                          key={`${msg.role}-${index}-${msg.content.slice(0, 20)}`}
                          className={`p-3 rounded-lg ${
                            msg.role === "user"
                              ? "bg-[#1f2335] border-l-2 border-[#7aa2f7]"
                              : msg.role === "assistant"
                                ? "bg-[#1f2335] border-l-2 border-[#9ece6a]"
                                : msg.role === "tool"
                                  ? "bg-[#1f2335] border-l-2 border-[#e0af68]"
                                  : "bg-[#1f2335] border-l-2 border-[#565f89]"
                          }`}
                        >
                          <div className="flex items-center gap-2 mb-2">
                            {msg.role === "user" && (
                              <span className="text-xs font-medium text-[#7aa2f7]">User</span>
                            )}
                            {msg.role === "assistant" && (
                              <span className="text-xs font-medium text-[#9ece6a]">Assistant</span>
                            )}
                            {msg.role === "tool" && (
                              <span className="text-xs font-medium text-[#e0af68]">
                                Tool: {msg.tool_name || "unknown"}
                              </span>
                            )}
                            {msg.role === "system" && (
                              <span className="text-xs font-medium text-[#565f89]">System</span>
                            )}
                          </div>
                          <p className="text-sm text-[#c0caf5] whitespace-pre-wrap break-words">
                            {truncateAtWord(msg.content, 500)}
                          </p>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="p-4 text-center text-[#565f89] py-8">
                      Failed to load session details
                    </div>
                  )}
                </ScrollArea>
              </>
            ) : (
              <div className="flex-1 flex items-center justify-center text-[#565f89]">
                <div className="text-center">
                  <FileText className="h-12 w-12 mx-auto mb-3 opacity-50" />
                  <p>Select a session to view details</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
