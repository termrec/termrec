use std::{fmt::Display, fs, path::PathBuf};

use chrono::{DateTime, Utc};
use minijinja::{Environment, Error, Template, context, value::Value, value::ViaDeserialize};

use crate::{paths::templates_dir, transcript::Transcript};

pub(crate) struct RecordingTemplate<'a> {
    pub(crate) template: Template<'a, 'a>,
}

impl<'a> RecordingTemplate<'a> {
    pub(crate) fn name(&self) -> String {
        let mut t = PathBuf::from(self.template.name());
        t.set_extension("");

        t.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("<Unknown>")
            .to_string()
    }
}

impl<'a> Display for RecordingTemplate<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

fn date_format(
    value: ViaDeserialize<DateTime<Utc>>,
    format: Option<String>,
) -> Result<String, Error> {
    let fmt = format.unwrap_or_else(|| "%Y-%m-%d %H:%M %Z".to_string());

    Ok(value.format(&fmt).to_string())
}

/// Serialize a value as a JSON string (for machine-readable templates).
fn to_json(value: &Value) -> Result<String, Error> {
    serde_json::to_string(value)
        .map_err(|e| Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string()))
}

pub fn create_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_filter("date_format", date_format);
    env.add_filter("tojson", to_json);

    minijinja_embed::load_templates!(&mut env);

    if let Ok(entries) = fs::read_dir(templates_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else {
                continue;
            };
            let Ok(source) = fs::read_to_string(&path) else {
                eprintln!("failed to read template {}", path.display());
                continue;
            };
            env.add_template_owned(name, source)
                .unwrap_or_else(|e| eprintln!("failed to load template {}: {e}", path.display()));
        }
    }

    env
}

pub fn template_recording(recording: &Transcript, template: &Template, prompt: &str) -> String {
    template
        .render(context! { recording => recording, prompt => prompt })
        .unwrap()
}

pub(crate) fn get_templates<'a>(env: &'a Environment<'a>) -> Vec<RecordingTemplate<'a>> {
    env.templates()
        .map(|x| RecordingTemplate { template: x.1 })
        .collect()
}
