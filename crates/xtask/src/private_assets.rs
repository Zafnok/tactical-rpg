//! `cargo xtask private-assets [--library | --pin]`: fetches the bought art
//! (ticket 0110, ADR-0040). Its licence forbids redistributing the files and
//! this repository is public, so they live in a private repository, checked
//! out into the git-ignored `assets-private/`:
//!
//! - `game/`: the files embedded in the game over `assets/` when it is built
//!   with the `private-assets` feature.
//! - `voice/`: the voice clips (ADR-0046), which ship as files beside the
//!   game. Copied to the git-ignored `voice/` in this repository's root
//!   when the private repository has them.
//! - `library/`: the bought packs as sorted, which the importers read.
//! - the rest (the original downloads, the seller's tools): only in a full
//!   clone.
//!
//! `assets-private.rev` in this repository names the commit of the private
//! one that this checkout's code is built with. With no flag the command
//! clones `assets-private/` if it isn't there (only `game/` and `voice/`,
//! and without the history's files), fetches, and puts it at that commit. `--library` also
//! brings `library/`. `--pin` writes the commit `assets-private/` is at into
//! `assets-private.rev`, after checking that it is pushed.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Usage line for bad arguments.
pub const USAGE: &str = "usage: cargo xtask private-assets [--library | --pin]";

/// The private repository.
pub const REPO_URL: &str = "https://github.com/Zafnok/visions-of-shuyi-assets.git";

/// Where it is checked out, in the repo root.
const CHECKOUT: &str = "assets-private";

/// The file in the repo root that names the commit to build with.
const PIN_FILE: &str = "assets-private.rev";

/// The folder of the private repository that is embedded in the game.
const GAME_DIR: &str = "game";
/// The folder of the private repository with the voice clips (ADR-0046).
/// Beside `game/`, not in it: everything in `game/` is embedded, and
/// voices ship as files next to the game. Copied to `voice/` in this
/// repository's root, where the game and the packaging look for it.
const VOICE_DIR: &str = "voice";

/// The folder of the private repository that the importers read.
const LIBRARY_DIR: &str = "library";

/// What the command was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Clone or update `assets-private/` and put it at the pinned commit.
    Fetch {
        /// Whether to bring `library/` as well as `game/`.
        library: bool,
    },
    /// Write the commit `assets-private/` is at into the pin file.
    Pin,
}

/// Parses the arguments after `private-assets`.
pub fn parse_args(args: &[String]) -> Result<Mode, String> {
    match args {
        [] => Ok(Mode::Fetch { library: false }),
        [flag] if flag == "--library" => Ok(Mode::Fetch { library: true }),
        [flag] if flag == "--pin" => Ok(Mode::Pin),
        _ => Err(USAGE.to_string()),
    }
}

/// Runs `git <args>` in `dir`: its output, or what it printed as the error.
/// It never stops to ask for a password on the terminal.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("spawn git: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The error for a clone or fetch of `url` that git refused with `said`.
fn no_access(url: &str, said: &str) -> String {
    format!(
        "can't read {url}\n  git said: {said}\n  \
         The bought art is in a private repository that only Nick's GitHub account and \
         the build key can read. Without it the game still builds and runs, with the \
         public placeholders: leave out `--features private-assets`."
    )
}

/// The commit named by the pin file: 40 lower-case hex digits.
fn read_pin(repo_root: &Path) -> Result<String, String> {
    let text =
        fs::read_to_string(repo_root.join(PIN_FILE)).map_err(|e| format!("{PIN_FILE}: {e}"))?;
    let pin = text.trim();
    let is_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    if pin.len() != 40 || !pin.chars().all(is_hex) {
        return Err(format!(
            "{PIN_FILE}: expected a 40-character commit id, found `{pin}`"
        ));
    }
    Ok(pin.to_string())
}

