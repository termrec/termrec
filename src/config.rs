//! Runtime configuration for termrec.
//!
//! A setting can come from three places, in order of precedence:
//!   1. a command-line flag (e.g. `--prompt`, `--cmd-width`)
//!   2. a TOML config file
//!   3. a built-in default
//!
//! The config file is optional. When one is not given on the command line,
//! termrec looks for it in the platform's standard configuration directory.

use std::path::PathBuf;

use serde::Deserialize;

use crate::paths::default_config_path;

/// Termrec settings.
///
/// Every field is optional: a missing field simply means "use the built-in
/// default" (or the value of the matching command-line flag, which wins).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    /// Prompt printed in front of each command in a rendered transcript.
    prompt: Option<String>,

    /// Terminal width (in columns) used when replaying a recording.
    cmd_width: Option<usize>,

    /// Terminal height (in rows) used when replaying a recording.
    cmd_height: Option<usize>,
}

impl Config {
    /// Load settings from a TOML config file.
    ///
    /// The file location is either the one passed on the command line
    /// (`--config`) or the default location under the platform's standard
    /// configuration directory. A missing or unparseable file is not an
    /// error: the resulting empty config simply falls back to defaults.
    pub(crate) fn load(cli_path: Option<PathBuf>) -> Self {
        let path = cli_path.unwrap_or_else(default_config_path);

        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Resolve the prompt string: CLI flag, then config file, then `$ `.
    pub(crate) fn prompt(&self, cli: Option<String>) -> String {
        cli.or(self.prompt.clone())
            .unwrap_or_else(|| String::from("$ "))
    }

    /// Resolve the replay width: CLI flag, then config file, then 80.
    pub(crate) fn cmd_width(&self, cli: Option<usize>) -> usize {
        cli.or(self.cmd_width).unwrap_or(80)
    }

    /// Resolve the replay height: CLI flag, then config file, then 1000.
    pub(crate) fn cmd_height(&self, cli: Option<usize>) -> usize {
        cli.or(self.cmd_height).unwrap_or(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway directory for one test's config file.
    struct ConfigDir(PathBuf);

    impl ConfigDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("termrec-config-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            ConfigDir(dir)
        }

        fn write(&self, content: &str) -> PathBuf {
            let path = self.0.join("config.toml");
            std::fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for ConfigDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn loads_settings_from_explicit_path() {
        let dir = ConfigDir::new("explicit");
        let path = dir.write("prompt = \">>> \"\ncmd_width = 120\ncmd_height = 40\n");

        let config = Config::load(Some(path));

        assert_eq!(config.prompt(None), ">>> ");
        assert_eq!(config.cmd_width(None), 120);
        assert_eq!(config.cmd_height(None), 40);
    }

    #[test]
    fn missing_config_file_falls_back_to_defaults() {
        let config = Config::load(Some("/nonexistent/termrec-config.toml".into()));

        assert_eq!(config.prompt(None), "$ ");
        assert_eq!(config.cmd_width(None), 80);
        assert_eq!(config.cmd_height(None), 1000);
    }

    #[test]
    fn malformed_config_falls_back_to_defaults() {
        let dir = ConfigDir::new("malformed");
        let path = dir.write("this is not [valid toml");

        let config = Config::load(Some(path));

        assert_eq!(config.prompt(None), "$ ");
        assert_eq!(config.cmd_width(None), 80);
    }

    #[test]
    fn empty_config_uses_defaults() {
        let dir = ConfigDir::new("empty");
        let path = dir.write("");

        let config = Config::load(Some(path));

        assert_eq!(config.prompt(None), "$ ");
        assert_eq!(config.cmd_width(None), 80);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let dir = ConfigDir::new("unknown");
        let path = dir.write("prompt = \"cfg\"\nfuture_option = 42\n");

        let config = Config::load(Some(path));

        assert_eq!(config.prompt(None), "cfg");
    }

    #[test]
    fn partial_config_defaults_missing_fields() {
        let dir = ConfigDir::new("partial");
        let path = dir.write("cmd_width = 120\n");

        let config = Config::load(Some(path));

        assert_eq!(config.cmd_width(None), 120);
        assert_eq!(config.cmd_height(None), 1000);
        assert_eq!(config.prompt(None), "$ ");
    }

    #[test]
    fn cli_flag_overrides_config_file() {
        let dir = ConfigDir::new("precedence");
        let path = dir.write("prompt = \"cfg\"\ncmd_width = 99\n");

        let config = Config::load(Some(path));

        // Flag provided: flag wins.
        assert_eq!(config.prompt(Some("flag".to_owned())), "flag");
        assert_eq!(config.cmd_width(Some(120)), 120);
        // Flag absent: config file wins.
        assert_eq!(config.prompt(None), "cfg");
        assert_eq!(config.cmd_width(None), 99);
    }
}
