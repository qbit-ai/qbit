#![allow(dead_code)] // PTY parser implemented but integrated via Tauri feature only
use vte::{Params, Parser, Perform};

/// Semantic regions in terminal output based on OSC 133 shell integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalRegion {
    /// Not in any tracked region - output is passed through
    #[default]
    Output,
    /// Between OSC 133;A and OSC 133;B - prompt text, should be suppressed
    Prompt,
    /// Between OSC 133;B and OSC 133;C - user typing, should be suppressed
    Input,
}

/// Result of parsing terminal output with filtering
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Extracted semantic events
    pub events: Vec<OscEvent>,
    /// Filtered output bytes - only includes Output region content (not Prompt or Input)
    pub output: Vec<u8>,
}

/// Events extracted from terminal escape sequences (OSC and CSI)
#[derive(Debug, Clone)]
pub enum OscEvent {
    /// OSC 133 ; A - Prompt start
    PromptStart,
    /// OSC 133 ; B - Prompt end (user can type)
    PromptEnd,
    /// OSC 133 ; C [; command] - Command execution started
    CommandStart { command: Option<String> },
    /// OSC 133 ; D ; N - Command finished with exit code N
    CommandEnd { exit_code: i32 },
    /// OSC 7 - Current working directory changed
    DirectoryChanged { path: String },
    /// OSC 1337 ; CurrentDir=PATH ; VirtualEnv=NAME - Virtual environment activated
    /// Reports the virtual environment name when activated (e.g., Python venv, conda)
    VirtualEnvChanged { name: Option<String> },
    /// CSI ? 1049 h (or 47, 1047) - Alternate screen buffer enabled
    /// Indicates a TUI application (vim, htop, less, etc.) has started
    AlternateScreenEnabled,
    /// CSI ? 1049 l (or 47, 1047) - Alternate screen buffer disabled
    /// Indicates a TUI application has exited
    AlternateScreenDisabled,
    /// CSI ? 2026 h - Synchronized output enabled
    /// Applications use this to batch screen updates atomically to prevent flickering
    SynchronizedOutputEnabled,
    /// CSI ? 2026 l - Synchronized output disabled
    /// Signals that batched updates should be flushed to the screen
    SynchronizedOutputDisabled,
}

impl OscEvent {
    /// Convert to a tuple of (event_name, CommandBlockEvent) for emission.
    /// Returns None for DirectoryChanged events (handled separately).
    pub fn to_command_block_event(
        &self,
        session_id: &str,
    ) -> Option<(&'static str, super::manager::CommandBlockEvent)> {
        use super::manager::CommandBlockEvent;

        Some(match self {
            OscEvent::PromptStart => (
                "command_block",
                CommandBlockEvent {
                    session_id: session_id.to_string(),
                    command: None,
                    exit_code: None,
                    event_type: "prompt_start".to_string(),
                },
            ),
            OscEvent::PromptEnd => (
                "command_block",
                CommandBlockEvent {
                    session_id: session_id.to_string(),
                    command: None,
                    exit_code: None,
                    event_type: "prompt_end".to_string(),
                },
            ),
            OscEvent::CommandStart { command } => (
                "command_block",
                CommandBlockEvent {
                    session_id: session_id.to_string(),
                    command: command.clone(),
                    exit_code: None,
                    event_type: "command_start".to_string(),
                },
            ),
            OscEvent::CommandEnd { exit_code } => (
                "command_block",
                CommandBlockEvent {
                    session_id: session_id.to_string(),
                    command: None,
                    exit_code: Some(*exit_code),
                    event_type: "command_end".to_string(),
                },
            ),
            OscEvent::DirectoryChanged { .. } => return None,
            OscEvent::VirtualEnvChanged { .. } => return None,
            // Alternate screen and synchronized output events are handled separately
            OscEvent::AlternateScreenEnabled
            | OscEvent::AlternateScreenDisabled
            | OscEvent::SynchronizedOutputEnabled
            | OscEvent::SynchronizedOutputDisabled => return None,
        })
    }
}

/// Terminal output parser that extracts OSC sequences
pub struct TerminalParser {
    parser: Parser,
    performer: OscPerformer,
}

