//! Repo tooling commands. See CLAUDE.md.
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

mod check_keys;
mod check_text;
mod clean_targets;
mod effect_marks;
mod font_atlas;
mod frame_png;
mod lang_status;
mod lines;
mod map_sprite_import;
mod playtest;
mod portrait_import;
mod private_assets;
mod sfx;
mod test_card;
mod test_tileset;
mod test_units;
mod tickets;
mod web;

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;

const USAGE: &str = "usage: cargo xtask <command>\n\n\
available commands:\n  \
ticket-lint [--pr-branch <name>]   check tickets/{open,done} against tickets/README.md\n  \
check-keys                         fail on keys hard-coded in game code or text\n  \
check-text                         count screen text still written as literals in crates/ui\n  \
lang-status <code>                 list a language pack's missing and stale keys\n  \
clean-merged-targets [--dry-run]   delete target/ in worktrees whose PR has merged\n  \
font-atlas <font.bdf>... <out-dir> build the font atlas from BDF fonts\n  \
sfx [--check]                      render our own sounds into assets/audio/sfx/\n  \
frame-png <out.png> [steps]        render a scripted game frame to a PNG (frame-png --help)\n  \
test-card                          write the sprite test image, assets/images/test_card.png\n  \
test-tileset                       write the sprite map skins' test tilesets, assets/tilesets/test*\n  \
effect-marks                       write the effect arrows, assets/images/effect_marks.png\n  \
map-sprite-import [--list]         copy the bought map sprites the game uses into assets-private/game/\n  \
playtest <battle-id> [options]     a bot plays a battle many times and reports (playtest --help)\n  \
private-assets [--library | --pin] fetch the bought art into assets-private/ (ADR-0040)\n  \
portrait-import <busts> <id>       cut bought busts into portraits (portrait-import --help)\n  \
lines [scene]                      list every dialogue line with its line id (lines --help)\n  \
web [--release] [--debug-tools] [--private-assets]\n                                     build and package the web (WASM) shell into dist/web/";

fn main() -> ExitCode {
    ExitCode::from(dispatch(env::args().skip(1)))
}

