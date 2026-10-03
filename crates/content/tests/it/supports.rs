//! Supports in a battle built from the real content (ticket 1002): the
//! test pairs of `assets/data/supports.ron` (the test lord with the test
//! knight and with the test mage), the real rules and the real `Heal`
//! spell. A healer heals a partner and two units fight side by side; the
//! test checks the support events and the pairs' final state against the
//! numbers in `docs/design/supports.md`.

use std::sync::OnceLock;

use trpg_content::{Content, character_unit};
use trpg_core::{
    BattleMap, BattlePack, BattleSetup, BattleState, CastTarget, CharacterId, ClassId, Command,
    Event, Faction, GameMode, Grid, ItemId, LoadoutDef, Objective, Phase, Pos, SpellId, Stats,
    Stock, SupportBook, SupportPair, SupportRank, SupportRules, Unit, UnitAction, UnitId,
};

fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| trpg_content::load_embedded().unwrap_or_else(|e| panic!("{e}")))
}

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// The named character `id` as player unit `unit` at `pos`.
fn character(id: &str, unit: u32, pos: Pos) -> Unit {
    let c = content();
    let def = c
        .characters
        .characters
        .get(&CharacterId(id.into()))
        .unwrap_or_else(|| panic!("no character {id}"));
    character_unit(
        def,
        UnitId(unit),
        &c.classes,
        &c.items,
        Faction::Player,
        pos,
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// An enemy swordsman that can't hurt anyone much (Str 0) and has 40 HP,
/// so that nobody falls whatever the luck.
fn dummy(unit: u32, pos: Pos) -> Unit {
    let c = content();
    let class = ClassId("swordsman".into());
    let mut u = Unit::generic(UnitId(unit), &class, &c.classes, 1, Faction::Enemy, pos)
        .unwrap_or_else(|e| panic!("{e}"));
    let mov = u.stats.mov;
    u.stats = Stats::from_growable([40, 0, 0, 0, 0, 0, 0], mov);
    u.hp = 40;
    let loadout = LoadoutDef {
        weapons: vec![ItemId::new("iron_sword")],
        ..LoadoutDef::default()
    };
    u.with_loadout(&loadout, &c.classes, &c.items)
        .unwrap_or_else(|e| panic!("{e}"))
}

/// A battle on an 8×5 plain map, with the content's supports and `bonds`.
fn battle(units: Vec<Unit>, bonds: SupportBook) -> BattleState {
    let c = content();
    let plain = c.terrain.display.id_of("plain");
    let plain = plain.unwrap_or_else(|| panic!("no plain terrain"));
    let tables = c.tables();
    BattleState::new(BattleSetup {
        map: BattleMap::new("Supports", Grid::filled(8, 5, plain)),
        terrain: tables.terrain,
        classes: tables.classes,
        items: tables.items,
        spells: tables.spells,
        skills: tables.skills,
        arts: tables.arts,
        supports: tables.supports,
        bonds,
        pack: BattlePack::default(),
        gold: 0,
        stock: Stock::default(),
        units,
        reinforcements: vec![],
        objective: Objective::Rout { turn_limit: None },
        rewind_charges: 0,
        seed: 1,
        triggers: vec![],
        mode: GameMode::Classic,
        battle_notes: vec![],
    })
    .0
}

/// Applies `cmd` and returns its support events.
fn support(s: &mut BattleState, cmd: &Command) -> Vec<Event> {
    let events = s.apply(cmd).unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    let is_support = |e: &Event| matches!(e, Event::SupportPoints { .. });
    events.into_iter().filter(is_support).collect()
}

fn act(unit: u32, dest: Pos, action: UnitAction) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest,
        action,
    }
}

fn attack(target: u32) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
        active: None,
        art: None,
    }
}

fn grew(a: u32, b: u32, amount: u32) -> Event {
    Event::SupportPoints {
        a: UnitId(a),
        b: UnitId(b),
        amount,
    }
}

fn pair(a: &str, b: &str) -> SupportPair {
    SupportPair::new(CharacterId(a.into()), CharacterId(b.into()))
}

const LORD: u32 = 1;
const KNIGHT: u32 = 2;
const MAGE: u32 = 3;
const ENEMY: u32 = 4;