impl TerminalParser {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            performer: OscPerformer::new(),
        }
    }

    /// Parse terminal output and extract OSC events
    pub fn parse(&mut self, data: &[u8]) -> Vec<OscEvent> {
        self.performer.events.clear();
        for byte in data {
            self.parser.advance(&mut self.performer, *byte);
        }
        std::mem::take(&mut self.performer.events)
    }

    /// Parse terminal output, extract OSC events, and filter output to only include
    /// content from the Output region (excludes Prompt and Input regions).
    ///
    /// When in alternate screen mode (TUI apps like vim, htop), filtering is disabled
    /// and all raw bytes are passed through to preserve escape sequences needed for
    /// proper rendering.
    pub fn parse_filtered(&mut self, data: &[u8]) -> ParseResult {
        // If already in alternate screen mode, pass through raw data
        // TUI apps need all escape sequences for proper rendering
        let was_in_alternate = self.performer.alternate_screen_active;

        self.performer.events.clear();
        self.performer.visible_bytes.clear();

        for byte in data {
            self.parser.advance(&mut self.performer, *byte);
        }

        // If we were in alternate screen OR just entered it, use raw output
        // This ensures TUI apps get all their escape sequences
        let use_raw_output = was_in_alternate || self.performer.alternate_screen_active;

        ParseResult {
            events: std::mem::take(&mut self.performer.events),
            output: if use_raw_output {
                data.to_vec()
            } else {
                std::mem::take(&mut self.performer.visible_bytes)
            },
        }
    }

    /// Check if the parser is currently tracking alternate screen mode as active
    pub fn in_alternate_screen(&self) -> bool {
        self.performer.alternate_screen_active
    }
}

impl Default for TerminalParser {
    fn default() -> Self {
        Self::new()
    }
}

struct OscPerformer {
    events: Vec<OscEvent>,
    /// Track last directory to deduplicate OSC 7 events
    last_directory: Option<String>,
    /// Track last virtual environment to deduplicate OSC 1337 events
    last_virtual_env: Option<String>,
    /// Current semantic region based on OSC 133 markers
    current_region: TerminalRegion,
    /// Accumulated visible output bytes (only from Output region)
    visible_bytes: Vec<u8>,
    /// Track alternate screen state to deduplicate CSI events
    alternate_screen_active: bool,
}

impl OscPerformer {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            last_directory: None,
            last_virtual_env: None,
            current_region: TerminalRegion::Output,
            visible_bytes: Vec::new(),
            alternate_screen_active: false,
        }
    }

    fn handle_osc(&mut self, params: &[&[u8]]) {
        if params.is_empty() {
            return;
        }

        // Parse the OSC command number
        let cmd = match std::str::from_utf8(params[0]) {
            Ok(s) => s,
            Err(_) => return,
        };

        match cmd {
            // OSC 133 - Semantic prompt sequences
            "133" => self.handle_osc_133(params),
            // OSC 7 - Current working directory
            "7" => self.handle_osc_7(params),
            // OSC 1337 - Custom data (virtual environment)
            "1337" => self.handle_osc_1337(params),
            _ => {}
        }
    }

    fn handle_osc_133(&mut self, params: &[&[u8]]) {
        if params.len() < 2 {
            tracing::debug!("[OSC 133] Received but params.len() < 2");
            return;
        }

        let marker = match std::str::from_utf8(params[1]) {
            Ok(s) => s,
            Err(_) => {
                tracing::debug!("[OSC 133] Marker is not valid UTF-8");
                return;
            }
        };

        tracing::debug!("[OSC 133] marker={:?}, params_len={}", marker, params.len());

        // Get extra argument from params[2] if present
        let extra_arg = params.get(2).and_then(|p| std::str::from_utf8(p).ok());

        // Match on first character, handling both "C" and "C;command" formats
        match marker.chars().next() {
            Some('A') => {
                tracing::trace!("[OSC 133] PromptStart");
                self.current_region = TerminalRegion::Prompt;
                self.events.push(OscEvent::PromptStart);
            }
            Some('B') => {
                tracing::trace!("[OSC 133] PromptEnd");
                self.current_region = TerminalRegion::Input;
                self.events.push(OscEvent::PromptEnd);
            }
            Some('C') => {
                // Command may come from marker suffix (C;cmd) or params[2]
                self.current_region = TerminalRegion::Output;
                let command = marker
                    .strip_prefix("C;")
                    .or(extra_arg)
                    .map(|s| s.to_string());
                tracing::debug!("[OSC 133] CommandStart: {:?}", command);
                self.events.push(OscEvent::CommandStart { command });
            }
            Some('D') => {
                // Exit code may come from marker suffix (D;0) or params[2]
                // Stay in Output region (will transition to Prompt on next 'A')
                self.current_region = TerminalRegion::Output;
                let exit_code = marker
                    .strip_prefix("D;")
                    .or(extra_arg)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                tracing::debug!("[OSC 133] CommandEnd: exit_code={}", exit_code);
                self.events.push(OscEvent::CommandEnd { exit_code });
            }
            _ => {}
        }
    }

    fn handle_osc_7(&mut self, params: &[&[u8]]) {
        // OSC 7 format: file://hostname/path
        if params.len() < 2 {
            tracing::trace!("[cwd-sync] OSC 7 received but params.len() < 2");
            return;
        }

        let url = match std::str::from_utf8(params[1]) {
            Ok(s) => s,
            Err(_) => {
                tracing::trace!("[cwd-sync] OSC 7 URL is not valid UTF-8");
                return;
            }
        };

        tracing::trace!("[cwd-sync] OSC 7 URL: {}", url);

        // Parse file:// URL
        if let Some(path) = url.strip_prefix("file://") {
            // Remove hostname (everything up to the next /)
            if let Some(idx) = path.find('/') {
                let path = &path[idx..];
                // URL decode the path
                let path = urlencoding_decode(path);

                // Only emit if directory actually changed
                let is_duplicate = self
                    .last_directory
                    .as_ref()
                    .map(|last| last == &path)
                    .unwrap_or(false);

                if is_duplicate {
                    tracing::trace!("[cwd-sync] Duplicate OSC 7 ignored: {}", path);
                } else {
                    // DEBUG: Log with backtrace to trace where OSC 7 is coming from
                    tracing::warn!(
                        "[cwd-debug] OSC 7 directory changed: prev={:?}, new={}, (set RUST_BACKTRACE=1 for trace)",
                        self.last_directory,
                        path
                    );
                    tracing::info!("[cwd-sync] Directory changed to: {}", path);
                    self.last_directory = Some(path.clone());
                    self.events.push(OscEvent::DirectoryChanged { path });
                }
            } else {
                tracing::trace!("[cwd-sync] OSC 7 path has no slash after hostname");
            }
        } else {
            tracing::trace!("[cwd-sync] OSC 7 URL does not start with file://");
        }
    }

    fn handle_osc_1337(&mut self, params: &[&[u8]]) {
        // OSC 1337 format: VirtualEnv=name or just name
        if params.len() < 2 {
            tracing::trace!("[venv-sync] OSC 1337 received but params.len() < 2");
            return;
        }

        let data = match std::str::from_utf8(params[1]) {
            Ok(s) => s,
            Err(_) => {
                tracing::trace!("[venv-sync] OSC 1337 data is not valid UTF-8");
                return;
            }
        };

        tracing::trace!("[venv-sync] OSC 1337 data: {}", data);

        // Parse VirtualEnv=name format, or just use the whole string
        let venv_name = if let Some(name) = data.strip_prefix("VirtualEnv=") {
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        } else if data.is_empty() {
            None
        } else {
            Some(data.to_string())
        };

        // Only emit if virtual env actually changed
        let is_duplicate = self
            .last_virtual_env
            .as_ref()
            .map(|last| Some(last) == venv_name.as_ref())
            .unwrap_or(venv_name.is_none());

        if is_duplicate {
            tracing::trace!("[venv-sync] Duplicate OSC 1337 ignored: {:?}", venv_name);
        } else {
            tracing::info!(
                "[venv-sync] Virtual environment changed to: {:?}",
                venv_name
            );
            self.last_virtual_env.clone_from(&venv_name);
            self.events
                .push(OscEvent::VirtualEnvChanged { name: venv_name });
        }
    }
}

