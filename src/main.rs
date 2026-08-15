use clap::Parser;

use termrec::cli::Cli;
use termrec::commands;

fn main() {
    // Parse the command line (clap prints help/errors and exits as needed),
    // then hand off to the chosen subcommand.
    let cli = Cli::parse();

    if let Err(err) = commands::dispatch(cli) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
