//! `cargo xtask clean-merged-targets [--dry-run]`: deletes the `target/`
//! build folder of every git worktree whose branch's PR has merged (ticket
//! 0113). Each ticket session builds in its own worktree under
//! `.claude/worktrees/`, and nothing else removes a worktree's 10+ GB of
//! build output once its PR is in.
//!
//! It asks `git worktree list --porcelain` for the worktrees and
//! `gh pr list` for the state of every PR, then decides per worktree with
//! [`verdict`]. It only ever deletes a folder named `target` directly inside
//! a worktree: never a worktree, a branch or anything else (Claude chat
//! sessions are tied to their worktree folder). If `git` or `gh` fails,
//! nothing is deleted.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use serde_norway::Value;

/// Usage line for bad arguments.
pub const USAGE: &str = "usage: cargo xtask clean-merged-targets [--dry-run]";

/// A `target/` with a file written more recently than this is kept: another
/// session may be building there.
const QUIET_TIME: Duration = Duration::from_mins(30);

/// Bytes in a GB, as sizes are printed.
const GB: u64 = 1 << 30;

/// Parses the arguments after `clean-merged-targets`: `Ok(dry_run)`.
pub fn parse_args(args: &[String]) -> Result<bool, String> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--dry-run" => Ok(true),
        _ => Err(USAGE.to_string()),
    }
}

/// One entry of `git worktree list`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Worktree {
    /// The worktree's folder.
    path: PathBuf,
    /// Its checked-out branch (without `refs/heads/`); `None` when detached.
    branch: Option<String>,
}

/// Reads `git worktree list --porcelain`: blocks of `key value` lines, each
/// starting with `worktree <path>`. The main checkout comes first.
fn parse_worktrees(porcelain: &str) -> Vec<Worktree> {
    let mut worktrees: Vec<Worktree> = Vec::new();
    for line in porcelain.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            worktrees.push(Worktree {
                path: PathBuf::from(path),
                branch: None,
            });
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/")
            && let Some(worktree) = worktrees.last_mut()
        {
            worktree.branch = Some(branch.to_string());
        }
    }
    worktrees
}

/// A PR's state, as `gh` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrState {
    /// `OPEN`
    Open,
    /// `MERGED`
    Merged,
    /// `CLOSED` (without merging)
    Closed,
}

/// Reads `gh pr list --json headRefName,state` into (branch, state) pairs.
/// JSON is valid YAML, so `serde_norway` reads it. Anything unexpected is an
/// error: a guess here could delete the wrong folder.
fn parse_prs(json: &str) -> Result<Vec<(String, PrState)>, String> {
    let Ok(Value::Sequence(items)) = serde_norway::from_str::<Value>(json) else {
        return Err("gh pr list: expected a JSON list".to_string());
    };
    items
        .iter()
        .map(|item| {
            let field = |key: &str| item.get(key).and_then(Value::as_str);
            let branch = field("headRefName").ok_or("gh pr list: a PR without headRefName")?;
            let state = match field("state") {
                Some("OPEN") => PrState::Open,
                Some("MERGED") => PrState::Merged,
                Some("CLOSED") => PrState::Closed,
                other => return Err(format!("gh pr list: unknown PR state {other:?}")),
            };
            Ok((branch.to_string(), state))
        })
        .collect()
}

/// Which worktree this is, to the running command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// The main checkout (git lists it first).
    Main,
    /// The worktree the command runs in.
    Current,
    /// Any other worktree.
    Other,
}

/// What is at a worktree's `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// Nothing.
    Missing,
    /// A link, a junction or a file: not ours to delete.
    NotAFolder,
    /// A folder that couldn't be read all the way through.
    Unreadable(String),
    /// A real folder. `newest_write_age` is how long ago its most recently
    /// modified file was written; `None` when it holds no files.
    Folder {
        /// See [`Target::Folder`].
        newest_write_age: Option<Duration>,
    },
}

