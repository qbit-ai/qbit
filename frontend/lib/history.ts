import { invoke } from "@tauri-apps/api/core";

export interface CommandHistoryEntry {
  t: "cmd";
  v: number;
  ts: number;
  sid: string;
  c: string;
  exit: number;
  count: number;
}

export interface PromptHistoryEntry {
  t: "prompt";
  v: number;
  ts: number;
  sid: string;
  c: string;
  model: string;
  provider: string;
  tok_in: number;
  tok_out: number;
  ok: boolean;
  count: number;
}

export type HistoryEntry = CommandHistoryEntry | PromptHistoryEntry;

export async function addCommandHistory(
  sessionId: string,
  command: string,
  exitCode: number
): Promise<void> {
  return invoke("add_command_history", {
    sessionId,
    command,
    exitCode,
  });
}

export async function addPromptHistory(
  sessionId: string,
  prompt: string,
  model: string,
  provider: string,
  tokensIn: number,
  tokensOut: number,
  success: boolean
): Promise<void> {
  return invoke("add_prompt_history", {
    sessionId,
    prompt,
    model,
    provider,
    tokensIn,
    tokensOut,
    success,
  });
}

export async function loadHistory(
  limit: number = 500,
  entryType?: "cmd" | "prompt"
): Promise<HistoryEntry[]> {
  return invoke("load_history", { limit, entryType });
}

export async function searchHistory(
  query: string,
  includeArchives: boolean = false,
  limit: number = 200,
  entryType?: "cmd" | "prompt"
): Promise<HistoryEntry[]> {
  return invoke("search_history", { query, includeArchives, limit, entryType });
}

export async function clearHistory(): Promise<void> {
  return invoke("clear_history");
}