/// The actual command dispatch, as a plain exit code (0 success) rather than
/// `ExitCode` so it's directly comparable in tests.
fn dispatch(mut args: impl Iterator<Item = String>) -> u8 {
    match args.next().as_deref() {
        Some("ticket-lint") => ticket_lint(&args.collect::<Vec<_>>()),
        Some("check-keys") => check_keys(&args.collect::<Vec<_>>()),
        Some("check-text") => check_text(&args.collect::<Vec<_>>()),
        Some("lang-status") => lang_status(&args.collect::<Vec<_>>()),
        Some("clean-merged-targets") => clean_merged_targets(&args.collect::<Vec<_>>()),
        Some("font-atlas") => font_atlas(&args.collect::<Vec<_>>()),
        Some("web") => web(&args.collect::<Vec<_>>()),
        Some("sfx") => sfx(&args.collect::<Vec<_>>()),
        Some("frame-png") => frame_png(&args.collect::<Vec<_>>()),
        Some("test-card") => test_card(&args.collect::<Vec<_>>()),
        Some("test-tileset") => test_tileset(&args.collect::<Vec<_>>()),
        Some("effect-marks") => effect_marks(&args.collect::<Vec<_>>()),
        Some("map-sprite-import") => map_sprite_import(&args.collect::<Vec<_>>()),
        Some("playtest") => playtest(&args.collect::<Vec<_>>()),
        Some("private-assets") => private_assets(&args.collect::<Vec<_>>()),
        Some("portrait-import") => portrait_import(&args.collect::<Vec<_>>()),
        Some("lines") => lines(&args.collect::<Vec<_>>()),
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

fn check_text(args: &[String]) -> u8 {
    if !args.is_empty() {
        eprintln!("usage: cargo xtask check-text");
        return 2;
    }
    let (hits, errors) = check_text::run(&repo_root());
    print!("{}", check_text::report(&hits, check_text::MAX_LITERALS));
    for error in &errors {
        eprintln!("check-text: {error}");
    }
    match check_text::verdict(hits.len(), check_text::MAX_LITERALS) {
        Ok(note) => {
            if let Some(note) = note {
                println!("{note}");
            }
            u8::from(!errors.is_empty())
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// Prints what a language pack lacks. Missing and stale keys are not a
/// failure (ADR-0045): the exit code is 0 unless the content doesn't load
/// (1) or there is no such pack (2).
fn lang_status(args: &[String]) -> u8 {
    let [code] = args else {
        eprintln!("usage: cargo xtask lang-status <code>");
        return 2;
    };
    let content = match trpg_content::load_embedded() {
        Ok(content) => content,
        Err(e) => {
            eprintln!("lang-status: {e}");
            return 1;
        }
    };
    match lang_status::report(&content.lang, code) {
        Ok(report) => {
            print!("{report}");
            0
        }
        Err(e) => {
            eprintln!("lang-status: {e}");
            2
        }
    }
}

fn clean_merged_targets(args: &[String]) -> u8 {
    let dry_run = match clean_targets::parse_args(args) {
        Ok(dry_run) => dry_run,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let repo_root = repo_root();
    let tools = clean_targets::RealTools {
        repo_root: &repo_root,
    };
    let print = |line| println!("{line}");
    match clean_targets::run(&tools, &repo_root, SystemTime::now(), dry_run, print) {
        Ok(outcome) => u8::from(outcome.failed),
        Err(e) => {
            eprintln!("clean-merged-targets: {e}");
            1
        }
    }
}

fn font_atlas(args: &[String]) -> u8 {
    let Some((out_dir, fonts @ [_, ..])) = args.split_last() else {
        eprintln!("usage: cargo xtask font-atlas <font.bdf>... <out-dir>");
        return 2;
    };
    let fonts: Vec<PathBuf> = fonts.iter().map(PathBuf::from).collect();
    match font_atlas::run(&fonts, Path::new(out_dir)) {
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

fn test_card(args: &[String]) -> u8 {
    if !args.is_empty() {
        eprintln!("usage: cargo xtask test-card");
        return 2;
    }
    match test_card::run(&repo_root()) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("test-card: {e}");
            1
        }
    }
}

fn test_tileset(args: &[String]) -> u8 {
    if !args.is_empty() {
        eprintln!("usage: cargo xtask test-tileset");
        return 2;
    }
    let root = repo_root();
    let written = test_tileset::run(&root).and_then(|tiles| Ok([tiles, test_units::run(&root)?]));
    match written {
        Ok(summaries) => {
            println!("{}", summaries.join("\n"));
            0
        }
        Err(e) => {
            eprintln!("test-tileset: {e}");
            1
        }
    }
}

fn map_sprite_import(args: &[String]) -> u8 {
    map_sprite_import_in(&repo_root(), args)
}

/// `map-sprite-import` with the repo at `root`.
fn map_sprite_import_in(root: &Path, args: &[String]) -> u8 {
    use map_sprite_import::{SPRITES, USAGE, list, run};
    match args {
        [] => match run(root, &SPRITES) {
            Ok(summary) => {
                println!("{summary}");
                0
            }
            Err(e) => {
                eprintln!("map-sprite-import: {e}");
                1
            }
        },
        [flag] if flag == "--list" => {
            print!("{}", list(&SPRITES));
            0
        }
        [flag] if flag == "--help" => {
            println!("{USAGE}");
            0
        }
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

fn effect_marks(args: &[String]) -> u8 {
    if !args.is_empty() {
        eprintln!("usage: cargo xtask effect-marks");
        return 2;
    }
    match effect_marks::run(&repo_root()) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("effect-marks: {e}");
            1
        }
    }
}

fn frame_png(args: &[String]) -> u8 {
    if args.iter().any(|a| a == "--help") {
        println!("{}", frame_png::USAGE);
        return 0;
    }
    let options = match frame_png::parse_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!(
                "{e}

{}",
                frame_png::USAGE
            );
            return 2;
        }
    };
    match frame_png::run(&repo_root(), &options) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("frame-png: {e}");
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

fn playtest(args: &[String]) -> u8 {
    if args.iter().any(|a| a == "--help") {
        println!("{}", playtest::USAGE);
        return 0;
    }
    let options = match playtest::parse_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}\n\n{}", playtest::USAGE);
            return 2;
        }
    };
    match playtest::run(&repo_root(), &options) {
        Ok(report) => {
            print!("{report}");
            0
        }
        Err(e) => {
            eprintln!("playtest: {e}");
            1
        }
    }
}

fn private_assets(args: &[String]) -> u8 {
    let mode = match private_assets::parse_args(args) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    match private_assets::run(&repo_root(), private_assets::REPO_URL, mode) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("private-assets: {e}");
            1
        }
    }
}

