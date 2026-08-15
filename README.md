# termrec

> Record a terminal session. Replay it through a real ANSI terminal emulator.
> Get back a clean, readable Markdown transcript of every command and its output.

![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)

`script` gives you a raw byte dump — escape codes, cursor movements, redraws
and control sequences all mixed together. Nobody wants to read that.

termrec records your session, then replays the raw bytes through Alacritty's
terminal emulator to reconstruct what you actually saw on screen. The result is
a structured transcript — one section per command, output rendered exactly as
it appeared.

## Example

Here's what a session looks like captured by the plain `script` utility:

```text
ESC[1;36m$ ESC[0mESC]777;termrec;command;ls -laESC7
total 96
drwxr-xr-x@ 13 oskar staff  416 Aug 11 13:54 .
drwxr-xr-x@ 17 oskar staff  544 Aug  6 10:16 ..
-rw-r--r--@  1 oskar staff  502 Aug 10 12:36 Cargo.toml
ESC]777;termrec;promptESC7ESC[1;36m$ ESC[0mESC]777;termrec;command;cargo buildESC7
ESC[1mESC[32m   CompilingESC[0m termrec v0.1.0
ESC[1mESC[32m    FinishedESC[0m dev [unoptimized + debuginfo] target(s) in 0.42s
```

And here's the same session after termrec reconstructs it:

````md
# demo

## Command 1

```
$ ls -la
total 96
drwxr-xr-x@ 13 oskar staff  416 Aug 11 13:54 .
drwxr-xr-x@ 17 oskar staff  544 Aug  6 10:16 ..
-rw-r--r--@  1 oskar staff  502 Aug 10 12:36 Cargo.toml
```

## Command 2

```
$ cargo build
Compiling termrec v0.1.0
    Finished dev profile [unoptimized + debuginfo] target(s) in 0.42s
```
````

Because reconstruction goes through a real terminal emulator, interactive
programs (editors, `top`, progress bars, line-editing prompts) are transcribed
from their final on-screen state — not their raw byte stream. A full example is
in [`examples/transcript.md`](examples/transcript.md).

## Features

- **Replay, don't parse** — output is rebuilt through Alacritty's terminal
  emulator, so what you get is what you saw. No regexing escape codes.
- **Interactive programs just work** — final screen state, not raw bytes.
- **Custom templates** — drop a Jinja template in your config dir to change the
  output format.
- **Shell hooks for bash, zsh, fish** — each in its own repository.

## Table of contents

