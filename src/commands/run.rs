use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;

use chrono::Utc;
use clap::Args;

use crate::config::Config;
use crate::paths::new_recording_path;
use crate::paths::new_typescript_path;
use crate::paths::recordings_dir;
use crate::paths::typescript_dir;
use crate::recorder;
use crate::transcript::Transcript;
use crate::ui;

/// Arguments for the `run` subcommand.
#[derive(Debug, Args)]
pub struct RunArguments {
    /// Name of the recording.
    ///
    /// Used as the base name for the stored files and as the transcript's
    /// title. Also exported to the recording session for external tools
    /// (e.g. a prompt indicator) via `TERMREC_RECORDING_NAME`.
    pub(crate) name: String,

    /// Terminal height (in rows) used when replaying the recording.
    ///
    /// Overrides the config file value; defaults to 1000.
    #[arg(long)]
    pub(crate) cmd_height: Option<usize>,

    /// Terminal width (in columns) used when replaying the recording.
    ///
    /// Overrides the config file value; defaults to 80.
    #[arg(long)]
    pub(crate) cmd_width: Option<usize>,
}

/// Record a terminal session and turn it into a transcript.
pub(crate) fn run(args: RunArguments, config: &Config) -> io::Result<()> {
    // Refuse to record inside an existing recording so sessions never nest.
    if env::var_os("TERMREC_ACTIVE").is_some() {
        ui::error("Already running inside a termrec session.");
        ui::info("Run `termrec run` from a plain terminal instead.");

        std::process::exit(1);
    }

    // The shell hooks are what draw the command-boundary markers, so a
    // recording is only meaningful when one of them is installed.
    if env::var_os("TERMREC_SUPPORT_ENABLED").is_none() {
        ui::error("termrec requires its shell support hook to be installed.");
        ui::detail("Source the hook for your shell and add it to your startup file:");
        ui::detail("  bash: https://github.com/termrec/termrec.sh");
        ui::detail("  zsh:  https://github.com/termrec/termrec.zsh");
        ui::detail("  fish: https://github.com/termrec/termrec.fish");

        std::process::exit(1);
    }

    // Resolve the replay dimensions: CLI flag, then config file, then default.
    let cmd_width = config.cmd_width(args.cmd_width);
    let cmd_height = config.cmd_height(args.cmd_height);

    let started_ts = Utc::now();

    // Environment exported to the recording session:
    // - TERMREC_ACTIVE: lets the shell hooks know they're inside a recording.
    // - TERMREC_RECORDING_NAME: the recording's name, for external tools.
    let mut env = HashMap::new();
    env.insert(String::from("TERMREC_ACTIVE"), String::from("1"));
    env.insert(String::from("TERMREC_RECORDING_NAME"), args.name.clone());

    fs::create_dir_all(recordings_dir()).expect("Failed to create recordings directory");
    fs::create_dir_all(typescript_dir()).expect("Failed to create typescripts directory");

    let typescript_path = new_typescript_path(&args.name, &started_ts);

    ui::info(&format!(
        "Recording \"{}\". Type `exit` or press Ctrl-D to stop.",
        args.name
    ));

    // Spawn `script` with the terminal attached; every byte the session
    // prints lands in the typescript file.
    let mut child = recorder::start(&typescript_path, env)?;

    // Block until the user exits the recording shell.
    child.wait()?;

    let elapsed = (Utc::now() - started_ts).num_seconds();
    let recorded_data = std::fs::read(&typescript_path)?;

    ui::success("Session captured.");

    // Replay the raw bytes through the terminal emulator and split them into
    // one section per command.
    let transcript =
        Transcript::from_recording(&recorded_data, args.name, started_ts, cmd_width, cmd_height);

    // Persist the transcript so `render` can format it at any time.
    let recording_path = new_recording_path(&transcript.name, &started_ts);

    let document: String = serde_json::to_string(&transcript)?;
    fs::write(&recording_path, document)?;

    let count = transcript.sections.len();
    let plural = if count == 1 { "" } else { "s" };

    ui::detail(&format!("Typescript: {}", typescript_path.display()));
    ui::detail(&format!("Recording:  {}", recording_path.display()));
    ui::success(&format!(
        "Saved \"{}\": {} command{} captured in {}.",
        transcript.name,
        count,
        plural,
        format_duration(elapsed),
    ));

    ui::info("You can render your recording using \"termrec render\".");

    Ok(())
}

/// Format a duration in seconds as a compact human string (`42s`, `1m 12s`).
fn format_duration(seconds: i64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    if minutes == 0 {
        format!("{secs}s")
    } else {
        format!("{minutes}m {secs}s")
    }
}
