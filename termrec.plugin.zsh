# termrec hook for zsh.
#
# Source it from ~/.zshrc:
#   source /path/to/termrec.zsh/termrec.plugin.zsh
#
# Emits OSC 777 markers so termrec can reconstruct a command-aware transcript:
#   \033]777;termrec;command;<command>\007  before a command runs
#   \033]777;termrec;prompt\007            before a new prompt is drawn

# True when running inside a termrec recording. TERMREC_ACTIVE is exported
# by the `script` process that termrec spawns, so the hook stays inert in a
# normal shell.
termrec_active() {
    [[ -n "$TERMREC_ACTIVE" ]]
}

# precmd runs right before each prompt is drawn — the perfect moment to mark
# the end of the previous command cycle.
_termrec_precmd() {
    if termrec_active; then
        printf "\033]777;termrec;prompt\007"
    fi
}

# preexec runs right after a command is accepted but before it executes, with
# the expanded command line as $1. Emitting the command marker here places it
# before the command's output.
_termrec_preexec() {
    if termrec_active; then
        printf "\033]777;termrec;command;%s\007" "$1"
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec _termrec_preexec
add-zsh-hook precmd _termrec_precmd

# Marks that the termrec support hook is enabled in this shell.
export TERMREC_SUPPORT_ENABLED=1
