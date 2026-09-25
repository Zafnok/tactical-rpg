//! Repo tooling commands. See CLAUDE.md.
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

use std::env;
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <command>\n\navailable commands:\n  (none yet)";

fn main() -> ExitCode {
    if let Some(command) = env::args().nth(1) {
        eprintln!("unknown command: {command}");
        eprintln!("{USAGE}");
    } else {
        println!("{USAGE}");
    }
    ExitCode::from(2)
}