/// The lord (wounded) at (1,1), the knight at (1,3), the mage at (0,1) and
/// the enemy at (2,1).
fn cast() -> Vec<Unit> {
    let mut lord = character("test_lord", LORD, p(1, 1));
    lord.hp -= 6;
    vec![
        lord,
        character("test_knight", KNIGHT, p(1, 3)),
        character("test_mage", MAGE, p(0, 1)),
        dummy(ENEMY, p(2, 1)),
    ]
}

#[test]
fn a_healer_and_two_fighters_side_by_side_build_their_supports() {
    let table = &content().supports;
    assert_eq!(table.rules, SupportRules::STARTING);
    let mut s = battle(cast(), SupportBook::default());
    // The mage heals the lord: heal, +3.
    let heal = UnitAction::Cast {
        spell: SpellId::new("heal"),
        target: CastTarget::Unit(UnitId(LORD)),
        active: None,
    };
    assert_eq!(
        support(&mut s, &act(MAGE, p(0, 1), heal)),
        [grew(MAGE, LORD, 3)]
    );
    // The knight steps next to the lord: nothing yet.
    assert_eq!(support(&mut s, &act(KNIGHT, p(1, 2), UnitAction::Wait)), []);
    // The lord attacks with the knight and the mage adjacent: +3 each.
    assert_eq!(
        support(&mut s, &act(LORD, p(1, 1), attack(ENEMY))),
        [grew(LORD, KNIGHT, 3), grew(LORD, MAGE, 3)]
    );
    // The player phase ends with both next to the lord: +1 each. The
    // knight and the mage have no support with each other.
    assert_eq!(
        support(&mut s, &Command::EndPhase),
        [grew(LORD, KNIGHT, 1), grew(LORD, MAGE, 1)]
    );
    assert_eq!(s.phase(), Phase::Enemy);
    // The enemy attacks the knight, the lord adjacent to it: +3.
    assert_eq!(
        support(&mut s, &act(ENEMY, p(2, 2), attack(KNIGHT))),
        [grew(KNIGHT, LORD, 3)]
    );
    assert_eq!(s.outcome(), None);
    // The enemy phase's end gives nothing.
    assert_eq!(support(&mut s, &Command::EndPhase), []);
    for partner in ["test_knight", "test_mage"] {
        let state = s.supports().state(&pair("test_lord", partner), table);
        let state = state.unwrap_or_else(|| panic!("no pair with {partner}"));
        assert_eq!(
            (state.points(), state.rank(), state.unlocked()),
            (7, None, None),
            "{partner}"
        );
    }
    assert_eq!(
        s.supports().state(&pair("test_knight", "test_mage"), table),
        None
    );
}

#[test]
fn the_points_stop_at_c_until_the_conversation_is_viewed() {
    let table = &content().supports;
    let lord_knight = pair("test_lord", "test_knight");
    let mut bonds = SupportBook::default();
    assert_eq!(bonds.gain(&lord_knight, 19, table), 19);
    let mut s = battle(cast(), bonds);
    support(&mut s, &act(KNIGHT, p(1, 2), UnitAction::Wait));
    // 19 points: the attack's 3 give the last one, and C is unlocked.
    assert_eq!(
        support(&mut s, &act(LORD, p(1, 1), attack(ENEMY))),
        [grew(LORD, KNIGHT, 1), grew(LORD, MAGE, 3)]
    );
    assert_eq!(support(&mut s, &Command::EndPhase), [grew(LORD, MAGE, 1)]);
    let mut bonds = s.supports().clone();
    let state = bonds
        .state(&lord_knight, table)
        .expect("the pair is listed");
    assert_eq!(
        (state.points(), state.rank(), state.unlocked()),
        (20, None, Some(SupportRank::C))
    );
    // Viewing it names the placeholder scene, which exists.
    let viewed = bonds.view(&lord_knight, table).expect("C is unlocked");
    assert_eq!(viewed.rank, SupportRank::C);
    assert_eq!(viewed.conversation, "test_support_lord_knight_c");
    assert!(content().dialogue.get(&viewed.conversation).is_some());
    assert_eq!(bonds.rank(&lord_knight), Some(SupportRank::C));
}