/// Whether `dir` is a folder with something in it.
fn has_entries(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// Clones `url` into `assets-private/` if it isn't there, fetches, and puts
/// it at the pinned commit. A clone made here holds only `game/` (and
/// `library/` once asked for with `library`); a full clone made by hand is
/// left whole. Returns a one-line summary.
fn fetch(repo_root: &Path, url: &str, library: bool) -> Result<String, String> {
    let pin = read_pin(repo_root)?;
    let checkout = repo_root.join(CHECKOUT);
    if checkout.join(".git").exists() {
        git(&checkout, &["fetch", "--quiet", "origin"]).map_err(|e| no_access(url, &e))?;
    } else {
        if has_entries(&checkout) {
            return Err(format!(
                "{CHECKOUT}/ has files but isn't a git checkout: move it away first"
            ));
        }
        let clone = ["clone", "--quiet", "--filter=blob:none", "--no-checkout"];
        let long_paths = ["--config", "core.longpaths=true"];
        git(
            repo_root,
            &[&clone[..], &long_paths[..], &[url, CHECKOUT]].concat(),
        )
        .map_err(|e| no_access(url, &e))?;
        let folders = ["sparse-checkout", "set", "--cone", GAME_DIR, VOICE_DIR];
        git(&checkout, &folders)?;
    }

    // Unset (a full clone) is an error from `git config --get`.
    let sparse =
        git(&checkout, &["config", "--get", "core.sparseCheckout"]).as_deref() == Ok("true");
    if sparse {
        // A clone made before voices existed holds only `game/`.
        git(&checkout, &["sparse-checkout", "add", VOICE_DIR])?;
    }
    if library && sparse {
        git(&checkout, &["sparse-checkout", "add", LIBRARY_DIR])?;
    }

    // A clone whose files were never checked out (new, or a first fetch that
    // stopped half-way) has an empty index. Otherwise a checkout already at
    // the pin is left as it is (on its branch, if any).
    let never_checked_out = git(&checkout, &["ls-files"])?.is_empty();
    if never_checked_out || git(&checkout, &["rev-parse", "HEAD"])? != pin {
        git(&checkout, &["checkout", "--quiet", "--detach", &pin]).map_err(|e| {
            format!("can't check out the pinned commit {pin} in {CHECKOUT}/\n  git said: {e}")
        })?;
    }

    let holds = if sparse {
        let folders = git(&checkout, &["sparse-checkout", "list"])?;
        folders.lines().collect::<Vec<_>>().join(", ")
    } else {
        "the whole repository".to_string()
    };
    let voices = match copy_voices(&checkout.join(VOICE_DIR), &repo_root.join(VOICE_DIR))? {
        Some(files) => format!("; {VOICE_DIR}/ holds its {files} voice files"),
        None => String::new(),
    };
    Ok(format!("{CHECKOUT}/ is at {pin} ({holds}){voices}"))
}

/// Replaces `dst` with a copy of the private repository's voice folder
/// `src`, returning how many files it holds. With no such folder (no
/// voices yet), `dst` is left as it is and `None` is returned.
fn copy_voices(src: &Path, dst: &Path) -> Result<Option<usize>, String> {
    if !src.is_dir() {
        return Ok(None);
    }
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("clear {}: {e}", dst.display()))?;
    }
    crate::web::copy_tree(src, dst, &|_| true).map(Some)
}

/// Writes the commit `assets-private/` is at into the pin file. Refuses when
/// there are uncommitted changes, or when the commit isn't on the private
/// repository's `main` yet: the builds that ship fetch the pin from there.
fn pin(repo_root: &Path, url: &str) -> Result<String, String> {
    let checkout = repo_root.join(CHECKOUT);
    if !checkout.join(".git").exists() {
        return Err(format!(
            "{CHECKOUT}/ isn't there: run `cargo xtask private-assets` first"
        ));
    }
    let changes = git(&checkout, &["status", "--porcelain"])?;
    if !changes.is_empty() {
        return Err(format!(
            "{CHECKOUT}/ has changes that aren't committed:\n{changes}\n\
             commit and push them, then run this again"
        ));
    }
    git(&checkout, &["fetch", "--quiet", "origin"]).map_err(|e| no_access(url, &e))?;
    let head = git(&checkout, &["rev-parse", "HEAD"])?;
    let pushed = git(&checkout, &["branch", "--remotes", "--contains", &head])?;
    if !pushed.lines().any(|branch| branch.trim() == "origin/main") {
        return Err(format!(
            "commit {head} isn't on the private repository's main branch: push it \
             (`git -C {CHECKOUT} push origin HEAD:main`), then run this again"
        ));
    }
    fs::write(repo_root.join(PIN_FILE), format!("{head}\n"))
        .map_err(|e| format!("{PIN_FILE}: {e}"))?;
    Ok(format!(
        "{PIN_FILE} now names {head}: commit it with your change"
    ))
}

