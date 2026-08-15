use std::path::PathBuf;

use clap::Parser;

use crate::commands::Commands;

/// Command-line interface for termrec.
///
/// Top-level flags (`--config`, `--debug`) are parsed before the subcommand.
/// `arg_required_else_help` makes a bare `termrec` invocation print the help
/// text instead of silently doing nothing.
#[derive(Parser)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// Load settings from this TOML config file.
    ///
    /// When omitted, termrec looks for its config file in the platform's
    /// standard configuration directory.
    #[arg(short, long, value_name = "FILE")]
    pub(crate) config: Option<PathBuf>,

    /// Increase logging verbosity. May be repeated for more detail.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub(crate) debug: u8,

    /// The action to perform: record a session or render a recording.
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_subcommand_with_name() {
        let cli = Cli::try_parse_from(["termrec", "run", "demo"]).unwrap();

        match cli.command {
            Some(Commands::Run(args)) => assert_eq!(args.name, "demo"),
            other => panic!("expected run subcommand, got {other:?}"),
        }
    }

    #[test]
    fn run_without_name_is_rejected() {
        assert!(Cli::try_parse_from(["termrec", "run"]).is_err());
    }

    #[test]
    fn run_accepts_optional_dimension_flags() {
        let cli = Cli::try_parse_from(["termrec", "run", "demo", "--cmd-width", "120"]).unwrap();

        match cli.command {
            Some(Commands::Run(args)) => {
                assert_eq!(args.cmd_width, Some(120));
                assert_eq!(args.cmd_height, None);
            }
            other => panic!("expected run subcommand, got {other:?}"),
        }
    }

    #[test]
    fn parses_render_subcommand_with_flags_and_name() {
        let cli = Cli::try_parse_from([
            "termrec", "render", "demo", "--prompt", "> ", "--out", "out.md",
        ])
        .unwrap();

        match cli.command {
            Some(Commands::Render(args)) => {
                assert_eq!(args.name.as_deref(), Some("demo"));
                assert_eq!(args.prompt.as_deref(), Some("> "));
                assert_eq!(args.out_file_name.as_deref(), Some("out.md"));
            }
            other => panic!("expected render subcommand, got {other:?}"),
        }
    }

    #[test]
    fn config_flag_is_top_level() {
        let cli =
            Cli::try_parse_from(["termrec", "--config", "custom.toml", "run", "demo"]).unwrap();

        assert_eq!(cli.config, Some(PathBuf::from("custom.toml")));
        assert!(matches!(cli.command, Some(Commands::Run(_))));
    }

    #[test]
    fn debug_flag_counts_repetitions() {
        let cli = Cli::try_parse_from(["termrec", "-d", "-d", "run", "demo"]).unwrap();
        assert_eq!(cli.debug, 2);
    }

    #[test]
    fn bare_invocation_prints_help() {
        assert!(Cli::try_parse_from(["termrec"]).is_err());
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["termrec", "frobnicate"]).is_err());
    }
}
