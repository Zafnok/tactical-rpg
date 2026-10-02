//! `cargo xtask playtest` (ticket 0505, ADR-0033): a bot plays a battle
//! file many times, each try with fresh luck, and the report says how the
//! tries went (`docs/playtesting.md`, `docs/design/playtest-bots.md`).
//!
//! `trpg-bots` is pure; the files, the clock and the threads are here. A
//! report depends only on the arguments: the tries are sorted by seed
//! before anything is counted, so the thread count changes nothing but the
//! speed line.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use trpg_bots::{BaselineBot, BattleMeasures, PlayerBot, play_battle};
use trpg_content::{Content, battle_campaign};
use trpg_core::{
    BattleSetup, BattleState, GameMode, LeadGender, LeadProfile, Outcome, Turn, UnitId,
};

/// The command's usage text.
pub const USAGE: &str = "usage: cargo xtask playtest <battle-id> [options]\n\n\
A bot plays the battle assets/battles/<battle-id>.ron many times, each try\n\
with fresh luck, and reports how the tries went (docs/playtesting.md).\n\n\
options:\n  \
--runs <n>         tries to play (default 100)\n  \
--seed <n>         seed of the first try; try i plays seed + i (default 1)\n  \
--mode <mode>      classic or casual (default classic)\n  \
--turn-cap <n>     stop a try after this many turns (default 60)\n  \
--bot <bot>        who plays the player's side: baseline (default)\n  \
--json <path>      also write the report as JSON\n  \
--history <dir>    where every run is kept, to compare the next one with\n                     \
(default target/playtest-history/)\n  \
--threads <n>      threads to play on (default: every core)";

/// Where runs are kept, from the repo root. Not committed.
const DEFAULT_HISTORY: &str = "target/playtest-history";

/// The lead's name in a playtest.
const LEAD_NAME: &str = "Lead";

/// Who plays the player's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bot {
    /// The enemy AI on the player's side ([`BaselineBot`]).
    Baseline,
}

impl Bot {
    fn parse(name: &str) -> Option<Bot> {
        match name {
            "baseline" => Some(Bot::Baseline),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Bot::Baseline => "baseline",
        }
    }

    /// The bot for one try, seeded with the try's seed.
    fn build(self, content: &Content, _seed: u64) -> Box<dyn PlayerBot> {
        match self {
            Bot::Baseline => Box::new(BaselineBot::new(content.ai)),
        }
    }
}

/// The command's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The battle's id (its file stem).
    pub battle: String,
    /// Tries to play.
    pub runs: u32,
    /// Seed of the first try.
    pub seed: u64,
    /// Classic or Casual.
    pub mode: GameMode,
    /// A try stops when this turn has passed.
    pub turn_cap: Turn,
    /// Who plays.
    pub bot: Bot,
    /// Where to write the report as JSON.
    pub json: Option<PathBuf>,
    /// Where runs are kept; `None` is [`DEFAULT_HISTORY`] in the repo.
    pub history: Option<PathBuf>,
    /// Threads to play on; `None` is every core.
    pub threads: Option<usize>,
}

impl Options {
    /// The defaults, for `battle`.
    pub fn new(battle: impl Into<String>) -> Self {
        Self {
            battle: battle.into(),
            runs: 100,
            seed: 1,
            mode: GameMode::Classic,
            turn_cap: 60,
            bot: Bot::Baseline,
            json: None,
            history: None,
            threads: None,
        }
    }
}

/// Parses the command's arguments.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options::new("");
    let mut battle: Option<&String> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if !arg.starts_with("--") {
            if battle.is_some() {
                return Err(format!("unexpected argument: {arg}"));
            }
            battle = Some(arg);
            continue;
        }
        let value = iter
            .next()
            .ok_or_else(|| format!("{arg} requires a value"))?;
        let bad = || format!("{arg}: bad value \"{value}\"");
        match arg.as_str() {
            "--runs" => options.runs = positive(value).ok_or_else(bad)?,
            "--seed" => options.seed = value.parse().map_err(|_| bad())?,
            "--mode" => options.mode = parse_mode(value).ok_or_else(bad)?,
            "--turn-cap" => options.turn_cap = positive(value).ok_or_else(bad)?,
            "--bot" => options.bot = Bot::parse(value).ok_or_else(bad)?,
            "--json" => options.json = Some(PathBuf::from(value)),
            "--history" => options.history = Some(PathBuf::from(value)),
            "--threads" => {
                let threads = positive(value).and_then(|n| usize::try_from(n).ok());
                options.threads = Some(threads.ok_or_else(bad)?);
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }
    battle
        .ok_or("no battle given")?
        .clone_into(&mut options.battle);
    Ok(options)
}

