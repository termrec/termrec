use std::fmt::Display;

use alacritty_terminal::Term;
use alacritty_terminal::event::EventListener;
use alacritty_terminal::term::Config;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::vte::ansi::Processor;
use alacritty_terminal::vte::ansi::StdSyncHandler;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::transcript::parser::Commands;
use crate::transcript::parser::TypescriptParser;

struct NullListener;

impl EventListener for NullListener {}

/// One recorded command: the command line plus its rendered output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub command: String,
    pub output: String,
}

/// A reconstructed recording: an ordered list of command sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub name: String,
    pub started_ts: DateTime<Utc>,
    pub sections: Vec<Section>,
}

impl Display for Transcript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Transcript {
    /// Replay a raw recording through the terminal emulator, splitting it
    /// into one section per command.
    pub fn from_recording(
        data: &[u8],
        name: String,
        started_ts: DateTime<Utc>,
        cmd_width: usize,
        cmd_height: usize,
    ) -> Self {
        let sections = Commands::new(data)
            .map(|command| Section {
                command: command.command.to_owned(),
                output: render(command.output, cmd_width, cmd_height),
            })
            .collect();

        Self {
            sections,
            name,
            started_ts,
        }
    }
}

fn render(segment: &[u8], cmd_width: usize, cmd_height: usize) -> String {
    let size = TermSize::new(cmd_width, cmd_height);

    let mut term: Term<NullListener> = Term::new(Config::default(), &size, NullListener);
    let processor = Processor::<StdSyncHandler>::new();

    let mut parser = TypescriptParser::new(&mut term, processor);

    parser.parse(segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(data: &[u8]) -> Transcript {
        Transcript::from_recording(data, "test".to_owned(), Utc::now(), 80, 1000)
    }

    #[test]
    fn splits_recording_into_sections() {
        let data = b"\x1b]777;termrec;command;echo hi\x07hi\r\n\x1b]777;termrec;prompt\x07\x1b]777;termrec;command;echo bye\x07bye\r\n";

        let transcript = build(data);

        assert_eq!(transcript.name, "test");
        assert_eq!(transcript.sections.len(), 2);
        assert_eq!(transcript.sections[0].command, "echo hi");
        assert_eq!(transcript.sections[0].output, "hi");
        assert_eq!(transcript.sections[1].command, "echo bye");
        assert_eq!(transcript.sections[1].output, "bye");
    }

    #[test]
    fn empty_recording_has_no_sections() {
        let transcript = build(b"");
        assert!(transcript.sections.is_empty());
    }

    #[test]
    fn rendering_strips_ansi_colors() {
        let data = b"\x1b]777;termrec;command;echo red\x07\x1b[31mred\x1b[0m\r\n";

        let transcript = build(data);

        assert_eq!(transcript.sections[0].output, "red");
    }

    #[test]
    fn rendering_applies_carriage_return_overwrite() {
        // `one\rTWO` paints TWO over one, so only TWO remains visible.
        let data = b"\x1b]777;termrec;command;foo\x07one\rTWO\r\n";

        let transcript = build(data);

        assert_eq!(transcript.sections[0].output, "TWO");
    }

    #[test]
    fn rendering_applies_backspace_erasure() {
        // `abc\x08d` erases the c and writes d over it.
        let data = b"\x1b]777;termrec;command;foo\x07abc\x08d\r\n";

        let transcript = build(data);

        assert_eq!(transcript.sections[0].output, "abd");
    }

    #[test]
    fn rendering_handles_clear_screen() {
        let data = b"\x1b]777;termrec;command;foo\x07\x1b[2Jfresh\r\n";

        let transcript = build(data);

        assert_eq!(transcript.sections[0].output, "fresh");
    }

    #[test]
    fn rendering_joins_multiline_output() {
        let data = b"\x1b]777;termrec;command;foo\x07line one\r\nline two\r\n";

        let transcript = build(data);

        assert_eq!(transcript.sections[0].output, "line one\nline two");
    }

    #[test]
    fn rendering_uses_configured_terminal_size() {
        // A 4-column terminal wraps the word "abcdef" onto two lines.
        let data = b"\x1b]777;termrec;command;foo\x07abcdef\r\n";

        let transcript = Transcript::from_recording(data, "test".to_owned(), Utc::now(), 4, 1000);

        assert_eq!(transcript.sections[0].output, "abcd\nef");
    }

    #[test]
    fn transcript_serializes_and_deserializes() {
        let transcript = build(b"\x1b]777;termrec;command;ls\x07file\r\n");

        let json = serde_json::to_string(&transcript).unwrap();
        let round_tripped: Transcript = serde_json::from_str(&json).unwrap();

        assert_eq!(round_tripped.name, transcript.name);
        assert_eq!(round_tripped.sections.len(), transcript.sections.len());
        assert_eq!(round_tripped.sections[0].command, "ls");
        assert_eq!(round_tripped.sections[0].output, "file");
    }
}
