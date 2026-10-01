//! Repo tooling commands. See CLAUDE.md.
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

mod check_keys;
mod font_atlas;
mod sfx;
mod tickets;
mod web;

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <command>\n\n\
available commands:\n  \
ticket-lint [--pr-branch <name>]   check tickets/{open,done} against tickets/README.md\n  \
check-keys                         fail on keys hard-coded in game code or text\n  \
font-atlas <font.bdf> <out-dir>    build the font atlas from a BDF font\n  \
sfx [--check]                      render our own sounds into assets/audio/sfx/\n  \
web [--release] [--debug-tools]    build and package the web (WASM) shell into dist/web/";

fn main() -> ExitCode {
    ExitCode::from(dispatch(env::args().skip(1)))
}

/// The actual command dispatch, as a plain exit code (0 success) rather than
/// `ExitCode` so it's directly comparable in tests.
fn dispatch(mut args: impl Iterator<Item = String>) -> u8 {
    match args.next().as_deref() {
        Some("ticket-lint") => ticket_lint(&args.collect::<Vec<_>>()),
        Some("check-keys") => check_keys(&args.collect::<Vec<_>>()),
        Some("font-atlas") => font_atlas(&args.collect::<Vec<_>>()),
        Some("web") => web(&args.collect::<Vec<_>>()),
        Some("sfx") => sfx(&args.collect::<Vec<_>>()),
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

fn check_keys(args: &[String]) -> u8 {
    if !args.is_empty() {
        eprintln!("usage: cargo xtask check-keys");
        return 2;
    }
    let errors = check_keys::run(&repo_root());
    if errors.is_empty() {
        println!("check-keys: OK");
        return 0;
    }
    eprintln!("check-keys: {} hard-coded key(s)", errors.len());
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

fn sfx(args: &[String]) -> u8 {
    let Some(check) = parse_sfx_check(args) else {
        eprintln!("usage: cargo xtask sfx [--check]");
        return 2;
    };
    match sfx::run(&repo_root(), check) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("sfx: {e}");
            1
        }
    }
}

/// `sfx`'s arguments: `Some(check)`, or `None` if they're wrong.
fn parse_sfx_check(args: &[String]) -> Option<bool> {
    match args {
        [] => Some(false),
        [flag] if flag == "--check" => Some(true),
        _ => None,
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

    /// The lines of `manifest`'s `[header]` table, without comments and
    /// blank lines.
    fn table<'a>(manifest: &'a str, header: &str) -> Vec<&'a str> {
        let lines = manifest.lines().skip_while(|l| l.trim() != header).skip(1);
        lines
            .take_while(|l| !l.starts_with('['))
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .collect()
    }

    /// `trpg-app` copies the workspace lints because one of them differs
    /// (`unsafe_code`, ADR-0034); the copy must not drift.
    #[test]
    fn app_lints_are_the_workspace_lints_except_unsafe_code() {
        let root = repo_root();
        let workspace = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        let app = std::fs::read_to_string(root.join("crates/app/Cargo.toml")).unwrap();
        let clippy = table(&workspace, "[workspace.lints.clippy]");
        assert!(clippy.len() > 5, "{clippy:?}");
        assert_eq!(table(&app, "[lints.clippy]"), clippy);
        let rust: Vec<String> = table(&workspace, "[workspace.lints.rust]")
            .iter()
            .map(|l| l.replace("unsafe_code = \"forbid\"", "unsafe_code = \"deny\""))
            .collect();
        assert!(
            rust.contains(&"unsafe_code = \"deny\"".to_owned()),
            "{rust:?}"
        );
        assert_eq!(table(&app, "[lints.rust]"), rust);
        // Every other crate inherits the workspace's `forbid`.
        for other in ["core", "content", "ui", "xtask"] {
            let manifest =
                std::fs::read_to_string(root.join(format!("crates/{other}/Cargo.toml"))).unwrap();
            assert_eq!(table(&manifest, "[lints]"), ["workspace = true"], "{other}");
        }
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
    fn parse_sfx_check_reads_the_flag() {
        assert_eq!(parse_sfx_check(&[]), Some(false));
        assert_eq!(parse_sfx_check(&args(&["--check"])), Some(true));
        assert_eq!(parse_sfx_check(&args(&["--bogus"])), None);
    }

    #[test]
    fn sfx_rejects_unknown_args() {
        assert_eq!(sfx(&args(&["--bogus"])), 2);
        assert_eq!(dispatch(args(&["sfx", "--check", "x"]).into_iter()), 2);
    }

    #[test]
    fn sfx_check_passes_on_the_committed_files() {
        assert_eq!(sfx(&args(&["--check"])), 0);
    }

    #[test]
    fn check_keys_passes_on_the_real_repo() {
        assert_eq!(check_keys(&[]), 0);
        assert_eq!(dispatch(args(&["check-keys"]).into_iter()), 0);
    }

    #[test]
    fn check_keys_rejects_args() {
        assert_eq!(check_keys(&args(&["--bogus"])), 2);
    }

    #[test]
    fn dispatch_runs_ticket_lint() {
        assert_eq!(dispatch(args(&["ticket-lint"]).into_iter()), 0);
    }
}
