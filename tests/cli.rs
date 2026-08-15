//! End-to-end tests that exercise the compiled binary.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// A scratch directory for a single test, removed on drop.
struct TestDir(PathBuf);

impl TestDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("termrec-e2e-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Run the binary with the given arguments, environment overrides, and stdin.
fn run_with(args: &[&str], envs: &[(&str, String)], stdin: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_termrec"));
    cmd.args(args)
        // Never inherit a recording context from the test runner.
        .env_remove("TERMREC_ACTIVE")
        .env_remove("TERMREC_SUPPORT_ENABLED")
        .envs(envs.iter().map(|(k, v)| (k.to_owned(), v.clone())))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn data_home(dir: &TestDir) -> String {
    dir.path().join("data").to_str().unwrap().to_owned()
}

fn config_home(dir: &TestDir) -> String {
    dir.path().join("config").to_str().unwrap().to_owned()
}

/// XDG dirs pointing at `dir`'s scratch data and config locations.
fn xdg_env(dir: &TestDir) -> Vec<(&'static str, String)> {
    vec![
        ("XDG_DATA_HOME", data_home(dir)),
        ("XDG_CONFIG_HOME", config_home(dir)),
    ]
}

/// Write a single stored recording into the XDG data dir of `dir`.
fn write_recording(dir: &TestDir, name: &str, command: &str, output: &str) {
    let recordings = dir.path().join("data").join("termrec").join("recordings");
    fs::create_dir_all(&recordings).unwrap();

    let json = format!(
        "{{\"started_ts\":\"2026-08-09T12:00:00Z\",\
         \"sections\":[{{\"command\":\"{command}\",\"output\":\"{output}\"}}],\
         \"name\":\"{name}\"}}"
    );
    fs::write(recordings.join(format!("1-{name}.json")), json).unwrap();
}

// ---------------------------------------------------------------------------
// CLI surface
// ---------------------------------------------------------------------------

#[test]
fn help_exits_zero_and_describes_subcommands() {
    let out = run_with(&["--help"], &[], "");

    assert!(out.status.success());
    let text = stdout(&out);
    assert!(text.contains("Usage: termrec"));
    assert!(text.contains("run"));
    assert!(text.contains("render"));
}

#[test]
fn bare_invocation_prints_help_and_exits_two() {
    let out = run_with(&[], &[], "");

    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("Usage: termrec"));
}

#[test]
fn version_flag_prints_version() {
    let out = run_with(&["--version"], &[], "");

    assert!(out.status.success());
    assert!(stdout(&out).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_subcommand_fails() {
    let out = run_with(&["frobnicate"], &[], "");

    assert!(!out.status.success());
    assert!(stderr(&out).contains("unrecognized subcommand") || stderr(&out).contains("error"));
}

// ---------------------------------------------------------------------------
// Guards
// ---------------------------------------------------------------------------

#[test]
fn run_inside_recording_is_rejected() {
    let out = run_with(&["run", "demo"], &[("TERMREC_ACTIVE", "1".to_owned())], "");

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Already running inside a termrec session"));
}

#[test]
fn run_without_shell_hook_is_rejected() {
    let out = run_with(&["run", "demo"], &[], "");

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("shell support hook"));
}

// ---------------------------------------------------------------------------
// `run` recording
// ---------------------------------------------------------------------------

#[test]
fn run_records_a_session_and_stores_typescript_and_recording() {
    let dir = TestDir::new("run-record");
    let mut env = xdg_env(&dir);
    env.push(("TERMREC_SUPPORT_ENABLED", "1".to_owned()));

    let out = run_with(&["run", "e2e-session"], &env, "echo recorded line\nexit\n");

    assert!(out.status.success(), "stderr:\n{}", stderr(&out));

    let typescripts = dir.path().join("data").join("termrec").join("typescripts");
    let typescript_files: Vec<PathBuf> = fs::read_dir(&typescripts)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(typescript_files.len(), 1);
    assert!(!fs::read(&typescript_files[0]).unwrap().is_empty());

    let recordings = dir.path().join("data").join("termrec").join("recordings");
    let recording_files: Vec<PathBuf> = fs::read_dir(&recordings)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(recording_files.len(), 1);

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&recording_files[0]).unwrap()).unwrap();
    assert_eq!(json["name"], "e2e-session");
    // No shell hooks inside the piped session, so no command markers: the
    // transcript is empty but valid.
    assert_eq!(json["sections"], serde_json::json!([]));
}

// ---------------------------------------------------------------------------
// `render`
// ---------------------------------------------------------------------------

#[test]
fn render_without_recordings_fails_cleanly() {
    let dir = TestDir::new("render-empty");

    let out = run_with(
        &["render", "demo"],
        &[("XDG_DATA_HOME", data_home(&dir))],
        "",
    );

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("No recordings found. Record one first with `termrec run <name>`.")
    );
}

#[test]
fn render_unknown_recording_name_fails() {
    let dir = TestDir::new("render-missing");
    write_recording(&dir, "demo", "ls", "out");
    let env = xdg_env(&dir);

    let out = run_with(&["render", "nope"], &env, "");

    assert!(!out.status.success());
    assert!(stderr(&out).contains("No recording named 'nope'"));
}

