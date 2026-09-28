//! `cargo xtask web [--release] [--debug-tools]`: builds `trpg-app` for
//! `wasm32-unknown-unknown` and packages the resulting binary with the web
//! shell (`web/index.html`) and the vendored JS loaders (`web/mq_js_bundle.js`,
//! and `web/sapp_jsutils.js` + `web/quad-storage.js` for `localStorage`,
//! ticket 0207) into `dist/web/` (ticket 0206). `--debug-tools` turns on the
//! app's `debug-tools` feature (Quick Battle, glyph sampler) for the Pages
//! build (ADR-0023); shipped builds never pass it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The `[[bin]]` name in `crates/app/Cargo.toml`.
const BIN_NAME: &str = "tactical-rpg";

/// Files copied from `web/` into `dist/web/` unchanged.
const SHELL_FILES: &[&str] = &[
    "index.html",
    "mq_js_bundle.js",
    "sapp_jsutils.js",
    "quad-storage.js",
];

/// Parsed `cargo xtask web` arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Whether to build with `--release` (and run `wasm-opt` afterwards).
    pub release: bool,
    /// Whether to turn on the `debug-tools` feature (ADR-0023).
    pub debug_tools: bool,
}

/// Parses the arguments after `web`: `--release` and `--debug-tools`, each
/// at most once, in any order.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        release: false,
        debug_tools: false,
    };
    for arg in args {
        let flag = match arg.as_str() {
            "--release" => &mut options.release,
            "--debug-tools" => &mut options.debug_tools,
            _ => return Err(USAGE.to_string()),
        };
        if *flag {
            return Err(USAGE.to_string());
        }
        *flag = true;
    }
    Ok(options)
}

/// Usage line for bad arguments.
const USAGE: &str = "usage: cargo xtask web [--release] [--debug-tools]";

/// The `cargo` arguments that build the wasm binary for `options`.
fn cargo_args(options: Options) -> Vec<&'static str> {
    let mut args = vec![
        "build",
        "-p",
        "trpg-app",
        "--target",
        "wasm32-unknown-unknown",
    ];
    if options.release {
        args.push("--release");
    }
    if options.debug_tools {
        args.extend(["--features", "debug-tools"]);
    }
    args
}

/// Cargo's build profile directory name for a given release flag.
fn profile_dir(release: bool) -> &'static str {
    if release { "release" } else { "debug" }
}

/// Where cargo places the wasm binary for the given options.
fn wasm_artifact_path(repo_root: &Path, options: Options) -> PathBuf {
    repo_root
        .join("target/wasm32-unknown-unknown")
        .join(profile_dir(options.release))
        .join(format!("{BIN_NAME}.wasm"))
}

/// The packaged output directory.
fn dist_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("dist/web")
}

/// Builds and packages the web build, returning a one-line summary on success.
pub fn run(repo_root: &Path, options: Options) -> Result<String, String> {
    let status = Command::new("cargo")
        .args(cargo_args(options))
        .current_dir(repo_root)
        .status()
        .map_err(|e| format!("spawn cargo build: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build exited with {status}"));
    }
    package(&RealWasmOpt, repo_root, options)
}

/// Copies the wasm artifact and web shell into `dist/web/`, optimizing the
/// wasm binary with `opt` first if `options.release`. Split out from [`run`]
/// so tests can exercise it (with a fake [`WasmOpt`]) without needing a real
/// `cargo build`.
fn package(opt: &impl WasmOpt, repo_root: &Path, options: Options) -> Result<String, String> {
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
        run_wasm_opt(opt, &wasm_dst);
    }

    let size = fs::metadata(&wasm_dst)
        .map_err(|e| format!("stat {}: {e}", wasm_dst.display()))?
        .len();
    Ok(format!("{} ({size} bytes)", dist.display()))
}

/// Runs `wasm-opt -Oz` on a wasm binary, abstracted so tests can substitute a
/// fake instead of depending on a real `wasm-opt` install (which is optional
/// and often absent — see the "skip silently" rule on [`WasmOpt::optimize`]).
trait WasmOpt {
    /// Optimizes `input` into `output` (mirroring `wasm-opt -Oz -o <output>
    /// <input>`). `Ok(true)`/`Ok(false)` is a completed process's exit
    /// status; `Err` means it couldn't be spawned at all (e.g. not on
    /// `PATH`), which the caller treats the same as "skip silently".
    fn optimize(&self, input: &Path, output: &Path) -> Result<bool, String>;
}

