use std::path::PathBuf;

use chrono::{DateTime, Utc};
use xdg::BaseDirectories;

fn get_data_dir() -> PathBuf {
    BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
        .get_data_home()
        .expect("could not resolve XDG data home directory (set $XDG_DATA_HOME or $HOME)")
}

fn get_config_dir() -> PathBuf {
    BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
        .get_config_home()
        .expect("could not resolve XDG config home directory (set $XDG_CONFIG_HOME or $HOME)")
}

pub(crate) fn default_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

pub(crate) fn templates_dir() -> PathBuf {
    get_config_dir().join("templates")
}

pub(crate) fn typescript_dir() -> PathBuf {
    get_data_dir().join("typescripts")
}

pub(crate) fn recordings_dir() -> PathBuf {
    get_data_dir().join("recordings")
}

pub(crate) fn new_typescript_path(name: &String, ts: &DateTime<Utc>) -> PathBuf {
    let file_name = format!("{}-{}", ts.timestamp(), name);
    typescript_dir().join(file_name)
}

pub(crate) fn new_recording_path(name: &String, ts: &DateTime<Utc>) -> PathBuf {
    let file_name = format!("{}-{}.json", ts.timestamp(), name);
    recordings_dir().join(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at_epoch() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn typescript_path_combines_timestamp_and_name() {
        let path = new_typescript_path(&"demo".to_owned(), &at_epoch());

        assert_eq!(path.file_name().unwrap(), "1700000000-demo");
        assert!(path.starts_with(typescript_dir()));
    }

    #[test]
    fn recording_path_combines_timestamp_name_and_json() {
        let path = new_recording_path(&"demo".to_owned(), &at_epoch());

        assert_eq!(path.file_name().unwrap(), "1700000000-demo.json");
        assert!(path.starts_with(recordings_dir()));
    }

    #[test]
    fn names_containing_dashes_are_kept() {
        let path = new_typescript_path(&"my-session".to_owned(), &at_epoch());

        assert_eq!(path.file_name().unwrap(), "1700000000-my-session");
    }

    #[test]
    fn different_timestamps_produce_different_files() {
        let a = new_recording_path(&"demo".to_owned(), &at_epoch());
        let b = new_recording_path(
            &"demo".to_owned(),
            &Utc.timestamp_opt(1_700_000_001, 0).unwrap(),
        );

        assert_ne!(a, b);
    }
}