#[test]
fn render_writes_markdown_for_named_recording() {
    let dir = TestDir::new("render-md");
    write_recording(&dir, "demo", "ls", "one\\nfile");
    let out_path = dir.path().join("out.md");
    let env = xdg_env(&dir);

    let out = run_with(
        &[
            "render",
            "demo",
            "--template",
            "markdown",
            "-o",
            out_path.to_str().unwrap(),
        ],
        &env,
        "",
    );

    assert!(out.status.success(), "stderr:\n{}", stderr(&out));

    let document = fs::read_to_string(&out_path).unwrap();
    assert!(document.contains("# demo"));
    assert!(document.contains("$ ls"));
    assert!(document.contains("one"));
    assert!(document.contains("file"));
    assert!(document.contains("## Command 1"));
}

#[test]
fn render_prompt_flag_overrides_everything() {
    let dir = TestDir::new("render-prompt-flag");
    write_recording(&dir, "demo", "ls", "out");
    let out_path = dir.path().join("out.md");
    let env = xdg_env(&dir);

    // Config file sets a prompt too, but the flag must win.
    let config_dir = dir.path().join("config").join("termrec");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), "prompt = \"cfg> \"\n").unwrap();

    let out = run_with(
        &[
            "render",
            "demo",
            "--prompt",
            "flag> ",
            "--template",
            "markdown",
            "-o",
            out_path.to_str().unwrap(),
        ],
        &env,
        "",
    );

    assert!(out.status.success());
    let document = fs::read_to_string(&out_path).unwrap();
    assert!(document.contains("flag> ls"));
    assert!(!document.contains("cfg> ls"));
}

#[test]
fn render_uses_config_file_via_explicit_path() {
    let dir = TestDir::new("render-config-flag");
    write_recording(&dir, "demo", "ls", "out");
    let out_path = dir.path().join("out.md");
    let env = xdg_env(&dir);

    let config_path = dir.path().join("custom.toml");
    fs::write(&config_path, "prompt = \">>> \"\n").unwrap();

    let out = run_with(
        &[
            "--config",
            config_path.to_str().unwrap(),
            "render",
            "demo",
            "--template",
            "markdown",
            "-o",
            out_path.to_str().unwrap(),
        ],
        &env,
        "",
    );

    assert!(out.status.success(), "stderr:\n{}", stderr(&out));
    let document = fs::read_to_string(&out_path).unwrap();
    assert!(document.contains(">>> ls"));
}

#[test]
fn render_uses_config_from_xdg_default_location() {
    let dir = TestDir::new("render-xdg-config");
    write_recording(&dir, "demo", "ls", "out");
    let out_path = dir.path().join("out.md");
    let env = xdg_env(&dir);

    // Config placed in the default location (XDG_CONFIG_HOME/termrec).
    let config_dir = dir.path().join("config").join("termrec");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), "prompt = \"xdg> \"\n").unwrap();

    let out = run_with(
        &[
            "render",
            "demo",
            "--template",
            "markdown",
            "-o",
            out_path.to_str().unwrap(),
        ],
        &env,
        "",
    );

    assert!(out.status.success(), "stderr:\n{}", stderr(&out));
    let document = fs::read_to_string(&out_path).unwrap();
    assert!(document.contains("xdg> ls"));
}

#[test]
fn render_template_flag_selects_a_template_without_prompting() {
    let dir = TestDir::new("render-template-flag");
    write_recording(&dir, "demo", "ls", "out");
    let out_path = dir.path().join("out.md");
    let env = xdg_env(&dir);

    let out = run_with(
        &[
            "render",
            "demo",
            "--template",
            "markdown",
            "-o",
            out_path.to_str().unwrap(),
        ],
        &env,
        "",
    );

    assert!(out.status.success(), "stderr:\n{}", stderr(&out));
    let document = fs::read_to_string(&out_path).unwrap();
    assert!(document.contains("# demo"));
    assert!(document.contains("## Command 1"));
    assert!(document.contains("$ ls"));
}

#[test]
fn render_template_flag_uses_a_user_configured_template() {
    let dir = TestDir::new("render-template-user");
    write_recording(&dir, "demo", "ls", "out");

    // A custom template shipped in the user's config directory.
    let templates = dir.path().join("config").join("termrec").join("templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("custom.txt"), "REC: {{ recording.name }}\n").unwrap();

    let out_path = dir.path().join("out.txt");
    let env = xdg_env(&dir);

    let out = run_with(
        &[
            "render",
            "demo",
            "--template",
            "custom",
            "-o",
            out_path.to_str().unwrap(),
        ],
        &env,
        "",
    );

    assert!(out.status.success(), "stderr:\n{}", stderr(&out));
    let document = fs::read_to_string(&out_path).unwrap();
    assert_eq!(document, "REC: demo");
}

#[test]
fn render_unknown_template_fails() {
    let dir = TestDir::new("render-template-missing");
    write_recording(&dir, "demo", "ls", "out");
    let env = xdg_env(&dir);

    let out = run_with(&["render", "demo", "--template", "nope"], &env, "");

    assert!(!out.status.success());
    assert!(stderr(&out).contains("No template named 'nope'"));
}