/// The real `wasm-opt` binary on `PATH`.
struct RealWasmOpt;

impl WasmOpt for RealWasmOpt {
    /// Not covered by mutation testing (`#[mutants::skip]`): its only job is
    /// spawning an optional external tool that isn't guaranteed to be
    /// installed anywhere tests run, so there's no way to exercise it
    /// without one; [`package`]'s tests cover every branch of the logic that
    /// consumes its result via a fake [`WasmOpt`] instead.
    #[mutants::skip]
    fn optimize(&self, input: &Path, output: &Path) -> Result<bool, String> {
        Command::new("wasm-opt")
            .args(["-Oz", "-o"])
            .arg(output)
            .arg(input)
            .status()
            .map(|status| status.success())
            .map_err(|e| e.to_string())
    }
}

/// Runs `opt` on `wasm_path` in place, replacing it with the optimized
/// output on success. Does nothing (silently) if `opt` reports the tool
/// isn't available, per the ticket's spec.
fn run_wasm_opt(opt: &impl WasmOpt, wasm_path: &Path) {
    let optimized = wasm_path.with_extension("wasm.opt");
    match opt.optimize(wasm_path, &optimized) {
        Ok(true) => {
            if let Err(e) = fs::rename(&optimized, wasm_path) {
                eprintln!("web: wasm-opt ran but replacing the binary failed: {e}");
            }
        }
        Ok(false) => eprintln!("web: wasm-opt exited with a failure, skipping optimization"),
        Err(_) => {} // wasm-opt not on PATH: skip silently.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeWasmOpt<F>(F);

    impl<F: Fn(&Path, &Path) -> Result<bool, String>> WasmOpt for FakeWasmOpt<F> {
        fn optimize(&self, input: &Path, output: &Path) -> Result<bool, String> {
            (self.0)(input, output)
        }
    }

    /// A fresh scratch repo root under `env::temp_dir()`, with `web/`
    /// populated like the real one, per the pattern in `font_atlas::tests`.
    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-web-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("web")).unwrap();
        fs::write(dir.join("web/index.html"), "<html></html>").unwrap();
        fs::write(dir.join("web/mq_js_bundle.js"), "// bundle").unwrap();
        fs::write(dir.join("web/sapp_jsutils.js"), "// sapp_jsutils").unwrap();
        fs::write(dir.join("web/quad-storage.js"), "// quad-storage").unwrap();
        dir
    }

    fn write_wasm(repo_root: &Path, options: Options, content: &[u8]) {
        let path = wasm_artifact_path(repo_root, options);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn parse_args_with_no_args_is_debug() {
        assert_eq!(
            parse_args(&[]),
            Ok(Options {
                release: false,
                debug_tools: false
            })
        );
    }

    #[test]
    fn parse_args_with_release_flag() {
        let args = vec!["--release".to_string()];
        assert_eq!(
            parse_args(&args),
            Ok(Options {
                release: true,
                debug_tools: false
            })
        );
    }

    #[test]
    fn parse_args_rejects_unknown_flags() {
        let args = vec!["--bogus".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn parse_args_takes_debug_tools_in_any_order_once() {
        let args = |a: &[&str]| parse_args(&a.iter().map(ToString::to_string).collect::<Vec<_>>());
        let both = Options {
            release: true,
            debug_tools: true,
        };
        assert_eq!(args(&["--release", "--debug-tools"]), Ok(both));
        assert_eq!(args(&["--debug-tools", "--release"]), Ok(both));
        assert_eq!(
            args(&["--debug-tools"]),
            Ok(Options {
                release: false,
                debug_tools: true
            })
        );
        assert_eq!(args(&["--release", "--release"]), Err(USAGE.to_string()));
        assert_eq!(
            args(&["--debug-tools", "--debug-tools"]),
            Err(USAGE.to_string())
        );
    }

    #[test]
    fn cargo_args_add_release_and_the_debug_tools_feature() {
        let base = [
            "build",
            "-p",
            "trpg-app",
            "--target",
            "wasm32-unknown-unknown",
        ];
        let opts = |release, debug_tools| Options {
            release,
            debug_tools,
        };
        assert_eq!(cargo_args(opts(false, false)), base);
        assert_eq!(cargo_args(opts(true, false))[5..], ["--release"]);
        assert_eq!(
            cargo_args(opts(false, true))[5..],
            ["--features", "debug-tools"]
        );
        assert_eq!(
            cargo_args(opts(true, true))[5..],
            ["--release", "--features", "debug-tools"]
        );
    }

    #[test]
    fn parse_args_rejects_extra_args() {
        let args = vec!["--release".to_string(), "extra".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn wasm_artifact_path_uses_debug_profile_by_default() {
        let root = Path::new("/repo");
        let path = wasm_artifact_path(
            root,
            Options {
                release: false,
                debug_tools: false,
            },
        );
        assert_eq!(
            path,
            Path::new("/repo/target/wasm32-unknown-unknown/debug/tactical-rpg.wasm")
        );
    }

    #[test]
    fn wasm_artifact_path_uses_release_profile() {
        let root = Path::new("/repo");
        let path = wasm_artifact_path(
            root,
            Options {
                release: true,
                debug_tools: false,
            },
        );
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

    #[test]
    fn package_copies_wasm_and_shell_files() {
        let root = fixture("package-basic");
        write_wasm(
            &root,
            Options {
                release: false,
                debug_tools: false,
            },
            b"wasm-bytes",
        );
        let opt = FakeWasmOpt(|_: &Path, _: &Path| -> Result<bool, String> {
            panic!("wasm-opt should not run for a debug build")
        });
        let summary = package(
            &opt,
            &root,
            Options {
                release: false,
                debug_tools: false,
            },
        )
        .unwrap();
        assert!(summary.contains("10 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/tactical-rpg.wasm")).unwrap(),
            b"wasm-bytes"
        );
        assert_eq!(
            fs::read_to_string(root.join("dist/web/index.html")).unwrap(),
            "<html></html>"
        );
        assert_eq!(
            fs::read_to_string(root.join("dist/web/mq_js_bundle.js")).unwrap(),
            "// bundle"
        );
        assert_eq!(
            fs::read_to_string(root.join("dist/web/sapp_jsutils.js")).unwrap(),
            "// sapp_jsutils"
        );
        assert_eq!(
            fs::read_to_string(root.join("dist/web/quad-storage.js")).unwrap(),
            "// quad-storage"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_runs_wasm_opt_on_release_and_uses_its_output() {
        let root = fixture("package-release-ok");
        write_wasm(
            &root,
            Options {
                release: true,
                debug_tools: false,
            },
            b"unoptimized-bytes",
        );
        let opt = FakeWasmOpt(|_input: &Path, output: &Path| {
            fs::write(output, b"opt").unwrap();
            Ok(true)
        });
        let summary = package(
            &opt,
            &root,
            Options {
                release: true,
                debug_tools: false,
            },
        )
        .unwrap();
        assert!(summary.contains("3 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/tactical-rpg.wasm")).unwrap(),
            b"opt"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_keeps_original_when_wasm_opt_is_not_installed() {
        let root = fixture("package-release-missing");
        write_wasm(
            &root,
            Options {
                release: true,
                debug_tools: false,
            },
            b"unoptimized",
        );
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Err("not found".to_string()));
        let summary = package(
            &opt,
            &root,
            Options {
                release: true,
                debug_tools: false,
            },
        )
        .unwrap();
        assert!(summary.contains("11 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/tactical-rpg.wasm")).unwrap(),
            b"unoptimized"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_keeps_original_when_wasm_opt_fails() {
        let root = fixture("package-release-fails");
        write_wasm(
            &root,
            Options {
                release: true,
                debug_tools: false,
            },
            b"unoptimized",
        );
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        let summary = package(
            &opt,
            &root,
            Options {
                release: true,
                debug_tools: false,
            },
        )
        .unwrap();
        assert!(summary.contains("11 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/tactical-rpg.wasm")).unwrap(),
            b"unoptimized"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_reports_a_missing_wasm_artifact() {
        let root = fixture("package-missing-wasm");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| panic!("not reached"));
        let err = package(
            &opt,
            &root,
            Options {
                release: false,
                debug_tools: false,
            },
        )
        .unwrap_err();
        assert!(err.contains("copy"), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn run_reports_cargo_build_failure() {
        // An empty directory has no Cargo.toml, so `cargo build` fails fast
        // (no compilation), without needing a real broken build to test the
        // failure path.
        let root = fixture("run-no-manifest");
        let err = run(
            &root,
            Options {
                release: false,
                debug_tools: false,
            },
        )
        .unwrap_err();
        assert!(err.contains("cargo build"), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }
}
