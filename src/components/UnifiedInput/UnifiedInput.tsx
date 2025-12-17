import { SendHorizontal } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { FileCommandPopup } from "@/components/FileCommandPopup";
import { filterPrompts, SlashCommandPopup } from "@/components/SlashCommandPopup";
import { useCommandHistory } from "@/hooks/useCommandHistory";
import { useFileCommands } from "@/hooks/useFileCommands";
import { useSlashCommands } from "@/hooks/useSlashCommands";
import { sendPromptSession } from "@/lib/ai";
import { notify } from "@/lib/notify";
import { type FileInfo, type PromptInfo, ptyWrite, readPrompt } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { useInputMode, useStore, useStreamingBlocks } from "@/store";

interface UnifiedInputProps {
  sessionId: string;
  workingDirectory?: string;
}

// Commands that require full terminal (interactive programs)
const INTERACTIVE_COMMANDS = [
  "vim",
  "vi",
  "nvim",
  "nano",
  "emacs",
  "pico",
  "less",
  "more",
  "man",
  "htop",
  "top",
  "btop",
  "ssh",
  "telnet",
  "ftp",
  "sftp",
  "python",
  "python3",
  "node",
  "irb",
  "ruby",
  "ghci",
  "mysql",
  "psql",
  "sqlite3",
  "redis-cli",
  "mongo",
  "tmux",
  "screen",
  "watch",
];

