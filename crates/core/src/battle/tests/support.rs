//! Supports in battle (ticket 1002, `docs/design/supports.md`): each rule
//! that gives a pair points, the stop at an unviewed threshold, the
//! Hit/Avoid bonus of the best partner in range, and rewind and saves.
//!
//! The characters: `ann` (the lord, unit 1), `ben` (2), `cal` (5), `dan`
//! (7, in no pair) and `eve` (6, a green unit). The point values differ
//! from each other here ([`RULES`]) so that a mixed-up rule shows.

use super::art::{art_attack, art_setup, artist};
use super::*;
use crate::history::BattleHistory;
use crate::support::{
    ByRank, PairDef, PointValues, SupportBook, SupportPair, SupportRank, SupportRules, SupportTable,
};

/// The design's rules, with every point value different: adjacent at the
/// phase's end 1, fighting beside 4, heal 5, buff 6, item 2.
const RULES: SupportRules = SupportRules {
    points: PointValues {
        adjacent_at_phase_end: 1,
        fight_beside: 4,
        heal: 5,
        buff: 6,
        item: 2,
    },
    ..SupportRules::STARTING
};

fn c(id: &str) -> CharacterId {
    CharacterId(id.into())
}

fn pair(a: &str, b: &str) -> SupportPair {
    SupportPair::new(c(a), c(b))
}

/// `u` as character `id`.
fn named(u: Unit, id: &str) -> Unit {
    Unit {
        character: Some(c(id)),
        ..u
    }
}

fn ann(pos: Pos) -> Unit {
    named(lord(1, pos), "ann")
}

fn ben(pos: Pos) -> Unit {
    named(unit(2, Faction::Player, pos), "ben")
}

fn cal(pos: Pos) -> Unit {
    named(unit(5, Faction::Player, pos), "cal")
}

fn dan(pos: Pos) -> Unit {
    named(unit(7, Faction::Player, pos), "dan")
}

fn eve(pos: Pos) -> Unit {
    named(unit(6, Faction::Ally, pos), "eve")
}

fn foe(id: u32, pos: Pos) -> Unit {
    unit(id, Faction::Enemy, pos)
}

fn wounded(u: Unit, hp: StatValue) -> Unit {
    Unit { hp, ..u }
}

/// A hit-50 test sword, so hit and avoid bonuses show.
fn blade() -> ItemId {
    weapon_id(1, 1, 3, 50, 0)
}

/// `ann` with `ben`, `cal` and `eve`. `ben`–`cal` and anyone with `dan`
/// have no support.
fn table() -> Arc<SupportTable> {
    let def = |a: &str, b: &str| PairDef {
        pair: pair(a, b),
        thresholds: None,
        starting_points: 0,
        conversations: ByRank {
            c: "c".into(),
            b: "b".into(),
            a: "a".into(),
        },
    };
    let pairs = [def("ann", "ben"), def("ann", "cal"), def("ann", "eve")];
    Arc::new(SupportTable::new(RULES, pairs))
}

fn with_supports(setup: BattleSetup, bonds: SupportBook) -> BattleState {
    start(BattleSetup {
        supports: table(),
        bonds,
        ..setup
    })
}

fn battle(units: Vec<Unit>) -> BattleState {
    with_supports(setup(units), SupportBook::default())
}

/// A book where each pair has `points` (never viewed).
fn book(points: &[(&str, &str, u32)]) -> SupportBook {
    let mut book = SupportBook::default();
    for (a, b, n) in points {
        book.gain(&pair(a, b), *n, &table());
    }
    book
}

/// A book where each pair has viewed its conversations up to `rank`.
fn ranked(ranks: &[(&str, &str, SupportRank)]) -> SupportBook {
    let mut book = SupportBook::default();
    for (a, b, rank) in ranks {
        while book.rank(&pair(a, b)) < Some(*rank) {
            book.gain(&pair(a, b), 500, &table());
            book.view(&pair(a, b), &table()).unwrap();
        }
    }
    book
}