impl Perform for OscPerformer {
    fn print(&mut self, c: char) {
        // Pass through Output and Input regions
        // Input region includes PS2 continuation prompts which should be visible
        // Only suppress Prompt region (the shell prompt itself)
        if self.current_region != TerminalRegion::Prompt {
            // Encode char as UTF-8 and add to visible_bytes
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            self.visible_bytes.extend_from_slice(encoded.as_bytes());
        }
    }

    fn execute(&mut self, byte: u8) {
        // Pass through Output and Input regions
        // Input region includes PS2 continuation prompts which should be visible
        if self.current_region != TerminalRegion::Prompt {
            // Pass through control characters
            // Common ones: LF (0x0A), CR (0x0D), TAB (0x09), BS (0x08)
            match byte {
                0x0A | 0x0D | 0x09 | 0x08 => {
                    self.visible_bytes.push(byte);
                }
                _ => {}
            }
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        // Only handle DEC private modes (intermediate byte '?')
        // These are CSI sequences like ESC [ ? 1049 h
        if intermediates != [b'?'] {
            return;
        }

        // Check for set (h) or reset (l)
        let is_enable = match action {
            'h' => true,  // DECSET - enable mode
            'l' => false, // DECRST - disable mode
            _ => return,
        };

        // Check for alternate screen buffer modes
        for param in params {
            // params is an iterator of &[u16] slices (for subparameters)
            let mode = param.first().copied().unwrap_or(0);

            match mode {
                // 1049: xterm alternate screen with saved cursor (most common)
                // 47: legacy alternate screen
                // 1047: alternate screen without cursor save
                1049 | 47 | 1047 => {
                    // Deduplicate: only emit if state actually changes
                    if is_enable && !self.alternate_screen_active {
                        self.alternate_screen_active = true;
                        self.events.push(OscEvent::AlternateScreenEnabled);
                    } else if !is_enable && self.alternate_screen_active {
                        self.alternate_screen_active = false;
                        self.events.push(OscEvent::AlternateScreenDisabled);
                    }
                }
                // 2026: Synchronized output (DEC private mode)
                // Used by modern CLI apps to batch screen updates atomically
                2026 => {
                    if is_enable {
                        self.events.push(OscEvent::SynchronizedOutputEnabled);
                    } else {
                        self.events.push(OscEvent::SynchronizedOutputDisabled);
                    }
                }
                _ => {}
            }
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        self.handle_osc(params);
    }
}

/// Simple URL decoding for paths
fn urlencoding_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let mut hex = String::new();
            if let Some(&h1) = chars.peek() {
                hex.push(h1);
                chars.next();
            }
            if let Some(&h2) = chars.peek() {
                hex.push(h2);
                chars.next();
            }
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // OSC 133 - Prompt lifecycle tests
    // ===========================================

    #[test]
    fn test_osc_133_prompt_start() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; A ST (using BEL as terminator)
        let data = b"\x1b]133;A\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::PromptStart));
    }

