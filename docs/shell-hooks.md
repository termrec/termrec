# Shell hooks

termrec splits a recording into per-command sections using markers that a
small shell hook injects into the session. This guide covers installing the
hooks and the environment variables they use.

## Environment variables

termrec exports three variables into a recording session:

| Variable | Meaning |
| -------- | ------- |
| `TERMREC_ACTIVE` | Set to `1` while a recording is running. The hooks guard on this, so they do nothing outside a recording. |
| `TERMREC_RECORDING_NAME` | The recording's name, for external tools (e.g. a prompt indicator). |
| `TERMREC_SUPPORT_ENABLED` | Set by the hooks themselves. `termrec run` refuses to start until a hook is installed. |

## Installing a hook

Each shell's hook lives in its own repository:

| Shell | Repository |
| ----- | ---------- |
| bash | [termrec/termrec.sh](https://github.com/termrec/termrec.sh) |
| zsh | [termrec/termrec.zsh](https://github.com/termrec/termrec.zsh) |
| fish | [termrec/termrec.fish](https://github.com/termrec/termrec.fish) |

Add one `source` line to your shell's startup file:

**bash** — `~/.bashrc`:

```sh
source /path/to/termrec.sh/termrec.plugin.sh
```

**zsh** — `~/.zshrc`:

```sh
source /path/to/termrec.zsh/termrec.plugin.zsh
```

**fish** — install as a plugin with [fisher](https://github.com/jorgebucaran/fisher):

```fish
fisher install termrec/termrec.fish
```

or copy the plugin files into `~/.config/fish/`.

The hook is inert outside a recording: it only emits markers when
`TERMREC_ACTIVE` is set, and it exports `TERMREC_SUPPORT_ENABLED=1` so
`termrec run` knows it's available. Each hook works through the shell's
preexec/precmd mechanism (the `DEBUG` trap and `PROMPT_COMMAND` for bash) — see
[`CONTRIBUTING.md`](../CONTRIBUTING.md#command-boundaries-via-osc-777-markers)
for the details.

## Recording indicators

While a recording is running, `TERMREC_RECORDING_NAME` holds the session's
name. The [termrec/termrec.starship.toml](https://github.com/termrec/termrec.starship.toml)
project adds a Starship prompt indicator on top of it.
