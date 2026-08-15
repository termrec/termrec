//! In-process end-to-end tests: raw recording bytes -> transcript -> document.
//!
//! These drive the full reconstruction pipeline through the library API the
//! same way `termrec render` does, without spawning processes.

use chrono::Utc;

use termrec::template::{create_env, template_recording};
use termrec::transcript::Transcript;

/// A raw typescript as the shell hooks would produce it: banner bytes, then
/// prompt/command markers around each command's output.
const RECORDING: &[u8] = b"shell banner bytes\r\n\
    $ \x1b]777;termrec;prompt\x07\
    \x1b]777;termrec;command;echo hi\x07hi\r\n\
    \x1b]777;termrec;prompt\x07\
    \x1b]777;termrec;command;ls -la\x07file1\r\nfile2\r\n\
    \x1b]777;termrec;prompt\x07";

/// Render a transcript with the default embedded markdown template.
fn render(transcript: &Transcript) -> String {
    let env = create_env();
    let template = env.get_template("markdown.jinja").unwrap();
    template_recording(transcript, &template, "$ ")
}

#[test]
fn full_pipeline_builds_a_markdown_document() {
    let transcript = Transcript::from_recording(RECORDING, "demo".to_owned(), Utc::now(), 80, 1000);

    // Reconstruction: banner bytes are dropped, markers split the commands.
    assert_eq!(transcript.name, "demo");
    assert_eq!(transcript.sections.len(), 2);
    assert_eq!(transcript.sections[0].command, "echo hi");
    assert_eq!(transcript.sections[0].output, "hi");
    assert_eq!(transcript.sections[1].command, "ls -la");
    assert_eq!(transcript.sections[1].output, "file1\nfile2");

    // Formatting: header + one fenced section per command.
    let document = render(&transcript);

    assert!(document.contains("> Termrec recording from"));
    assert!(document.contains("# demo"));
    assert!(document.contains("## Command 1"));
    assert!(document.contains("$ echo hi\nhi"));
    assert!(document.contains("## Command 2"));
    assert!(document.contains("$ ls -la\nfile1\nfile2"));
}

#[test]
fn pipeline_with_interactive_output_renders_final_screen_state() {
    // A progress-bar style command: the last write wins (and the erase
    // clears the rest of the line, as real progress bars do).
    const DATA: &[u8] = b"\x1b]777;termrec;command;progress\x07\
        compiling...\r\
        100%\x1b[K\r\n\
        \x1b]777;termrec;prompt\x07";

    let transcript = Transcript::from_recording(DATA, "demo".to_owned(), Utc::now(), 80, 1000);

    assert_eq!(transcript.sections.len(), 1);
    assert_eq!(transcript.sections[0].output, "100%");

    let document = render(&transcript);
    assert!(document.contains("$ progress\n100%"));
}

#[test]
fn pipeline_round_trips_through_the_stored_json() {
    let transcript = Transcript::from_recording(RECORDING, "demo".to_owned(), Utc::now(), 80, 1000);

    // This is the exact shape `run` persists and `render` reads back.
    let json = serde_json::to_string(&transcript).unwrap();
    let restored: Transcript = serde_json::from_str(&json).unwrap();

    let original = render(&transcript);
    let after_round_trip = render(&restored);
    assert_eq!(after_round_trip, original);
}
