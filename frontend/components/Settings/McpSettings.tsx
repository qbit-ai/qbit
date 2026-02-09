import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-shell";
import {
  AlertCircle,
  Check,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  Loader2,
  Plug,
  PlugZap,
  RefreshCw,
  Server,
  Wrench,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent } from "@/components/ui/collapsible";
import { logger } from "@/lib/logger";
import * as mcp from "@/lib/mcp";
import { notify } from "@/lib/notify";

interface McpSettingsProps {
  workspacePath?: string;
}

export function McpSettings({ workspacePath }: McpSettingsProps) {
  const [servers, setServers] = useState<mcp.McpServerInfo[]>([]);
  const [tools, setTools] = useState<mcp.McpToolInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [expandedServers, setExpandedServers] = useState<Set<string>>(new Set());
  const [connectingServers, setConnectingServers] = useState<Set<string>>(new Set());
  const [disconnectingServers, setDisconnectingServers] = useState<Set<string>>(new Set());

  // Load servers and tools
  const loadData = useCallback(async () => {
    setIsLoading(true);
    try {
      const serverList = await mcp.listServers(workspacePath);
      setServers(serverList);

      try {
        const toolList = await mcp.listTools();
        setTools(toolList);
      } catch {
        // MCP manager not yet initialized - that's OK
        setTools([]);
      }
    } catch (err) {
      logger.error("Failed to load MCP servers:", err);
      notify.error("Failed to load MCP servers");
    } finally {
      setIsLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Listen for MCP background initialization events and auto-refresh
  useEffect(() => {
    const unlisten = listen<mcp.McpEvent>("mcp-event", (event) => {
      const payload = event.payload;
      if (payload.type === "ready") {
        // MCP servers finished connecting - refresh the UI
        loadData();
      } else if (payload.type === "error") {
        logger.error("[mcp-event] Error:", payload.message);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadData]);

  // Connect to a server
  const handleConnect = useCallback(
    async (serverName: string) => {
      setConnectingServers((prev) => new Set(prev).add(serverName));
      try {
        await mcp.connect(serverName);
        notify.success(`Connected to ${serverName}`);
        await loadData();
      } catch (err) {
        logger.error(`Failed to connect to ${serverName}:`, err);
        notify.error(err instanceof Error ? err.message : `Failed to connect to ${serverName}`);
      } finally {
        setConnectingServers((prev) => {
          const next = new Set(prev);
          next.delete(serverName);
          return next;
        });
      }
    },
    [loadData]
  );

  // Disconnect from a server
  const handleDisconnect = useCallback(
    async (serverName: string) => {
      setDisconnectingServers((prev) => new Set(prev).add(serverName));
      try {
        await mcp.disconnect(serverName);
        notify.success(`Disconnected from ${serverName}`);
        await loadData();
      } catch (err) {
        logger.error(`Failed to disconnect from ${serverName}:`, err);
        notify.error(
          err instanceof Error ? err.message : `Failed to disconnect from ${serverName}`
        );
      } finally {
        setDisconnectingServers((prev) => {
          const next = new Set(prev);
          next.delete(serverName);
          return next;
        });
      }
    },
    [loadData]
  );

  // Toggle server expansion
  const toggleExpanded = useCallback((serverName: string) => {
    setExpandedServers((prev) => {
      const next = new Set(prev);
      if (next.has(serverName)) {
        next.delete(serverName);
      } else {
        next.add(serverName);
      }
      return next;
    });
  }, []);

  // Get tools for a specific server
  const getToolsForServer = useCallback(
    (serverName: string) => {
      return tools.filter((t) => t.serverName === serverName);
    },
    [tools]
  );

  // Render status indicator
  const renderStatus = (status: mcp.McpServerStatus, error?: string | null) => {
    switch (status) {
      case "connected":
        return (
          <div className="flex items-center gap-1.5">
            <Check className="w-3.5 h-3.5 text-green-500" />
            <span className="text-xs text-green-600">Connected</span>
          </div>
        );
      case "connecting":
        return (
          <div className="flex items-center gap-1.5">
            <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin" />
            <span className="text-xs text-blue-600">Connecting...</span>
          </div>
        );
      case "error":
        return (
          <div className="flex items-center gap-1.5" title={error || "Connection error"}>
            <AlertCircle className="w-3.5 h-3.5 text-red-500" />
            <span className="text-xs text-red-600 truncate max-w-[150px]">{error || "Error"}</span>
          </div>
        );
      default:
        return (
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-muted-foreground/50" />
            <span className="text-xs text-muted-foreground">Disconnected</span>
          </div>
        );
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="w-6 h-6 text-muted-foreground animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h3 className="text-sm font-medium text-foreground">MCP Servers</h3>
          <p className="text-xs text-muted-foreground">
            Model Context Protocol servers provide additional tools to the AI agent
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={loadData} title="Refresh">
            <RefreshCw className="w-4 h-4" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => open("https://modelcontextprotocol.io/servers")}
          >
            <ExternalLink className="w-4 h-4 mr-2" />
            Browse servers
          </Button>
        </div>
      </div>

      {/* Config location info */}
      <div className="text-xs text-muted-foreground bg-[var(--bg-secondary)] rounded-md px-3 py-2 border border-[var(--border-subtle)]">
        <p>
          Configure servers in <code className="text-accent">~/.qbit/mcp.json</code> (global) or{" "}
          <code className="text-accent">&lt;project&gt;/.qbit/mcp.json</code> (project-specific).
        </p>
      </div>

      {/* Server list */}
      {servers.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground text-sm">
          <Server className="w-8 h-8 mx-auto mb-3 opacity-50" />
          <p>No MCP servers configured.</p>
          <p className="mt-1 text-xs">
            Create <code>~/.qbit/mcp.json</code> to add servers.
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {servers.map((server) => {
            const isConnecting = connectingServers.has(server.name);
            const isDisconnecting = disconnectingServers.has(server.name);
            const isExpanded = expandedServers.has(server.name);
            const serverTools = getToolsForServer(server.name);
            const isConnected = server.status === "connected";
            const isDisabled = !server.enabled;

            return (
              <div
                key={server.name}
                className={`rounded-lg border bg-[var(--bg-secondary)] ${
                  isDisabled
                    ? "border-[var(--border-subtle)] opacity-60"
                    : "border-[var(--border-medium)]"
                }`}
              >
                {/* Server header */}
                <div className="flex items-center justify-between px-4 py-3">
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    {/* Expand/collapse button (only if connected with tools) */}
                    {isConnected && serverTools.length > 0 ? (
                      <button
                        type="button"
                        onClick={() => toggleExpanded(server.name)}
                        className="p-0.5 hover:bg-[var(--bg-hover)] rounded"
                      >
                        {isExpanded ? (
                          <ChevronDown className="w-4 h-4 text-muted-foreground" />
                        ) : (
                          <ChevronRight className="w-4 h-4 text-muted-foreground" />
                        )}
                      </button>
                    ) : (
                      <div className="w-5" />
                    )}

                    {/* Server info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-foreground truncate">
                          {server.name}
                        </span>
                        <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                          {server.transport}
                        </Badge>
                        {server.source === "project" && (
                          <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                            project
                          </Badge>
                        )}
                        {isDisabled && (
                          <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                            disabled
                          </Badge>
                        )}
                      </div>
                      <div className="mt-1 flex items-center gap-3">
                        {renderStatus(server.status, server.error)}
                        {isConnected && serverTools.length > 0 && (
                          <span className="text-xs text-muted-foreground">
                            {serverTools.length} tool{serverTools.length !== 1 ? "s" : ""}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-2">
                    {isConnected ? (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDisconnect(server.name)}
                        disabled={isDisconnecting}
                        className="text-muted-foreground hover:text-foreground"
                      >
                        {isDisconnecting ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          <Plug className="w-4 h-4" />
                        )}
                        <span className="ml-2">Disconnect</span>
                      </Button>
                    ) : (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleConnect(server.name)}
                        disabled={isConnecting || isDisabled}
                      >
                        {isConnecting ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          <PlugZap className="w-4 h-4" />
                        )}
                        <span className="ml-2">Connect</span>
                      </Button>
                    )}
                  </div>
                </div>

                {/* Expanded tools list */}
                {isConnected && serverTools.length > 0 && (
                  <Collapsible open={isExpanded}>
                    <CollapsibleContent>
                      <div className="border-t border-[var(--border-subtle)] px-4 py-3 bg-[var(--bg-primary)]">
                        <div className="text-xs font-medium text-muted-foreground mb-2 flex items-center gap-1.5">
                          <Wrench className="w-3.5 h-3.5" />
                          Available Tools
                        </div>
                        <div className="space-y-1.5">
                          {serverTools.map((tool) => (
                            <div
                              key={tool.name}
                              className="text-xs py-1.5 px-2 rounded bg-[var(--bg-secondary)] border border-[var(--border-subtle)]"
                            >
                              <div className="font-mono text-foreground">{tool.toolName}</div>
                              {tool.description && (
                                <div className="text-muted-foreground mt-0.5 line-clamp-2">
                                  {tool.description}
                                </div>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    </CollapsibleContent>
                  </Collapsible>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