- [Installation](#installation)
- [Setup: shell hooks](#setup-shell-hooks)
- [Quick start](#quick-start)
- [CLI reference](#cli-reference)
- [Configuration](#configuration)
- [Custom templates](#custom-templates)
- [Data locations](#data-locations)
- [Related projects](#related-projects)
- [Documentation](#documentation)
- [Roadmap](#roadmap)
- [License](#license)

## Installation

```sh
cargo install --git https://github.com/termrec/termrec
```

Dependencies: a Rust toolchain and the BSD `script` utility (built into macOS,
and available on most Unix-like systems).

## Setup: shell hooks

To split a recording into commands, termrec needs a small hook in your shell
that marks each command boundary. It's ignored outside a recording, so it's safe
to enable permanently.

Each hook lives in its own repository:

| Shell | Repository | Startup file |
| ----- | ---------- | ------------ |
| bash | [termrec.sh](https://github.com/termrec/termrec.sh) | `~/.bashrc` |
| zsh | [termrec.zsh](https://github.com/termrec/termrec.zsh) | `~/.zshrc` |
| fish | [termrec.fish](https://github.com/termrec/termrec.fish) | `~/.config/fish/config.fish` |

```sh
echo 'source /path/to/termrec.zsh/termrec.plugin.zsh' >> ~/.zshrc
```

Full details — including the env vars the hooks use — live in
[`docs/shell-hooks.md`](docs/shell-hooks.md).

## Quick start

Record a session:

```sh
$ termrec run demo
→ Recording "demo". Type `exit` or press Ctrl-D to stop.
$ ls -la
...
$ exit
✓ Session captured.
✓ Saved "demo": 3 commands captured in 42s.
→ You can render your recording using "termrec render".
```

Render it to Markdown:

```sh
$ termrec render demo -o demo.md
✓ Rendered "demo": 3 commands → demo.md
```

Omitting the recording name opens an interactive picker. Omitting `-o` prints
the transcript to stdout.

## CLI reference

| Command | Description |
| ------- | ----------- |
| `termrec run <name>` | Record a new terminal session. |
| `termrec render [name]` | Render a recording. Without a name, pick one interactively. |

### `termrec run`

| Flag | Description |
| ---- | ----------- |
| `<name>` | Recording name — used for the stored files and the transcript title. |
| `--cmd-width <columns>` | Terminal width used when replaying (default `80`). |
| `--cmd-height <rows>` | Terminal height used when replaying (default `1000`). |

### `termrec render`

| Flag | Description |
| ---- | ----------- |
| `[name]` | Recording to render. Omit for an interactive picker. |
| `-p, --prompt <prompt>` | Prompt printed before each command (default `$ `). |
| `-o, --out <file>` | Write to a file instead of stdout. |
| `--template <name>` | Render with a specific template, skipping the picker. |

### Global

| Flag | Description |
| ---- | ----------- |
| `-c, --config <file>` | Load settings from this TOML file. |
| `-d, --debug` | Increase logging verbosity (repeatable). |

## Configuration

termrec is configurable through a TOML file at
`~/.config/termrec/config.toml` (`$XDG_CONFIG_HOME/termrec/config.toml` when
set). It's picked up automatically — no setup required.

| Setting | Description | Default |
| ------- | ----------- | ------- |
| `prompt` | Prompt shown before each command in a transcript. | `$ ` |
| `cmd_width` | Terminal width used when replaying. | `80` |
| `cmd_height` | Terminal height used when replaying. | `1000` |

Precedence is **command-line flag > config file > built-in default**, so you
can set your favourite prompt in the config and still override it per run.

```toml
# ~/.config/termrec/config.toml
prompt = "❯ "
cmd_width = 120
cmd_height = 40
```

See [`examples/config.toml`](examples/config.toml) for an annotated version.

## Custom templates

Output is rendered from Jinja templates. Put a template in
`~/.config/termrec/templates/` and pick it with `--template <name>` (the name
is the filename without its extension).

```jinja
{# ~/.config/termrec/templates/plain.txt #}
Recording: {{ recording.name }}

{% for section in recording.sections %}
{{ prompt }}{{ section.command }}
{{ section.output }}
{% endfor %}
```

```sh
termrec render demo --template plain
```

Available template variables: `recording.name`, `recording.started_ts` (with
the `date_format` filter), `recording.sections[]` (`command`, `output`), and
`prompt`. Filters include `escape` and `tojson`. Every template ends with a
"Generated by termrec" attribution.

Built-in templates:

| Name | File | Output |
| ---- | ---- | ------ |
| `markdown` | `templates/markdown.jinja` | Clean Markdown, one section per command (default). |
| `html` | `templates/html.jinja` | Standalone HTML page, dark/light theme, copy buttons. |
| `plain` | `templates/plain.txt` | Minimal plain text, good for pasting into issues. |
| `json` | `templates/json.jinja` | Structured JSON for scripts and CI. |
| `slides` | `templates/slides.jinja` | Self-contained slide deck with keyboard navigation. |
| `blog` | `templates/blog.jinja` | Markdown with YAML frontmatter for static sites. |

```sh
termrec render demo --template html -o demo.html
termrec render demo --template json | jq '.sections'
```

## Data locations

| Path | Contents |
| ---- | -------- |
| `~/.local/share/termrec/recordings/` | Reconstructed transcripts (JSON). |
| `~/.local/share/termrec/typescripts/` | Raw session byte dumps. |
| `~/.config/termrec/config.toml` | Settings. |
| `~/.config/termrec/templates/` | Custom output templates. |

Respects `$XDG_DATA_HOME` and `$XDG_CONFIG_HOME`.

## Related projects

termrec is split into several repositories:

- [termrec.sh](https://github.com/termrec/termrec.sh) — bash hook.
- [termrec.zsh](https://github.com/termrec/termrec.zsh) — zsh hook.
- [termrec.fish](https://github.com/termrec/termrec.fish) — fish hook.
- [termrec.starship.toml](https://github.com/termrec/termrec.starship.toml) — a
  Starship prompt indicator that shows the current recording.

## Documentation

- [`docs/shell-hooks.md`](docs/shell-hooks.md) — shell hooks in depth.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how termrec works under the hood and
  how to build, test and extend it.
- [`examples/`](examples/) — sample config and rendered transcripts
  (Markdown and HTML).

## Roadmap

- **Richer CLI options** — command length limits, custom output formats.
- **Semantic event stream** — model terminal activity as events to produce
  richer transcripts (separate prompts, commands and output structurally).
- **More shells** — the marker technique is shell-agnostic; only the hook
  differs.

## License

[MIT](LICENSE).