fn portrait_import(args: &[String]) -> u8 {
    if args.iter().any(|a| a == "--help") {
        println!("{}", portrait_import::USAGE);
        return 0;
    }
    let options = match portrait_import::parse_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}\n\n{}", portrait_import::USAGE);
            return 2;
        }
    };
    match portrait_import::run(&repo_root(), &options) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("portrait-import: {e}");
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

fn lines(args: &[String]) -> u8 {
    if args.iter().any(|a| a == "--help") {
        println!("{}", lines::USAGE);
        return 0;
    }
    let scene = match args {
        [] => None,
        [scene] => Some(scene.as_str()),
        _ => {
            eprintln!("{}", lines::USAGE);
            return 2;
        }
    };
    let rows = trpg_content::load_embedded()
        .map_err(|errors| errors.to_string())
        .and_then(|content| lines::rows(&content.dialogue, scene));
    match rows {
        Ok(rows) => {
            for row in rows {
                println!("{row}");
            }
            0
        }
        Err(e) => {
            eprintln!("lines: {e}");
            1
        }
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
        for other in ["bots", "core", "content", "ui", "xtask"] {
            let manifest =
                std::fs::read_to_string(root.join(format!("crates/{other}/Cargo.toml"))).unwrap();
            assert_eq!(table(&manifest, "[lints]"), ["workspace = true"], "{other}");
        }
    }

    /// The playtest bots are dev tooling (ADR-0033): nothing the game is
    /// built from may depend on them, and they may not depend on the game's
    /// look (ADR-0038). Every way to a crate of this workspace is a
    /// manifest here, so reading the manifests covers indirect ones too.
    #[test]
    fn bots_stay_out_of_the_game() {
        // A manifest without its comments.
        let manifest = |name: &str| {
            let path = repo_root().join(format!("crates/{name}/Cargo.toml"));
            let text = std::fs::read_to_string(path).unwrap();
            let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
            lines.join("\n")
        };
        for game in ["core", "content", "ui", "app"] {
            assert!(!manifest(game).contains("trpg-bots"), "{game}");
        }
        for bots in ["bots", "core", "content"] {
            let manifest = manifest(bots);
            assert!(!manifest.contains("trpg-ui"), "{bots}");
            assert!(!manifest.contains("trpg-app"), "{bots}");
        }
        assert!(manifest("xtask").contains("trpg-bots"));
    }

    /// The names of the `.rs` files directly in `dir` (not in its subfolders).
    fn top_level_rs_files(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "rs"))
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The workflow and action files under `.github`.
    fn workflow_files() -> Vec<PathBuf> {
        let github = repo_root().join(".github");
        let mut files = Vec::new();
        for entry in std::fs::read_dir(github.join("workflows")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
                files.push(path);
            }
        }
        for entry in std::fs::read_dir(github.join("actions")).unwrap() {
            let action = entry.unwrap().path().join("action.yml");
            if action.is_file() {
                files.push(action);
            }
        }
        files.sort();
        files
    }

    /// GitHub runs no job of a workflow file that is not valid YAML, and only
    /// says so after the merge: an unquoted `run:` line ending in `::`
    /// stopped every Pages deploy (ticket 0118).
    #[test]
    fn every_workflow_file_parses() {
        let files = workflow_files();
        assert!(files.len() >= 9, "{files:?}");
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            if let Err(e) = serde_norway::from_str::<serde_norway::Value>(&text) {
                panic!("{} is not valid YAML: {e}", file.display());
            }
        }

        // The check sees the mistake.
        let broken = "steps:
  - run: cargo test private_assets::