fn points(s: &BattleState, a: &str, b: &str) -> u32 {
    let state = s.supports().state(&pair(a, b), s.support_table());
    state.map_or(0, |state| state.points())
}

fn grew(a: u32, b: u32, amount: u32) -> Event {
    Event::SupportPoints {
        a: UnitId(a),
        b: UnitId(b),
        amount,
    }
}

/// The support events among `events`.
fn support(events: &[Event]) -> Vec<Event> {
    let is_support = |e: &&Event| matches!(e, Event::SupportPoints { .. });
    events.iter().filter(is_support).cloned().collect()
}

fn cast_heal(target: u32) -> UnitAction {
    UnitAction::Cast {
        spell: SpellId::new("heal"),
        target: CastTarget::Unit(UnitId(target)),
        active: None,
    }
}

/// `u` knowing the `heal` spell (range 1), with no weapons.
fn healer(u: Unit) -> Unit {
    Unit {
        learned: BTreeSet::from([SpellId::new("heal")]),
        loadout: Loadout::default(),
        ..u
    }
}

fn use_skill(skill: &str) -> UnitAction {
    UnitAction::UseSkill {
        skill: SkillId::new(skill),
        target: None,
    }
}

/// `u` in the class whose active is `skill`.
fn with_skill(u: Unit, skill: &str) -> Unit {
    Unit {
        class: skill_class(skill),
        ..u
    }
}

// ---- Gaining points ----------------------------------------------------------------

#[test]
fn ending_the_player_phase_adjacent_gives_the_pair_a_point() {
    // Ben is next to Ann. Cal is diagonal to her. Dan is next to her but
    // they have no support. Eve is next to her but isn't a player unit.
    let mut s = battle(vec![
        ann(p(1, 1)),
        ben(p(1, 2)),
        cal(p(2, 2)),
        dan(p(1, 0)),
        eve(p(0, 1)),
        foe(3, p(7, 4)),
    ]);
    assert_eq!(
        end(&mut s),
        [grew(1, 2, 1), started(1, Phase::Enemy)],
        "the points come before the next phase starts"
    );
    // Only the player phase's end counts.
    assert_eq!(end(&mut s), [started(1, Phase::Other)]);
    assert_eq!(end(&mut s), [started(2, Phase::Player)]);
    assert_eq!(points(&s, "ann", "ben"), 1);
    assert_eq!(support(&end(&mut s)), [grew(1, 2, 1)]);
    assert_eq!(points(&s, "ann", "ben"), 2);
    assert_eq!(points(&s, "ann", "cal"), 0);
    assert_eq!(points(&s, "ann", "eve"), 0);
    assert_eq!(s.supports().state(&pair("ann", "dan"), &table()), None);
}

#[test]
fn every_adjacent_pair_gets_its_point_once() {
    // Ann between Ben and Cal: two pairs, one point each, in unit-list
    // order, each named from the unit that comes first in it.
    let mut s = battle(vec![
        cal(p(1, 0)),
        ben(p(1, 2)),
        ann(p(1, 1)),
        foe(3, p(7, 4)),
    ]);
    assert_eq!(support(&end(&mut s)), [grew(5, 1, 1), grew(2, 1, 1)]);
    assert_eq!(points(&s, "ann", "ben"), 1);
    assert_eq!(points(&s, "ann", "cal"), 1);
}