/// A number of at least 1.
fn positive(value: &str) -> Option<u32> {
    value.parse().ok().filter(|&n| n > 0)
}

fn parse_mode(name: &str) -> Option<GameMode> {
    match name {
        "classic" => Some(GameMode::Classic),
        "casual" => Some(GameMode::Casual),
        _ => None,
    }
}

fn mode_name(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Classic => "Classic",
        GameMode::Casual => "Casual",
    }
}

/// How a try ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TryResult {
    /// The battle was won.
    Won,
    /// The battle was lost.
    Lost,
    /// The battle was still on when the turn cap passed.
    TurnCap,
}

/// A player unit that fell in a try.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fall {
    /// The unit's name.
    pub unit: String,
    /// The turn it fell on.
    pub turn: Turn,
    /// Whether it is the lord (whose fall loses the battle).
    pub lord: bool,
}

/// One try of the battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Try {
    /// The battle seed it played; `--seed <this> --runs 1` plays it again.
    pub seed: u64,
    /// How it ended.
    pub result: TryResult,
    /// Turns it took (the turn cap if that stopped it).
    pub turns: Turn,
    /// The player units that fell, in order.
    pub fallen: Vec<Fall>,
    /// Consumables the player units used, by item id.
    pub items: BTreeMap<String, u32>,
    /// Commands applied, every side's.
    pub commands: u32,
}

impl Try {
    fn items_total(&self) -> u32 {
        self.items.values().sum()
    }

    fn fallen_count(&self) -> u32 {
        u32::try_from(self.fallen.len()).unwrap_or(u32::MAX)
    }
}

/// The numbers over every try of a run.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Stats {
    /// Tries won.
    pub won: u32,
    /// Tries lost.
    pub lost: u32,
    /// Tries stopped by the turn cap.
    pub turn_cap: u32,
    /// Tries won, in percent of all tries (rounded).
    pub win_percent: u32,
    /// Median turns (the lower of the two middle tries of an even count).
    pub turns_median: Turn,
    /// Fewest turns.
    pub turns_min: Turn,
    /// Most turns.
    pub turns_max: Turn,
    /// Player units fallen, over all tries.
    pub fallen_total: u32,
    /// Most player units fallen in one try.
    pub fallen_max: u32,
    /// Tries in which 0, 1, 2, and 3 or more player units fell.
    pub fallen_per_try: [u32; 4],
    /// How many tries each unit fell in, most often first (then by name).
    pub fell_most: Vec<(String, u32)>,
    /// Consumables used, over all tries.
    pub items_total: u32,
    /// Consumables used over all tries, by item id.
    pub items: BTreeMap<String, u32>,
}

impl Stats {
    /// The numbers of `tries`.
    pub fn of(tries: &[Try]) -> Stats {
        let count = |result| count(tries.iter().filter(|t| t.result == result));
        let runs = count(TryResult::Won) + count(TryResult::Lost) + count(TryResult::TurnCap);
        let won = count(TryResult::Won);
        let mut turns: Vec<Turn> = tries.iter().map(|t| t.turns).collect();
        turns.sort_unstable();
        let mut stats = Stats {
            won,
            lost: count(TryResult::Lost),
            turn_cap: count(TryResult::TurnCap),
            win_percent: (won * 100 + runs / 2).checked_div(runs).unwrap_or(0),
            turns_median: turns
                .get(turns.len().saturating_sub(1) / 2)
                .map_or(0, |&t| t),
            turns_min: turns.first().map_or(0, |&t| t),
            turns_max: turns.last().map_or(0, |&t| t),
            ..Stats::default()
        };
        let mut fell: BTreeMap<&str, u32> = BTreeMap::new();
        for t in tries {
            let fallen = t.fallen_count();
            stats.fallen_total += fallen;
            stats.fallen_max = stats.fallen_max.max(fallen);
            stats.fallen_per_try[t.fallen.len().min(3)] += 1;
            for fall in &t.fallen {
                *fell.entry(&fall.unit).or_default() += 1;
            }
            stats.items_total += t.items_total();
            for (item, n) in &t.items {
                *stats.items.entry(item.clone()).or_default() += n;
            }
        }
        stats.fell_most = most_first(fell.into_iter().map(|(name, n)| (name.to_owned(), n)));
        stats
    }
}