";
        assert!(serde_norway::from_str::<serde_norway::Value>(broken).is_err());
    }

    /// Cargo links every file directly under a crate's `tests/` as its own
    /// program (about 100 MB each), so each crate keeps one: new integration
    /// tests are modules of `tests/it/main.rs` (ticket 0114).
    #[test]
    fn each_crate_has_at_most_one_integration_test_program() {
        let crates = std::fs::read_dir(repo_root().join("crates")).unwrap();
        let mut checked = 0;
        for krate in crates {
            let tests = krate.unwrap().path().join("tests");
            if !tests.is_dir() {
                continue;
            }
            checked += 1;
            let files = top_level_rs_files(&tests);
            assert!(
                files.len() <= 1,
                "{} has {files:?}: put new integration tests in tests/it/ \
                 as a module listed in tests/it/main.rs",
                tests.display()
            );
        }
        assert!(checked >= 4, "{checked}");

        // The check sees a second file.
        let dir = std::env::temp_dir().join(format!("xtask-tests-dir-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("it")).unwrap();
        std::fs::write(dir.join("it/main.rs"), "").unwrap();
        std::fs::write(dir.join("a.rs"), "").unwrap();
        std::fs::write(dir.join("notes.md"), "").unwrap();
        assert_eq!(top_level_rs_files(&dir), ["a.rs"]);
        std::fs::write(dir.join("b.rs"), "").unwrap();
        assert_eq!(top_level_rs_files(&dir), ["a.rs", "b.rs"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn playtest_help_and_bad_args() {
        assert_eq!(playtest(&args(&["--help"])), 0);
        assert_eq!(playtest(&args(&["quick", "--help"])), 0);
        assert_eq!(playtest(&[]), 2);
        assert_eq!(playtest(&args(&["quick", "--runs", "0"])), 2);
        assert_eq!(dispatch(args(&["playtest", "--bogus", "1"]).into_iter()), 2);
    }

    #[test]
    fn playtest_reports_a_battle_and_fails_on_an_unknown_one() {
        let history = std::env::temp_dir().join(format!("xtask-playtest-{}", std::process::id()));
        let history_arg = history.to_string_lossy().into_owned();
        let run = |battle: &str| {
            let run = ["playtest", battle, "--runs", "2", "--history", &history_arg];
            dispatch(args(&run).into_iter())
        };
        assert_eq!(run("quick"), 0);
        assert!(history.join("quick-classic-baseline.jsonl").is_file());
        assert_eq!(run("no_such_battle"), 1);
        std::fs::remove_dir_all(&history).unwrap();
    }

    #[test]
    fn frame_png_help_bad_args_and_a_run() {
        assert_eq!(frame_png(&args(&["--help"])), 0);
        assert_eq!(frame_png(&[]), 2);
        assert_eq!(
            dispatch(args(&["frame-png", "a.png", "--bogus"]).into_iter()),
            2
        );
        let out = std::env::temp_dir().join(format!("xtask-frame-png-{}.png", std::process::id()));
        let out_arg = out.to_string_lossy().into_owned();
        assert_eq!(frame_png(&args(&[&out_arg, "--scale", "1"])), 0);
        assert!(out.is_file());
        std::fs::remove_file(&out).unwrap();
    }

    #[test]
    fn lines_lists_a_scene_and_fails_on_an_unknown_one() {
        assert_eq!(dispatch(args(&["lines", "ch01_intro"]).into_iter()), 0);
        assert_eq!(lines(&[]), 0);
        assert_eq!(lines(&args(&["--help"])), 0);
        assert_eq!(lines(&args(&["no_such_scene"])), 1);
        assert_eq!(lines(&args(&["ch01_intro", "ch01_prebattle"])), 2);
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
    fn private_assets_fails_fast_on_bad_args() {
        // Only the argument check: a real run needs the private repository
        // and would move this checkout's `assets-private/`.
        assert_eq!(private_assets(&args(&["--bogus"])), 2);
        assert_eq!(
            dispatch(args(&["private-assets", "--pin", "--library"]).into_iter()),
            2
        );
    }

    #[test]
    fn portrait_import_help_and_bad_args() {
        // Only the argument check: a real run writes into `assets-private/`.
        assert_eq!(portrait_import(&args(&["--help"])), 0);
        assert_eq!(portrait_import(&[]), 2);
        assert_eq!(portrait_import(&args(&["busts", "Not An Id"])), 2);
        // A source that isn't there fails before anything is written.
        assert_eq!(portrait_import(&args(&["no/such/folder", "k"])), 1);
        assert_eq!(
            dispatch(args(&["portrait-import", "busts", "k", "--shift-x", "9"]).into_iter()),
            2
        );
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
    fn test_card_rejects_args() {
        assert_eq!(test_card(&args(&["--bogus"])), 2);
        assert_eq!(dispatch(args(&["test-card", "x"]).into_iter()), 2);
    }

    #[test]
    fn map_sprite_import_help_list_and_bad_args() {
        // Only the argument check: a real run writes into `assets-private/`.
        assert_eq!(map_sprite_import(&args(&["--help"])), 0);
        assert_eq!(map_sprite_import(&args(&["--list"])), 0);
        assert_eq!(map_sprite_import(&args(&["--bogus"])), 2);
        // No arguments runs it: in a folder with no checkout, that fails.
        let bare = std::env::temp_dir().join(format!("xtask-msi-none-{}", std::process::id()));
        assert_eq!(map_sprite_import_in(&bare, &[]), 1);
        assert_eq!(
            dispatch(args(&["map-sprite-import", "--list", "x"]).into_iter()),
            2
        );
    }

    #[test]
    fn effect_marks_rejects_args() {
        assert_eq!(effect_marks(&args(&["--bogus"])), 2);
        assert_eq!(dispatch(args(&["effect-marks", "x"]).into_iter()), 2);
    }

    #[test]
    fn test_tileset_rejects_args() {
        assert_eq!(test_tileset(&args(&["--bogus"])), 2);
        assert_eq!(dispatch(args(&["test-tileset", "x"]).into_iter()), 2);
    }

    #[test]
    fn check_keys_passes_on_the_real_repo() {
        assert_eq!(check_keys(&[]), 0);
        assert_eq!(dispatch(args(&["check-keys"]).into_iter()), 0);
    }

    #[test]
    fn check_text_passes_on_the_real_repo() {
        assert_eq!(check_text(&[]), 0);
        assert_eq!(dispatch(args(&["check-text"]).into_iter()), 0);
        assert_eq!(check_text(&args(&["--bogus"])), 2);
    }

    #[test]
    fn lang_status_exits_zero_with_missing_and_stale_keys() {
        // The test pack has one of each.
        assert_eq!(lang_status(&args(&["test"])), 0);
        assert_eq!(dispatch(args(&["lang-status", "test"]).into_iter()), 0);
        assert_eq!(lang_status(&args(&["zz"])), 2);
        assert_eq!(lang_status(&[]), 2);
        assert_eq!(lang_status(&args(&["test", "ja"])), 2);
    }

    #[test]
    fn check_keys_rejects_args() {
        assert_eq!(check_keys(&args(&["--bogus"])), 2);
    }

    #[test]
    fn clean_merged_targets_rejects_unknown_args() {
        // Only the argument check: a real run needs `gh` and would delete
        // build folders.
        assert_eq!(clean_merged_targets(&args(&["--bogus"])), 2);
        assert_eq!(
            dispatch(args(&["clean-merged-targets", "--dry-run", "x"]).into_iter()),
            2
        );
    }

    #[test]
    fn dispatch_runs_ticket_lint() {
        assert_eq!(dispatch(args(&["ticket-lint"]).into_iter()), 0);
    }
}
