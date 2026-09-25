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
