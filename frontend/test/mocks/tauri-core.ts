import { vi } from "vitest";

// Mock invoke function for Tauri commands
export const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
  switch (command) {
    case "get_git_branch":
      // Return mock branch name for testing
      return "main";
    case "git_status":
      // Return mock git status for testing
      return {
        branch: "main",
        ahead: 0,
        behind: 0,
        entries: [],
        insertions: 0,
        deletions: 0,
      };
    case "is_ai_session_initialized":
      return false;
    case "update_ai_workspace":
      return undefined;
    case "get_settings":
      return {
        terminal: { fullterm_commands: [] },
        ai: {
          openrouter: { api_key: null, show_in_selector: false },
          openai: { api_key: null, show_in_selector: false },
          anthropic: { api_key: null, show_in_selector: false },
          ollama: { show_in_selector: false },
          gemini: { api_key: null, show_in_selector: false },
          groq: { api_key: null, show_in_selector: false },
          xai: { api_key: null, show_in_selector: false },
          zai: { api_key: null, use_coding_endpoint: true, show_in_selector: false },
          vertex_ai: {
            credentials_path: null,
            project_id: null,
            location: null,
            show_in_selector: false,
          },
        },
      };
    default:
      console.warn(`[mock] Unhandled invoke command: ${command}`, args);
      return undefined;
  }
});

// Export other core functions as needed
export const convertFileSrc = vi.fn((path: string) => `asset://localhost/${path}`);
