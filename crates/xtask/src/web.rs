//! `cargo xtask web [--release]`: builds `trpg-app` for
//! `wasm32-unknown-unknown` and packages the resulting binary with the web
//! shell (`web/index.html`) and the vendored JS loader (`web/mq_js_bundle.js`)
//! into `dist/web/` (ticket 0206).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The `[[bin]]` name in `crates/app/Cargo.toml`.
const BIN_NAME: &str = "tactical-rpg";

/// Files copied from `web/` into `dist/web/` unchanged.
const SHELL_FILES: &[&str] = &["index.html", "mq_js_bundle.js"];

/// Parsed `cargo xtask web` arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Whether to build with `--release` (and run `wasm-opt` afterwards).
    pub release: bool,
}

/// Parses the arguments after `web`. Only `--release` (or nothing) is valid.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    match args {
        [] => Ok(Options { release: false }),
        [flag] if flag == "--release" => Ok(Options { release: true }),
        _ => Err("usage: cargo xtask web [--release]".to_string()),
    }
}

/// Cargo's build profile directory name for a given release flag.
fn profile_dir(release: bool) -> &'static str {
    if release { "release" } else { "debug" }
}

/// Where cargo places the wasm binary for the given options.
pub fn wasm_artifact_path(repo_root: &Path, options: Options) -> PathBuf {
    repo_root
        .join("target/wasm32-unknown-unknown")
        .join(profile_dir(options.release))
        .join(format!("{BIN_NAME}.wasm"))
}

/// The packaged output directory.
pub fn dist_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("dist/web")
}

/// Builds and packages the web build, returning a one-line summary on success.
pub fn run(repo_root: &Path, options: Options) -> Result<String, String> {
    let status = Command::new("cargo")
        .args([
            "build",
            "-p",
            "trpg-app",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .args(options.release.then_some("--release"))
        .current_dir(repo_root)
        .status()
        .map_err(|e| format!("spawn cargo build: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build exited with {status}"));
    }

    let dist = dist_dir(repo_root);
    fs::create_dir_all(&dist).map_err(|e| format!("create {}: {e}", dist.display()))?;

    let wasm_src = wasm_artifact_path(repo_root, options);
    let wasm_dst = dist.join(format!("{BIN_NAME}.wasm"));
    fs::copy(&wasm_src, &wasm_dst)
        .map_err(|e| format!("copy {} to {}: {e}", wasm_src.display(), wasm_dst.display()))?;

    for name in SHELL_FILES {
        let shell_src = repo_root.join("web").join(name);
        let shell_dst = dist.join(name);
        fs::copy(&shell_src, &shell_dst).map_err(|e| {
            format!(
                "copy {} to {}: {e}",
                shell_src.display(),
                shell_dst.display()
            )
        })?;
    }

    if options.release {
        run_wasm_opt(&wasm_dst);
    }

    let size = fs::metadata(&wasm_dst)
        .map_err(|e| format!("stat {}: {e}", wasm_dst.display()))?
        .len();
    Ok(format!("{} ({} bytes)", dist.display(), size))
}

/// Runs `wasm-opt -Oz` on `wasm_path` in place, if `wasm-opt` is on `PATH`.
/// Silently does nothing otherwise, per the ticket's spec.
fn run_wasm_opt(wasm_path: &Path) {
    let optimized = wasm_path.with_extension("wasm.opt");
    match Command::new("wasm-opt")
        .args(["-Oz", "-o"])
        .arg(&optimized)
        .arg(wasm_path)
        .status()
    {
        Ok(status) if status.success() => {
            if let Err(e) = fs::rename(&optimized, wasm_path) {
                eprintln!("web: wasm-opt ran but replacing the binary failed: {e}");
            }
        }
        Ok(status) => eprintln!("web: wasm-opt exited with {status}, skipping optimization"),
        Err(_) => {} // wasm-opt not on PATH: skip silently.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_with_no_args_is_debug() {
        assert_eq!(parse_args(&[]), Ok(Options { release: false }));
    }

    #[test]
    fn parse_args_with_release_flag() {
        let args = vec!["--release".to_string()];
        assert_eq!(parse_args(&args), Ok(Options { release: true }));
    }

    #[test]
    fn parse_args_rejects_unknown_flags() {
        let args = vec!["--bogus".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn parse_args_rejects_extra_args() {
        let args = vec!["--release".to_string(), "extra".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn wasm_artifact_path_uses_debug_profile_by_default() {
        let root = Path::new("/repo");
        let path = wasm_artifact_path(root, Options { release: false });
        assert_eq!(
            path,
            Path::new("/repo/target/wasm32-unknown-unknown/debug/tactical-rpg.wasm")
        );
    }

    #[test]
    fn wasm_artifact_path_uses_release_profile() {
        let root = Path::new("/repo");
        let path = wasm_artifact_path(root, Options { release: true });
        assert_eq!(
            path,
            Path::new("/repo/target/wasm32-unknown-unknown/release/tactical-rpg.wasm")
        );
    }

    #[test]
    fn dist_dir_is_under_repo_root() {
        let root = Path::new("/repo");
        assert_eq!(dist_dir(root), Path::new("/repo/dist/web"));
    }
}