#[test]
fn fighting_with_a_partner_adjacent_gives_the_pair_points() {
    // Ann attacks from 2 tiles away (any range counts), next to Ben.
    let mut s = battle(vec![
        carrying(ann(p(0, 0)), &[weapon(1, 2, 3)]),
        ben(p(2, 1)),
        cal(p(5, 4)),
        foe(3, p(4, 0)),
        foe(4, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(2, 0), attack(3));
    assert_eq!(
        events[events.len() - 2..],
        [grew(1, 2, 4), Event::UnitActed { unit: UnitId(1) }]
    );
    assert_eq!(support(&events).len(), 1);
    assert_eq!(support(&end(&mut s)), [grew(1, 2, 1)]);
    // Being attacked counts too: enemy 3 attacks Ben, Ann next to him.
    let events = act(&mut s, 3, p(3, 1), attack(2));
    assert_eq!(support(&events), [grew(2, 1, 4)]);
    assert_eq!(points(&s, "ann", "ben"), 9);
    // Cal is attacked with no partner next to her: nothing.
    let events = act(&mut s, 4, p(6, 4), attack(5));
    assert_eq!(support(&events), []);
    assert_eq!(points(&s, "ann", "cal"), 0);
}

#[test]
fn a_fight_beside_a_partner_two_tiles_away_gives_nothing() {
    let mut s = battle(vec![ann(p(0, 0)), ben(p(1, 2)), foe(3, p(2, 0))]);
    assert_eq!(support(&act(&mut s, 1, p(1, 0), attack(3))), []);
    assert_eq!(points(&s, "ann", "ben"), 0);
}

#[test]
fn a_unit_that_falls_in_the_fight_gains_nothing() {
    // Ben falls to the counter, next to Ann.
    let mut s = battle(vec![
        ann(p(0, 0)),
        wounded(ben(p(0, 1)), 3),
        foe(3, p(1, 1)),
    ]);
    let events = act(&mut s, 2, p(0, 1), attack(3));
    assert!(events.contains(&Event::UnitFell { unit: UnitId(2) }));
    assert_eq!(support(&events), []);
    // Ben is attacked and falls, next to Ann.
    let mut s = battle(vec![
        ann(p(0, 0)),
        wounded(ben(p(0, 1)), 3),
        foe(3, p(1, 1)),
    ]);
    end(&mut s);
    let events = act(&mut s, 3, p(1, 1), attack(2));
    assert!(events.contains(&Event::UnitFell { unit: UnitId(2) }));
    assert_eq!(support(&events), []);
    assert_eq!(points(&s, "ann", "ben"), 1);
}

#[test]
fn one_attack_gives_a_pair_its_points_once() {
    // A boss's Line Pierce strikes Ann, then Ben behind her: both fought,
    // side by side, and the pair still gets one award. Cal, next to Ben
    // only, has no support with him.
    let boss = Unit {
        faction: Faction::Enemy,
        role: Role::Boss,
        ..artist(3, p(0, 0), "pike")
    };
    let units = vec![ann(p(2, 0)), ben(p(3, 0)), cal(p(3, 1)), boss];
    let mut s = with_supports(art_setup(units), SupportBook::default());
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), art_attack(1, "line_pierce"));
    let fights = events
        .iter()
        .filter(|e| matches!(e, Event::CombatResolved { .. }));
    assert_eq!(fights.count(), 2);
    assert_eq!(support(&events), [grew(1, 2, 4)]);
    assert_eq!(points(&s, "ann", "ben"), 5);
}

#[test]
fn a_line_pierces_victim_was_attacked_too() {
    // The boss strikes Dan, then Ann behind him, with Ben next to Ann.
    let boss = Unit {
        faction: Faction::Enemy,
        role: Role::Boss,
        ..artist(3, p(0, 0), "pike")
    };
    let units = vec![dan(p(2, 0)), ann(p(3, 0)), ben(p(3, 1)), boss];
    let mut s = with_supports(art_setup(units), SupportBook::default());
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), art_attack(7, "line_pierce"));
    assert_eq!(support(&events), [grew(1, 2, 4)]);
}

