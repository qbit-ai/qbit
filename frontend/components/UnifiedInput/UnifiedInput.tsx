import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ArrowDown, ArrowUp, Folder, GitBranch, Package, SendHorizontal } from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { FileCommandPopup } from "@/components/FileCommandPopup";
import { HistorySearchPopup } from "@/components/HistorySearchPopup";
import { InlineTaskPlan } from "@/components/InlineTaskPlan";
import { PathCompletionPopup } from "@/components/PathCompletionPopup";
import { filterCommands, SlashCommandPopup } from "@/components/SlashCommandPopup";
import { useCommandHistory } from "@/hooks/useCommandHistory";
import { useFileCommands } from "@/hooks/useFileCommands";
import { type HistoryMatch, useHistorySearch } from "@/hooks/useHistorySearch";
import { usePathCompletion } from "@/hooks/usePathCompletion";
import { type SlashCommand, useSlashCommands } from "@/hooks/useSlashCommands";
import {
  getVisionCapabilities,
  type ImagePart,
  sendPromptSession,
  sendPromptWithAttachments,
  type VisionCapabilities,
} from "@/lib/ai";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import {
  type FileInfo,
  type PathCompletion,
  ptyWrite,
  readFileAsBase64 as readFileAsBase64FromPath,
  readPrompt,
  readSkillBody,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { useAgentMessages, useSessionAiConfig, useStore } from "@/store";
import { useUnifiedInputState } from "@/store/selectors/unified-input";
import { ImageAttachment, readFileAsBase64 } from "./ImageAttachment";
import { InputStatusRow } from "./InputStatusRow";

const clearTerminal = (sessionId: string) => {
  const store = useStore.getState();
  store.clearBlocks(sessionId);
  store.clearTimeline(sessionId);
};

interface UnifiedInputProps {
  sessionId: string;
  workingDirectory?: string;
  onOpenGitPanel?: () => void;
}

// Extract word at cursor for tab completion
function extractWordAtCursor(
  input: string,
  cursorPos: number
): { word: string; startIndex: number } {
  const beforeCursor = input.slice(0, cursorPos);
  const match = beforeCursor.match(/[^\s|;&]+$/);
  if (!match) return { word: "", startIndex: cursorPos };
  return {
    word: match[0],
    startIndex: cursorPos - match[0].length,
  };
}

// Check if cursor is on the first line of textarea content
function isCursorOnFirstLine(text: string, cursorPos: number): boolean {
  const textBeforeCursor = text.substring(0, cursorPos);
  return !textBeforeCursor.includes("\n");
}

// Check if cursor is on the last line of textarea content
function isCursorOnLastLine(text: string, cursorPos: number): boolean {
  const textAfterCursor = text.substring(cursorPos);
  return !textAfterCursor.includes("\n");
}

// Static style constant for ghost text hint (top position is always 0)
const ghostTextBaseStyle = { top: 0 } as const;

// Memoized component for ghost text hint to avoid inline style object recreation
const GhostTextHint = memo(function GhostTextHint({
  text,
  inputLength,
}: {
  text: string;
  inputLength: number;
}) {
  // Memoize style with dynamic left position
  const style = useMemo(
    () => ({
      ...ghostTextBaseStyle,
      // Position at end of current input text using ch unit for monospace character width
      left: `${inputLength}ch`,
    }),
    [inputLength]
  );

  return (
    <span
      className="absolute pointer-events-none font-mono text-[13px] text-muted-foreground/50 leading-relaxed whitespace-pre"
      style={style}
      aria-hidden="true"
    >
      {text}
    </span>
  );
});

export function UnifiedInput({ sessionId, workingDirectory, onOpenGitPanel }: UnifiedInputProps) {
  const [input, setInput] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showSlashPopup, setShowSlashPopup] = useState(false);
  const [slashSelectedIndex, setSlashSelectedIndex] = useState(0);
  const [showFilePopup, setShowFilePopup] = useState(false);
  const [fileSelectedIndex, setFileSelectedIndex] = useState(0);
  const [showPathPopup, setShowPathPopup] = useState(false);
  const [pathSelectedIndex, setPathSelectedIndex] = useState(0);
  const [pathQuery, setPathQuery] = useState("");
  const [showHistorySearch, setShowHistorySearch] = useState(false);
  const [historySearchQuery, setHistorySearchQuery] = useState("");
  const [historySelectedIndex, setHistorySelectedIndex] = useState(0);
  const [originalInput, setOriginalInput] = useState("");
  const [imageAttachments, setImageAttachments] = useState<ImagePart[]>([]);
  const [visionCapabilities, setVisionCapabilities] = useState<VisionCapabilities | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const [dragError, setDragError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropZoneRef = useRef<HTMLDivElement>(null);
  const paneContainerRef = useRef<HTMLElement | null>(null);
  // Cached bounding rect for drop zone hit testing to avoid getBoundingClientRect() on every drag-over
  const dropZoneRectRef = useRef<DOMRect | null>(null);

  // Combined selector for optimized state access (reduces ~12 subscriptions to 1)
  const {
    inputMode,
    virtualEnv,
    isAgentResponding,
    isCompacting,
    isSessionDead,
    streamingBlocksLength,
    gitBranch,
    gitStatus,
  } = useUnifiedInputState(sessionId);

  // AI config for tracking provider changes (used to refresh vision capabilities)
  const aiConfig = useSessionAiConfig(sessionId);

  // Store actions (stable references, don't cause re-renders)
  const setInputMode = useStore((state) => state.setInputMode);
  const setLastSentCommand = useStore((state) => state.setLastSentCommand);
  const addAgentMessage = useStore((state) => state.addAgentMessage);
  const agentMessages = useAgentMessages(sessionId);

  // Command history for up/down navigation
  const {
    history,
    add: addToHistory,
    navigateUp,
    navigateDown,
    reset: resetHistory,
  } = useCommandHistory({
    entryType: inputMode === "terminal" ? "cmd" : "prompt",
  });

  // History search (Ctrl+R)
  const { matches: historyMatches } = useHistorySearch({
    history,
    query: historySearchQuery,
  });

  // Slash commands (prompts and skills)
  const { commands } = useSlashCommands(workingDirectory);
  // Split slash input into command name (for filtering) - args are extracted in handleKeyDown
  const slashInput = input.startsWith("/") ? input.slice(1) : "";
  const slashSpaceIndex = slashInput.indexOf(" ");
  const slashCommandName =
    slashSpaceIndex === -1 ? slashInput : slashInput.slice(0, slashSpaceIndex);
  const filteredSlashCommands = useMemo(
    () => filterCommands(commands, slashCommandName),
    [commands, slashCommandName]
  );

  // File commands (@ trigger)
  // Detect @ at end of input (e.g., "Look at @But" -> query is "But")
  const atMatch = input.match(/@([^\s@]*)$/);
  const fileQuery = atMatch?.[1] ?? "";
  const { files } = useFileCommands(workingDirectory, fileQuery);

  // Path completions (Tab in terminal mode)
  const { completions: pathCompletions, totalCount: pathTotalCount } = usePathCompletion({
    sessionId,
    partialPath: pathQuery,
    enabled: showPathPopup && inputMode === "terminal",
  });

  // Ghost text shows the remainder of the top completion as a hint
  const ghostText = useMemo(() => {
    if (!showPathPopup || pathCompletions.length === 0 || !pathQuery) {
      return "";
    }

    // Use the selected completion, or the top one if nothing selected
    const completion = pathCompletions[pathSelectedIndex] || pathCompletions[0];

    // Ghost shows the part that would be added after current input
    // Extract what the completion would add beyond the current path query
    const nameLower = completion.name.toLowerCase();
    const queryLower = pathQuery.toLowerCase();

    // If the name starts with the query (fuzzy match may not be exact prefix), show the rest
    if (nameLower.startsWith(queryLower)) {
      return completion.name.slice(pathQuery.length);
    }

    // For fuzzy matches, don't show ghost (would be confusing)
    return "";
  }, [showPathPopup, pathCompletions, pathQuery, pathSelectedIndex]);

  // Agent is busy when submitting, streaming content, actively responding, or compacting
  const isAgentBusy =
    inputMode === "agent" &&
    (isSubmitting || streamingBlocksLength > 0 || isAgentResponding || isCompacting);

  // Input is disabled when agent is busy OR session is dead
  const isInputDisabled = isAgentBusy || isSessionDead;

  // Ref to store current state values for stable callbacks
  // This pattern allows callbacks to always access current values without being recreated
  // Updated directly in render (not in useEffect) because:
  // 1. Ref assignments are synchronous and don't trigger re-renders
  // 2. The value is available immediately for callbacks called during render
  // 3. No wasted useEffect overhead
  //
  // OPTIMIZATION: We update individual properties rather than creating a new object
  // every render. This avoids allocating a new 20+ field object on each render.
  const stateRef = useRef({
    input: "",
    inputMode: "terminal" as "terminal" | "agent",
    isAgentBusy: false,
    isSubmitting: false,
    imageAttachments: [] as ImagePart[],
    visionCapabilities: null as VisionCapabilities | null,
    showSlashPopup: false,
    filteredSlashCommands: [] as SlashCommand[],
    slashSelectedIndex: 0,
    showFilePopup: false,
    files: [] as FileInfo[],
    fileSelectedIndex: 0,
    showPathPopup: false,
    pathCompletions: [] as PathCompletion[],
    pathSelectedIndex: 0,
    showHistorySearch: false,
    historySearchQuery: "",
    historyMatches: [] as HistoryMatch[],
    historySelectedIndex: 0,
    originalInput: "",
    commands: [] as SlashCommand[],
  });

  // Update ref properties directly in render (no object allocation)
  // This is more efficient than creating a new object with 20+ fields every render
  const ref = stateRef.current;
  ref.input = input;
  ref.inputMode = inputMode;
  ref.isAgentBusy = isAgentBusy;
  ref.isSubmitting = isSubmitting;
  ref.imageAttachments = imageAttachments;
  ref.visionCapabilities = visionCapabilities;
  ref.showSlashPopup = showSlashPopup;
  ref.filteredSlashCommands = filteredSlashCommands;
  ref.slashSelectedIndex = slashSelectedIndex;
  ref.showFilePopup = showFilePopup;
  ref.files = files;
  ref.fileSelectedIndex = fileSelectedIndex;
  ref.showPathPopup = showPathPopup;
  ref.pathCompletions = pathCompletions;
  ref.pathSelectedIndex = pathSelectedIndex;
  ref.showHistorySearch = showHistorySearch;
  ref.historySearchQuery = historySearchQuery;
  ref.historyMatches = historyMatches;
  ref.historySelectedIndex = historySelectedIndex;
  ref.originalInput = originalInput;
  ref.commands = commands;

  // Supported image MIME types for drag-and-drop and paste
  const SUPPORTED_IMAGE_TYPES = ["image/png", "image/jpeg", "image/jpg", "image/gif", "image/webp"];

  // Process image files into ImagePart format
  const processImageFiles = useCallback(
    async (files: FileList | File[]): Promise<ImagePart[]> => {
      const newAttachments: ImagePart[] = [];
      const fileArray = Array.from(files);

      for (const file of fileArray) {
        // Check if it's a supported image type
        if (!SUPPORTED_IMAGE_TYPES.includes(file.type)) {
          notify.warning(`Unsupported file type: ${file.type}`);
          continue;
        }

        // Check file size if we have vision capabilities
        if (visionCapabilities && file.size > visionCapabilities.max_image_size_bytes) {
          const maxMB = (visionCapabilities.max_image_size_bytes / 1024 / 1024).toFixed(0);
          const fileMB = (file.size / 1024 / 1024).toFixed(1);
          notify.warning(`Image too large: ${fileMB}MB (max ${maxMB}MB)`);
          continue;
        }

        try {
          const base64 = await readFileAsBase64(file);
          newAttachments.push({
            type: "image",
            data: base64,
            media_type: file.type,
            filename: file.name,
          });
        } catch (error) {
          notify.error("Failed to read file");
        }
      }

      return newAttachments;
    },
    [visionCapabilities]
  );

  // Cache for last known textarea height to avoid unnecessary reflows
  const lastTextareaHeightRef = useRef<number>(0);

  // Auto-resize textarea using requestAnimationFrame to batch DOM reads/writes
  // and avoid layout thrashing (write -> read -> write pattern)
  const adjustTextareaHeight = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    // Use rAF to batch the layout operation
    requestAnimationFrame(() => {
      // Read phase: reset to auto and measure
      textarea.style.height = "auto";
      const scrollHeight = textarea.scrollHeight;
      const newHeight = Math.min(scrollHeight, 200);

      // Only write if height actually changed
      if (newHeight !== lastTextareaHeightRef.current) {
        lastTextareaHeightRef.current = newHeight;
        textarea.style.height = `${newHeight}px`;
      } else {
        // Restore the height since we set it to "auto" for measurement
        textarea.style.height = `${newHeight}px`;
      }
    });
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

  // Reset submission state when switching sessions to prevent input lock across tabs
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentionally only reset on sessionId change
  useEffect(() => {
    setIsSubmitting(false);
    // Reset ref to 0 so the message length check works correctly for the new session
    prevMessagesLengthRef.current = 0;
    // Clear attachments when switching sessions
    setImageAttachments([]);
  }, [sessionId]);

  // Fetch vision capabilities when in agent mode or when provider changes
  // biome-ignore lint/correctness/useExhaustiveDependencies: aiConfig.provider triggers refetch when user switches providers
  useEffect(() => {
    if (inputMode === "agent") {
      getVisionCapabilities(sessionId)
        .then(setVisionCapabilities)
        .catch((err) => {
          logger.debug("Failed to get vision capabilities:", err);
          setVisionCapabilities(null);
        });
    }
  }, [sessionId, inputMode, aiConfig?.provider]);

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

  // Find and cache the parent pane container element for drag-drop zone detection
  // Use requestAnimationFrame to ensure DOM is ready after render
  useEffect(() => {
    const findPaneContainer = () => {
      const paneContainer = document.querySelector<HTMLElement>(
        `[data-pane-drop-zone="${sessionId}"]`
      );
      paneContainerRef.current = paneContainer;
    };

    // Try immediately, then defer to next frame as fallback
    findPaneContainer();
    const handle = requestAnimationFrame(findPaneContainer);

    return () => cancelAnimationFrame(handle);
  }, [sessionId]);

  // Toggle input mode
  const toggleInputMode = useCallback(() => {
    setInputMode(sessionId, inputMode === "terminal" ? "agent" : "terminal");
  }, [sessionId, inputMode, setInputMode]);

  // Update the cached drop zone bounding rect (called on drag-enter and periodically)
  const updateDropZoneRect = useCallback(() => {
    let dropZone = paneContainerRef.current;
    if (!dropZone) {
      dropZone = document.querySelector<HTMLElement>(`[data-pane-drop-zone="${sessionId}"]`);
      if (dropZone) {
        paneContainerRef.current = dropZone;
      }
    }
    if (!dropZone) {
      dropZone = dropZoneRef.current;
    }
    if (dropZone) {
      dropZoneRectRef.current = dropZone.getBoundingClientRect();
    }
  }, [sessionId]);

  // Check if a position is within the pane container (entire pane is a drop zone)
  // Uses cached bounding rect to avoid layout thrashing on every drag-over event
  const isPositionOverDropZone = useCallback((x: number, y: number): boolean => {
    const rect = dropZoneRectRef.current;
    if (!rect) return false;
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  }, []);

  // Process file paths from Tauri drag-drop into ImagePart format
  const processFilePaths = useCallback(async (filePaths: string[]): Promise<ImagePart[]> => {
    const newAttachments: ImagePart[] = [];

    for (const filePath of filePaths) {
      // Check if it's an image by extension
      const ext = filePath.toLowerCase().split(".").pop();
      const imageExtensions = ["png", "jpg", "jpeg", "gif", "webp"];
      if (!ext || !imageExtensions.includes(ext)) {
        notify.warning(`Skipping non-image file: ${filePath}`);
        continue;
      }

      // Get MIME type from extension
      const mimeTypes: Record<string, string> = {
        png: "image/png",
        jpg: "image/jpeg",
        jpeg: "image/jpeg",
        gif: "image/gif",
        webp: "image/webp",
      };
      const mediaType = mimeTypes[ext] || "image/png";

      try {
        // Use Tauri command to read file as base64 data URL
        const base64 = await readFileAsBase64FromPath(filePath);

        const filename = filePath.split("/").pop() || filePath.split("\\").pop() || "image";
        newAttachments.push({
          type: "image",
          data: base64,
          media_type: mediaType,
          filename,
        });
      } catch (error) {
        notify.error(`Failed to read image: ${filePath}`);
      }
    }

    return newAttachments;
  }, []);

  // Tauri drag-drop event listeners
  // Track last known drag position for drop zone detection
  const lastDragPositionRef = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    // Skip in browser mode (no Tauri)
    if (typeof window !== "undefined" && window.__MOCK_BROWSER_MODE__) {
      return;
    }

    const unlisteners: UnlistenFn[] = [];

    const setupListeners = async () => {
      // Listen for drag enter - cache the drop zone rect for hit testing
      const unlistenEnter = await listen("tauri://drag-enter", () => {
        // Cache the bounding rect once on drag-enter to avoid
        // calling getBoundingClientRect() on every drag-over event
        updateDropZoneRect();
      });
      unlisteners.push(unlistenEnter);

      // Listen for drag over - update visual state based on position
      const unlistenOver = await listen<{ position: { x: number; y: number } }>(
        "tauri://drag-over",
        (event) => {
          const { x, y } = event.payload.position;
          lastDragPositionRef.current = { x, y };

          if (inputMode === "agent" && isPositionOverDropZone(x, y)) {
            setIsDragOver(true);
            setDragError(null);
          } else {
            setIsDragOver(false);
          }
        }
      );
      unlisteners.push(unlistenOver);

      // Listen for drag leave
      const unlistenLeave = await listen("tauri://drag-leave", () => {
        setIsDragOver(false);
        setDragError(null);
        lastDragPositionRef.current = null;
        dropZoneRectRef.current = null; // Clear cached rect
      });
      unlisteners.push(unlistenLeave);

      // Listen for drop
      const unlistenDrop = await listen<{ paths: string[]; position: { x: number; y: number } }>(
        "tauri://drag-drop",
        async (event) => {
          setIsDragOver(false);
          setDragError(null);
          lastDragPositionRef.current = null;

          // Use the drop event's position to check if over drop zone
          const { x, y } = event.payload.position;
          const isOverDropZone = isPositionOverDropZone(x, y);

          // Only process if in agent mode and over drop zone
          if (inputMode !== "agent" || !isOverDropZone) {
            return;
          }

          const filePaths = event.payload.paths;
          if (filePaths.length === 0) return;

          // Check if any paths are images
          const imageExtensions = ["png", "jpg", "jpeg", "gif", "webp"];
          const hasImages = filePaths.some((path) => {
            const ext = path.toLowerCase().split(".").pop();
            return ext && imageExtensions.includes(ext);
          });

          if (!hasImages) {
            notify.warning("Only image files are supported");
            return;
          }

          const newAttachments = await processFilePaths(filePaths);
          if (newAttachments.length > 0) {
            setImageAttachments((prev) => [...prev, ...newAttachments]);
          }
        }
      );
      unlisteners.push(unlistenDrop);
    };

    setupListeners();

    return () => {
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  }, [inputMode, isPositionOverDropZone, processFilePaths, updateDropZoneRect]);

  // Clipboard paste handler for image attachment
  const handlePaste = useCallback(
    async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
      // Only handle in agent mode
      if (inputMode !== "agent") return;

      const clipboardItems = e.clipboardData.items;
      const imageItems: File[] = [];

      for (const item of clipboardItems) {
        if (item.kind === "file" && item.type.startsWith("image/")) {
          const file = item.getAsFile();
          if (file) {
            imageItems.push(file);
          }
        }
      }

      // If no images, let default paste behavior handle text
      if (imageItems.length === 0) return;

      // Prevent default only if we have images to process
      e.preventDefault();

      const newAttachments = await processImageFiles(imageItems);
      if (newAttachments.length > 0) {
        setImageAttachments((prev) => [...prev, ...newAttachments]);
      }
    },
    [inputMode, processImageFiles]
  );

  // Stable handleSubmit callback - reads current values from stateRef
  // This prevents recreation when streaming state (isAgentResponding, streamingBlocks) changes
  const handleSubmit = useCallback(async () => {
    // Read current values from ref
    const { input, inputMode, isAgentBusy, imageAttachments, visionCapabilities } =
      stateRef.current;

    // Allow submit if: (1) has text input, OR (2) agent mode with image attachments
    const hasContent = input.trim() || (inputMode === "agent" && imageAttachments.length > 0);
    if (!hasContent || isAgentBusy) {
      return;
    }

    const value = input.trim();
    setInput("");
    resetHistory();

    if (inputMode === "terminal") {
      // Terminal mode: send to PTY

      // Handle clear command - clear timeline and command blocks
      if (value === "clear") {
        clearTerminal(sessionId);
        // Don't send to PTY - just clear the UI
        return;
      }

      // Add to history
      addToHistory(value);

      // Note: Fullterm mode switching is now handled automatically via
      // alternate_screen events from the PTY parser detecting ANSI sequences

      // Store command before sending (for bash integration which may not include command in OSC 133)
      setLastSentCommand(sessionId, value);

      // Send command + newline to PTY
      await ptyWrite(sessionId, `${value}\n`);
    } else {
      // Agent mode: send to AI

      // Validate images if attached but provider doesn't support vision
      if (imageAttachments.length > 0 && !visionCapabilities?.supports_vision) {
        notify.error(
          "Current model doesn't support images. Remove images or switch to a vision-capable model (Claude 3+, GPT-4+, Gemini)."
        );
        return;
      }

      setIsSubmitting(true);

      // Add to history
      addToHistory(value);

      // Capture attachments before clearing (we need them for the message and backend)
      const attachmentsToSend = imageAttachments.length > 0 ? [...imageAttachments] : [];

      // Add user message to store (with attachments if any)
      addAgentMessage(sessionId, {
        id: crypto.randomUUID(),
        sessionId,
        role: "user",
        content: value,
        timestamp: new Date().toISOString(),
        attachments: attachmentsToSend.length > 0 ? attachmentsToSend : undefined,
      });

      // Clear attachments immediately after adding to message (triggers pop animation)
      if (attachmentsToSend.length > 0) {
        setImageAttachments([]);
      }

      // Send to AI backend - response will come via useAiEvents hook
      try {
        if (attachmentsToSend.length > 0) {
          // Build payload with text and images
          const payload = {
            parts: [
              ...(value ? [{ type: "text" as const, text: value }] : []),
              ...attachmentsToSend,
            ],
          };
          await sendPromptWithAttachments(sessionId, payload);
        } else {
          await sendPromptSession(sessionId, value);
        }
        // Response will be handled by useAiEvents when AI completes
        // Don't set isSubmitting to false here - wait for completed/error event
      } catch (error) {
        notify.error(`Agent error: ${error}`);
        setIsSubmitting(false);
      }
    }
  }, [sessionId, addAgentMessage, addToHistory, resetHistory, setLastSentCommand]);

  // Handle slash command selection (prompts and skills)
  const handleSlashSelect = useCallback(
    async (command: SlashCommand, args?: string) => {
      setShowSlashPopup(false);
      setInput("");

      // Switch to agent mode if in terminal mode
      if (inputMode === "terminal") {
        setInputMode(sessionId, "agent");
      }

      // Read and send the command content
      try {
        // Skills use readSkillBody (just the instructions), prompts use readPrompt (full file)
        const content =
          command.type === "skill"
            ? await readSkillBody(command.path)
            : await readPrompt(command.path);
        // Append args to content if provided
        const fullContent = args ? `${content}\n\n${args}` : content;
        setIsSubmitting(true);

        // Add user message to store (show the slash command name with args)
        addAgentMessage(sessionId, {
          id: crypto.randomUUID(),
          sessionId,
          role: "user",
          content: args ? `/${command.name} ${args}` : `/${command.name}`,
          timestamp: new Date().toISOString(),
        });

        // Send the actual content (with args) to AI
        await sendPromptSession(sessionId, fullContent);
      } catch (error) {
        notify.error(`Failed to run ${command.type}: ${error}`);
        setIsSubmitting(false);
      }
    },
    [sessionId, inputMode, setInputMode, addAgentMessage]
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

  // Handle path completion selection (Tab in terminal mode)
  // For directories: complete and show contents (keep popup open)
  // For files: complete and close popup
  const handlePathSelect = useCallback(
    (completion: PathCompletion) => {
      const cursorPos = textareaRef.current?.selectionStart ?? input.length;
      const { startIndex } = extractWordAtCursor(input, cursorPos);

      const newInput = input.slice(0, startIndex) + completion.insert_text + input.slice(cursorPos);

      setInput(newInput);
      setPathSelectedIndex(0);

      if (completion.entry_type === "directory") {
        // Keep popup open and update query to show directory contents
        setPathQuery(completion.insert_text);
        // Popup stays open to show directory contents
      } else {
        // Close popup for files
        setShowPathPopup(false);
      }
    },
    [input]
  );

  // Note: Previously had auto-complete when there's only one unique match (bash/zsh behavior).
  // Removed to allow users to keep typing and filtering without the popup auto-closing.
  // Users can press Tab or Enter to explicitly select the completion.

  // Handle path completion final selection (Enter) - closes popup without continuing
  const handlePathSelectFinal = useCallback(
    (completion: PathCompletion) => {
      const cursorPos = textareaRef.current?.selectionStart ?? input.length;
      const { startIndex } = extractWordAtCursor(input, cursorPos);

      const newInput = input.slice(0, startIndex) + completion.insert_text + input.slice(cursorPos);

      setInput(newInput);
      setShowPathPopup(false);
      setPathSelectedIndex(0);
      // Don't continue for directories - just close the popup
    },
    [input]
  );

  // Handle history search selection
  const handleHistorySelect = useCallback((match: HistoryMatch) => {
    setInput(match.command);
    setShowHistorySearch(false);
    setHistorySearchQuery("");
    setHistorySelectedIndex(0);
    textareaRef.current?.focus();
  }, []);

  // Stable handleKeyDown callback - reads current values from stateRef
  // This prevents recreation on every keystroke (when input changes)
  const handleKeyDown = useCallback(
    async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Read current values from ref
      const {
        input,
        inputMode,
        showSlashPopup,
        filteredSlashCommands,
        slashSelectedIndex,
        showFilePopup,
        files,
        fileSelectedIndex,
        showPathPopup,
        pathCompletions,
        pathSelectedIndex,
        showHistorySearch,
        historySearchQuery,
        historyMatches,
        historySelectedIndex,
        originalInput,
        commands,
      } = stateRef.current;

      // History search mode keyboard navigation
      if (showHistorySearch) {
        // Escape or Ctrl+G - cancel search and restore original input
        if (e.key === "Escape" || (e.ctrlKey && e.key === "g")) {
          e.preventDefault();
          setShowHistorySearch(false);
          setInput(originalInput);
          setHistorySearchQuery("");
          setHistorySelectedIndex(0);
          return;
        }

        // Enter - select current match and close
        if (e.key === "Enter" && !e.shiftKey && historyMatches.length > 0) {
          e.preventDefault();
          handleHistorySelect(historyMatches[historySelectedIndex]);
          return;
        }

        // Ctrl+R - cycle to next match
        if (e.ctrlKey && e.key === "r") {
          e.preventDefault();
          if (historyMatches.length > 0) {
            setHistorySelectedIndex((prev) => (prev < historyMatches.length - 1 ? prev + 1 : 0));
          }
          return;
        }

        // Arrow down - navigate to next match
        if (e.key === "ArrowDown") {
          e.preventDefault();
          if (historyMatches.length > 0) {
            setHistorySelectedIndex((prev) => (prev < historyMatches.length - 1 ? prev + 1 : prev));
          }
          return;
        }

        // Arrow up - navigate to previous match
        if (e.key === "ArrowUp") {
          e.preventDefault();
          if (historyMatches.length > 0) {
            setHistorySelectedIndex((prev) => (prev > 0 ? prev - 1 : 0));
          }
          return;
        }

        // Backspace - remove character from search query or exit if empty
        if (e.key === "Backspace") {
          e.preventDefault();
          if (historySearchQuery.length > 0) {
            setHistorySearchQuery((prev) => prev.slice(0, -1));
            setHistorySelectedIndex(0);
          } else {
            // Exit search mode if query is empty
            setShowHistorySearch(false);
            setInput(originalInput);
            setHistorySearchQuery("");
            setHistorySelectedIndex(0);
          }
          return;
        }

        // Any printable character - add to search query
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
          e.preventDefault();
          setHistorySearchQuery((prev) => prev + e.key);
          setHistorySelectedIndex(0);
          return;
        }

        // Block all other keys when in search mode
        return;
      }

      // Ctrl+R to open history search
      if (e.ctrlKey && e.key === "r" && !showHistorySearch) {
        e.preventDefault();
        setOriginalInput(input);
        setShowHistorySearch(true);
        setHistorySearchQuery("");
        setHistorySelectedIndex(0);
        return;
      }

      // Cmd+I to toggle input mode - handle first to ensure it works in all modes
      // Check both lowercase 'i' and the key code for reliability across platforms
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && (e.key === "i" || e.key === "I")) {
        e.preventDefault();
        e.stopPropagation();
        toggleInputMode();
        return;
      }

      // Path completion keyboard navigation (terminal mode)
      if (showPathPopup && pathCompletions.length > 0) {
        if (e.key === "Escape") {
          e.preventDefault();
          setShowPathPopup(false);
          return;
        }
        // Arrow keys and vim-style navigation: Ctrl+N/J = down, Ctrl+P/K = up
        // Navigation wraps around at boundaries
        if (e.key === "ArrowDown" || (e.ctrlKey && (e.key === "n" || e.key === "j"))) {
          e.preventDefault();
          e.stopPropagation();
          setPathSelectedIndex((prev) => (prev + 1) % pathCompletions.length);
          return;
        }
        if (e.key === "ArrowUp" || (e.ctrlKey && (e.key === "p" || e.key === "k"))) {
          e.preventDefault();
          e.stopPropagation();
          setPathSelectedIndex(
            (prev) => (prev - 1 + pathCompletions.length) % pathCompletions.length
          );
          return;
        }
        // Tab - select and continue into directories
        if (e.key === "Tab" && !e.shiftKey) {
          e.preventDefault();
          handlePathSelect(pathCompletions[pathSelectedIndex]);
          return;
        }
        // Enter - select and close popup (final selection)
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          handlePathSelectFinal(pathCompletions[pathSelectedIndex]);
          return;
        }
      }

      // When slash popup is open, handle navigation
      if (showSlashPopup && filteredSlashCommands.length > 0) {
        if (e.key === "Escape") {
          e.preventDefault();
          setShowSlashPopup(false);
          return;
        }

        // Arrow down - move selection down
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setSlashSelectedIndex((prev) =>
            prev < filteredSlashCommands.length - 1 ? prev + 1 : prev
          );
          return;
        }

        // Arrow up - move selection up
        if (e.key === "ArrowUp") {
          e.preventDefault();
          setSlashSelectedIndex((prev) => (prev > 0 ? prev - 1 : 0));
          return;
        }

        // Tab - complete the selected option into the input field with space for args
        if (e.key === "Tab") {
          e.preventDefault();
          const selectedPrompt = filteredSlashCommands[slashSelectedIndex];
          if (selectedPrompt) {
            setInput(`/${selectedPrompt.name} `);
            setShowSlashPopup(false);
          }
          return;
        }

        // Enter - execute the selected option
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          const selectedPrompt = filteredSlashCommands[slashSelectedIndex];
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

      // Handle Enter for slash commands with args (popup closed due to exact match + space)
      if (e.key === "Enter" && !e.shiftKey && input.startsWith("/")) {
        const afterSlash = input.slice(1);
        const spaceIdx = afterSlash.indexOf(" ");
        const cmdName = spaceIdx === -1 ? afterSlash : afterSlash.slice(0, spaceIdx);
        const args = spaceIdx === -1 ? "" : afterSlash.slice(spaceIdx + 1).trim();
        const matchingCommand = commands.find((c) => c.name === cmdName);
        if (matchingCommand) {
          e.preventDefault();
          handleSlashSelect(matchingCommand, args || undefined);
          return;
        }
      }

      // Handle Enter - execute/send (Shift+Enter for newline)
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        await handleSubmit();
        return;
      }

      // History navigation - shared between terminal and agent modes
      // Only activate history if cursor is on the first/last line of input
      if (e.key === "ArrowUp") {
        const cursorPos = textareaRef.current?.selectionStart ?? 0;
        if (isCursorOnFirstLine(input, cursorPos)) {
          e.preventDefault();
          const cmd = navigateUp();
          if (cmd !== null) {
            setInput(cmd);
          }
        }
        // Otherwise, let default behavior move cursor up
        return;
      }

      if (e.key === "ArrowDown") {
        const cursorPos = textareaRef.current?.selectionStart ?? input.length;
        if (isCursorOnLastLine(input, cursorPos)) {
          e.preventDefault();
          setInput(navigateDown());
        }
        // Otherwise, let default behavior move cursor down
        return;
      }

      // Terminal-specific shortcuts
      if (inputMode === "terminal") {
        // Handle Tab - show path completion popup
        if (e.key === "Tab") {
          e.preventDefault();

          // If popup already open, select current item
          if (showPathPopup && pathCompletions.length > 0) {
            handlePathSelect(pathCompletions[pathSelectedIndex]);
            return;
          }

          // Extract word at cursor and show popup
          const cursorPos = textareaRef.current?.selectionStart ?? input.length;
          const { word } = extractWordAtCursor(input, cursorPos);
          setPathQuery(word);
          setShowPathPopup(true);
          setPathSelectedIndex(0);
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

        // Handle Ctrl+L - clear timeline and command blocks
        if (e.ctrlKey && e.key === "l") {
          e.preventDefault();
          clearTerminal(sessionId);
          return;
        }
      }
    },
    [
      sessionId,
      handleSubmit,
      navigateUp,
      navigateDown,
      toggleInputMode,
      handleSlashSelect,
      handleFileSelect,
      handlePathSelect,
      handlePathSelectFinal,
      handleHistorySelect,
    ]
  );

  // Abbreviate path like fish shell: ~/C/p/my-project
  const displayPath = (() => {
    if (!workingDirectory) return "~";
    // Replace home directory with ~
    const withTilde = workingDirectory.replace(/^\/Users\/[^/]+/, "~");
    const parts = withTilde.split("/");
    if (parts.length <= 2) return withTilde; // e.g., "~" or "~/foo"
    // Keep first (~ or root) and last part full, abbreviate middle parts to first char
    const first = parts[0];
    const last = parts[parts.length - 1];
    const middle = parts.slice(1, -1).map((p) => p[0] || p);
    return [first, ...middle, last].join("/");
  })();

  // Render pane-level drop zone overlay using portal
  const paneDropOverlay =
    isDragOver && paneContainerRef.current
      ? createPortal(
          <div className="absolute inset-0 flex items-center justify-center z-50 pointer-events-none bg-background/60 backdrop-blur-[1px] border-2 border-dashed border-accent rounded-lg m-1">
            <div
              className={cn(
                "px-6 py-3 rounded-lg text-sm font-medium shadow-lg",
                dragError
                  ? "bg-destructive/90 text-destructive-foreground"
                  : "bg-accent text-accent-foreground"
              )}
            >
              {dragError || "Drop images here"}
            </div>
          </div>,
          paneContainerRef.current
        )
      : null;

  return (
    <>
      {paneDropOverlay}
      <div className="border-t border-[var(--border-subtle)]">
        {/* Inline Task Plan - only shown when a plan exists */}
        <InlineTaskPlan sessionId={sessionId} />

        {/* Path and badges row - shows shimmer when agent is busy */}
        <div
          className={cn(
            "flex items-center gap-2 px-4 py-1.5",
            isAgentBusy && "agent-loading-shimmer"
          )}
        >
          {/* Path badge (left) */}
          <div
            className="h-5 px-1.5 gap-1 text-xs rounded bg-muted/50 border border-border/50 inline-flex items-center"
            title={workingDirectory || "~"}
          >
            <Folder className="w-3 h-3 text-[#e0af68] shrink-0" />
            <span className="text-muted-foreground">{displayPath}</span>
          </div>

          {/* Git badge */}
          {gitBranch && (
            <button
              type="button"
              onClick={onOpenGitPanel}
              disabled={!onOpenGitPanel}
              className={cn(
                "h-5 px-1.5 gap-1 text-xs rounded flex items-center border transition-colors shrink-0",
                onOpenGitPanel
                  ? "bg-muted/50 hover:bg-muted border-border/50 cursor-pointer"
                  : "bg-muted/30 border-border/30 cursor-default"
              )}
              title={onOpenGitPanel ? "Toggle Git Panel" : undefined}
            >
              <GitBranch className="w-3 h-3 text-[#7dcfff]" />
              {gitBranch && (
                <>
                  <span className="text-muted-foreground">{gitBranch}</span>
                  {gitStatus && (
                    <>
                      <span className="text-muted-foreground ml-0.5">|</span>
                      <span className="text-[#9ece6a]">+{gitStatus.insertions ?? 0}</span>
                      <span className="text-muted-foreground">/</span>
                      <span className="text-[#f7768e]">-{gitStatus.deletions ?? 0}</span>
                      {((gitStatus.ahead ?? 0) > 0 || (gitStatus.behind ?? 0) > 0) && (
                        <>
                          <span className="text-muted-foreground ml-0.5">|</span>
                          {(gitStatus.ahead ?? 0) > 0 && (
                            <span
                              className="flex items-center text-[#9ece6a]"
                              title={`${gitStatus.ahead} to push`}
                            >
                              <ArrowUp className="w-2.5 h-2.5" />
                              {gitStatus.ahead}
                            </span>
                          )}
                          {(gitStatus.behind ?? 0) > 0 && (
                            <span
                              className="flex items-center text-[#e0af68]"
                              title={`${gitStatus.behind} to pull`}
                            >
                              <ArrowDown className="w-2.5 h-2.5" />
                              {gitStatus.behind}
                            </span>
                          )}
                        </>
                      )}
                    </>
                  )}
                </>
              )}
            </button>
          )}

          {virtualEnv && (
            <div className="h-5 px-1.5 gap-1 text-xs rounded bg-[#9ece6a]/10 text-[#9ece6a] flex items-center border border-[#9ece6a]/20 shrink-0">
              <Package className="w-3 h-3" />
              <span>{virtualEnv}</span>
            </div>
          )}
        </div>

        {/* Input row with container */}
        <div className="px-3 py-1.5 border-y border-[var(--border-subtle)]">
          <div
            ref={dropZoneRef}
            className={cn(
              "relative flex items-end gap-2 rounded-md bg-background px-2 py-1",
              "transition-all duration-150",
              // Drag-over states
              isDragOver && !dragError && ["bg-accent/10"],
              isDragOver && dragError && ["bg-destructive/10"]
            )}
          >
            <HistorySearchPopup
              open={showHistorySearch}
              onOpenChange={setShowHistorySearch}
              matches={historyMatches}
              selectedIndex={historySelectedIndex}
              searchQuery={historySearchQuery}
              onSelect={handleHistorySelect}
            >
              <PathCompletionPopup
                open={showPathPopup}
                onOpenChange={setShowPathPopup}
                completions={pathCompletions}
                totalCount={pathTotalCount}
                selectedIndex={pathSelectedIndex}
                onSelect={handlePathSelect}
              >
                <SlashCommandPopup
                  open={showSlashPopup}
                  onOpenChange={setShowSlashPopup}
                  commands={filteredSlashCommands}
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
                    <div className="relative flex-1 min-w-0">
                      <textarea
                        ref={textareaRef}
                        data-testid="unified-input"
                        data-mode={inputMode}
                        value={showHistorySearch ? "" : input}
                        onChange={(e) => {
                          const value = e.target.value;
                          setInput(value);
                          resetHistory();

                          // Update path query when typing with popup open (for live filtering)
                          if (showPathPopup && inputMode === "terminal") {
                            // Use the new cursor position (end of input after typing)
                            const newCursorPos = e.target.selectionStart ?? value.length;
                            const { word } = extractWordAtCursor(value, newCursorPos);
                            if (word) {
                              // Update query for live filtering
                              setPathQuery(word);
                              setPathSelectedIndex(0);
                            } else {
                              // Close popup if word becomes empty (e.g., typed a space)
                              setShowPathPopup(false);
                            }
                          }

                          // Show slash popup when "/" is typed at the start
                          if (value.startsWith("/") && value.length >= 1) {
                            const afterSlash = value.slice(1);
                            const spaceIdx = afterSlash.indexOf(" ");
                            const commandPart =
                              spaceIdx === -1 ? afterSlash : afterSlash.slice(0, spaceIdx);
                            const exactMatch = commands.some((c) => c.name === commandPart);

                            // Close popup after space when there's an exact command match
                            if (spaceIdx === -1 || !exactMatch) {
                              setShowSlashPopup(true);
                              setSlashSelectedIndex(0);
                            } else {
                              setShowSlashPopup(false);
                            }
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
                        onPaste={handlePaste}
                        disabled={isInputDisabled}
                        placeholder={
                          showHistorySearch
                            ? ""
                            : isSessionDead
                              ? "Session limit exceeded. Please start a new session."
                              : isCompacting
                                ? "Compacting conversation..."
                                : ""
                        }
                        rows={1}
                        className={cn(
                          "w-full min-h-[24px] max-h-[200px] py-0",
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
                      {/* Ghost completion hint - shown inline after current input */}
                      {ghostText && inputMode === "terminal" && !showHistorySearch && (
                        <GhostTextHint text={ghostText} inputLength={input.length} />
                      )}
                    </div>
                  </FileCommandPopup>
                </SlashCommandPopup>
              </PathCompletionPopup>
            </HistorySearchPopup>

            {/* Image attachment (only shown in agent mode when vision is supported) */}
            {inputMode === "agent" && (
              <ImageAttachment
                attachments={imageAttachments}
                onAttachmentsChange={setImageAttachments}
                capabilities={visionCapabilities}
                disabled={isInputDisabled}
              />
            )}

            {/* Send button */}
            <button
              type="button"
              onClick={handleSubmit}
              disabled={(!input.trim() && imageAttachments.length === 0) || isInputDisabled}
              className={cn(
                "h-7 w-7 flex items-center justify-center rounded-md shrink-0",
                "transition-all duration-150",
                (input.trim() || imageAttachments.length > 0) && !isInputDisabled
                  ? "bg-accent text-accent-foreground hover:bg-accent/90"
                  : "bg-muted text-muted-foreground cursor-not-allowed"
              )}
            >
              <SendHorizontal className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* Status row - model selector, token usage */}
        <InputStatusRow sessionId={sessionId} />
      </div>
    </>
  );
}