/// Why a `target/` is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Keep {
    /// It is the main checkout's.
    MainCheckout,
    /// It is the current worktree's.
    CurrentWorktree,
    /// The branch (or detached HEAD) has no PR.
    NoPr,
    /// The branch has an open PR.
    OpenPr,
    /// The branch's PRs were closed without merging.
    ClosedPr,
    /// There is no `target/`.
    NoTarget,
    /// `target` is a link or a file.
    NotAFolder,
    /// `target/` couldn't be read.
    Unreadable(String),
    /// A file in it was written this long ago, less than [`QUIET_TIME`].
    RecentWrite(Duration),
}

impl std::fmt::Display for Keep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MainCheckout => write!(f, "main checkout"),
            Self::CurrentWorktree => write!(f, "current worktree"),
            Self::NoPr => write!(f, "no PR"),
            Self::OpenPr => write!(f, "open PR"),
            Self::ClosedPr => write!(f, "PR closed without merging"),
            Self::NoTarget => write!(f, "no target folder"),
            Self::NotAFolder => write!(f, "target is a link, not a folder"),
            Self::Unreadable(error) => write!(f, "could not read target: {error}"),
            Self::RecentWrite(age) => {
                write!(f, "target written {} min ago", age.as_secs() / 60)
            }
        }
    }
}

/// The decision for one worktree's `target/`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    /// Delete it.
    Delete,
    /// Keep it, and why.
    Keep(Keep),
}

/// Decides one worktree: its `target/` goes only when the branch has a
/// merged PR and no open one, the worktree is neither the main checkout nor
/// the current one, and `target/` is a real folder nobody wrote to in the
/// last [`QUIET_TIME`]. `prs` are the states of the branch's PRs. `target`
/// is only looked at (it walks the whole folder) when everything else says
/// delete.
fn verdict(place: Place, prs: &[PrState], target: impl FnOnce() -> Target) -> Verdict {
    let keep = match place {
        Place::Main => Some(Keep::MainCheckout),
        Place::Current => Some(Keep::CurrentWorktree),
        Place::Other if prs.is_empty() => Some(Keep::NoPr),
        Place::Other if prs.contains(&PrState::Open) => Some(Keep::OpenPr),
        Place::Other if !prs.contains(&PrState::Merged) => Some(Keep::ClosedPr),
        Place::Other => match target() {
            Target::Missing => Some(Keep::NoTarget),
            Target::NotAFolder => Some(Keep::NotAFolder),
            Target::Unreadable(error) => Some(Keep::Unreadable(error)),
            Target::Folder {
                newest_write_age: Some(age),
            } if age < QUIET_TIME => Some(Keep::RecentWrite(age)),
            Target::Folder { .. } => None,
        },
    };
    keep.map_or(Verdict::Delete, Verdict::Keep)
}

/// Total size and newest modification time of the files under a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Scan {
    /// Sum of the file sizes.
    bytes: u64,
    /// The most recent file modification; `None` when there are no files.
    newest_write: Option<SystemTime>,
}

/// Walks `dir` into `scan`, without following links.
fn scan_into(dir: &Path, scan: &mut Scan) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            scan_into(&entry.path(), scan)?;
        } else {
            let metadata = entry.metadata()?;
            scan.bytes += metadata.len();
            scan.newest_write = scan.newest_write.max(Some(metadata.modified()?));
        }
    }
    Ok(())
}

/// Looks at `target` for [`verdict`], and reports its size through `bytes`.
/// A file dated after `now` counts as written just now.
fn inspect(target: &Path, now: SystemTime, bytes: &mut u64) -> Target {
    let Ok(metadata) = fs::symlink_metadata(target) else {
        return Target::Missing;
    };
    if !metadata.is_dir() {
        return Target::NotAFolder;
    }
    let mut scan = Scan::default();
    if let Err(error) = scan_into(target, &mut scan) {
        return Target::Unreadable(error.to_string());
    }
    *bytes = scan.bytes;
    Target::Folder {
        newest_write_age: scan
            .newest_write
            .map(|written| now.duration_since(written).unwrap_or(Duration::ZERO)),
    }
}

/// `bytes` as GB with one decimal: `12.5 GB`.
fn gb(bytes: u64) -> String {
    let tenths = (bytes * 10 + GB / 2) / GB;
    format!("{}.{} GB", tenths / 10, tenths % 10)
}

