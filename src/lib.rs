//! termrec: record a terminal session and reconstruct a readable transcript.
//!
//! The core pipeline (raw recording -> transcript -> formatted document)
//! lives in this library so it can be tested in-process. The binary target
//! is a thin wrapper that parses the command line and dispatches.

pub mod cli;
pub mod commands;
pub mod template;
pub mod transcript;

mod config;
mod paths;
mod picker;
mod recorder;
mod ui;
