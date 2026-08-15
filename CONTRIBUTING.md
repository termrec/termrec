# Contributing to termrec

Thanks for wanting to help out. This document explains how termrec works
under the hood, how the code is organized, and how to build and test it.

## How termrec works

termrec records a terminal session and reconstructs a *readable* transcript
from the raw bytes. There are two halves: a live recording phase and an
offline reconstruction phase.

### The problem

The canonical `script` output is a raw byte stream. Reading it means wading
through:

- ANSI escape sequences (`ESC[1;36m`, `ESC[2J`, `ESC[K`, ...)
- cursor movement and in-place redraws
- line-editing artifacts (backspaces, typed-then-erased characters)
- control characters (`\r`, `\b`, bell, ...)

A transcript should be *readable*. The information is all in the byte stream —
the hard part is reconstructing the final visible state from it.

### The core technique: replay through a real emulator

Instead of writing a fragile ANSI parser, termrec replays the raw bytes
through [`alacritty_terminal`](https://github.com/alacritty/alacritty)'s
`Term` — the same terminal emulator that powers Alacritty. The bytes are fed
through Alacritty's VTE ANSI processor, which executes every escape sequence
and paints a grid of cells. That grid *is* what the user saw.

This buys correctness for free: anything Alacritty renders correctly, termrec
transcribes correctly. There is no regex-ing a byte stream, no heuristic
"strip the escape codes" pass — the emulator already did all that work.

### Command boundaries via OSC 777 markers

A screen render is continuous. To turn it into a *transcript* — one section
per command — we need to know where one command ends and the next begins.

The recording hooks print OSC 777 sequences into the stream at exactly the
right moments using each shell's preexec/precmd equivalent:

- `\033]777;termrec;command;<command>\007` — right before a command executes
- `\033]777;termrec;prompt\007` — when a new prompt is about to be drawn

The hooks ship as separate projects — [termrec.sh](https://github.com/termrec/termrec.sh)
(bash), [termrec.zsh](https://github.com/termrec/termrec.zsh) (zsh) and
[termrec.fish](https://github.com/termrec/termrec.fish) (fish) — each with its
own contributing guide covering the shell-specific wiring (the bash `DEBUG`
trap and `PROMPT_COMMAND`, zsh `preexec`/`precmd`, fish events). All three
share the same functions and markers: `termrec_active` guards on
`$TERMREC_ACTIVE`, `_termrec_precmd` emits the prompt marker, and
`_termrec_preexec` emits the command marker.

From the terminal's perspective these are just more bytes: they scroll by
without disturbing the display. Reconstruction walks the raw bytes and does
two things with each `command` marker:

1. **Command text** — the marker *is* the command: `;command;<command>\007`.
   The text between the marker and its terminating `\007` is the bare command
   line, taken verbatim. Bytes before the first marker (shell banner, anything
   typed before the hook was sourced) are dropped.
2. **Output** — the bytes *after* the marker, up to the next `prompt` marker,
   are exactly what the command printed (the typed echo happens before
   preexec fires). Replaying those bytes through a fresh `Term` yields pure
   output with no prompt and no echo.

Why use the `command` marker and not the `prompt` marker as the boundary? The
command marker sits precisely at the moment the command starts and carries the
command text, so it gives both the heading *and* a clean cut point: everything
after it belongs to this command. The trailing `prompt` marker simply marks
where the next command's stream begins.

OSC 777 is a well-established convention for custom terminal messaging (used
by `iterm2` for its own protocols); it's rendered as nothing or a subtle
notification, never part of the screen content.

### Why a hook at all?

The markers must come from inside the recording session — termrec itself can't
see what the user is typing (it only inherited a tty). A shell hook has
perfect timing knowledge: it fires exactly when a command starts. The same
trick works for any shell that exposes preexec/precmd hooks.

### Active-session detection

`TERMREC_ACTIVE` is exported into the recording session. termrec refuses to
start if it's already set, preventing accidental recursive recordings. All
three hooks guard on `$TERMREC_ACTIVE` (via `termrec_active`), so they stay
inert outside a recording.

## Codebase overview

The core pipeline (raw recording → transcript → formatted document) lives in
the library (`src/lib.rs`) so it can be tested in-process. The binary
(`src/main.rs`) is a thin wrapper that parses the command line and dispatches.

| Module | Responsibility |
| ------ | -------------- |
| `main.rs` | thin wrapper: parse CLI, dispatch, report errors |
| `cli.rs` | clap definitions for the whole CLI surface |
| `commands/run.rs` | the `run` subcommand: record + replay + store |
| `commands/render.rs` | the `render` subcommand: pick recording + format + write |
| `config.rs` | TOML config loading and precedence (flag > file > default) |
| `paths.rs` | XDG data/config dirs and timestamped file names |
| `recorder.rs` | spawn `script -q`, inherit the tty, set `TERMREC_ACTIVE` |
| `transcript/parser.rs` | marker splitting (`Commands`) + grid → text (`TypescriptParser`) |
| `transcript/builder.rs` | transcript document assembly (`Transcript`/`Section`) |
| `template.rs` | minijinja environment, embedded + user templates, rendering |
| `picker.rs` | interactive fuzzy picker for recordings and templates |
| `ui.rs` | colored status lines to stderr (plain when piped) |

## Development

### Building

```sh
cargo build
```

### Testing

```sh
cargo test
```

The test suite is layered:

- **Unit tests** alongside the code (`cli.rs`, `config.rs`, `paths.rs`,
  `transcript/parser.rs`, `transcript/builder.rs`) — pure, fast, no I/O.
- **Integration tests** (`tests/pipeline.rs`) — drive the full pipeline
  through the library API in-process.
- **End-to-end tests** (`tests/cli.rs`) — exercise the compiled binary against
  scratch XDG dirs (`$XDG_DATA_HOME`/`$XDG_CONFIG_HOME` point at temp dirs), so
  they never touch your real recordings.

### Conventions

- Keep the pipeline (record → build → format) testable in-process; add
  behavior through the library, not the binary.
- Status messages go to stderr via `ui.rs`; stdout stays clean for data so
  `> file` redirection never captures them.
- ANSI colors are only emitted when stderr is a terminal.

## Where things are headed

- **Semantic event stream** — model terminal activity as discrete events
  (`Print`, `Newline`, `ClearScreen`, `Prompt`, ...) instead of a flat
  `# Command N` list. This is the foundation for richer transcripts:
  distinguishing prompts from commands from output, skipping
  interactive-cleanup artifacts, or syntax-highlighting command output.
- **More output backends** — today a `Transcript` is rendered through Jinja
  templates (Markdown ships built in). Decoupling parse from format keeps it
  open to HTML, JSON or plain text.
- **Richer CLI options** — command length limits, custom output paths and
  formats, playback speed, paging large transcripts.
- **More shells** — the marker technique is shell-agnostic; only the hook
  differs.