fn count<T>(items: impl Iterator<Item = T>) -> u32 {
    u32::try_from(items.count()).unwrap_or(u32::MAX)
}

/// `counts`, highest first, then by name.
fn most_first(counts: impl Iterator<Item = (String, u32)>) -> Vec<(String, u32)> {
    let mut counts: Vec<(String, u32)> = counts.collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}

/// A run's report: the same for the same arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// The battle's id.
    pub battle: String,
    /// `Classic` or `Casual`.
    pub mode: String,
    /// The bot's name.
    pub bot: String,
    /// Seed of the first try.
    pub seed: u64,
    /// Tries played.
    pub runs: u32,
    /// The turn cap.
    pub turn_cap: Turn,
    /// The numbers over every try.
    pub stats: Stats,
    /// Every try, by seed.
    pub tries: Vec<Try>,
}

/// A run as kept in the history and written by `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// When it ran, e.g. `2026-10-01 14:05 UTC`.
    pub date: String,
    /// The git commit it ran on (short hash), or `unknown`.
    pub commit: String,
    /// Commands applied per second of play.
    pub commands_per_second: u64,
    /// The report.
    #[serde(flatten)]
    pub report: Report,
}

/// Runs the command: plays the tries, keeps the run in the history, writes
/// the JSON file if asked, and returns the report's text.
pub fn run(repo_root: &Path, options: &Options) -> Result<String, String> {
    let content = trpg_content::load_embedded().map_err(|e| format!("content: {e}"))?;
    let (report, commands_per_second) = play(&content, options)?;
    let record = Record {
        date: utc_stamp(now()),
        commit: git_commit(repo_root),
        commands_per_second,
        report,
    };
    let default_history = repo_root.join(DEFAULT_HISTORY);
    let history = options.history.as_deref().unwrap_or(&default_history);
    let file = history_file(history, &record.report);
    let previous = last_record(&file);
    append(&file, &record).map_err(|e| format!("{}: {e}", file.display()))?;
    if let Some(path) = &options.json {
        let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
        std::fs::write(path, json + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(render(&record, previous.as_ref()))
}

/// A try and how long it took to play, or why it stopped.
type Played = Result<(Try, Duration), String>;

/// Plays every try of `options`' battle. Returns the report and the speed
/// (commands per second of play).
pub fn play(content: &Content, options: &Options) -> Result<(Report, u64), String> {
    let Some(def) = content.battles.get(&options.battle) else {
        let known: Vec<&str> = content.battles.keys().map(String::as_str).collect();
        return Err(format!(
            "no battle \"{}\" in assets/battles (there: {})",
            options.battle,
            known.join(", ")
        ));
    };
    // The army: the characters of the battle's player slots, fresh from the
    // character data (as the debug Quick Battle fields them).
    let lead = LeadProfile::new(LEAD_NAME, LeadGender::Male);
    let campaign = battle_campaign(content, def, options.mode, lead);
    let setup = campaign.battle_setup(def, &content.tables());

    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let threads = options.threads.unwrap_or(cores).max(1);
    let (setup, runs) = (&setup, options.runs);
    let mut played: Vec<(u32, Played)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    (0..runs)
                        .filter(|&i| usize::try_from(i).is_ok_and(|i| i % threads == worker))
                        .map(|i| (i, play_try(content, setup, options, i)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let joined = workers.into_iter().flat_map(|worker| match worker.join() {
            Ok(tries) => tries,
            Err(panic) => std::panic::resume_unwind(panic),
        });
        joined.collect()
    });
    played.sort_by_key(|&(i, _)| i);

    let mut tries = Vec::new();
    let mut elapsed = Duration::ZERO;
    for (_, result) in played {
        let (one, took) = result?;
        tries.push(one);
        elapsed += took;
    }
    let commands: u64 = tries.iter().map(|t| u64::from(t.commands)).sum();
    let report = Report {
        battle: options.battle.clone(),
        mode: mode_name(options.mode).to_owned(),
        bot: options.bot.name().to_owned(),
        seed: options.seed,
        runs,
        turn_cap: options.turn_cap,
        stats: Stats::of(&tries),
        tries,
    };
    Ok((report, per_second(commands, elapsed)))
}

/// Plays try `index` of `setup`: the battle with seed `options.seed + index`.
fn play_try(content: &Content, setup: &BattleSetup, options: &Options, index: u32) -> Played {
    let seed = options.seed.wrapping_add(u64::from(index));
    let mut setup = setup.clone();
    setup.seed = seed;
    let (state, _) = BattleState::new(setup);
    let cast = cast(&state);
    let mut bot = options.bot.build(content, seed);
    let started = Instant::now();
    let measures = play_battle(state, bot.as_mut(), &content.ai, options.turn_cap)
        .map_err(|e| format!("try {seed}: {e}"))?;
    let took = started.elapsed();
    Ok((to_try(seed, &measures, &cast, options.turn_cap), took))
}

/// Name and whether it is the lord, for every unit of the battle
/// (reinforcements included).
fn cast(state: &BattleState) -> BTreeMap<UnitId, (String, bool)> {
    let waiting = state.reinforcements().iter().map(|r| &r.unit);
    state
        .units()
        .iter()
        .chain(waiting)
        .map(|u| (u.id, (u.name.clone(), u.is_lord)))
        .collect()
}

/// The try with seed `seed` whose measures are `measures`.
fn to_try(
    seed: u64,
    measures: &BattleMeasures,
    cast: &BTreeMap<UnitId, (String, bool)>,
    turn_cap: Turn,
) -> Try {
    let fallen = measures.player_fallen.iter().map(|&(id, turn)| {
        let (unit, lord) = cast
            .get(&id)
            .cloned()
            .unwrap_or_else(|| (format!("unit {}", id.0), false));
        Fall { unit, turn, lord }
    });
    let items = measures.items_used.iter();
    Try {
        seed,
        result: match measures.outcome {
            Some(Outcome::Victory) => TryResult::Won,
            Some(Outcome::Defeat) => TryResult::Lost,
            None => TryResult::TurnCap,
        },
        turns: measures.turns.min(turn_cap),
        fallen: fallen.collect(),
        items: items.map(|(item, &n)| (item.0.clone(), n)).collect(),
        commands: measures.commands,
    }
}

/// `commands` per second over `elapsed`; 0 if no time passed.
fn per_second(commands: u64, elapsed: Duration) -> u64 {
    let per_second = (u128::from(commands) * 1_000_000_000).checked_div(elapsed.as_nanos());
    u64::try_from(per_second.unwrap_or(0)).unwrap_or(u64::MAX)
}

/// The report's text: the numbers, with `previous`' headline numbers next
/// to them if there was a run before, then one line per try.
pub fn render(record: &Record, previous: Option<&Record>) -> String {
    let report = &record.report;
    let stats = &report.stats;
    let before = previous.map(|p| (&p.report.stats, p.report.runs));
    let was = |number: &dyn Fn(&Stats, u32) -> String| {
        before.map_or_else(String::new, |(stats, runs)| {
            format!(" (was {})", number(stats, runs))
        })
    };
    let mut out = String::new();
    // Writing to a `String` can't fail.
    let _ = writeln!(
        out,
        "{} · {} · {} · {}",
        report.battle,
        report.mode,
        report.bot,
        tries_and_seeds(report)
    );
    let _ = writeln!(
        out,
        "won {}  lost {}  turn cap {}   ({}%{})",
        stats.won,
        stats.lost,
        stats.turn_cap,
        stats.win_percent,
        before.map_or_else(String::new, |(s, _)| format!(", was {}%", s.win_percent)),
    );
    let _ = writeln!(
        out,
        "turns     median {}{}  min {}  max {}",
        stats.turns_median,
        was(&|s, _| s.turns_median.to_string()),
        stats.turns_min,
        stats.turns_max
    );
    let _ = writeln!(
        out,
        "fallen    mean {}{}   max {}",
        mean(stats.fallen_total, report.runs),
        was(&|s, runs| mean(s.fallen_total, runs)),
        stats.fallen_max
    );
    let per_item: Vec<String> = most_first(stats.items.clone().into_iter())
        .iter()
        .map(|(item, n)| format!("{item} {}", mean(*n, report.runs)))
        .collect();
    let _ = write!(
        out,
        "items     mean {}{}",
        mean(stats.items_total, report.runs),
        was(&|s, runs| mean(s.items_total, runs))
    );
    if !per_item.is_empty() {
        let _ = write!(out, "   ({})", per_item.join(", "));
    }
    out.push('\n');
    let [none, one, two, more] = stats.fallen_per_try;
    let most: Vec<String> = stats.fell_most.iter().take(3).map(times).collect();
    let _ = writeln!(
        out,
        "fallen/try 0: {none}  1: {one}  2: {two}  3+: {more}   most: {}",
        if most.is_empty() {
            "nobody".to_owned()
        } else {
            most.join(", ")
        }
    );
    let _ = writeln!(
        out,
        "speed     {} commands/s",
        grouped(record.commands_per_second)
    );
    if let Some(p) = previous {
        let _ = writeln!(
            out,
            "previous  {} · commit {} · {}",
            p.date,
            p.commit,
            tries_and_seeds(&p.report)
        );
    }
    out.push('\n');
    for one in &report.tries {
        out.push_str(&try_line(one));
        out.push('\n');
    }
    out
}

/// `100 tries (seeds 1–100)`.
fn tries_and_seeds(report: &Report) -> String {
    let last = u64::from(report.runs.saturating_sub(1));
    format!(
        "{} tries (seeds {}–{})",
        report.runs,
        report.seed,
        report.seed.wrapping_add(last)
    )
}

/// `Mira 22×`.
fn times((name, n): &(String, u32)) -> String {
    format!("{name} {n}×")
}

/// `total / runs` to one decimal.
fn mean(total: u32, runs: u32) -> String {
    format!("{:.1}", f64::from(total) / f64::from(runs.max(1)))
}

/// `n` with a space between each three digits: `41 200`.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(digit);
    }
    out
}

