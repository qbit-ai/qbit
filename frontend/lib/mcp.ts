/**
 * MCP (Model Context Protocol) API for Qbit.
 *
 * This module provides typed wrappers for MCP server management commands,
 * enabling the frontend to list servers, view tools, and manage connections.
 *
 * The MCP manager is global (shared across all sessions) and initialized
 * in the background during app startup.
 */

import { invoke } from "@tauri-apps/api/core";

// =============================================================================
// Type Definitions
// =============================================================================

/**
 * Server connection status.
 */
export type McpServerStatus = "connected" | "disconnected" | "connecting" | "error";

/**
 * Information about a configured MCP server.
 */
export interface McpServerInfo {
  /** Server name (from config key) */
  name: string;
  /** Transport type (stdio, http, sse) */
  transport: string;
  /** Whether the server is enabled in config */
  enabled: boolean;
  /** Connection status */
  status: McpServerStatus;
  /** Number of tools available (if connected) */
  toolCount: number | null;
  /** Error message (if status is "error") */
  error: string | null;
  /** Source: "user" for ~/.qbit/mcp.json, "project" for <project>/.qbit/mcp.json */
  source: "user" | "project";
}

/**
 * Information about an MCP tool.
 */
export interface McpToolInfo {
  /** Full tool name (mcp__{server}__{tool}) */
  name: string;
  /** Server this tool belongs to */
  serverName: string;
  /** Original tool name from the server */
  toolName: string;
  /** Tool description */
  description: string | null;
}

/**
 * MCP server configuration (matches Rust McpServerConfig).
 */
export interface McpServerConfig {
  /** Transport type (default: stdio) */
  transport?: "stdio" | "http" | "sse";
  /** Command for stdio transport */
  command?: string;
  /** Arguments for the command */
  args?: string[];
  /** Environment variables */
  env?: Record<string, string>;
  /** URL for HTTP/SSE transport */
  url?: string;
  /** HTTP headers for remote servers */
  headers?: Record<string, string>;
  /** Whether this server is enabled */
  enabled?: boolean;
}

/**
 * MCP background initialization event payload.
 */
export interface McpEvent {
  /** Event type: "initializing", "ready", or "error" */
  type: "initializing" | "ready" | "error";
  /** Human-readable message */
  message: string;
  /** Number of configured servers (on "ready") */
  serverCount?: number;
  /** Number of available tools (on "ready") */
  toolCount?: number;
}

// =============================================================================
// Server Management
// =============================================================================

/**
 * List all configured MCP servers with their status.
 *
 * Returns servers from both user-global (~/.qbit/mcp.json) and
 * project-specific (<project>/.qbit/mcp.json) configurations.
 * Live connection status is reported from the global MCP manager.
 *
 * @param workspacePath - Optional workspace path (defaults to current directory)
 */
export async function listServers(workspacePath?: string): Promise<McpServerInfo[]> {
  return invoke<McpServerInfo[]>("mcp_list_servers", { workspacePath });
}

/**
 * Connect to an MCP server.
 *
 * After connecting, all active agent sessions have their MCP tools refreshed.
 *
 * @param serverName - The server name from config
 */
export async function connect(serverName: string): Promise<void> {
  return invoke("mcp_connect", { serverName });
}

/**
 * Disconnect from an MCP server.
 *
 * After disconnecting, all active agent sessions have their MCP tools refreshed.
 *
 * @param serverName - The server name from config
 */
export async function disconnect(serverName: string): Promise<void> {
  return invoke("mcp_disconnect", { serverName });
}

// =============================================================================
// Tool Management
// =============================================================================

/**
 * List all tools from connected MCP servers.
 *
 * Retrieves tools from the global MCP manager.
 */
export async function listTools(): Promise<McpToolInfo[]> {
  return invoke<McpToolInfo[]>("mcp_list_tools");
}

// =============================================================================
// Configuration
// =============================================================================

/**
 * Get MCP configuration for a workspace.
 *
 * Returns the merged configuration from user-global and project-specific sources.
 *
 * @param workspacePath - Path to the workspace
 */
export async function getConfig(workspacePath: string): Promise<Record<string, McpServerConfig>> {
  return invoke<Record<string, McpServerConfig>>("mcp_get_config", { workspacePath });
}

/**
 * Check if a project has an MCP configuration file.
 *
 * @param workspacePath - Path to the workspace
 */
export async function hasProjectConfig(workspacePath: string): Promise<boolean> {
  return invoke<boolean>("mcp_has_project_config", { workspacePath });
}

// =============================================================================
// Trust Management
// =============================================================================

/**
 * Check if a project's MCP configuration is trusted.
 *
 * Project configs must be explicitly trusted before their servers are connected.
 *
 * @param projectPath - Path to the project
 */
export async function isProjectTrusted(projectPath: string): Promise<boolean> {
  return invoke<boolean>("mcp_is_project_trusted", { projectPath });
}

/**
 * Mark a project's MCP configuration as trusted.
 *
 * This should be called after the user explicitly approves a project's
 * MCP configuration in the UI.
 *
 * @param projectPath - Path to the project
 */
export async function trustProjectConfig(projectPath: string): Promise<void> {
  return invoke("mcp_trust_project_config", { projectPath });
}
