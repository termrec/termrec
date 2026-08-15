use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub(crate) fn start(path: &Path, envs: HashMap<String, String>) -> io::Result<Child> {
    Command::new("script")
        .arg("-q")
        .arg(path)
        .envs(envs)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}
