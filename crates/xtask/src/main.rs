//! Repo tooling commands. See CLAUDE.md.
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

mod tickets;

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <command>\n\n\
available commands:\n  \
ticket-lint [--pr-branch <name>]   check tickets/{open,done} against tickets/README.md";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("ticket-lint") => ticket_lint(&args.collect::<Vec<_>>()),
        Some(command) => {
            eprintln!("unknown command: {command}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        None => {
            println!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn ticket_lint(args: &[String]) -> ExitCode {
    let pr_branch = match parse_pr_branch(args) {
        Ok(branch) => branch,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };

    let repo_root = repo_root();
    let errors = tickets::run(&repo_root, pr_branch.as_deref());

    if errors.is_empty() {
        println!("ticket-lint: OK");
        return ExitCode::SUCCESS;
    }

    eprintln!("ticket-lint: {} error(s)", errors.len());
    for error in &errors {
        eprintln!("  {error}");
    }
    ExitCode::FAILURE
}

fn parse_pr_branch(args: &[String]) -> Result<Option<String>, String> {
    let mut iter = args.iter();
    match iter.next() {
        None => Ok(None),
        Some(flag) if flag == "--pr-branch" => match iter.next() {
            Some(branch) => Ok(Some(branch.clone())),
            None => Err("--pr-branch requires a value".to_string()),
        },
        Some(other) => Err(format!("unknown argument to ticket-lint: {other}")),
    }
}

/// `xtask` always runs via `cargo xtask`, so `CARGO_MANIFEST_DIR` (this
/// crate's directory, `<repo>/crates/xtask`) locates the repo root.
#[allow(clippy::expect_used)] // CARGO_MANIFEST_DIR is baked in at compile time; the two
// `parent()` calls can only fail if xtask's own manifest ever moves.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask is at <repo>/crates/xtask")
        .to_path_buf()
}
