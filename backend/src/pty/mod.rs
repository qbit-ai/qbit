mod manager;
mod parser;
mod shell;

#[allow(unused_imports)]
pub use manager::{PtyManager, PtySession};
// Parser types are used internally by manager
#[allow(unused_imports)]
pub use parser::{OscEvent, TerminalParser};
// Shell detection types
#[allow(unused_imports)]
pub use shell::{detect_shell, ShellInfo, ShellType};