/// Runs the command in `repo_root` against the private repository at `url`.
pub fn run(repo_root: &Path, url: &str, mode: Mode) -> Result<String, String> {
    match mode {
        Mode::Fetch { library } => fetch(repo_root, url, library),
        Mode::Pin => pin(repo_root, url),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Who commits in the scratch repositories (a runner has no git identity).
    const AS_TESTER: [&str; 6] = [
        "-c",
        "user.name=tester",
        "-c",
        "user.email=tester@example.com",
        "-c",
        "commit.gpgsign=false",
    ];

    /// `git <args>` in `dir`, which must work.
    fn g(dir: &Path, args: &[&str]) -> String {
        git(dir, args).unwrap_or_else(|e| panic!("git {args:?}: {e}"))
    }

    /// Commits everything in `dir` and returns the commit id.
    fn commit(dir: &Path, message: &str) -> String {
        g(dir, &["add", "--all"]);
        g(
            dir,
            &[&AS_TESTER[..], &["commit", "--quiet", "-m", message]].concat(),
        );
        g(dir, &["rev-parse", "HEAD"])
    }

    /// A scratch folder under `env::temp_dir()` holding a stand-in for the
    /// private repository (`remote.git`, filled from `src/`) and for this
    /// one (`public/`).
    struct Scratch {
        root: PathBuf,
        /// The private repository's first commit: `game/` says `one`.
        one: String,
        /// Its second commit, the tip of `main`: `game/` says `two`, and
        /// there is a `voice/` folder with a manifest and a clip.
        two: String,
    }

    impl Scratch {
        fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("xtask-private-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let src = root.join("src");
            for dir in ["game/portraits", "library/tiny-tales", "originals"] {
                fs::create_dir_all(src.join(dir)).unwrap();
            }
            fs::create_dir_all(root.join("public")).unwrap();
            g(&root, &["init", "--quiet", "--initial-branch=main", "src"]);
            fs::write(src.join("README.md"), "private").unwrap();
            fs::write(src.join("game/portraits/a.portrait"), "one").unwrap();
            fs::write(src.join("library/tiny-tales/face.png"), "face").unwrap();
            fs::write(src.join("originals/pack.zip"), "zip").unwrap();
            let one = commit(&src, "one");
            fs::write(src.join("game/portraits/a.portrait"), "two").unwrap();
            fs::create_dir_all(src.join("voice/en/scene")).unwrap();
            fs::write(src.join("voice/en/voice.ron"), "manifest").unwrap();
            fs::write(src.join("voice/en/scene/line.ogg"), "clip").unwrap();
            let two = commit(&src, "two");
            g(&root, &["clone", "--quiet", "--bare", "src", "remote.git"]);
            Self { root, one, two }
        }

        /// The stand-in for this repository's root.
        fn public(&self) -> PathBuf {
            self.root.join("public")
        }

        /// The address of the stand-in private repository.
        fn url(&self) -> String {
            self.root.join("remote.git").to_string_lossy().into_owned()
        }

        /// A path inside the checkout, `public/assets-private/`.
        fn checkout(&self, path: &str) -> PathBuf {
            self.public().join(CHECKOUT).join(path)
        }

        fn set_pin(&self, commit: &str) {
            fs::write(self.public().join(PIN_FILE), format!("{commit}\n")).unwrap();
        }

        fn fetch(&self, library: bool) -> Result<String, String> {
            run(&self.public(), &self.url(), Mode::Fetch { library })
        }

        fn pin(&self) -> Result<String, String> {
            run(&self.public(), &self.url(), Mode::Pin)
        }

        /// What `game/portraits/a.portrait` says in the checkout.
        fn game_says(&self) -> String {
            fs::read_to_string(self.checkout("game/portraits/a.portrait")).unwrap()
        }

        /// A full clone of the private repository in place of the checkout,
        /// as on the machine the art was bought on.
        fn clone_whole(&self) {
            g(&self.public(), &["clone", "--quiet", &self.url(), CHECKOUT]);
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn parse_args_reads_the_mode() {
        assert_eq!(parse_args(&[]), Ok(Mode::Fetch { library: false }));
        assert_eq!(
            parse_args(&args(&["--library"])),
            Ok(Mode::Fetch { library: true })
        );
        assert_eq!(parse_args(&args(&["--pin"])), Ok(Mode::Pin));
        for bad in [&["--bogus"][..], &["--library", "--pin"], &["--pin", "x"]] {
            assert_eq!(parse_args(&args(bad)), Err(USAGE.to_string()), "{bad:?}");
        }
    }

    #[test]
    fn the_pin_file_must_name_a_commit() {
        let scratch = Scratch::new("pin-file");
        let root = scratch.public();
        assert!(
            read_pin(&root)
                .unwrap_err()
                .starts_with("assets-private.rev: ")
        );
        scratch.set_pin(&scratch.one);
        assert_eq!(read_pin(&root), Ok(scratch.one.clone()));
        for bad in [
            "",
            "main",
            &scratch.one[..39],
            &format!("{}0", scratch.one),
            &scratch.one.to_uppercase().replace('0', "A"),
            &format!("g{}", &scratch.one[1..]),
        ] {
            scratch.set_pin(bad);
            assert_eq!(
                read_pin(&root),
                Err(format!(
                    "assets-private.rev: expected a 40-character commit id, found `{bad}`"
                )),
            );
        }
    }

    #[test]
    fn a_first_fetch_clones_only_the_game_and_voice_folders() {
        let scratch = Scratch::new("first");
        scratch.set_pin(&scratch.two);
        // An empty folder is fine to clone into.
        fs::create_dir_all(scratch.checkout("")).unwrap();
        assert_eq!(
            scratch.fetch(false),
            Ok(format!(
                "assets-private/ is at {} (game, voice); voice/ holds its 2 voice files",
                scratch.two
            ))
        );
        assert_eq!(scratch.game_says(), "two");
        // Files in the private repository's root come along; nothing else.
        assert!(scratch.checkout("README.md").is_file());
        assert!(!scratch.checkout("library").exists());
        assert!(!scratch.checkout("originals").exists());
    }

    /// Ticket 0238: the private repository's `voice/` is copied to this
    /// repository's `voice/`, replacing what was there; without one,
    /// `voice/` is left alone.
    #[test]
    fn a_fetch_copies_the_voice_folder_beside_the_game() {
        let scratch = Scratch::new("voice");
        let voice = scratch.public().join("voice");
        let read = |path: &str| fs::read_to_string(voice.join(path)).ok();
        // The first commit has no voices: a folder made by hand stays.
        scratch.set_pin(&scratch.one);
        fs::create_dir_all(voice.join("en")).unwrap();
        fs::write(voice.join("en/mine.ogg"), "mine").unwrap();
        assert_eq!(
            scratch.fetch(false),
            Ok(format!(
                "assets-private/ is at {} (game, voice)",
                scratch.one
            ))
        );
        assert_eq!(read("en/mine.ogg").as_deref(), Some("mine"));
        // The second has: its files replace the folder's.
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(read("en/voice.ron").as_deref(), Some("manifest"));
        assert_eq!(read("en/scene/line.ogg").as_deref(), Some("clip"));
        assert_eq!(read("en/mine.ogg"), None);
        // A clone made before voices existed (only `game/`) gets them too.
        let old = Scratch::new("voice-old-clone");
        old.set_pin(&old.two);
        old.fetch(false).unwrap();
        let checkout = old.checkout("");
        git(&checkout, &["sparse-checkout", "set", "--cone", GAME_DIR]).unwrap();
        assert!(!old.checkout("voice").exists());
        fs::remove_dir_all(old.public().join("voice")).unwrap();
        old.fetch(false).unwrap();
        assert!(old.public().join("voice/en/voice.ron").is_file());
    }

    #[test]
    fn copy_voices_needs_a_source_folder() {
        let root = std::env::temp_dir().join(format!("xtask-copy-voices-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("dst")).unwrap();
        fs::write(root.join("dst/kept.ogg"), "kept").unwrap();
        // No source, or a file by that name: nothing happens.
        assert_eq!(copy_voices(&root.join("src"), &root.join("dst")), Ok(None));
        fs::write(root.join("src"), "a file").unwrap();
        assert_eq!(copy_voices(&root.join("src"), &root.join("dst")), Ok(None));
        assert!(root.join("dst/kept.ogg").is_file());
        // An empty source empties the destination.
        fs::remove_file(root.join("src")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        assert_eq!(
            copy_voices(&root.join("src"), &root.join("dst")),
            Ok(Some(0))
        );
        assert!(!root.join("dst/kept.ogg").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_fetch_puts_the_checkout_at_the_pinned_commit() {
        let scratch = Scratch::new("pinned");
        // The pin is an older commit than the tip of `main`.
        scratch.set_pin(&scratch.one);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "one");
        // Fetching again changes nothing.
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "one");
        // The pin moves: so does the checkout.
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "two");
    }

    #[test]
    fn a_fetch_sees_commits_pushed_after_the_clone() {
        let scratch = Scratch::new("later");
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        let src = scratch.root.join("src");
        fs::write(src.join("game/portraits/a.portrait"), "three").unwrap();
        let three = commit(&src, "three");
        g(&src, &["push", "--quiet", "../remote.git", "main"]);
        scratch.set_pin(&three);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "three");
    }

    #[test]
    fn library_brings_the_library_folder_and_keeps_it() {
        let scratch = Scratch::new("library");
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(
            scratch.fetch(true),
            Ok(format!(
                "assets-private/ is at {} (game, library, voice); voice/ holds its 2 voice files",
                scratch.two
            ))
        );
        assert!(scratch.checkout("library/tiny-tales/face.png").is_file());
        assert!(!scratch.checkout("originals").exists());
        // A later plain fetch keeps it.
        scratch.fetch(false).unwrap();
        assert!(scratch.checkout("library/tiny-tales/face.png").is_file());

        // Asked for on the first fetch, it comes with the clone.
        let first = Scratch::new("library-first");
        first.set_pin(&first.one);
        first.fetch(true).unwrap();
        assert!(first.checkout("library/tiny-tales/face.png").is_file());
        assert_eq!(first.game_says(), "one");
    }

    #[test]
    fn a_whole_clone_is_left_whole_and_on_its_branch() {
        let scratch = Scratch::new("whole");
        scratch.clone_whole();
        scratch.set_pin(&scratch.two);
        assert_eq!(
            scratch.fetch(true),
            Ok(format!(
                "assets-private/ is at {} (the whole repository); voice/ holds its 2 voice files",
                scratch.two
            ))
        );
        assert!(scratch.checkout("originals/pack.zip").is_file());
        assert!(scratch.checkout("library/tiny-tales/face.png").is_file());
        let branch = |scratch: &Scratch| git(&scratch.checkout(""), &["symbolic-ref", "HEAD"]);
        assert_eq!(branch(&scratch), Ok("refs/heads/main".to_string()));
        // A pin elsewhere moves it off the branch, which stays where it was.
        scratch.set_pin(&scratch.one);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "one");
        assert!(branch(&scratch).is_err());
        assert_eq!(
            g(&scratch.checkout(""), &["rev-parse", "main"]),
            scratch.two
        );
        assert!(scratch.checkout("originals/pack.zip").is_file());
    }

    #[test]
    fn a_fetch_says_why_it_failed() {
        let scratch = Scratch::new("fails");
        // No pin file.
        assert!(
            scratch
                .fetch(false)
                .unwrap_err()
                .starts_with("assets-private.rev: ")
        );
        assert!(!scratch.checkout("").exists());

        // A folder that isn't a checkout is never cloned over.
        scratch.set_pin(&scratch.two);
        fs::create_dir_all(scratch.checkout("game")).unwrap();
        assert_eq!(
            scratch.fetch(false),
            Err("assets-private/ has files but isn't a git checkout: move it away first".into())
        );
        fs::remove_dir_all(scratch.checkout("")).unwrap();

        // A repository that can't be read.
        let nowhere = scratch.root.join("nowhere.git");
        let nowhere = nowhere.to_string_lossy();
        let error = run(&scratch.public(), &nowhere, Mode::Fetch { library: false }).unwrap_err();
        assert!(
            error.starts_with(&format!("can't read {nowhere}\n  git said: ")),
            "{error}"
        );
        assert!(error.contains("private repository"), "{error}");
        assert!(error.contains("public placeholders"), "{error}");

        // A pin that isn't a commit of the repository.
        let unknown = "0123456789abcdef0123456789abcdef01234567";
        scratch.set_pin(unknown);
        let error = scratch.fetch(false).unwrap_err();
        assert!(
            error.starts_with(&format!(
                "can't check out the pinned commit {unknown} in assets-private/\n  git said: "
            )),
            "{error}"
        );

        // That left a clone with no files checked out. With a good pin the
        // next fetch finishes the job, though the clone is already "at" it.
        assert!(!scratch.checkout("game").exists());
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "two");
        assert!(!scratch.checkout("library").exists());

        // The repository going away fails the fetch.
        fs::remove_dir_all(scratch.root.join("remote.git")).unwrap();
        let error = scratch.fetch(false).unwrap_err();
        assert!(error.starts_with("can't read "), "{error}");
    }

    #[test]
    fn pin_writes_the_commit_once_it_is_pushed() {
        let scratch = Scratch::new("pin");
        assert_eq!(
            scratch.pin(),
            Err("assets-private/ isn't there: run `cargo xtask private-assets` first".into())
        );

        scratch.clone_whole();
        let checkout = scratch.checkout("");
        scratch.set_pin(&scratch.one);

        // A new file that isn't committed.
        fs::write(scratch.checkout("game/portraits/b.portrait"), "new").unwrap();
        let error = scratch.pin().unwrap_err();
        assert!(
            error.starts_with("assets-private/ has changes that aren't committed:\n"),
            "{error}"
        );
        assert!(error.contains("game/portraits/b.portrait"), "{error}");

        // Committed, not pushed.
        let new = commit(&checkout, "b");
        let error = scratch.pin().unwrap_err();
        assert!(
            error.starts_with(&format!(
                "commit {new} isn't on the private repository's main branch"
            )),
            "{error}"
        );
        assert_eq!(read_pin(&scratch.public()), Ok(scratch.one.clone()));

        // Pushed, but to another branch than `main`: the builds that ship
        // can't be sure to find it there.
        g(&checkout, &["push", "--quiet", "origin", "HEAD:side"]);
        let error = scratch.pin().unwrap_err();
        assert!(
            error.starts_with(&format!(
                "commit {new} isn't on the private repository's main branch"
            )),
            "{error}"
        );
        assert_eq!(read_pin(&scratch.public()), Ok(scratch.one.clone()));

        // Pushed to `main`.
        g(&checkout, &["push", "--quiet", "origin", "HEAD:main"]);
        assert_eq!(
            scratch.pin(),
            Ok(format!(
                "assets-private.rev now names {new}: commit it with your change"
            ))
        );
        assert_eq!(
            fs::read_to_string(scratch.public().join(PIN_FILE)).unwrap(),
            format!("{new}\n")
        );

        // The private repository can't be read: no pin is written.
        scratch.set_pin(&scratch.one);
        fs::remove_dir_all(scratch.root.join("remote.git")).unwrap();
        assert!(scratch.pin().unwrap_err().starts_with("can't read "));
        assert_eq!(read_pin(&scratch.public()), Ok(scratch.one.clone()));
    }
}