    #[test]
    fn test_osc_133_prompt_end() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; B ST (using BEL as terminator)
        let data = b"\x1b]133;B\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::PromptEnd));
    }

    #[test]
    fn test_osc_133_prompt_start_with_st_terminator() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; A ST (using ESC \ as string terminator)
        let data = b"\x1b]133;A\x1b\\";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::PromptStart));
    }

    // ===========================================
    // OSC 133 - Command lifecycle tests
    // ===========================================

    #[test]
    fn test_osc_133_command_start_no_command() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; C ST (no command text)
        let data = b"\x1b]133;C\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandStart { command } = &events[0] {
            assert!(command.is_none());
        } else {
            panic!("Expected CommandStart");
        }
    }

    #[test]
    fn test_osc_133_command_with_text() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; C ; ls -la ST
        let data = b"\x1b]133;C;ls -la\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandStart { command } = &events[0] {
            assert_eq!(command.as_deref(), Some("ls -la"));
        } else {
            panic!("Expected CommandStart");
        }
    }

    #[test]
    fn test_osc_133_command_with_complex_text() {
        let mut parser = TerminalParser::new();
        // Complex command with pipes, flags, etc.
        let data = b"\x1b]133;C;cat file.txt | grep -E 'pattern' | head -n 10\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandStart { command } = &events[0] {
            assert_eq!(
                command.as_deref(),
                Some("cat file.txt | grep -E 'pattern' | head -n 10")
            );
        } else {
            panic!("Expected CommandStart");
        }
    }

    #[test]
    fn test_osc_133_command_end_success() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; D ; 0 ST
        let data = b"\x1b]133;D;0\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandEnd { exit_code } = &events[0] {
            assert_eq!(*exit_code, 0);
        } else {
            panic!("Expected CommandEnd");
        }
    }

    #[test]
    fn test_osc_133_command_end_failure() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; D ; 1 ST (command failed)
        let data = b"\x1b]133;D;1\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandEnd { exit_code } = &events[0] {
            assert_eq!(*exit_code, 1);
        } else {
            panic!("Expected CommandEnd");
        }
    }

    #[test]
    fn test_osc_133_command_end_signal() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; D ; 130 ST (Ctrl+C typically gives 128 + 2 = 130)
        let data = b"\x1b]133;D;130\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandEnd { exit_code } = &events[0] {
            assert_eq!(*exit_code, 130);
        } else {
            panic!("Expected CommandEnd");
        }
    }

    #[test]
    fn test_osc_133_command_end_no_exit_code() {
        let mut parser = TerminalParser::new();
        // OSC 133 ; D ST (no exit code, defaults to 0)
        let data = b"\x1b]133;D\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandEnd { exit_code } = &events[0] {
            assert_eq!(*exit_code, 0);
        } else {
            panic!("Expected CommandEnd");
        }
    }

    // ===========================================
    // Full command lifecycle test
    // ===========================================

    #[test]
    fn test_full_command_lifecycle() {
        let mut parser = TerminalParser::new();

        // Simulate a full command lifecycle:
        // 1. Prompt starts
        // 2. Prompt ends (user can type)
        // 3. Command starts (user pressed enter)
        // 4. Command ends with exit code

        let prompt_start = b"\x1b]133;A\x07";
        let prompt_end = b"\x1b]133;B\x07";
        let command_start = b"\x1b]133;C;echo hello\x07";
        let command_end = b"\x1b]133;D;0\x07";

        let events = parser.parse(prompt_start);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::PromptStart));

        let events = parser.parse(prompt_end);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::PromptEnd));

        let events = parser.parse(command_start);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandStart { command } = &events[0] {
            assert_eq!(command.as_deref(), Some("echo hello"));
        } else {
            panic!("Expected CommandStart");
        }

        let events = parser.parse(command_end);
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandEnd { exit_code } = &events[0] {
            assert_eq!(*exit_code, 0);
        } else {
            panic!("Expected CommandEnd");
        }
    }

    #[test]
    fn test_multiple_events_in_single_parse() {
        let mut parser = TerminalParser::new();
        // Multiple OSC sequences in one chunk
        let data = b"\x1b]133;A\x07\x1b]133;B\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], OscEvent::PromptStart));
        assert!(matches!(events[1], OscEvent::PromptEnd));
    }

    // ===========================================
    // OSC 7 - Directory change tests
    // ===========================================

    #[test]
    fn test_osc_7_directory() {
        let mut parser = TerminalParser::new();
        // OSC 7 ; file://hostname/Users/test ST
        let data = b"\x1b]7;file://localhost/Users/test\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::DirectoryChanged { path } = &events[0] {
            assert_eq!(path, "/Users/test");
        } else {
            panic!("Expected DirectoryChanged");
        }
    }

    #[test]
    fn test_osc_7_directory_with_spaces() {
        let mut parser = TerminalParser::new();
        // Path with URL-encoded spaces (%20)
        let data = b"\x1b]7;file://localhost/Users/test/My%20Documents\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::DirectoryChanged { path } = &events[0] {
            assert_eq!(path, "/Users/test/My Documents");
        } else {
            panic!("Expected DirectoryChanged");
        }
    }

    #[test]
    fn test_osc_7_directory_deep_path() {
        let mut parser = TerminalParser::new();
        let data = b"\x1b]7;file://macbook.local/Users/xlyk/Code/qbit/src-tauri\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::DirectoryChanged { path } = &events[0] {
            assert_eq!(path, "/Users/xlyk/Code/qbit/src-tauri");
        } else {
            panic!("Expected DirectoryChanged");
        }
    }

    // ===========================================
    // URL encoding/decoding tests
    // ===========================================

    #[test]
    fn test_urlencoding_decode_simple() {
        assert_eq!(urlencoding_decode("/path/to/file"), "/path/to/file");
    }

    #[test]
    fn test_urlencoding_decode_space() {
        assert_eq!(
            urlencoding_decode("/path/My%20Documents"),
            "/path/My Documents"
        );
    }

    #[test]
    fn test_urlencoding_decode_multiple_encoded() {
        assert_eq!(
            urlencoding_decode("/path%20with%20multiple%20spaces"),
            "/path with multiple spaces"
        );
    }

    #[test]
    fn test_urlencoding_decode_special_chars() {
        // %23 = #, %26 = &, %3D = =
        assert_eq!(urlencoding_decode("/path%23file"), "/path#file");
    }

    #[test]
    fn test_urlencoding_decode_invalid_hex() {
        // Invalid hex sequence should be preserved
        assert_eq!(urlencoding_decode("/path%ZZ"), "/path%ZZ");
    }

    #[test]
    fn test_urlencoding_decode_incomplete_percent() {
        // Incomplete percent encoding at end - only 1 hex digit
        // Current implementation tries to decode anyway (0x02 = STX control char)
        assert_eq!(urlencoding_decode("/path%2"), "/path\u{2}");
    }

    // ===========================================
    // Edge cases and robustness tests
    // ===========================================

    #[test]
    fn test_parser_ignores_regular_text() {
        let mut parser = TerminalParser::new();
        // Regular terminal output with no OSC sequences
        let data = b"Hello, world!\nThis is normal output.\n";
        let events = parser.parse(data);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_parser_handles_mixed_content() {
        let mut parser = TerminalParser::new();
        // Normal text mixed with OSC sequences
        let data = b"Some output\x1b]133;A\x07more output\x1b]133;B\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], OscEvent::PromptStart));
        assert!(matches!(events[1], OscEvent::PromptEnd));
    }

    #[test]
    fn test_parser_handles_ansi_escape_codes() {
        let mut parser = TerminalParser::new();
        // ANSI color codes should be ignored, OSC should be parsed
        let data = b"\x1b[32mgreen text\x1b[0m\x1b]133;A\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::PromptStart));
    }

    #[test]
    fn test_parser_ignores_unknown_osc() {
        let mut parser = TerminalParser::new();
        // OSC 0 (window title) - should be ignored
        let data = b"\x1b]0;Window Title\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_parser_empty_input() {
        let mut parser = TerminalParser::new();
        let events = parser.parse(b"");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_parser_partial_osc_sequence() {
        let mut parser = TerminalParser::new();
        // Incomplete OSC sequence (no terminator)
        let data = b"\x1b]133;A";
        let events = parser.parse(data);
        // VTE parser buffers incomplete sequences, so nothing should be emitted yet
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_parser_is_stateless_between_calls() {
        let mut parser = TerminalParser::new();

        // First parse
        let events1 = parser.parse(b"\x1b]133;A\x07");
        assert_eq!(events1.len(), 1);

        // Second parse - events from first call should be cleared
        let events2 = parser.parse(b"\x1b]133;B\x07");
        assert_eq!(events2.len(), 1);
        assert!(matches!(events2[0], OscEvent::PromptEnd));
    }

    #[test]
    fn test_parser_default_trait() {
        let mut parser = TerminalParser::default();
        assert!(parser.parse(b"\x1b]133;A\x07").len() == 1);
    }

    // ===========================================
    // Alternate Screen Buffer tests (CSI sequences)
    // ===========================================

    #[test]
    fn test_alternate_screen_enable_1049() {
        let mut parser = TerminalParser::new();
        // ESC [ ? 1049 h - xterm-style alternate screen with saved cursor
        let data = b"\x1b[?1049h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::AlternateScreenEnabled));
    }

    #[test]
    fn test_alternate_screen_disable_1049() {
        let mut parser = TerminalParser::new();
        // First enable, then disable
        parser.parse(b"\x1b[?1049h");
        let events = parser.parse(b"\x1b[?1049l");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::AlternateScreenDisabled));
    }

    #[test]
    fn test_alternate_screen_enable_47() {
        let mut parser = TerminalParser::new();
        // ESC [ ? 47 h - legacy alternate screen
        let data = b"\x1b[?47h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::AlternateScreenEnabled));
    }

    #[test]
    fn test_alternate_screen_enable_1047() {
        let mut parser = TerminalParser::new();
        // ESC [ ? 1047 h - alternate screen without cursor save
        let data = b"\x1b[?1047h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::AlternateScreenEnabled));
    }

    #[test]
    fn test_alternate_screen_deduplication_enable() {
        let mut parser = TerminalParser::new();
        // Enable twice - should only emit once
        let events1 = parser.parse(b"\x1b[?1049h");
        assert_eq!(events1.len(), 1);

        let events2 = parser.parse(b"\x1b[?1049h");
        assert_eq!(events2.len(), 0); // Deduplicated
    }

    #[test]
    fn test_alternate_screen_deduplication_disable() {
        let mut parser = TerminalParser::new();
        // Disable without prior enable - should not emit
        let events = parser.parse(b"\x1b[?1049l");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_alternate_screen_full_cycle() {
        let mut parser = TerminalParser::new();
        // Full cycle: enable -> disable
        let enable_events = parser.parse(b"\x1b[?1049h");
        assert_eq!(enable_events.len(), 1);
        assert!(matches!(enable_events[0], OscEvent::AlternateScreenEnabled));

        let disable_events = parser.parse(b"\x1b[?1049l");
        assert_eq!(disable_events.len(), 1);
        assert!(matches!(
            disable_events[0],
            OscEvent::AlternateScreenDisabled
        ));
    }

    #[test]
    fn test_alternate_screen_mixed_with_osc() {
        let mut parser = TerminalParser::new();
        // OSC 133 A (prompt start) + CSI ? 1049 h (alt screen)
        let data = b"\x1b]133;A\x07\x1b[?1049h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], OscEvent::PromptStart));
        assert!(matches!(events[1], OscEvent::AlternateScreenEnabled));
    }

    #[test]
    fn test_non_dec_private_mode_ignored() {
        let mut parser = TerminalParser::new();
        // Standard CSI (no ?) should be ignored - this is not a DEC private mode
        let data = b"\x1b[1049h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_alternate_screen_other_modes_ignored() {
        let mut parser = TerminalParser::new();
        // Other DEC private modes should be ignored (e.g., mode 1 for application cursor)
        let data = b"\x1b[?1h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_vim_like_startup_sequence() {
        let mut parser = TerminalParser::new();
        // Simulate vim-like startup: various CSI sequences including alt screen
        // Real vim sends more, but this tests the key part
        let data = b"\x1b[?1049h\x1b[22;0;0t\x1b[?1h\x1b=";
        let events = parser.parse(data);
        // Only the alternate screen event should be captured
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::AlternateScreenEnabled));
    }

    #[test]
    fn test_vim_like_exit_sequence() {
        let mut parser = TerminalParser::new();
        // First enter alternate screen
        parser.parse(b"\x1b[?1049h");
        // Simulate vim-like exit
        let data = b"\x1b[?1049l\x1b[23;0;0t\x1b[?1l\x1b>";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::AlternateScreenDisabled));
    }

    // ===========================================
    // Synchronized Output (DEC 2026) tests
    // ===========================================

    #[test]
    fn test_synchronized_output_enable() {
        let mut parser = TerminalParser::new();
        // ESC [ ? 2026 h - Enable synchronized output
        let data = b"\x1b[?2026h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::SynchronizedOutputEnabled));
    }

    #[test]
    fn test_synchronized_output_disable() {
        let mut parser = TerminalParser::new();
        // ESC [ ? 2026 l - Disable synchronized output
        let data = b"\x1b[?2026l";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OscEvent::SynchronizedOutputDisabled));
    }

    #[test]
    fn test_synchronized_output_full_cycle() {
        let mut parser = TerminalParser::new();
        // Enable then disable
        let enable_events = parser.parse(b"\x1b[?2026h");
        assert_eq!(enable_events.len(), 1);
        assert!(matches!(
            enable_events[0],
            OscEvent::SynchronizedOutputEnabled
        ));

        let disable_events = parser.parse(b"\x1b[?2026l");
        assert_eq!(disable_events.len(), 1);
        assert!(matches!(
            disable_events[0],
            OscEvent::SynchronizedOutputDisabled
        ));
    }

    #[test]
    fn test_synchronized_output_with_alternate_screen() {
        let mut parser = TerminalParser::new();
        // Both modes in same sequence: CSI ? 2026 ; 1049 h
        let data = b"\x1b[?2026;1049h";
        let events = parser.parse(data);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], OscEvent::SynchronizedOutputEnabled));
        assert!(matches!(events[1], OscEvent::AlternateScreenEnabled));
    }

    #[test]
    fn test_synchronized_output_no_deduplication() {
        let mut parser = TerminalParser::new();
        // Unlike alternate screen, sync output does not deduplicate
        // Apps may toggle it multiple times
        let events1 = parser.parse(b"\x1b[?2026h");
        assert_eq!(events1.len(), 1);

        let events2 = parser.parse(b"\x1b[?2026h");
        assert_eq!(events2.len(), 1); // Should still emit
    }

    #[test]
    fn test_synchronized_output_mixed_with_content() {
        let mut parser = TerminalParser::new();
        // Content mixed with sync output sequences
        let data = b"Hello\x1b[?2026hWorld\x1b[?2026l";
        let events = parser.parse(data);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], OscEvent::SynchronizedOutputEnabled));
        assert!(matches!(events[1], OscEvent::SynchronizedOutputDisabled));
    }

    // ===========================================
    // OSC 1337 - Virtual Environment tests
    // ===========================================

    #[test]
    fn test_osc_1337_virtual_env() {
        let mut parser = TerminalParser::new();
        // OSC 1337 ; VirtualEnv=myenv ST (using ESC \ as terminator)
        let data = b"\x1b]1337;VirtualEnv=myenv\x1b\\";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::VirtualEnvChanged { name } = &events[0] {
            assert_eq!(name.as_deref(), Some("myenv"));
        } else {
            panic!("Expected VirtualEnvChanged, got {:?}", events[0]);
        }
    }

    #[test]
    fn test_osc_1337_virtual_env_bel() {
        let mut parser = TerminalParser::new();
        // OSC 1337 ; VirtualEnv=myenv BEL (using BEL as terminator)
        let data = b"\x1b]1337;VirtualEnv=myenv\x07";
        let events = parser.parse(data);
        assert_eq!(events.len(), 1);
        if let OscEvent::VirtualEnvChanged { name } = &events[0] {
            assert_eq!(name.as_deref(), Some("myenv"));
        } else {
            panic!("Expected VirtualEnvChanged, got {:?}", events[0]);
        }
    }

    #[test]
    fn test_osc_1337_virtual_env_clear() {
        let mut parser = TerminalParser::new();
        // First activate a venv
        parser.parse(b"\x1b]1337;VirtualEnv=myenv\x1b\\");
        // Then clear it
        let events = parser.parse(b"\x1b]1337;VirtualEnv=\x1b\\");
        assert_eq!(events.len(), 1);
        if let OscEvent::VirtualEnvChanged { name } = &events[0] {
            assert!(name.is_none());
        } else {
            panic!("Expected VirtualEnvChanged, got {:?}", events[0]);
        }
    }

    #[test]
    fn test_osc_1337_virtual_env_deduplication() {
        let mut parser = TerminalParser::new();
        // First activation
        let events1 = parser.parse(b"\x1b]1337;VirtualEnv=myenv\x1b\\");
        assert_eq!(events1.len(), 1);

        // Duplicate - should be ignored
        let events2 = parser.parse(b"\x1b]1337;VirtualEnv=myenv\x1b\\");
        assert_eq!(events2.len(), 0);
    }

    // ===========================================
    // Region filtering tests (parse_filtered)
    // ===========================================

    #[test]
    fn test_parse_filtered_output_only() {
        let mut parser = TerminalParser::new();
        // Just regular output text, no OSC sequences - should pass through
        let result = parser.parse_filtered(b"Hello, World!\n");
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.output, b"Hello, World!\n");
    }

    #[test]
    fn test_parse_filtered_suppresses_prompt() {
        let mut parser = TerminalParser::new();
        // PromptStart -> prompt text -> PromptEnd
        // The prompt text should be suppressed
        let result = parser.parse_filtered(b"\x1b]133;A\x07user@host:~$ \x1b]133;B\x07");
        assert_eq!(result.events.len(), 2);
        assert!(matches!(result.events[0], OscEvent::PromptStart));
        assert!(matches!(result.events[1], OscEvent::PromptEnd));
        // Prompt text "user@host:~$ " should be suppressed
        assert_eq!(result.output, b"");
    }

    #[test]
    fn test_parse_filtered_passes_through_input_region() {
        let mut parser = TerminalParser::new();
        // After PromptEnd (B), user types - this is the Input region
        // First set up the state: PromptStart -> PromptEnd
        parser.parse_filtered(b"\x1b]133;A\x07\x1b]133;B\x07");

        // Now user types "ls -la" and presses enter (CommandStart)
        // Input region output is now visible (includes PS2 continuation prompts)
        let result = parser.parse_filtered(b"ls -la\x1b]133;C;ls -la\x07");
        assert_eq!(result.events.len(), 1);
        if let OscEvent::CommandStart { command } = &result.events[0] {
            assert_eq!(command.as_deref(), Some("ls -la"));
        } else {
            panic!("Expected CommandStart");
        }
        // Input region is now visible (for PS2 prompts and shell output)
        assert_eq!(result.output, b"ls -la");
    }

    #[test]
    fn test_parse_filtered_shows_command_output() {
        let mut parser = TerminalParser::new();
        // Set up state: we're in Output region after CommandStart
        parser.parse_filtered(b"\x1b]133;C;ls\x07");

        // Command output should be visible
        let result = parser.parse_filtered(b"file1.txt\nfile2.txt\n");
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.output, b"file1.txt\nfile2.txt\n");
    }

    #[test]
    fn test_parse_filtered_full_lifecycle() {
        let mut parser = TerminalParser::new();

        // Full command lifecycle:
        // 1. Prompt (suppressed)
        let r1 = parser.parse_filtered(b"\x1b]133;A\x07user@host:~$ \x1b]133;B\x07");
        assert_eq!(r1.output, b""); // Prompt suppressed

        // 2. User input (visible - includes PS2 continuation prompts)
        let r2 = parser.parse_filtered(b"echo hello\x1b]133;C;echo hello\x07");
        assert_eq!(r2.output, b"echo hello"); // Input visible for PS2 prompts

        // 3. Command output (visible)
        let r3 = parser.parse_filtered(b"hello\n");
        assert_eq!(r3.output, b"hello\n"); // Output visible

        // 4. Command ends
        let r4 = parser.parse_filtered(b"\x1b]133;D;0\x07");
        assert_eq!(r4.events.len(), 1);
        assert!(matches!(
            r4.events[0],
            OscEvent::CommandEnd { exit_code: 0 }
        ));

        // 5. Next prompt (suppressed)
        let r5 = parser.parse_filtered(b"\x1b]133;A\x07user@host:~$ \x1b]133;B\x07");
        assert_eq!(r5.output, b""); // Prompt suppressed
    }

    #[test]
    fn test_parse_filtered_region_state_tracking() {
        let mut parser = TerminalParser::new();

        // Verify the region transitions are correct
        // Start in Output (default)
        assert_eq!(parser.performer.current_region, TerminalRegion::Output);

        parser.parse_filtered(b"\x1b]133;A\x07");
        assert_eq!(parser.performer.current_region, TerminalRegion::Prompt);

        parser.parse_filtered(b"\x1b]133;B\x07");
        assert_eq!(parser.performer.current_region, TerminalRegion::Input);

        parser.parse_filtered(b"\x1b]133;C\x07");
        assert_eq!(parser.performer.current_region, TerminalRegion::Output);

        parser.parse_filtered(b"\x1b]133;D;0\x07");
        assert_eq!(parser.performer.current_region, TerminalRegion::Output);
    }

    #[test]
    fn test_parse_filtered_handles_control_chars_in_output() {
        let mut parser = TerminalParser::new();
        // Ensure we're in Output region
        parser.parse_filtered(b"\x1b]133;C\x07");

        // Test that common control characters pass through
        let result = parser.parse_filtered(b"line1\r\nline2\tcolumn\n");
        assert_eq!(result.output, b"line1\r\nline2\tcolumn\n");
    }

    #[test]
    fn test_parse_filtered_suppresses_control_chars_in_prompt() {
        let mut parser = TerminalParser::new();
        // Enter Prompt region
        parser.parse_filtered(b"\x1b]133;A\x07");

        // Control characters in prompt should be suppressed too
        let result = parser.parse_filtered(b"prompt\r\n");
        assert_eq!(result.output, b"");
    }
}
