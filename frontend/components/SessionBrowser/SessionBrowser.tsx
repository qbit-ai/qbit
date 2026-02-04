import { useVirtualizer } from "@tanstack/react-virtual";
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
import { useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
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
import { notify } from "@/lib/notify";

interface SessionBrowserProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSessionRestore?: (identifier: string) => void;
}

const statusConfig = {
  active: {
    color: "text-accent",
    bgColor: "bg-[var(--accent-dim)]",
    icon: Loader2,
    label: "In Progress",
    dotClass: "bg-accent",
  },
  completed: {
    color: "text-[var(--success)]",
    bgColor: "bg-[var(--success-dim)]",
    icon: CheckCircle2,
    label: "Completed",
    dotClass: "bg-[var(--success)]",
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

// Threshold for enabling virtualization on messages list
const MESSAGES_VIRTUALIZATION_THRESHOLD = 20;

// Static style constants for virtualized items
const virtualItemBaseStyle = {
  position: "absolute",
  top: 0,
  left: 0,
  width: "100%",
} as const;

interface SessionMessage {
  role: string;
  content: string;
  tool_name?: string;
}

interface VirtualizedMessagesListProps {
  messages: SessionMessage[];
  truncateAtWord: (text: string, maxLength: number) => string;
}

function VirtualizedMessagesList({ messages, truncateAtWord }: VirtualizedMessagesListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 80, // Estimated height per message
    overscan: 5,
  });

  // For small lists, skip virtualization overhead
  if (messages.length < MESSAGES_VIRTUALIZATION_THRESHOLD) {
    return (
      <div
        ref={parentRef}
        data-testid="messages-container"
        className="h-full overflow-auto p-4 space-y-4"
      >
        {messages.map((msg, index) => (
          <MessageItem
            key={`${msg.role}-${index}-${msg.content.slice(0, 20)}`}
            msg={msg}
            truncateAtWord={truncateAtWord}
          />
        ))}
      </div>
    );
  }

  const virtualItems = virtualizer.getVirtualItems();

  return (
    <div ref={parentRef} data-testid="messages-container" className="h-full overflow-auto">
      <div className="relative w-full p-4" style={{ height: virtualizer.getTotalSize() }}>
        {virtualItems.map((virtualRow) => {
          const msg = messages[virtualRow.index];
          return (
            <div
              key={`${msg.role}-${virtualRow.index}`}
              data-index={virtualRow.index}
              ref={virtualizer.measureElement}
              style={{
                ...virtualItemBaseStyle,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <div className="pb-4">
                <MessageItem msg={msg} truncateAtWord={truncateAtWord} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface MessageItemProps {
  msg: SessionMessage;
  truncateAtWord: (text: string, maxLength: number) => string;
}

function MessageItem({ msg, truncateAtWord }: MessageItemProps) {
  return (
    <div
      className={`p-3 rounded-lg ${
        msg.role === "user"
          ? "bg-muted border-l-2 border-accent"
          : msg.role === "assistant"
            ? "bg-muted border-l-2 border-[var(--success)]"
            : msg.role === "tool"
              ? "bg-muted border-l-2 border-[var(--ansi-yellow)]"
              : "bg-muted border-l-2 border-muted-foreground"
      }`}
    >
      <div className="flex items-center gap-2 mb-2">
        {msg.role === "user" && <span className="text-xs font-medium text-accent">User</span>}
        {msg.role === "assistant" && (
          <span className="text-xs font-medium text-[var(--success)]">Assistant</span>
        )}
        {msg.role === "tool" && (
          <span className="text-xs font-medium text-[var(--ansi-yellow)]">
            Tool: {msg.tool_name || "unknown"}
          </span>
        )}
        {msg.role === "system" && (
          <span className="text-xs font-medium text-muted-foreground">System</span>
        )}
      </div>
      <p className="text-sm text-foreground whitespace-pre-wrap break-words">
        {truncateAtWord(msg.content, 500)}
      </p>
    </div>
  );
}

export function SessionBrowser({ open, onOpenChange, onSessionRestore }: SessionBrowserProps) {
  const [sessions, setSessions] = useState<SessionListingInfo[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<SessionListingInfo | null>(null);
  const [sessionDetail, setSessionDetail] = useState<SessionSnapshot | null>(null);
  const [isLoadingDetail, setIsLoadingDetail] = useState(false);

  // Use deferred value for search to avoid blocking UI during rapid typing
  const deferredSearchQuery = useDeferredValue(searchQuery);

  // Memoized filtered sessions - replaces useEffect+useState pattern
  const filteredSessions = useMemo(() => {
    if (!deferredSearchQuery.trim()) return sessions;
    const query = deferredSearchQuery.toLowerCase();
    return sessions.filter(
      (session) =>
        session.workspace_label.toLowerCase().includes(query) ||
        session.model.toLowerCase().includes(query) ||
        session.first_prompt_preview?.toLowerCase().includes(query) ||
        session.first_reply_preview?.toLowerCase().includes(query)
    );
  }, [deferredSearchQuery, sessions]);

  const loadSessions = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await listAiSessions(50);
      setSessions(result);
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
        className="h-[85vh] p-0 gap-0 bg-card border-[var(--border-medium)] flex flex-col"
        style={{ maxWidth: "90vw", width: "90vw" }}
      >
        <DialogHeader className="px-4 py-3 border-b border-[var(--border-medium)] shrink-0">
          <DialogTitle className="text-foreground flex items-center gap-2">
            <Clock className="h-5 w-5 text-accent" />
            Session History
          </DialogTitle>
          <DialogDescription className="text-muted-foreground">
            Browse and restore previous AI conversations
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* Session List */}
          <div className="w-[380px] shrink-0 border-r border-[var(--border-medium)] flex flex-col min-h-0 bg-background">
            {/* Search */}
            <div className="p-3 border-b border-[var(--border-subtle)] shrink-0">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search sessions..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 bg-muted border-[var(--border-medium)] text-foreground placeholder:text-muted-foreground focus:border-accent focus:ring-1 focus:ring-accent/20"
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
                      className={`w-full text-left p-3 rounded-lg mb-1 transition-colors cursor-pointer ${
                        selectedSession?.identifier === session.identifier
                          ? "bg-[var(--accent-dim)] border border-accent"
                          : "hover:bg-[var(--bg-hover)] border border-transparent"
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <Folder className="h-3.5 w-3.5 text-accent shrink-0" />
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
                          className="p-1.5 rounded hover:bg-[var(--bg-hover)] text-muted-foreground hover:text-accent transition-colors"
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
          <div className="flex-1 flex flex-col min-w-0 min-h-0 bg-card">
            {selectedSession ? (
              <>
                {/* Session Header */}
                <div className="p-4 border-b border-[var(--border-medium)] shrink-0">
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
                      <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
                        <StatusBadge status={selectedSession.status} />
                        <span className="flex items-center gap-1.5">
                          <Bot className="h-4 w-4 text-accent" />
                          {selectedSession.model}
                        </span>
                        <span className="flex items-center gap-1.5">
                          <Calendar className="h-4 w-4 text-accent" />
                          {formatDate(selectedSession.started_at)}
                        </span>
                        <span className="flex items-center gap-1.5">
                          <Clock className="h-4 w-4 text-[var(--success)]" />
                          {formatDuration(selectedSession.started_at, selectedSession.ended_at)}
                        </span>
                      </div>
                      {selectedSession.distinct_tools.length > 0 && (
                        <div className="flex items-center gap-2 mt-2">
                          <Wrench className="h-4 w-4 text-muted-foreground" />
                          <div className="flex flex-wrap gap-1">
                            {selectedSession.distinct_tools.slice(0, 5).map((tool) => (
                              <span
                                key={tool}
                                className="px-2 py-0.5 text-xs bg-muted text-muted-foreground rounded border border-[var(--border-subtle)]"
                              >
                                {tool}
                              </span>
                            ))}
                            {selectedSession.distinct_tools.length > 5 && (
                              <span className="px-2 py-0.5 text-xs bg-muted text-muted-foreground rounded border border-[var(--border-subtle)]">
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
                        className="px-4 py-2 bg-accent text-background rounded-lg font-medium hover:bg-accent/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                      >
                        Load Session
                      </button>
                    )}
                  </div>
                </div>

                {/* Messages Preview */}
                <div className="flex-1 min-h-0 overflow-hidden">
                  {isLoadingDetail ? (
                    <div className="p-4 text-center text-muted-foreground py-8">
                      Loading messages...
                    </div>
                  ) : sessionDetail ? (
                    <VirtualizedMessagesList
                      messages={sessionDetail.messages}
                      truncateAtWord={truncateAtWord}
                    />
                  ) : (
                    <div className="p-4 text-center text-muted-foreground py-8">
                      Failed to load session details
                    </div>
                  )}
                </div>
              </>
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground">
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
