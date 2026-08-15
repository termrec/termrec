mod render;
mod run;

use clap::Subcommand;

use crate::cli::Cli;
use crate::commands::render::RenderArguments;
use crate::commands::render::render;
use crate::commands::run::RunArguments;
use crate::commands::run::run;
use crate::config::Config;

/// termrec's subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Record a new terminal session.
    ///
    /// Spawns the recording shell, then replays the raw session through a
    /// terminal emulator to build a command-aware transcript and store it.
    Run(RunArguments),

    /// Render a previously recorded session.
    ///
    /// Picks a recording from the store and writes it out as a formatted
    /// document (Markdown by default).
    Render(RenderArguments),
}

/// Entry point after the top-level flags have been parsed.
///
/// Loads the config file (from `--config` or the default location) and
/// dispatches to the requested subcommand. Config settings act as fallbacks
/// for anything not provided on the command line.
pub fn dispatch(cli: Cli) -> std::io::Result<()> {
    let config = Config::load(cli.config);

    match cli.command {
        Some(Commands::Run(args)) => run(args, &config),
        Some(Commands::Render(args)) => render(args, &config),
        None => Ok(()),
    }
}
