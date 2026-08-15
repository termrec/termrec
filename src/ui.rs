//! User-facing status messages.
//!
//! Status lines always go to stderr so stdout stays clean for data (and so
//! `> file` redirection never captures them). ANSI colors are only emitted
//! when stderr is a terminal, so piped output stays plain and test-friendly.

use std::io::IsTerminal;

/// Whether stderr is a terminal (and we can safely use ANSI colors).
fn use_color() -> bool {
    std::io::stderr().is_terminal()
}

/// Wrap `text` in an ANSI color code, or leave it plain when piped.
fn paint(code: &str, text: &str) -> String {
    if use_color() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// An informational status line (e.g. "recording started").
pub(crate) fn info(message: &str) {
    eprintln!("{} {}", paint("36", "→"), message);
}

/// A success status line (e.g. "saved 3 commands").
pub(crate) fn success(message: &str) {
    eprintln!("{} {}", paint("32", "✓"), message);
}

/// A warning status line.
pub(crate) fn warn(message: &str) {
    eprintln!("{} {}", paint("33", "!"), message);
}

/// An error status line.
pub(crate) fn error(message: &str) {
    eprintln!("{} {}", paint("31", "✗"), message);
}

/// A muted detail line, typically indented and used for file paths.
pub(crate) fn detail(message: &str) {
    eprintln!("  {}", paint("90", message));
}
