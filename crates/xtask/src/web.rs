//! `cargo xtask web [--release] [--debug-tools] [--private-assets]`: builds
//! `trpg-app` for
//! `wasm32-unknown-unknown` and packages the resulting binary with the web
//! shell (`web/index.html`), the vendored JS loaders (`web/mq_js_bundle.js`,
//! and `web/sapp_jsutils.js` + `web/quad-storage.js` for `localStorage`,
//! ticket 0207) and our own controller plugin (`web/gamepad.js`, ticket
//! 0219) into `dist/web/` (ticket 0206). `--debug-tools` turns on the
//! app's `debug-tools` feature (Quick Battle, glyph sampler) for the Pages
//! build (ADR-0023); shipped builds never pass it. `--private-assets` turns
//! on its `private-assets` feature, which embeds the bought art in
//! `assets-private/game/` (ADR-0040). The game's music tracks
//! (`music/*.ogg`, not embedded: ADR-0026) are copied to `dist/web/music/`,
//! which exists even when there are none. The voice clips (`voice/`, not
//! embedded either: ADR-0046) are copied to `dist/web/voice/` when there
//! is such a folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The `[[bin]]` name in `crates/app/Cargo.toml`.
const BIN_NAME: &str = "visions-of-shuyi";

/// Files copied from `web/` into `dist/web/` unchanged.
const SHELL_FILES: &[&str] = &[
    "index.html",
    "mq_js_bundle.js",
    "sapp_jsutils.js",
    "quad-storage.js",
    "gamepad.js",
];

/// The music folder, at the repo root and in `dist/web/` (ADR-0026).
const MUSIC_DIR: &str = "music";

/// The voice folder, at the repo root and in `dist/web/` (ADR-0046).
const VOICE_DIR: &str = "voice";

/// Parsed `cargo xtask web` arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Whether to build with `--release` (and run `wasm-opt` afterwards).
    pub release: bool,
    /// Whether to turn on the `debug-tools` feature (ADR-0023).
    pub debug_tools: bool,
    /// Whether to turn on the `private-assets` feature (ADR-0040).
    pub private_assets: bool,
}

