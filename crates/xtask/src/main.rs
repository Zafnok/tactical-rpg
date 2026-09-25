//! Repo tooling commands. See CLAUDE.md.
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

mod font_atlas;
mod tickets;
mod web;

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <command>\n\n\
available commands:\n  \
ticket-lint [--pr-branch <name>]   check tickets/{open,done} against tickets/README.md\n  \
font-atlas <font.bdf> <out-dir>    build the font atlas from a BDF font\n  \
web [--release]                    build and package the web (WASM) shell into dist/web/";

fn main() -> ExitCode {
    ExitCode::from(dispatch(env::args().skip(1)))
}

/// The actual command dispatch, as a plain exit code (0 success) rather than
/// `ExitCode` so it's directly comparable in tests.
fn dispatch(mut args: impl Iterator<Item = String>) -> u8 {
    match args.next().as_deref() {
        Some("ticket-lint") => ticket_lint(&args.collect::<Vec<_>>()),
        Some("font-atlas") => font_atlas(&args.collect::<Vec<_>>()),
        Some("web") => web(&args.collect::<Vec<_>>()),
        Some(command) => {
            eprintln!("unknown command: {command}");
            eprintln!("{USAGE}");
            2
        }
        None => {
            println!("{USAGE}");
            2
        }
    }
}

fn ticket_lint(args: &[String]) -> u8 {
    let pr_branch = match parse_pr_branch(args) {
        Ok(branch) => branch,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };

    let repo_root = repo_root();
    let errors = tickets::run(&repo_root, pr_branch.as_deref());

    if errors.is_empty() {
        println!("ticket-lint: OK");
        return 0;
    }

    eprintln!("ticket-lint: {} error(s)", errors.len());
    for error in &errors {
        eprintln!("  {error}");
    }
    1
}

fn font_atlas(args: &[String]) -> u8 {
    let [font, out_dir] = args else {
        eprintln!("usage: cargo xtask font-atlas <font.bdf> <out-dir>");
        return 2;
    };
    match font_atlas::run(Path::new(font), Path::new(out_dir)) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("font-atlas: {e}");
            1
        }
    }
}

fn web(args: &[String]) -> u8 {
    let options = match web::parse_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    match web::run(&repo_root(), options) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("web: {e}");
            1
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn repo_root_points_at_the_workspace_root() {
        let root = repo_root();
        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("tickets/README.md").is_file());
    }

    #[test]
    fn parse_pr_branch_with_no_args_is_none() {
        assert_eq!(parse_pr_branch(&[]), Ok(None));
    }

    #[test]
    fn parse_pr_branch_reads_the_flag_value() {
        assert_eq!(
            parse_pr_branch(&args(&["--pr-branch", "t0106-x"])),
            Ok(Some("t0106-x".to_string()))
        );
    }

    #[test]
    fn parse_pr_branch_requires_a_value() {
        assert_eq!(
            parse_pr_branch(&args(&["--pr-branch"])),
            Err("--pr-branch requires a value".to_string())
        );
    }

    #[test]
    fn parse_pr_branch_rejects_unknown_flags() {
        assert_eq!(
            parse_pr_branch(&args(&["--bogus"])),
            Err("unknown argument to ticket-lint: --bogus".to_string())
        );
    }

    #[test]
    fn ticket_lint_fails_fast_on_bad_args() {
        assert_eq!(ticket_lint(&args(&["--pr-branch"])), 2);
    }

    #[test]
    fn ticket_lint_succeeds_on_the_real_repo() {
        assert_eq!(ticket_lint(&[]), 0);
    }

    #[test]
    fn ticket_lint_fails_when_pr_branch_names_an_unknown_ticket() {
        // Ticket 9999 will never exist (see tickets/README.md's numbering),
        // unlike a real open/done ticket id, whose status changes as tickets
        // are worked — this exercises the wrapper's non-zero exit path
        // without depending on the repo's current ticket state.
        assert_eq!(ticket_lint(&args(&["--pr-branch", "t9999-ghost"])), 1);
    }

    #[test]
    fn dispatch_with_no_command_prints_usage_and_fails() {
        assert_eq!(dispatch(std::iter::empty()), 2);
    }

    #[test]
    fn dispatch_with_unknown_command_fails() {
        assert_eq!(dispatch(args(&["bogus"]).into_iter()), 2);
    }

    #[test]
    fn font_atlas_needs_two_args() {
        assert_eq!(font_atlas(&args(&["only-one"])), 2);
        assert_eq!(dispatch(args(&["font-atlas"]).into_iter()), 2);
    }

    #[test]
    fn font_atlas_reports_failure() {
        assert_eq!(font_atlas(&args(&["no/such/font.bdf", "out"])), 1);
    }

    #[test]
    fn web_fails_fast_on_bad_args() {
        // Only checks argument parsing: a real `--release`/no-args run
        // spawns `cargo build`, which is exercised by `web::tests` and
        // manually, not here.
        assert_eq!(web(&args(&["--bogus"])), 2);
        assert_eq!(dispatch(args(&["web", "--bogus"]).into_iter()), 2);
    }

    #[test]
    fn dispatch_runs_ticket_lint() {
        assert_eq!(dispatch(args(&["ticket-lint"]).into_iter()), 0);
    }
}
