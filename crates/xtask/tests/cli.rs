//! End-to-end tests that run the real `xtask` binary as a subprocess, so
//! `main` itself (which reads the process's actual argv) is exercised.

use std::process::Command;

fn xtask() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
}

#[test]
fn no_command_prints_usage_and_exits_2() {
    let output = xtask()
        .output()
        .unwrap_or_else(|e| panic!("spawn xtask: {e}"));
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).contains("usage: cargo xtask"));
}

#[test]
fn ticket_lint_on_the_real_repo_exits_0() {
    let output = xtask()
        .arg("ticket-lint")
        .output()
        .unwrap_or_else(|e| panic!("spawn xtask: {e}"));
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ticket-lint: OK"));
}

#[test]
fn playtest_prints_the_report_and_exits_0() {
    let history = std::env::temp_dir().join(format!("xtask-cli-playtest-{}", std::process::id()));
    let output = xtask()
        .args(["playtest", "quick", "--runs", "3", "--history"])
        .arg(&history)
        .output()
        .unwrap_or_else(|e| panic!("spawn xtask: {e}"));
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = "quick · Classic · baseline · 3 tries (seeds 1–3)\nwon ";
    assert!(stdout.starts_with(first), "{stdout}");
    assert!(stdout.contains("\nfallen/try 0: "), "{stdout}");
    assert!(stdout.contains("\n\ntry 1 · "), "{stdout}");
    assert!(stdout.contains("\ntry 3 · "), "{stdout}");
    assert!(
        stdout.ends_with('\n') && !stdout.ends_with("\n\n"),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&history);
}