export function UnifiedInput({ sessionId, workingDirectory }: UnifiedInputProps) {
  const [input, setInput] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showSlashPopup, setShowSlashPopup] = useState(false);
  const [slashSelectedIndex, setSlashSelectedIndex] = useState(0);
  const [showFilePopup, setShowFilePopup] = useState(false);
  const [fileSelectedIndex, setFileSelectedIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Command history for up/down navigation
  const { add: addToHistory, navigateUp, navigateDown, reset: resetHistory } = useCommandHistory();

  // Slash commands
  const { prompts } = useSlashCommands(workingDirectory);
  const slashQuery = input.startsWith("/") ? input.slice(1) : "";
  const filteredSlashPrompts = filterPrompts(prompts, slashQuery);

  // File commands (@ trigger)
  // Detect @ at end of input (e.g., "Look at @But" -> query is "But")
  const atMatch = input.match(/@([^\s@]*)$/);
  const fileQuery = atMatch?.[1] ?? "";
  const { files } = useFileCommands(workingDirectory, fileQuery);

  // Use inputMode for unified input toggle (not session mode)
  const inputMode = useInputMode(sessionId);
  const setInputMode = useStore((state) => state.setInputMode);
  const streamingBlocks = useStreamingBlocks(sessionId);
  const addAgentMessage = useStore((state) => state.addAgentMessage);
  const agentMessages = useStore((state) => state.agentMessages[sessionId] ?? []);

  const isAgentBusy = inputMode === "agent" && (isSubmitting || streamingBlocks.length > 0);

  // Auto-resize textarea
  const adjustTextareaHeight = useCallback(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
    }
  }, []);

  // Reset isSubmitting when AI response completes
  const prevMessagesLengthRef = useRef(agentMessages.length);
  useEffect(() => {
    // If a new message was added and we were submitting, check if it's from assistant/system
    if (agentMessages.length > prevMessagesLengthRef.current && isSubmitting) {
      const lastMessage = agentMessages[agentMessages.length - 1];
      // Reset if assistant or system (error) responded
      if (lastMessage && (lastMessage.role === "assistant" || lastMessage.role === "system")) {
        setIsSubmitting(false);
      }
    }
    prevMessagesLengthRef.current = agentMessages.length;
  }, [agentMessages, isSubmitting]);

  // Auto-focus input when session or mode changes.
  // Defer to the next frame so it isn't immediately overridden by focus management
  // (e.g., Radix Tabs focusing the clicked tab trigger).
  useEffect(() => {
    void sessionId;
    void inputMode;
    const handle = requestAnimationFrame(() => {
      textareaRef.current?.focus();
    });

    return () => cancelAnimationFrame(handle);
  }, [sessionId, inputMode]);

  // Adjust height when input changes
  // biome-ignore lint/correctness/useExhaustiveDependencies: input triggers re-measurement of textarea scrollHeight
  useEffect(() => {
    adjustTextareaHeight();
  }, [input, adjustTextareaHeight]);

  // Toggle input mode
  const toggleInputMode = useCallback(() => {
    setInputMode(sessionId, inputMode === "terminal" ? "agent" : "terminal");
  }, [sessionId, inputMode, setInputMode]);

  // Check if command is interactive and needs full terminal
  const isInteractiveCommand = useCallback((cmd: string) => {
    const firstWord = cmd.trim().split(/\s+/)[0];
    return INTERACTIVE_COMMANDS.includes(firstWord);
  }, []);

  const handleSubmit = useCallback(async () => {
    if (!input.trim() || isAgentBusy) return;

    const value = input.trim();
    setInput("");
    resetHistory();

    if (inputMode === "terminal") {
      // Terminal mode: send to PTY
      // Block interactive commands for now
      if (isInteractiveCommand(value)) {
        const cmd = value.split(/\s+/)[0];
        notify.error(`Interactive command "${cmd}" is not supported yet`);
        return;
      }

      // Add to history
      addToHistory(value);

      // Send command + newline to PTY
      await ptyWrite(sessionId, `${value}\n`);
    } else {
      // Agent mode: send to AI
      setIsSubmitting(true);

      // Add to history
      addToHistory(value);

      // Add user message to store
      addAgentMessage(sessionId, {
        id: crypto.randomUUID(),
        sessionId,
        role: "user",
        content: value,
        timestamp: new Date().toISOString(),
      });

      // Send to AI backend - response will come via useAiEvents hook
      try {
        // Pass working directory and session context so the agent knows where the user is working
        // and can execute commands in the same terminal
        await sendPromptSession(sessionId, value, { workingDirectory });
        // Response will be handled by useAiEvents when AI completes
        // Don't set isSubmitting to false here - wait for completed/error event
      } catch (error) {
        notify.error(`Agent error: ${error}`);
        setIsSubmitting(false);
      }
    }
  }, [
    input,
    inputMode,
    sessionId,
    isAgentBusy,
    addAgentMessage,
    isInteractiveCommand,
    workingDirectory,
    addToHistory,
    resetHistory,
  ]);

  // Handle slash command selection
  const handleSlashSelect = useCallback(
    async (prompt: PromptInfo) => {
      setShowSlashPopup(false);
      setInput("");

      // Switch to agent mode if in terminal mode
      if (inputMode === "terminal") {
        setInputMode(sessionId, "agent");
      }

      // Read and send the prompt
      try {
        const content = await readPrompt(prompt.path);
        setIsSubmitting(true);

        // Add user message to store (show the slash command name)
        addAgentMessage(sessionId, {
          id: crypto.randomUUID(),
          sessionId,
          role: "user",
          content: `/${prompt.name}`,
          timestamp: new Date().toISOString(),
        });

        // Send the actual prompt content to AI
        await sendPromptSession(sessionId, content, { workingDirectory });
      } catch (error) {
        notify.error(`Failed to run prompt: ${error}`);
        setIsSubmitting(false);
      }
    },
    [sessionId, inputMode, setInputMode, addAgentMessage, workingDirectory]
  );

  // Handle file selection from @ popup
  const handleFileSelect = useCallback(
    (file: FileInfo) => {
      setShowFilePopup(false);
      // Replace @query with the selected file's relative path
      const newInput = input.replace(/@[^\s@]*$/, file.relative_path);
      setInput(newInput);
      setFileSelectedIndex(0);
    },
    [input]
  );

  const handleKeyDown = useCallback(
    async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Cmd+I to toggle input mode - handle first to ensure it works in all modes
      // Check both lowercase 'i' and the key code for reliability across platforms
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && (e.key === "i" || e.key === "I")) {
        e.preventDefault();
        e.stopPropagation();
        toggleInputMode();
        return;
      }

      // When slash popup is open, handle navigation
      if (showSlashPopup && filteredSlashPrompts.length > 0) {
        if (e.key === "Escape") {
          e.preventDefault();
          setShowSlashPopup(false);
          return;
        }

        // Arrow down - move selection down
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setSlashSelectedIndex((prev) =>
            prev < filteredSlashPrompts.length - 1 ? prev + 1 : prev
          );
          return;
        }

        // Arrow up - move selection up
        if (e.key === "ArrowUp") {
          e.preventDefault();
          setSlashSelectedIndex((prev) => (prev > 0 ? prev - 1 : 0));
          return;
        }

        // Tab - complete the selected option into the input field
        if (e.key === "Tab") {
          e.preventDefault();
          const selectedPrompt = filteredSlashPrompts[slashSelectedIndex];
          if (selectedPrompt) {
            setInput(`/${selectedPrompt.name}`);
            setShowSlashPopup(false);
          }
          return;
        }

        // Enter - execute the selected option
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          const selectedPrompt = filteredSlashPrompts[slashSelectedIndex];
          if (selectedPrompt) {
            handleSlashSelect(selectedPrompt);
          }
          return;
        }
      }

      // When file popup is open, handle navigation
      if (showFilePopup && files.length > 0) {
        if (e.key === "Escape") {
          e.preventDefault();
          setShowFilePopup(false);
          return;
        }

        // Arrow down - move selection down
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setFileSelectedIndex((prev) => (prev < files.length - 1 ? prev + 1 : prev));
          return;
        }

        // Arrow up - move selection up
        if (e.key === "ArrowUp") {
          e.preventDefault();
          setFileSelectedIndex((prev) => (prev > 0 ? prev - 1 : 0));
          return;
        }

        // Tab - complete the selected file
        if (e.key === "Tab") {
          e.preventDefault();
          const selectedFile = files[fileSelectedIndex];
          if (selectedFile) {
            handleFileSelect(selectedFile);
          }
          return;
        }

        // Enter - insert the selected file
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          const selectedFile = files[fileSelectedIndex];
          if (selectedFile) {
            handleFileSelect(selectedFile);
          }
          return;
        }
      }

      // Cmd+Shift+T to toggle input mode
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "t") {
        e.preventDefault();
        toggleInputMode();
        return;
      }

      // Handle Enter - execute/send (Shift+Enter for newline)
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        await handleSubmit();
        return;
      }

      // History navigation - shared between terminal and agent modes
      if (e.key === "ArrowUp") {
        e.preventDefault();
        const cmd = navigateUp();
        if (cmd !== null) {
          setInput(cmd);
        }
        return;
      }

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setInput(navigateDown());
        return;
      }

      // Terminal-specific shortcuts
      if (inputMode === "terminal") {
        // Handle Tab - send to PTY for completion
        if (e.key === "Tab") {
          e.preventDefault();
          await ptyWrite(sessionId, "\t");
          return;
        }

        // Handle Ctrl+C - send interrupt
        if (e.ctrlKey && e.key === "c") {
          e.preventDefault();
          await ptyWrite(sessionId, "\x03");
          setInput("");
          return;
        }

        // Handle Ctrl+D - send EOF
        if (e.ctrlKey && e.key === "d") {
          e.preventDefault();
          await ptyWrite(sessionId, "\x04");
          return;
        }

        // Handle Ctrl+L - clear
        if (e.ctrlKey && e.key === "l") {
          e.preventDefault();
          await ptyWrite(sessionId, "\x0c");
          return;
        }
      }
    },
    [
      inputMode,
      sessionId,
      handleSubmit,
      navigateUp,
      navigateDown,
      toggleInputMode,
      showSlashPopup,
      filteredSlashPrompts,
      slashSelectedIndex,
      handleSlashSelect,
      showFilePopup,
      files,
      fileSelectedIndex,
      handleFileSelect,
    ]
  );

  const displayPath = workingDirectory?.replace(/^\/Users\/[^/]+/, "~") || "~";

  return (
    <div className="border-t border-[var(--border-subtle)]">
      {/* Working directory */}
      <div className="text-[11px] font-mono text-muted-foreground truncate px-4 py-1.5">
        {displayPath}
      </div>

      {/* Input row with container */}
      <div className="px-3 pb-2">
        <div
          className={cn(
            "flex items-end gap-2 rounded-lg border border-[var(--border-medium)] bg-card px-3 py-2",
            "focus-within:border-accent focus-within:shadow-[0_0_0_3px_var(--accent-glow)]",
            "transition-all duration-150"
          )}
        >
          <SlashCommandPopup
            open={showSlashPopup}
            onOpenChange={setShowSlashPopup}
            prompts={filteredSlashPrompts}
            selectedIndex={slashSelectedIndex}
            onSelect={handleSlashSelect}
          >
            <FileCommandPopup
              open={showFilePopup}
              onOpenChange={setShowFilePopup}
              files={files}
              selectedIndex={fileSelectedIndex}
              onSelect={handleFileSelect}
            >
              <textarea
                ref={textareaRef}
                value={input}
                onChange={(e) => {
                  const value = e.target.value;
                  setInput(value);
                  resetHistory();

                  // Show slash popup when "/" is typed at the start
                  if (value.startsWith("/") && value.length >= 1) {
                    setShowSlashPopup(true);
                    setSlashSelectedIndex(0);
                    setShowFilePopup(false);
                  } else {
                    setShowSlashPopup(false);
                  }

                  // Show file popup when "@" is typed (agent mode only)
                  if (inputMode === "agent" && /@[^\s@]*$/.test(value)) {
                    setShowFilePopup(true);
                    setFileSelectedIndex(0);
                  } else {
                    setShowFilePopup(false);
                  }
                }}
                onKeyDown={handleKeyDown}
                disabled={isAgentBusy}
                placeholder={inputMode === "terminal" ? "Enter command..." : "Ask the AI..."}
                rows={1}
                className={cn(
                  "flex-1 min-h-[24px] max-h-[200px] py-0",
                  "bg-transparent border-none shadow-none resize-none",
                  "font-mono text-[13px] text-foreground leading-relaxed",
                  "focus:outline-none focus:ring-0",
                  "disabled:opacity-50",
                  "placeholder:text-muted-foreground"
                )}
                spellCheck={false}
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
              />
            </FileCommandPopup>
          </SlashCommandPopup>

          {/* Send button */}
          <button
            type="button"
            onClick={handleSubmit}
            disabled={!input.trim() || isAgentBusy}
            className={cn(
              "h-7 w-7 flex items-center justify-center rounded-md shrink-0",
              "transition-all duration-150",
              input.trim() && !isAgentBusy
                ? "bg-accent text-accent-foreground hover:bg-accent/90"
                : "bg-muted text-muted-foreground cursor-not-allowed"
            )}
          >
            <SendHorizontal className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}
