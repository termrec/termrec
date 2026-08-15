use alacritty_terminal::Term;
use alacritty_terminal::event::EventListener;
use alacritty_terminal::vte::ansi::Processor;

/// The full OSC sequence termrec's shell hooks emit before each command runs.
const COMMAND_MARKER: &[u8] = b"\x1b]777;termrec;command;";
/// The full OSC sequence emitted before each prompt is drawn.
const PROMPT_MARKER: &[u8] = b"\x1b]777;termrec;prompt";
/// The BEL character that terminates an OSC sequence.
const BEL: u8 = 0x07;

pub(crate) struct TypescriptParser<'a, T>
where
    T: EventListener,
{
    term: &'a mut Term<T>,
    parser: Processor,
}

impl<'a, T> TypescriptParser<'a, T>
where
    T: EventListener,
{
    pub(crate) fn new(term: &'a mut Term<T>, parser: Processor) -> Self {
        Self { term, parser }
    }

    /// Feed the raw bytes through the emulator and return the visible grid
    /// as plain text.
    pub(crate) fn parse(&mut self, data: &[u8]) -> String {
        self.parser.advance(self.term, data);
        let mut buffer = String::new();
        let mut current_row = None;

        for cell in self.term.grid().display_iter() {
            if current_row != Some(cell.point.line) {
                if current_row.is_some() {
                    buffer.truncate(buffer.trim_end().len());
                    buffer.push('\n');
                }

                current_row = Some(cell.point.line);
            }

            buffer.push(cell.c);
        }

        buffer.truncate(buffer.trim_end().len());
        buffer.trim().to_owned()
    }
}

/// A single command: its text (from the marker) and its raw output bytes.
pub(crate) struct Command<'a> {
    pub(crate) command: &'a str,
    pub(crate) output: &'a [u8],
}

/// Splits a raw typescript into commands on the OSC 777 markers.
///
/// Bytes before the first command marker (shell banner, setup) are dropped.
pub(crate) struct Commands<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Commands<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

impl<'a> Iterator for Commands<'a> {
    type Item = Command<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let rest = &self.data[self.pos..];

        let marker_offset = find_subslice(rest, COMMAND_MARKER)?;
        let command_start = marker_offset + COMMAND_MARKER.len();

        let marker_tail = &rest[command_start..];
        let command_len = marker_tail.iter().position(|&byte| byte == BEL)?;
        let command = std::str::from_utf8(&marker_tail[..command_len]).ok()?;

        let output_start = self.pos + command_start + command_len + 1;
        let output_rest = &self.data[output_start..];

        let prompt_offset = find_subslice(output_rest, PROMPT_MARKER);
        let command_offset = find_subslice(output_rest, COMMAND_MARKER);

        let (output, consumed) = match (prompt_offset, command_offset) {
            // A next command marker arrived before any prompt marker: its
            // start is the boundary.
            (Some(prompt), Some(next)) if next < prompt => (&output_rest[..next], next),
            // A prompt marker ends this command's output. Consume the marker
            // and its terminating BEL so the next segment starts clean.
            (Some(prompt), _) => {
                let mut end = prompt + PROMPT_MARKER.len();
                if output_rest.get(end) == Some(&BEL) {
                    end += 1;
                }
                (&output_rest[..prompt], end)
            }
            (None, Some(next)) => (&output_rest[..next], next),
            // Nothing else follows: everything left belongs to this command.
            (None, None) => (output_rest, output_rest.len()),
        };

        self.pos = output_start + consumed;

        Some(Command {
            command: command.trim_end(),
            output,
        })
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(data: &[u8]) -> Vec<(String, Vec<u8>)> {
        Commands::new(data)
            .map(|command| (command.command.to_owned(), command.output.to_vec()))
            .collect()
    }

    const CMD: &[u8] = b"\x1b]777;termrec;command;";
    const PROMPT: &[u8] = b"\x1b]777;termrec;prompt\x07";

    #[test]
    fn finds_marker_prefixes() {
        assert!(CMD.starts_with(b"\x1b]"));
        assert!(PROMPT.ends_with(&[0x07]));
    }

    #[test]
    fn splits_commands_and_output() {
        let data = b"\x1b]777;termrec;command;echo hi\x07hi\r\n\x1b]777;termrec;prompt\x07\x1b]777;termrec;command;echo bye\x07bye\r\n";

        let parsed = commands(data);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "echo hi");
        assert_eq!(parsed[0].1, b"hi\r\n");
        assert_eq!(parsed[1].0, "echo bye");
        assert_eq!(parsed[1].1, b"bye\r\n");
    }

    #[test]
    fn last_command_without_prompt_marker_takes_remaining_bytes() {
        let data = b"\x1b]777;termrec;command;ls\x07file1\r\nfile2\r\n";

        let parsed = commands(data);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "ls");
        assert_eq!(parsed[0].1, b"file1\r\nfile2\r\n");
    }

    #[test]
    fn empty_output_produces_empty_segment() {
        let data = b"\x1b]777;termrec;command;true\x07\x1b]777;termrec;prompt\x07\x1b]777;termrec;command;ls\x07x\r\n";

        let parsed = commands(data);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "true");
        assert!(parsed[0].1.is_empty());
        assert_eq!(parsed[1].0, "ls");
    }

    #[test]
    fn bytes_before_first_marker_are_dropped() {
        let data = b"shell banner\r\n$ setup stuff\r\n\x1b]777;termrec;command;ls\x07file\r\n";

        let parsed = commands(data);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "ls");
    }

    #[test]
    fn command_text_with_punctuation_and_whitespace_preserved() {
        let data = b"\x1b]777;termrec;command;echo \"a; b\" | tr ';' ','\x07x\r\n";

        let parsed = commands(data);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "echo \"a; b\" | tr ';' ','");
    }

    #[test]
    fn output_containing_marker_bytes_without_escape_is_not_a_boundary() {
        // A program printing a literal `]777;termrec;prompt` (no leading ESC)
        // must not be mistaken for the shell's prompt marker.
        let data = b"\x1b]777;termrec;command;cat x\x07the ]777;termrec;prompt marker\x1b]777;termrec;prompt\x07\x1b]777;termrec;command;next\x07y\r\n";

        let parsed = commands(data);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "cat x");
        assert_eq!(parsed[0].1, b"the ]777;termrec;prompt marker");
        assert_eq!(parsed[1].0, "next");
    }

    #[test]
    fn command_text_is_trimmed_of_trailing_whitespace() {
        let data = b"\x1b]777;termrec;command;ls  \x07file\r\n";

        let parsed = commands(data);

        assert_eq!(parsed[0].0, "ls");
    }

    #[test]
    fn no_markers_yields_no_commands() {
        let data = b"just some random output\r\nwith no markers";

        let parsed = commands(data);

        assert!(parsed.is_empty());
    }

    #[test]
    fn find_subslice_locates_needle() {
        assert_eq!(find_subslice(b"abcxyz", b"xy"), Some(3));
        assert_eq!(find_subslice(b"abc", b"xyz"), None);
        assert_eq!(find_subslice(b"aaa", b"aa"), Some(0));
    }
}