/// Parses the arguments after `web`: `--release`, `--debug-tools` and
/// `--private-assets`, each at most once, in any order.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        release: false,
        debug_tools: false,
        private_assets: false,
    };
    for arg in args {
        let flag = match arg.as_str() {
            "--release" => &mut options.release,
            "--debug-tools" => &mut options.debug_tools,
            "--private-assets" => &mut options.private_assets,
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
const USAGE: &str = "usage: cargo xtask web [--release] [--debug-tools] [--private-assets]";

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
    if options.private_assets {
        args.extend(["--features", "private-assets"]);
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

    copy_music(&repo_root.join(MUSIC_DIR), &dist.join(MUSIC_DIR))?;
    copy_voice(&repo_root.join(VOICE_DIR), &dist.join(VOICE_DIR))?;

    if options.release {
        run_wasm_opt(opt, &wasm_dst);
    }

    let size = fs::metadata(&wasm_dst)
        .map_err(|e| format!("stat {}: {e}", wasm_dst.display()))?
        .len();
    Ok(format!("{} ({size} bytes)", dist.display()))
}

/// Replaces `dst` with a folder holding every `.ogg` file in `src` (none
/// if `src` is missing), so the web build fetches tracks from `music/`.
fn copy_music(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("clear {}: {e}", dst.display()))?;
    }
    fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?;
    let Ok(entries) = fs::read_dir(src) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry
            .map_err(|e| format!("read {}: {e}", src.display()))?
            .path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "ogg") {
            let name = path.file_name().unwrap_or_default();
            fs::copy(&path, dst.join(name)).map_err(|e| format!("copy {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// Replaces `dst` with a copy of the voice folder `src`: its manifests
/// (`.ron`) and clips (`.ogg`), in the same folders, so the web build
/// fetches them from `voice/`. With no `src` there is no `dst` either: a
/// build without voices.
fn copy_voice(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("clear {}: {e}", dst.display()))?;
    }
    if !src.is_dir() {
        return Ok(());
    }
    let shipped = |path: &Path| {
        path.extension()
            .is_some_and(|ext| ext == "ogg" || ext == "ron")
    };
    copy_tree(src, dst, &shipped).map(|_| ())
}

/// Copies the files under `src` that `keep` accepts to the same places
/// under `dst` (made if missing), folders included. Returns how many files
/// were copied.
pub fn copy_tree(src: &Path, dst: &Path, keep: &dyn Fn(&Path) -> bool) -> Result<usize, String> {
    fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?;
    let entries = fs::read_dir(src).map_err(|e| format!("read {}: {e}", src.display()))?;
    let mut copied = 0;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("read {}: {e}", src.display()))?
            .path();
        let to = dst.join(path.file_name().unwrap_or_default());
        if path.is_dir() {
            copied += copy_tree(&path, &to, keep)?;
        } else if keep(&path) {
            fs::copy(&path, &to).map_err(|e| format!("copy {}: {e}", path.display()))?;
            copied += 1;
        }
    }
    Ok(copied)
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

    #[test]
    fn index_html_keeps_resuming_audio_until_it_runs() {
        // Ticket 0224: the shim must wrap `AudioContext` before the bundle
        // creates one, and retry on every key press.
        let html =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/index.html"))
                .unwrap();
        let shim = html
            .find("class TrackedAudioContext extends Base")
            .expect("index.html wraps AudioContext");
        let bundle = html
            .find(r#"<script src="mq_js_bundle.js">"#)
            .expect("index.html loads mq_js_bundle.js");
        assert!(shim < bundle, "wrap AudioContext before the bundle loads");
        assert!(html.contains(r#"for (const type of ["keydown", "mousedown", "touchend"])"#));
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
        fs::write(dir.join("web/gamepad.js"), "// gamepad").unwrap();
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
                debug_tools: false,
                private_assets: false,
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
                debug_tools: false,
                private_assets: false,
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
            private_assets: false,
        };
        assert_eq!(args(&["--release", "--debug-tools"]), Ok(both));
        assert_eq!(args(&["--debug-tools", "--release"]), Ok(both));
        assert_eq!(
            args(&["--debug-tools"]),
            Ok(Options {
                release: false,
                debug_tools: true,
                private_assets: false,
            })
        );
        assert_eq!(args(&["--release", "--release"]), Err(USAGE.to_string()));
        assert_eq!(
            args(&["--debug-tools", "--debug-tools"]),
            Err(USAGE.to_string())
        );
    }

    #[test]
    fn parse_args_takes_private_assets_with_the_other_flags() {
        let args = |a: &[&str]| parse_args(&a.iter().map(ToString::to_string).collect::<Vec<_>>());
        let all = Options {
            release: true,
            debug_tools: true,
            private_assets: true,
        };
        assert_eq!(
            args(&["--private-assets", "--release", "--debug-tools"]),
            Ok(all)
        );
        assert_eq!(
            args(&["--private-assets"]),
            Ok(Options {
                release: false,
                debug_tools: false,
                private_assets: true,
            })
        );
        assert_eq!(
            args(&["--private-assets", "--private-assets"]),
            Err(USAGE.to_string())
        );
    }

    #[test]
    fn cargo_args_add_release_and_the_features() {
        let base = [
            "build",
            "-p",
            "trpg-app",
            "--target",
            "wasm32-unknown-unknown",
        ];
        let opts = |release, debug_tools, private_assets| Options {
            release,
            debug_tools,
            private_assets,
        };
        assert_eq!(cargo_args(opts(false, false, false)), base);
        assert_eq!(cargo_args(opts(true, false, false))[5..], ["--release"]);
        assert_eq!(
            cargo_args(opts(false, true, false))[5..],
            ["--features", "debug-tools"]
        );
        assert_eq!(
            cargo_args(opts(false, false, true))[5..],
            ["--features", "private-assets"]
        );
        assert_eq!(
            cargo_args(opts(true, true, true))[5..],
            [
                "--release",
                "--features",
                "debug-tools",
                "--features",
                "private-assets"
            ]
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
                private_assets: false,
            },
        );
        assert_eq!(
            path,
            Path::new("/repo/target/wasm32-unknown-unknown/debug/visions-of-shuyi.wasm")
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
                private_assets: false,
            },
        );
        assert_eq!(
            path,
            Path::new("/repo/target/wasm32-unknown-unknown/release/visions-of-shuyi.wasm")
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
                private_assets: false,
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
                private_assets: false,
            },
        )
        .unwrap();
        assert!(summary.contains("10 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/visions-of-shuyi.wasm")).unwrap(),
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
        assert_eq!(
            fs::read_to_string(root.join("dist/web/gamepad.js")).unwrap(),
            "// gamepad"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_copies_the_music_tracks_only() {
        let root = fixture("package-music");
        let debug = Options {
            release: false,
            debug_tools: false,
            private_assets: false,
        };
        write_wasm(&root, debug, b"w");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        // No music folder: an empty one is still made.
        package(&opt, &root, debug).unwrap();
        let music = root.join("dist/web/music");
        assert!(music.is_dir());
        assert_eq!(fs::read_dir(&music).unwrap().count(), 0);
        fs::create_dir_all(root.join("music/sub")).unwrap();
        fs::write(root.join("music/title.ogg"), "ogg").unwrap();
        fs::write(root.join("music/README.md"), "doc").unwrap();
        fs::write(root.join("music/sub/x.ogg"), "nested").unwrap();
        fs::write(music.join("stale.ogg"), "old").unwrap();
        package(&opt, &root, debug).unwrap();
        let mut names: Vec<_> = fs::read_dir(&music)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["title.ogg"]);
        assert_eq!(fs::read_to_string(music.join("title.ogg")).unwrap(), "ogg");
        fs::remove_dir_all(&root).unwrap();
    }

    /// Ticket 0238: the voice folder goes along whole, manifests and
    /// clips only; without one the package has none.
    #[test]
    fn package_copies_the_voice_folder_if_there_is_one() {
        let root = fixture("package-voice");
        let debug = Options {
            release: false,
            debug_tools: false,
            private_assets: false,
        };
        write_wasm(&root, debug, b"w");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        let voice = root.join("dist/web/voice");
        // A stale copy from an earlier build goes when the source is gone.
        fs::create_dir_all(voice.join("en")).unwrap();
        fs::write(voice.join("en/stale.ogg"), "old").unwrap();
        package(&opt, &root, debug).unwrap();
        assert!(!voice.exists());
        fs::create_dir_all(root.join("voice/en/scene")).unwrap();
        fs::write(root.join("voice/en/voice.ron"), "manifest").unwrap();
        fs::write(root.join("voice/en/cast.ron"), "cast").unwrap();
        fs::write(root.join("voice/en/scene/line.ogg"), "clip").unwrap();
        fs::write(root.join("voice/en/scene/line.m.ogg"), "clip m").unwrap();
        fs::write(root.join("voice/en/scene/take.wav"), "source").unwrap();
        fs::write(root.join("voice/notes.md"), "doc").unwrap();
        fs::write(root.join("voice/ogg"), "no extension").unwrap();
        package(&opt, &root, debug).unwrap();
        let read = |path: &str| fs::read_to_string(voice.join(path)).ok();
        assert_eq!(read("en/voice.ron").as_deref(), Some("manifest"));
        assert_eq!(read("en/cast.ron").as_deref(), Some("cast"));
        assert_eq!(read("en/scene/line.ogg").as_deref(), Some("clip"));
        assert_eq!(read("en/scene/line.m.ogg").as_deref(), Some("clip m"));
        assert_eq!(read("en/scene/take.wav"), None);
        assert_eq!(read("notes.md"), None);
        assert_eq!(read("ogg"), None);
        // A file named `voice` is not a voice folder.
        fs::remove_dir_all(root.join("voice")).unwrap();
        fs::write(root.join("voice"), "a file").unwrap();
        package(&opt, &root, debug).unwrap();
        assert!(!voice.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn copy_tree_counts_the_files_it_copies_and_reports_a_missing_source() {
        let root = fixture("copy-tree");
        fs::create_dir_all(root.join("src/a/b")).unwrap();
        fs::write(root.join("src/one.ogg"), "1").unwrap();
        fs::write(root.join("src/a/two.ogg"), "2").unwrap();
        fs::write(root.join("src/a/b/three.txt"), "3").unwrap();
        let ogg = |path: &Path| path.extension().is_some_and(|ext| ext == "ogg");
        assert_eq!(copy_tree(&root.join("src"), &root.join("dst"), &ogg), Ok(2));
        assert!(root.join("dst/a/two.ogg").is_file());
        assert!(root.join("dst/a/b").is_dir());
        assert!(!root.join("dst/a/b/three.txt").exists());
        let all = copy_tree(&root.join("src"), &root.join("all"), &|_| true);
        assert_eq!(all, Ok(3));
        let error = copy_tree(&root.join("nowhere"), &root.join("dst2"), &ogg).unwrap_err();
        assert!(error.starts_with("read "), "{error}");
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
                private_assets: false,
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
                private_assets: false,
            },
        )
        .unwrap();
        assert!(summary.contains("3 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/visions-of-shuyi.wasm")).unwrap(),
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
                private_assets: false,
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
                private_assets: false,
            },
        )
        .unwrap();
        assert!(summary.contains("11 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/visions-of-shuyi.wasm")).unwrap(),
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
                private_assets: false,
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
                private_assets: false,
            },
        )
        .unwrap();
        assert!(summary.contains("11 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/visions-of-shuyi.wasm")).unwrap(),
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
                private_assets: false,
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
                private_assets: false,
            },
        )
        .unwrap_err();
        assert!(err.contains("cargo build"), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }

    /// The real repo root (`xtask` lives at `<repo>/crates/xtask`).
    fn real_repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask")
    }

    /// The version `quad_storage_crate_version()` reports for the
    /// `quad-storage-sys` version pinned in `Cargo.lock`:
    /// `(major << 24) + (minor << 16) + patch`.
    fn locked_quad_storage_sys_version(lock: &str) -> u32 {
        let version = lock
            .split("[[package]]")
            .find(|pkg| pkg.contains("name = \"quad-storage-sys\""))
            .and_then(|pkg| pkg.lines().find_map(|l| l.strip_prefix("version = \"")))
            .and_then(|v| v.strip_suffix('"'))
            .expect("quad-storage-sys is in Cargo.lock");
        let parts: Vec<u32> = version.split('.').map(|p| p.parse().unwrap()).collect();
        (parts[0] << 24) + (parts[1] << 16) + parts[2]
    }

    #[test]
    fn locked_quad_storage_sys_version_encodes_major_minor_patch() {
        let lock = r#"
[[package]]
name = "quad-storage"
version = "9.9.9"

[[package]]
name = "quad-storage-sys"
version = "1.2.3"
"#;
        assert_eq!(
            locked_quad_storage_sys_version(lock),
            (1 << 24) + (2 << 16) + 3
        );
    }

    #[test]
    fn index_html_declares_register_plugin_before_the_bundle() {
        // Ticket 0225: the strict-mode bundle assigns to `register_plugin`,
        // which throws unless the global already exists.
        let html = fs::read_to_string(real_repo_root().join("web/index.html")).unwrap();
        let declared = html
            .find("var register_plugin;")
            .expect("index.html declares `var register_plugin;`");
        let bundle = html
            .find(r#"<script src="mq_js_bundle.js">"#)
            .expect("index.html loads mq_js_bundle.js");
        assert!(
            declared < bundle,
            "declare register_plugin before the bundle"
        );
    }

    /// Every name in `text` that starts with `prefix` (which isn't the
    /// tail of a longer identifier), sorted, once each.
    fn identifiers_starting(text: &str, prefix: &str) -> Vec<String> {
        let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mut found: Vec<String> = text
            .match_indices(prefix)
            .filter(|&(at, _)| !text[..at].chars().next_back().is_some_and(is_ident))
            .map(|(at, _)| {
                let rest = text[at + prefix.len()..].chars();
                let name: String = rest.take_while(|&c| is_ident(c)).collect();
                format!("{prefix}{name}")
            })
            .collect();
        found.sort();
        found.dedup();
        found
    }

    #[test]
    fn identifiers_starting_finds_whole_names_once() {
        let text = "a.trpg_pad_poll = f; x_trpg_pad_no(); trpg_pad_axis(trpg_pad_poll())";
        assert_eq!(
            identifiers_starting(text, "trpg_pad_"),
            ["trpg_pad_axis", "trpg_pad_poll"]
        );
        assert!(identifiers_starting(text, "nope_").is_empty());
    }

    /// The number after `before` on the line of `text` that starts with it.
    fn number_after(text: &str, before: &str) -> u32 {
        let line = text
            .lines()
            .find_map(|l| l.trim().strip_prefix(before))
            .unwrap_or_else(|| panic!("no line starting `{before}`"));
        line.trim_end_matches(';').parse().unwrap()
    }

    #[test]
    fn gamepad_js_provides_exactly_the_functions_the_app_imports() {
        // Ticket 0219: a function missing on either side only shows up in
        // the browser, as a warning and a pad that does nothing.
        let root = real_repo_root();
        let js = fs::read_to_string(root.join("web/gamepad.js")).unwrap();
        let rust = fs::read_to_string(root.join("crates/app/src/pads/web.rs")).unwrap();
        let provided: Vec<String> = identifiers_starting(&js, "importObject.env.trpg_pad_")
            .iter()
            .map(|name| name.replace("importObject.env.", ""))
            .collect();
        let imported: Vec<String> = identifiers_starting(&rust, "safe fn trpg_pad_")
            .iter()
            .map(|name| name.replace("safe fn ", ""))
            .collect();
        assert!(
            imported.len() >= 5,
            "found only {imported:?} in pads/web.rs"
        );
        assert_eq!(provided, imported);
    }

    #[test]
    fn gamepad_js_version_matches_the_app() {
        // The loader logs a version-mismatch error unless the JS plugin's
        // `version` equals what `trpg_gamepad_crate_version()` returns.
        let root = real_repo_root();
        let js = fs::read_to_string(root.join("web/gamepad.js")).unwrap();
        let rust = fs::read_to_string(root.join("crates/app/src/pads/web.rs")).unwrap();
        assert_eq!(
            number_after(&js, "var VERSION = "),
            number_after(&rust, "const PLUGIN_VERSION: u32 = ")
        );
        assert!(js.contains("version: VERSION"));
        assert!(js.contains(r#"name: "trpg_gamepad""#));
        assert!(rust.contains("fn trpg_gamepad_crate_version() -> u32"));
    }

    #[test]
    fn index_html_loads_gamepad_js_between_the_bundle_and_the_game() {
        // The plugin registers itself with the bundle's
        // `miniquad_add_plugin`, and must have done so before `load(...)`.
        let html = fs::read_to_string(real_repo_root().join("web/index.html")).unwrap();
        let at = |needle: &str| {
            html.find(needle)
                .unwrap_or_else(|| panic!("index.html has `{needle}`"))
        };
        let bundle = at(r#"<script src="mq_js_bundle.js">"#);
        let gamepad = at(r#"<script src="gamepad.js">"#);
        let load = at(r#"load("visions-of-shuyi.wasm")"#);
        assert!(bundle < gamepad && gamepad < load);
    }

    #[test]
    fn quad_storage_js_version_matches_the_locked_crate() {
        // Ticket 0225: the loader logs a version-mismatch error unless the JS
        // plugin's `version` equals `quad_storage_crate_version()`.
        let root = real_repo_root();
        let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
        let js = fs::read_to_string(root.join("web/quad-storage.js")).unwrap();
        let expected = format!("version: {}", locked_quad_storage_sys_version(&lock));
        assert!(
            js.lines().any(|line| line.trim() == expected),
            "web/quad-storage.js must declare `{}`",
            expected.trim_end()
        );
    }
}