/// Whether two paths name the same folder (git prints `D:/x`, Rust has
/// `D:\x`).
fn same_folder(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outcome {
    /// Bytes deleted (or, on a dry run, that would be).
    pub freed: u64,
    /// Whether a delete failed.
    pub failed: bool,
}

/// Decides every worktree and deletes the `target/` folders that may go
/// (none of them when `dry_run`), sending one line per worktree and a total
/// to `print`. `current` is the worktree the command runs in; if git's list
/// doesn't have it, something is off and nothing is deleted.
fn clean(
    worktrees: &[Worktree],
    prs: &[(String, PrState)],
    current: &Path,
    now: SystemTime,
    dry_run: bool,
    mut print: impl FnMut(String),
) -> Result<Outcome, String> {
    if !worktrees.iter().any(|w| same_folder(&w.path, current)) {
        return Err(format!(
            "{} is not in git's worktree list",
            current.display()
        ));
    }
    let mut outcome = Outcome::default();
    for (index, worktree) in worktrees.iter().enumerate() {
        let place = if index == 0 {
            Place::Main
        } else if same_folder(&worktree.path, current) {
            Place::Current
        } else {
            Place::Other
        };
        let states: Vec<PrState> = prs
            .iter()
            .filter(|(branch, _)| Some(branch) == worktree.branch.as_ref())
            .map(|(_, state)| *state)
            .collect();
        let target = worktree.path.join("target");
        let mut bytes = 0;
        let shown = worktree.path.display();
        match verdict(place, &states, || inspect(&target, now, &mut bytes)) {
            Verdict::Keep(why) => print(format!("kept          {shown} ({why})")),
            Verdict::Delete if dry_run => {
                outcome.freed += bytes;
                print(format!("would delete  {shown}/target ({})", gb(bytes)));
            }
            Verdict::Delete => match fs::remove_dir_all(&target) {
                Ok(()) => {
                    outcome.freed += bytes;
                    print(format!("deleted       {shown}/target ({})", gb(bytes)));
                }
                Err(error) => {
                    outcome.failed = true;
                    print(format!("FAILED        {shown}/target: {error}"));
                }
            },
        }
    }
    let freed = if dry_run { "would free" } else { "freed" };
    print(format!(
        "clean-merged-targets: {freed} {}",
        gb(outcome.freed)
    ));
    Ok(outcome)
}

/// The two outside tools, abstracted so tests can run [`run`] without `git`
/// worktrees or a logged-in `gh`.
pub trait Tools {
    /// The output of `git worktree list --porcelain`.
    fn worktree_list(&self) -> Result<String, String>;
    /// The output of `gh pr list --state all --json headRefName,state`.
    fn pr_list(&self) -> Result<String, String>;
}

/// The real `git` and `gh` on `PATH`, run in the repo.
pub struct RealTools<'a> {
    /// The folder they run in.
    pub repo_root: &'a Path,
}