/// `try 17 · lost (lord fell T9) · fell: Mira T6, Kael T8 · items 2 · turns 9`.
fn try_line(one: &Try) -> String {
    let result = match one.result {
        TryResult::Won => "won".to_owned(),
        TryResult::TurnCap => "turn cap".to_owned(),
        TryResult::Lost => match one.fallen.iter().find(|f| f.lord) {
            Some(lord) => format!("lost (lord fell T{})", lord.turn),
            None => "lost".to_owned(),
        },
    };
    let fell: Vec<String> = one
        .fallen
        .iter()
        .map(|f| format!("{} T{}", f.unit, f.turn))
        .collect();
    format!(
        "try {} · {result} · fell: {} · items {} · turns {}",
        one.seed,
        if fell.is_empty() {
            "nobody".to_owned()
        } else {
            fell.join(", ")
        },
        one.items_total(),
        one.turns
    )
}

/// The history file of `report`'s battle, mode and bot in `dir`: one run
/// per line, oldest first.
fn history_file(dir: &Path, report: &Report) -> PathBuf {
    let name = format!(
        "{}-{}-{}.jsonl",
        report.battle,
        report.mode.to_lowercase(),
        report.bot
    );
    dir.join(name)
}

/// The last run kept in `file`, if any.
fn last_record(file: &Path) -> Option<Record> {
    let text = std::fs::read_to_string(file).ok()?;
    text.lines()
        .rev()
        .find_map(|line| serde_json::from_str(line).ok())
}

/// Adds `record` to `file`, creating it and its folder if needed.
fn append(file: &Path, record: &Record) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let line = serde_json::to_string(record)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?;
    writeln!(file, "{line}")
}

/// The short hash of the commit checked out in `repo_root`, or `unknown`.
fn git_commit(repo_root: &Path) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    let hash = output
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    hash.filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Seconds since 1970-01-01 00:00 UTC.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `secs` since 1970-01-01 00:00 UTC as `2026-10-01 14:05 UTC`.
fn utc_stamp(secs: u64) -> String {
    let leap =
        |year: u64| year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400);
    let mut days = secs / 86_400;
    let mut year = 1970;
    // Bounded, so a wrong count of days can't spin forever.
    for candidate in 1970..=9999 {
        year = candidate;
        let length = if leap(year) { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
    }
    let february = if leap(year) { 29 } else { 28 };
    let mut month = 1;
    for length in [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    let minutes = secs % 86_400 / 60;
    format!(
        "{year}-{month:02}-{:02} {:02}:{:02} UTC",
        days + 1,
        minutes / 60,
        minutes % 60
    )
}

#[cfg(test)]
mod tests;
