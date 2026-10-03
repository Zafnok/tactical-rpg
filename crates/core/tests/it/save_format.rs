//! The save format guard (ticket 0821, ADR-0039): golden saves, kept as
//! text in `tests/fixtures/save_v<version>_<kind>.ron`, that this build must
//! read and write back to the byte.
//!
//! # When a test here fails
//!
//! A saved type changed: raise `SAVE_VERSION` in `crates/core/src/save.rs`,
//! regenerate the fixtures as `save_v<N>_*.ron` and delete the old ones
//! (ADR-0039). Never edit a fixture or the facts below to make the test
//! pass without raising the version: players' saves would then fail to
//! load, or load as something else.
//!
//! To regenerate (it writes the files for the current `SAVE_VERSION`):
//!
//! ```text
//! cargo test -p trpg-core --test it save_format::regenerate -- --ignored
//! ```
//!
//! Then `git rm` the fixtures of the version before, and update the facts
//! in `the_suspend_save_replays_to_the_same_battle` if the rules changed.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use trpg_core::{
    BattleHistory, BattleState, Campaign, Faction, GameMode, ItemId, LeadGender, LeadProfile,
    Phase, Pos, SAVE_VERSION, SaveFile, SaveHeader, SavePoint, Stock, UnitId,
};

use super::replay;

/// What to do, in every failure of this module.
const HELP: &str = "a saved type changed: raise `SAVE_VERSION` in `crates/core/src/save.rs`, \
     regenerate the fixtures as `save_v<N>_*.ron` and delete the old ones (ADR-0039). \
     See crates/core/tests/it/save_format.rs for how";

/// The kinds of golden save: one per way a save is made.
const KINDS: [&str; 2] = ["chapter", "suspend"];

/// How many commands of the replay script the suspended battle is into.
const PLAYED: usize = 23;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(version: u32, kind: &str) -> PathBuf {
    fixtures().join(format!("save_v{version}_{kind}.ron"))
}

/// Every golden save in the folder, as (version, path).
fn golden_saves() -> Vec<(u32, PathBuf)> {
    let mut found = Vec::new();
    let dir = std::fs::read_dir(fixtures()).unwrap_or_else(|e| panic!("tests/fixtures: {e}"));
    for path in dir.flatten().map(|entry| entry.path()) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let Some(rest) = name.strip_prefix("save_v") else {
            continue;
        };
        let version = rest
            .split_once('_')
            .and_then(|(v, _)| v.parse().ok())
            .unwrap_or_else(|| panic!("{name}: name golden saves save_v<version>_<kind>.ron"));
        found.push((version, path));
    }
    found.sort();
    found
}

/// The player's game: two units, some gold, a stocked potion, a flag and
/// some playtime.
fn campaign() -> Campaign {
    let mut campaign = Campaign::new_game(
        GameMode::Classic,
        LeadProfile::new("Mara", LeadGender::Female),
        "ch01",
        vec![
            replay::unit(1, Faction::Player, 0, 0),
            replay::unit(2, Faction::Player, 0, 2),
        ],
        250,
        Stock {
            items: BTreeMap::from([(ItemId::new("potion"), 2)]),
            ..Stock::default()
        },
    );
    campaign.flags = BTreeMap::from([("met_the_smith".to_owned(), true)]);
    campaign.playtime_s = 3_725;
    campaign
}

/// The replay battle, [`PLAYED`] commands in (attacks, equips, a potion, a
/// talk), with one rewind charge spent on the way.
fn history() -> BattleHistory {
    let script = replay::script();
    let (state, _) = BattleState::new(replay::setup(42));
    let mut history = BattleHistory::new(state);
    for cmd in &script[..8] {
        history.push(cmd.clone());
    }
    if let Err(e) = history.rewind_to(6) {
        panic!("{e:?}");
    }
    for cmd in &script[6..PLAYED] {
        history.push(cmd.clone());
    }
    history
}

fn golden(kind: &str) -> SaveFile {
    match kind {
        "chapter" => SaveFile::chapter_cleared(campaign()),
        "suspend" => SaveFile::suspended(campaign(), history()),
        _ => panic!("no golden save of kind {kind}"),
    }
}

/// Reads the current version's golden save of `kind`.
fn read(kind: &str) -> (String, SaveFile) {
    let path = fixture(SAVE_VERSION, kind);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}\nno golden save for SAVE_VERSION {SAVE_VERSION}: {HELP}",
            path.display()
        )
    });
    let save = ron::from_str(&text)
        .unwrap_or_else(|e| panic!("{} no longer parses ({e}): {HELP}", path.display()));
    (text, save)
}

/// Writes the golden saves of the current version. Run it by hand after
/// raising `SAVE_VERSION` (see the module docs).
#[test]
#[ignore = "writes the fixtures; run by hand after raising SAVE_VERSION"]
fn regenerate() {
    for kind in KINDS {
        let text = ron::to_string(&golden(kind)).unwrap();
        std::fs::write(fixture(SAVE_VERSION, kind), text).unwrap();
    }
}

#[test]
fn the_current_versions_golden_saves_read_and_write_back_the_same() {
    for kind in KINDS {
        let (text, save) = read(kind);
        assert_eq!(save.version, SAVE_VERSION, "{kind}: {HELP}");
        // Not `assert_eq!`: it would print both saves, burying the message.
        assert!(
            ron::to_string(&save).unwrap() == text,
            "the {kind} save is written differently now: {HELP}"
        );
    }
    assert_eq!(read("chapter").1.point, SavePoint::ChapterCleared);
}

/// A rule change that makes old commands play out differently changes what
/// a suspend save means, so it needs a new version too.
#[test]
fn the_suspend_save_replays_to_the_same_battle() {
    let (_, save) = read("suspend");
    let SavePoint::Battle(mut history) = save.point else {
        panic!("the suspend save holds no battle: {HELP}");
    };
    history.restore_tables(&trpg_core::GameTables {
        terrain: replay::terrain(),
        classes: replay::classes(),
        items: replay::items(),
        spells: replay::spells(),
        skills: replay::skills(),
        arts: Arc::default(),
        supports: Arc::default(),
    });
    assert_eq!(history.len(), PLAYED, "{HELP}");
    assert_eq!(history.charges_left(), 2, "{HELP}");
    let state = history.state_at(history.len());
    let unit = |id| state.unit(UnitId(id)).unwrap();
    let facts = (
        state.turn(),
        state.phase(),
        [1, 2, 3, 4].map(|id| unit(id).hp),
        [1, 2].map(|id| (unit(id).level, unit(id).exp)),
        unit(2).pos,
    );
    assert_eq!(
        facts,
        (
            4,
            Phase::Player,
            [30, 25, 30, 30],
            [(1, 48); 2],
            Pos::new(1, 2)
        ),
        "the saved commands replay differently now: {HELP}"
    );
}

/// Golden saves of older versions may stay in the folder: this build
/// refuses them, but must still see which version wrote them.
#[test]
fn older_golden_saves_still_show_their_version() {
    for (version, path) in golden_saves() {
        assert!(
            version <= SAVE_VERSION,
            "{} is from a later version than SAVE_VERSION {SAVE_VERSION}",
            path.display()
        );
        if version == SAVE_VERSION {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let header: SaveHeader = ron::from_str(&text)
            .unwrap_or_else(|e| panic!("{}: its version can't be read: {e}", path.display()));
        assert_eq!(header.version, version, "{}", path.display());
        assert!(!header.is_current(), "{}", path.display());
    }
}