#[test]
fn healing_a_partner_with_a_spell_gives_the_pair_points() {
    let mut s = battle(vec![
        healer(ann(p(0, 0))),
        wounded(ben(p(1, 1)), 3),
        wounded(dan(p(0, 2)), 3),
        healer(cal(p(0, 3))),
        foe(3, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(1, 0), cast_heal(2));
    assert_eq!(
        events[events.len() - 2..],
        [grew(1, 2, 5), Event::UnitActed { unit: UnitId(1) }]
    );
    assert_eq!(points(&s, "ann", "ben"), 5);
    // Healing someone the caster has no support with gives nothing.
    assert_eq!(support(&act(&mut s, 5, p(0, 3), cast_heal(7))), []);
}

#[test]
fn a_heal_on_several_allies_gives_points_to_each_pair() {
    // Sanctuary heals the wounded allies next to Ann: Ben, Cal and Dan
    // (no support). Eve is a green unit.
    let mut s = battle(vec![
        with_skill(wounded(ann(p(1, 1)), 3), "sanctuary"),
        wounded(ben(p(1, 0)), 4),
        wounded(cal(p(2, 1)), 4),
        wounded(dan(p(1, 2)), 4),
        wounded(eve(p(0, 1)), 4),
        foe(3, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(1, 1), use_skill("sanctuary"));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Healed { .. }))
            .count(),
        4
    );
    assert_eq!(support(&events), [grew(1, 2, 5), grew(1, 5, 5)]);
    assert_eq!(events.last(), Some(&Event::UnitActed { unit: UnitId(1) }));
    assert_eq!(points(&s, "ann", "eve"), 0);
}

#[test]
fn a_partner_at_full_hp_isnt_healed_and_gains_nothing() {
    let mut s = battle(vec![
        with_skill(ann(p(1, 1)), "sanctuary"),
        ben(p(1, 0)),
        wounded(cal(p(2, 1)), 4),
        foe(3, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(1, 1), use_skill("sanctuary"));
    assert_eq!(support(&events), [grew(1, 5, 5)]);
}

#[test]
fn buffing_partners_gives_points_to_each_pair() {
    // War Cry buffs the allies next to Ann.
    let mut s = battle(vec![
        with_skill(ann(p(1, 1)), "war_cry"),
        ben(p(1, 0)),
        cal(p(2, 1)),
        dan(p(1, 2)),
        foe(3, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(1, 1), use_skill("war_cry"));
    assert_eq!(support(&events), [grew(1, 2, 6), grew(1, 5, 6)]);
    assert_eq!(events.last(), Some(&Event::UnitActed { unit: UnitId(1) }));
    // A buff on the user alone gives nothing.
    let mut s = battle(vec![
        with_skill(ann(p(1, 1)), "brace"),
        ben(p(1, 0)),
        foe(3, p(7, 4)),
    ]);
    assert_eq!(support(&act(&mut s, 1, p(1, 1), use_skill("brace"))), []);
    assert_eq!(points(&s, "ann", "ben"), 0);
}

#[test]
fn using_an_item_on_a_partner_gives_the_pair_points() {
    let mut s = battle(vec![
        ann(p(0, 0)),
        wounded(ben(p(1, 1)), 3),
        wounded(cal(p(4, 4)), 3),
        foe(3, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(1, 0), use_item(0, 2));
    assert_eq!(
        events[events.len() - 2..],
        [grew(1, 2, 2), Event::UnitActed { unit: UnitId(1) }]
    );
    assert_eq!(points(&s, "ann", "ben"), 2);
    // On itself: nothing.
    assert_eq!(support(&act(&mut s, 5, p(4, 4), use_item(0, 5))), []);
}

#[test]
fn points_stop_at_an_unviewed_threshold_in_battle() {
    // 18 of the 20 points C needs: a heal's 5 give 2, then nothing.
    let units = vec![
        healer(ann(p(0, 0))),
        wounded(ben(p(0, 1)), 1),
        foe(3, p(7, 4)),
    ];
    let mut s = with_supports(setup(units), book(&[("ann", "ben", 18)]));
    assert_eq!(
        support(&act(&mut s, 1, p(0, 0), cast_heal(2))),
        [grew(1, 2, 2)]
    );
    let state = s.supports().state(&pair("ann", "ben"), &table()).unwrap();
    assert_eq!(
        (state.points(), state.rank(), state.unlocked()),
        (20, None, Some(SupportRank::C))
    );
    // No points, no event: the conversation waits for camp.
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    end(&mut s);
    assert_eq!(support(&act(&mut s, 1, p(0, 0), use_item(0, 2))), []);
    assert_eq!(points(&s, "ann", "ben"), 20);
}

// ---- Battle bonus ------------------------------------------------------------------

/// Ann (hit 50) moves from (0,0) to (1,0) and attacks enemy 3 (hit 50) at
/// (2,0), with `others` on the map: her hit and the enemy's hit on her.
fn hits(others: Vec<Unit>, bonds: SupportBook) -> (u8, u8) {
    let mut units = vec![
        carrying(ann(p(0, 0)), &[blade()]),
        carrying(foe(3, p(2, 0)), &[blade()]),
    ];
    units.extend(others);
    let s = with_supports(setup(units), bonds);
    let shown = s.preview_attack(UnitId(1), p(1, 0), &attack(3)).unwrap();
    let f = shown.forecast;
    (f.attacker.hit, f.defender.map_or(0, |d| d.hit))
}

#[test]
fn a_supported_partner_in_range_adds_hit_and_avoid() {
    use SupportRank::{A, B, C};
    let near = || vec![ben(p(1, 3))];
    assert_eq!(hits(vec![], SupportBook::default()), (50, 50));
    // Points alone give nothing: the rank comes with the conversation.
    assert_eq!(hits(near(), book(&[("ann", "ben", 20)])), (50, 50));
    assert_eq!(hits(near(), ranked(&[("ann", "ben", C)])), (55, 45));
    assert_eq!(hits(near(), ranked(&[("ann", "ben", B)])), (60, 40));
    assert_eq!(hits(near(), ranked(&[("ann", "ben", A)])), (65, 35));
}

#[test]
fn the_partner_must_be_within_three_tiles_of_where_the_unit_fights() {
    let rank_c = || ranked(&[("ann", "ben", SupportRank::C)]);
    // 3 tiles from (1,0), where Ann attacks from (4 from where she was).
    assert_eq!(hits(vec![ben(p(1, 3))], rank_c()), (55, 45));
    // 4 tiles from (1,0) (3 from where she was).
    assert_eq!(hits(vec![ben(p(0, 3))], rank_c()), (50, 50));
    assert_eq!(hits(vec![ben(p(1, 4))], rank_c()), (50, 50));
    // A partner who isn't on the map gives nothing.
    assert_eq!(hits(vec![], rank_c()), (50, 50));
}

#[test]
fn bonuses_never_combine_only_the_best_partner_counts() {
    use SupportRank::{A, B, C};
    let both = || vec![ben(p(1, 1)), cal(p(0, 1))];
    let bonds = ranked(&[("ann", "ben", C), ("ann", "cal", A)]);
    assert_eq!(hits(both(), bonds), (65, 35));
    let bonds = ranked(&[("ann", "ben", B), ("ann", "cal", C)]);
    assert_eq!(hits(both(), bonds), (60, 40));
    // The best partner out of range: the other one's bonus.
    let bonds = ranked(&[("ann", "ben", C), ("ann", "cal", A)]);
    assert_eq!(hits(vec![ben(p(1, 1)), cal(p(5, 4))], bonds), (55, 45));
}

#[test]
fn only_player_units_give_and_get_the_bonus() {
    let bonds = || ranked(&[("ann", "eve", SupportRank::A)]);
    assert_eq!(hits(vec![eve(p(1, 1))], bonds()), (50, 50));
    // A green Ann gets nothing from Ben either.
    let green = Unit {
        faction: Faction::Ally,
        is_lord: false,
        ..carrying(ann(p(0, 0)), &[blade()])
    };
    let units = vec![
        lord(9, p(7, 4)),
        green,
        ben(p(1, 1)),
        carrying(foe(3, p(2, 0)), &[blade()]),
    ];
    let mut s = with_supports(setup(units), ranked(&[("ann", "ben", SupportRank::A)]));
    end(&mut s);
    end(&mut s);
    let shown = s.preview_attack(UnitId(1), p(1, 0), &attack(3)).unwrap();
    assert_eq!(shown.forecast.attacker.hit, 50);
}

#[test]
fn the_bonus_counts_when_countering() {
    // Enemy 3 attacks Ann, Ben 3 tiles from her.
    let units = vec![
        carrying(ann(p(0, 0)), &[blade()]),
        ben(p(0, 3)),
        carrying(foe(3, p(2, 0)), &[blade()]),
    ];
    let mut s = with_supports(setup(units), ranked(&[("ann", "ben", SupportRank::B)]));
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), attack(1));
    let fought = events.iter().find_map(|e| match e {
        Event::CombatResolved { forecast, .. } => Some(*forecast),
        _ => None,
    });
    let f = fought.unwrap();
    // The enemy's hit falls by her avoid; her counter's hit rises.
    assert_eq!((f.attacker.hit, f.defender.map(|d| d.hit)), (40, Some(60)));
}

// ---- Rewind, replay and saves --------------------------------------------------------

#[test]
fn rewinding_undoes_support_points_and_a_replay_gives_them_again() {
    let units = vec![
        healer(ann(p(0, 0))),
        wounded(ben(p(0, 1)), 1),
        foe(3, p(7, 4)),
    ];
    let first = battle(units);
    let mut s = first.clone();
    let mut history = BattleHistory::new(first.clone());
    let heal = Command::Act {
        unit: UnitId(1),
        dest: p(0, 0),
        action: cast_heal(2),
    };
    let mut seen = Vec::new();
    for cmd in [heal, Command::EndPhase] {
        seen.push(s.apply(&cmd).unwrap());
        history.push(cmd);
    }
    assert_eq!(points(&s, "ann", "ben"), 6);
    // The replay gives the same events and the same supports.
    let replayed: Vec<_> = history.replay().into_iter().map(|r| r.events).collect();
    assert_eq!(replayed, seen);
    assert_eq!(history.state_at(2), s);
    assert_eq!(points(&history.state_at(1), "ann", "ben"), 5);
    // Rewinding takes the points back.
    let back = history.rewind_to(0).unwrap();
    assert_eq!(points(&back, "ann", "ben"), 0);
    assert_eq!(back, first);
}

#[test]
fn supports_are_saved_with_the_battle_and_the_table_is_not() {
    let units = vec![
        healer(ann(p(0, 0))),
        wounded(ben(p(0, 1)), 1),
        foe(3, p(7, 4)),
    ];
    let mut s = with_supports(setup(units.clone()), book(&[("ann", "ben", 3)]));
    act(&mut s, 1, p(0, 0), cast_heal(2));
    let mut loaded: BattleState = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
    assert_eq!(loaded.supports(), s.supports());
    assert_eq!(points(&s, "ann", "ben"), 8);
    // The pairs and rules are content: gone until the tables are back.
    assert_eq!(loaded.support_table(), &SupportTable::default());
    loaded.restore_tables(&crate::GameTables {
        terrain: Arc::new(terrain()),
        classes: Arc::new(classes()),
        items: Arc::new(items_for(&units)),
        spells: Arc::new(spells()),
        skills: Arc::new(skills()),
        arts: Arc::new(test_arts()),
        supports: table(),
    });
    assert_eq!(loaded, s);
    assert_eq!(
        loaded.apply(&Command::EndPhase),
        s.apply(&Command::EndPhase)
    );
    assert_eq!(points(&loaded, "ann", "ben"), 9);
}

#[test]
fn a_battle_without_supports_gives_no_points() {
    // The same units, but no pair is listed.
    let mut s = start(setup(vec![ann(p(0, 0)), ben(p(0, 1)), foe(3, p(7, 4))]));
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    assert_eq!(s.supports(), &SupportBook::default());
}