impl RealTools<'_> {
    /// Not covered by mutation testing (`#[mutants::skip]`): it only spawns
    /// `git` or `gh`, and `gh` needs a login no test run has. [`run`]'s
    /// tests cover everything that consumes the output via a fake [`Tools`].
    #[mutants::skip]
    fn output(&self, program: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new(program)
            .args(args)
            .current_dir(self.repo_root)
            .output()
            .map_err(|e| format!("spawn {program}: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "{program} {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        String::from_utf8(output.stdout).map_err(|e| format!("{program} output: {e}"))
    }
}

impl Tools for RealTools<'_> {
    /// Not covered by mutation testing: see [`RealTools::output`].
    #[mutants::skip]
    fn worktree_list(&self) -> Result<String, String> {
        self.output("git", &["worktree", "list", "--porcelain"])
    }

    /// Not covered by mutation testing: see [`RealTools::output`].
    #[mutants::skip]
    fn pr_list(&self) -> Result<String, String> {
        let args = ["pr", "list", "--state", "all", "--limit", "1000"];
        self.output(
            "gh",
            &[&args[..], &["--json", "headRefName,state"]].concat(),
        )
    }
}

/// Runs the command for the worktree at `current`. Both tools are asked
/// before anything is deleted, so a failure of either deletes nothing.
pub fn run(
    tools: &impl Tools,
    current: &Path,
    now: SystemTime,
    dry_run: bool,
    print: impl FnMut(String),
) -> Result<Outcome, String> {
    let worktrees = parse_worktrees(&tools.worktree_list()?);
    let prs = parse_prs(&tools.pr_list()?)?;
    clean(&worktrees, &prs, current, now, dry_run, print)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: Duration = Duration::from_mins(1);

    fn folder(age_minutes: u64) -> Target {
        Target::Folder {
            newest_write_age: Some(MINUTE * u32::try_from(age_minutes).unwrap()),
        }
    }

    /// `verdict` for another worktree whose `target/` was last written two
    /// hours ago.
    fn verdict_for(prs: &[PrState]) -> Verdict {
        verdict(Place::Other, prs, || folder(120))
    }

    #[test]
    fn a_merged_pr_means_delete() {
        assert_eq!(verdict_for(&[PrState::Merged]), Verdict::Delete);
        assert_eq!(
            verdict_for(&[PrState::Closed, PrState::Merged]),
            Verdict::Delete
        );
    }

    #[test]
    fn an_open_pr_means_keep() {
        assert_eq!(verdict_for(&[PrState::Open]), Verdict::Keep(Keep::OpenPr));
    }

    #[test]
    fn no_pr_means_keep() {
        assert_eq!(verdict_for(&[]), Verdict::Keep(Keep::NoPr));
    }

    #[test]
    fn merged_and_open_means_keep() {
        assert_eq!(
            verdict_for(&[PrState::Merged, PrState::Open]),
            Verdict::Keep(Keep::OpenPr)
        );
    }

    #[test]
    fn a_pr_closed_without_merging_means_keep() {
        assert_eq!(
            verdict_for(&[PrState::Closed]),
            Verdict::Keep(Keep::ClosedPr)
        );
    }

    #[test]
    fn merged_but_written_five_minutes_ago_means_keep() {
        assert_eq!(
            verdict(Place::Other, &[PrState::Merged], || folder(5)),
            Verdict::Keep(Keep::RecentWrite(5 * MINUTE))
        );
    }

    #[test]
    fn the_quiet_time_is_thirty_minutes() {
        assert_eq!(
            verdict(Place::Other, &[PrState::Merged], || folder(29)),
            Verdict::Keep(Keep::RecentWrite(29 * MINUTE))
        );
        assert_eq!(
            verdict(Place::Other, &[PrState::Merged], || folder(30)),
            Verdict::Delete
        );
    }

    #[test]
    fn merged_but_the_current_worktree_or_the_main_checkout_means_keep() {
        assert_eq!(
            verdict(Place::Current, &[PrState::Merged], || folder(120)),
            Verdict::Keep(Keep::CurrentWorktree)
        );
        assert_eq!(
            verdict(Place::Main, &[PrState::Merged], || folder(120)),
            Verdict::Keep(Keep::MainCheckout)
        );
    }

    #[test]
    fn merged_but_no_real_target_folder_means_keep() {
        let merged = [PrState::Merged];
        assert_eq!(
            verdict(Place::Other, &merged, || Target::Missing),
            Verdict::Keep(Keep::NoTarget)
        );
        assert_eq!(
            verdict(Place::Other, &merged, || Target::NotAFolder),
            Verdict::Keep(Keep::NotAFolder)
        );
        assert_eq!(
            verdict(Place::Other, &merged, || Target::Unreadable("x".into())),
            Verdict::Keep(Keep::Unreadable("x".into()))
        );
    }

    #[test]
    fn merged_with_an_empty_target_folder_means_delete() {
        let empty = || Target::Folder {
            newest_write_age: None,
        };
        assert_eq!(
            verdict(Place::Other, &[PrState::Merged], empty),
            Verdict::Delete
        );
    }

    #[test]
    fn the_target_is_only_looked_at_when_everything_else_says_delete() {
        let mut looked = 0;
        for (place, prs) in [
            (Place::Main, &[PrState::Merged][..]),
            (Place::Current, &[PrState::Merged]),
            (Place::Other, &[]),
            (Place::Other, &[PrState::Open]),
            (Place::Other, &[PrState::Closed]),
        ] {
            verdict(place, prs, || {
                looked += 1;
                Target::Missing
            });
        }
        assert_eq!(looked, 0);
    }

    #[test]
    fn keep_reasons_read_as_plain_words() {
        let shown: Vec<String> = [
            Keep::MainCheckout,
            Keep::CurrentWorktree,
            Keep::NoPr,
            Keep::OpenPr,
            Keep::ClosedPr,
            Keep::NoTarget,
            Keep::NotAFolder,
            Keep::Unreadable("denied".into()),
            Keep::RecentWrite(Duration::from_secs(5 * 60 + 59)),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            shown,
            [
                "main checkout",
                "current worktree",
                "no PR",
                "open PR",
                "PR closed without merging",
                "no target folder",
                "target is a link, not a folder",
                "could not read target: denied",
                "target written 5 min ago",
            ]
        );
    }

    const PORCELAIN: &str = "\
worktree D:/tactical-rpg
HEAD fd6920d16eea24a7c5bade40a955bc9931ed026e
branch refs/heads/main

worktree D:/tactical-rpg/.claude/worktrees/battle-notes-bea940
HEAD e201bcddb60729ac29e1dce630985f763f33d725
branch refs/heads/t0411-battle-notes

worktree D:/tactical-rpg/.claude/worktrees/fix main-5e8ee0
HEAD bc43ff2883259fdb9774da57eb59f2af62701347
branch refs/heads/claude/fix-main-5e8ee0

worktree D:/tactical-rpg/.claude/worktrees/detached
HEAD 3a0c7d2bdb38b6223942d357b26d0c2012cdc222
detached
";

    #[test]
    fn parse_worktrees_reads_paths_and_branches() {
        let worktree = |path: &str, branch: Option<&str>| Worktree {
            path: PathBuf::from(path),
            branch: branch.map(str::to_string),
        };
        assert_eq!(
            parse_worktrees(PORCELAIN),
            [
                worktree("D:/tactical-rpg", Some("main")),
                worktree(
                    "D:/tactical-rpg/.claude/worktrees/battle-notes-bea940",
                    Some("t0411-battle-notes")
                ),
                worktree(
                    "D:/tactical-rpg/.claude/worktrees/fix main-5e8ee0",
                    Some("claude/fix-main-5e8ee0")
                ),
                worktree("D:/tactical-rpg/.claude/worktrees/detached", None),
            ]
        );
        assert_eq!(parse_worktrees(""), []);
        // A branch line with no worktree before it is ignored.
        assert_eq!(parse_worktrees("branch refs/heads/main\n"), []);
    }

    #[test]
    fn parse_prs_reads_branches_and_states() {
        let json = r#"[{"headRefName":"claude/worktree-size-d93e6e","state":"MERGED"},{"headRefName":"t0410-spell-menu","state":"OPEN"},{"headRefName":"t0001-x","state":"CLOSED"}]"#;
        assert_eq!(
            parse_prs(json),
            Ok(vec![
                ("claude/worktree-size-d93e6e".to_string(), PrState::Merged),
                ("t0410-spell-menu".to_string(), PrState::Open),
                ("t0001-x".to_string(), PrState::Closed),
            ])
        );
        assert_eq!(parse_prs("[]"), Ok(vec![]));
    }

    #[test]
    fn parse_prs_rejects_anything_unexpected() {
        assert_eq!(
            parse_prs(r#"{"message":"Bad credentials"}"#),
            Err("gh pr list: expected a JSON list".to_string())
        );
        assert_eq!(
            parse_prs("[{"),
            Err("gh pr list: expected a JSON list".to_string())
        );
        assert_eq!(
            parse_prs(r#"[{"state":"MERGED"}]"#),
            Err("gh pr list: a PR without headRefName".to_string())
        );
        assert_eq!(
            parse_prs(r#"[{"headRefName":"x","state":"DRAFT"}]"#),
            Err("gh pr list: unknown PR state Some(\"DRAFT\")".to_string())
        );
        assert_eq!(
            parse_prs(r#"[{"headRefName":"x"}]"#),
            Err("gh pr list: unknown PR state None".to_string())
        );
    }

    #[test]
    fn parse_args_reads_dry_run() {
        let args = |items: &[&str]| items.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert_eq!(parse_args(&[]), Ok(false));
        assert_eq!(parse_args(&args(&["--dry-run"])), Ok(true));
        assert_eq!(parse_args(&args(&["--bogus"])), Err(USAGE.to_string()));
        assert_eq!(
            parse_args(&args(&["--dry-run", "--dry-run"])),
            Err(USAGE.to_string())
        );
    }

    #[test]
    fn gb_rounds_to_one_decimal() {
        assert_eq!(gb(0), "0.0 GB");
        assert_eq!(gb(GB), "1.0 GB");
        assert_eq!(gb(GB * 25 / 2), "12.5 GB");
        assert_eq!(gb(GB / 20), "0.0 GB");
        assert_eq!(gb(GB / 20 + 1), "0.1 GB");
        assert_eq!(gb(GB * 258), "258.0 GB");
    }

    /// A scratch "repo" under `env::temp_dir()`: a main checkout and the
    /// worktrees `current`, `merged`, `open`, `nopr` and `detached`, each
    /// with a `target/` holding 3 bytes in two files.
    struct Scratch {
        root: PathBuf,
        worktrees: Vec<Worktree>,
    }

    const PRS: &str = r#"[{"headRefName":"main","state":"MERGED"},
        {"headRefName":"t-current","state":"MERGED"},
        {"headRefName":"t-merged","state":"MERGED"},
        {"headRefName":"t-open","state":"OPEN"}]"#;

    impl Scratch {
        fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("xtask-clean-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let worktrees = ["main", "current", "merged", "open", "nopr", "detached"]
                .iter()
                .map(|name| {
                    let path = root.join(name);
                    fs::create_dir_all(path.join("target/debug/deps")).unwrap();
                    fs::write(path.join("target/debug/deps/a.exe"), "ab").unwrap();
                    fs::write(path.join("target/CACHEDIR.TAG"), "c").unwrap();
                    fs::write(path.join("Cargo.toml"), "").unwrap();
                    Worktree {
                        path,
                        branch: match *name {
                            "detached" => None,
                            "main" => Some("main".to_string()),
                            other => Some(format!("t-{other}")),
                        },
                    }
                })
                .collect();
            Self { root, worktrees }
        }

        fn target(&self, name: &str) -> PathBuf {
            self.root.join(name).join("target")
        }

        fn porcelain(&self) -> String {
            let blocks: Vec<String> = self
                .worktrees
                .iter()
                .map(|w| {
                    let branch = w
                        .branch
                        .as_ref()
                        .map_or("detached".to_string(), |b| format!("branch refs/heads/{b}"));
                    format!("worktree {}\nHEAD 0000\n{branch}\n", w.path.display())
                })
                .collect();
            blocks.join("\n")
        }

        /// Runs the command from the `current` worktree, an hour from now,
        /// returning the outcome and the printed lines with the scratch
        /// root cut out.
        fn run(&self, tools: &impl Tools, dry_run: bool) -> (Result<Outcome, String>, Vec<String>) {
            let mut lines = Vec::new();
            let root = format!("{}", self.root.display());
            let outcome = run(
                tools,
                &self.root.join("current"),
                SystemTime::now() + 60 * MINUTE,
                dry_run,
                |line| lines.push(line.replace(&root, "").replace('\\', "/")),
            );
            (outcome, lines)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    struct FakeTools {
        worktrees: Result<String, String>,
        prs: Result<String, String>,
    }

    impl FakeTools {
        fn of(scratch: &Scratch) -> Self {
            Self {
                worktrees: Ok(scratch.porcelain()),
                prs: Ok(PRS.to_string()),
            }
        }
    }

    impl Tools for FakeTools {
        fn worktree_list(&self) -> Result<String, String> {
            self.worktrees.clone()
        }
        fn pr_list(&self) -> Result<String, String> {
            self.prs.clone()
        }
    }

    const ALL: [&str; 6] = ["main", "current", "merged", "open", "nopr", "detached"];

    #[test]
    fn run_deletes_only_the_merged_worktrees_target() {
        let scratch = Scratch::new("run");
        let (outcome, lines) = scratch.run(&FakeTools::of(&scratch), false);
        assert_eq!(
            outcome,
            Ok(Outcome {
                freed: 3,
                failed: false
            })
        );
        assert_eq!(
            lines,
            [
                "kept          /main (main checkout)",
                "kept          /current (current worktree)",
                "deleted       /merged/target (0.0 GB)",
                "kept          /open (open PR)",
                "kept          /nopr (no PR)",
                "kept          /detached (no PR)",
                "clean-merged-targets: freed 0.0 GB",
            ]
        );
        for name in ALL {
            assert_eq!(scratch.target(name).exists(), name != "merged", "{name}");
            // Only `target/` goes: the worktree and its files stay.
            assert!(scratch.root.join(name).join("Cargo.toml").is_file());
        }
    }

    #[test]
    fn a_dry_run_prints_the_same_verdicts_and_deletes_nothing() {
        let scratch = Scratch::new("dry");
        let (outcome, lines) = scratch.run(&FakeTools::of(&scratch), true);
        assert_eq!(
            outcome,
            Ok(Outcome {
                freed: 3,
                failed: false
            })
        );
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[2], "would delete  /merged/target (0.0 GB)");
        assert_eq!(lines[6], "clean-merged-targets: would free 0.0 GB");
        for name in ALL {
            assert!(scratch.target(name).join("debug/deps/a.exe").is_file());
        }
    }

    #[test]
    fn a_failing_tool_deletes_nothing() {
        let scratch = Scratch::new("fail");
        let no_git = FakeTools {
            worktrees: Err("spawn git: not found".to_string()),
            ..FakeTools::of(&scratch)
        };
        let no_gh = FakeTools {
            prs: Err("gh pr list: not logged in".to_string()),
            ..FakeTools::of(&scratch)
        };
        let bad_json = FakeTools {
            prs: Ok("{}".to_string()),
            ..FakeTools::of(&scratch)
        };
        for (tools, error) in [
            (no_git, "spawn git: not found"),
            (no_gh, "gh pr list: not logged in"),
            (bad_json, "gh pr list: expected a JSON list"),
        ] {
            let (outcome, lines) = scratch.run(&tools, false);
            assert_eq!(outcome, Err(error.to_string()));
            assert_eq!(lines, [] as [&str; 0]);
        }
        for name in ALL {
            assert!(scratch.target(name).is_dir(), "{name}");
        }
    }

    #[test]
    fn a_current_worktree_git_does_not_list_deletes_nothing() {
        let scratch = Scratch::new("unlisted");
        let elsewhere = scratch.root.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let mut lines = Vec::new();
        let outcome = clean(
            &scratch.worktrees,
            &parse_prs(PRS).unwrap(),
            &elsewhere,
            SystemTime::now() + 60 * MINUTE,
            false,
            |line| lines.push(line),
        );
        assert_eq!(
            outcome,
            Err(format!(
                "{} is not in git's worktree list",
                elsewhere.display()
            ))
        );
        assert!(lines.is_empty());
        assert!(scratch.target("merged").is_dir());
    }

    #[test]
    fn a_target_written_just_now_is_kept() {
        let scratch = Scratch::new("recent");
        let mut lines = Vec::new();
        let outcome = clean(
            &scratch.worktrees,
            &parse_prs(PRS).unwrap(),
            &scratch.root.join("current"),
            SystemTime::now() + 10 * MINUTE,
            false,
            |line| lines.push(line),
        );
        assert_eq!(outcome, Ok(Outcome::default()));
        assert!(
            lines[2].ends_with("(target written 10 min ago)") && lines[2].starts_with("kept "),
            "{lines:?}"
        );
        assert!(scratch.target("merged").is_dir());
    }

    #[test]
    fn a_failed_delete_is_reported_and_frees_nothing() {
        let scratch = Scratch::new("locked");
        // An open handle without delete sharing makes Windows refuse the
        // delete; elsewhere the delete just works.
        let held = fs::File::open(scratch.target("merged").join("CACHEDIR.TAG")).unwrap();
        #[cfg(windows)]
        let held = {
            use std::os::windows::fs::OpenOptionsExt;
            drop(held);
            fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(scratch.target("merged").join("CACHEDIR.TAG"))
                .unwrap()
        };
        let (outcome, lines) = scratch.run(&FakeTools::of(&scratch), false);
        drop(held);
        if cfg!(windows) {
            assert_eq!(
                outcome,
                Ok(Outcome {
                    freed: 0,
                    failed: true
                })
            );
            assert!(lines[2].starts_with("FAILED        /merged/target: "));
            assert_eq!(lines[6], "clean-merged-targets: freed 0.0 GB");
        } else {
            assert_eq!(outcome.map(|o| o.failed), Ok(false));
        }
    }

    #[test]
    fn inspect_reports_what_is_at_target() {
        let scratch = Scratch::new("inspect");
        let now = SystemTime::now();
        let mut bytes = 0;
        assert_eq!(
            inspect(&scratch.root.join("nothing"), now, &mut bytes),
            Target::Missing
        );
        assert_eq!(
            inspect(&scratch.root.join("main/Cargo.toml"), now, &mut bytes),
            Target::NotAFolder
        );
        assert_eq!(bytes, 0);

        let empty = scratch.root.join("empty");
        fs::create_dir_all(empty.join("debug")).unwrap();
        assert_eq!(
            inspect(&empty, now, &mut bytes),
            Target::Folder {
                newest_write_age: None
            }
        );

        // Files written "after now" count as written just now.
        let before = now - 60 * MINUTE;
        assert_eq!(
            inspect(&scratch.target("merged"), before, &mut bytes),
            Target::Folder {
                newest_write_age: Some(Duration::ZERO)
            }
        );
        assert_eq!(bytes, 3);

        let later = now + 60 * MINUTE;
        let Target::Folder {
            newest_write_age: Some(age),
        } = inspect(&scratch.target("merged"), later, &mut bytes)
        else {
            panic!("expected a folder with files");
        };
        assert!(age >= 59 * MINUTE && age <= 61 * MINUTE, "{age:?}");
    }

    #[test]
    fn scan_takes_the_newest_file_in_any_subfolder() {
        let scratch = Scratch::new("scan");
        let target = scratch.target("merged");
        let old = SystemTime::now() - 600 * MINUTE;
        let newest = SystemTime::now() - 60 * MINUTE;
        let set = |file: &str, time: SystemTime| {
            let file = fs::File::options()
                .write(true)
                .open(target.join(file))
                .unwrap();
            file.set_modified(time).unwrap();
        };
        set("CACHEDIR.TAG", old);
        set("debug/deps/a.exe", newest);
        let mut scan = Scan::default();
        scan_into(&target, &mut scan).unwrap();
        assert_eq!(scan.bytes, 3);
        let written = scan.newest_write.unwrap();
        let off = written
            .duration_since(newest)
            .unwrap_or_else(|e| e.duration());
        assert!(off < Duration::from_secs(2), "{off:?}");

        assert!(scan_into(&scratch.root.join("nothing"), &mut Scan::default()).is_err());
    }

    #[test]
    fn same_folder_ignores_how_the_path_is_written() {
        let scratch = Scratch::new("same");
        let main = scratch.root.join("main");
        let forward = PathBuf::from(format!("{}", main.display()).replace('\\', "/"));
        assert!(same_folder(&main, &forward));
        assert!(same_folder(&main, &main.join("target/..")));
        assert!(!same_folder(&main, &scratch.root.join("merged")));
        // Folders that don't exist are compared as written.
        assert!(same_folder(Path::new("no/such"), Path::new("no/such")));
        assert!(!same_folder(Path::new("no/such"), Path::new("no/other")));
        assert!(!same_folder(&main, Path::new("no/such")));
    }
}
