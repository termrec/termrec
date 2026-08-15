use std::fs;
use std::io;

use clap::Args;
use minijinja::Environment;

use crate::config::Config;
use crate::paths::recordings_dir;
use crate::picker::pick_interactively;
use crate::template::RecordingTemplate;
use crate::template::create_env;
use crate::template::get_templates;
use crate::template::template_recording;
use crate::transcript::Transcript;
use crate::ui;

/// Arguments for the `render` subcommand.
#[derive(Debug, Args)]
pub struct RenderArguments {
    /// Prompt string shown in front of each command in the transcript.
    ///
    /// Overrides the config file value; defaults to `$ `.
    #[arg(short, long)]
    pub(crate) prompt: Option<String>,

    /// File to write the rendered transcript to.
    ///
    /// Logs to the stdout if not present.
    #[arg(short = 'o', long = "out")]
    pub(crate) out_file_name: Option<String>,

    /// Render with a specific template, skipping the interactive picker.
    ///
    /// Accepts the template's name (e.g. `markdown`); the matching template is
    /// loaded from the embedded defaults or `~/.config/termrec/templates/`.
    #[arg(long, value_name = "NAME")]
    pub(crate) template: Option<String>,

    /// Render a specific recording by name, skipping the interactive picker.
    ///
    /// When omitted, an interactive picker lists all stored recordings.
    #[arg(value_name = "NAME")]
    pub(crate) name: Option<String>,
}

/// Render a recorded session into a formatted document.
///
/// Either picks a specific recording by name or lets the user choose one from
/// an interactive list, then writes the formatted transcript (Markdown by
/// default) to the output file.
pub(crate) fn render(args: RenderArguments, config: &Config) -> io::Result<()> {
    // Load every stored recording.
    let dir = fs::read_dir(recordings_dir()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No recordings found. Record one first with `termrec run <name>`.",
        )
    })?;

    let mut recordings: Vec<Transcript> = vec![];

    for path in dir {
        let content: String = fs::read_to_string(path.unwrap().path()).unwrap();
        let recording: Transcript = serde_json::from_str(&content).unwrap();
        recordings.push(recording);
    }

    if recordings.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No recordings found. Record one first with `termrec run <name>`.",
        ));
    }

    let transcript: Transcript = match &args.name {
        Some(name) => recordings
            .into_iter()
            .find(|recording| &recording.name == name)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("No recording named '{name}'."),
                )
            })?,
        None => pick_interactively(recordings, "Recording"),
    };

    // Resolve the prompt: CLI flag, then config file, then default.
    let prompt = config.prompt(args.prompt);

    let env: Environment = create_env();

    let templates = get_templates(&env);

    let template: RecordingTemplate = match &args.template {
        Some(name) => templates
            .into_iter()
            .find(|candidate| candidate.name() == *name)
            .ok_or_else(|| {
                let available = get_templates(&env)
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("No template named '{name}'. Available templates: {available}."),
                )
            })?,
        None => pick_interactively(templates, "Template"),
    };

    let text = template_recording(&transcript, &template.template, &prompt);

    if let Some(out_file_name) = args.out_file_name {
        let out_file_path = std::path::Path::new(&out_file_name);
        if out_file_path.exists() {
            ui::warn(&format!("Overwriting existing file {}", out_file_name));
        }

        fs::write(&out_file_name, text).expect("Failed to write output file");

        let count = transcript.sections.len();
        let plural = if count == 1 { "" } else { "s" };
        ui::success(&format!(
            "Rendered \"{}\": {} command{} → {}",
            transcript.name, count, plural, out_file_name,
        ));
    } else {
        println!("{}", text);
    }

    Ok(())
}
