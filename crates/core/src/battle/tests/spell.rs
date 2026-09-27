//! Spells in battle: uses per battle, casting attack and heal spells,
//! equipping spells, and the worked examples of `magic.md`.

use super::*;
use crate::combat::SideForecast;

fn sid(id: &str) -> SpellId {
    SpellId::new(id)
}

/// `u` knowing `spells`, with no weapons.
fn knowing(u: Unit, spells: &[&str]) -> Unit {
    Unit {
        learned: spells.iter().map(|s| sid(s)).collect(),
        loadout: Loadout::default(),
        ..u
    }
}

/// A test unit knowing `spells`, with no weapons.
fn mage(id: u32, faction: Faction, pos: Pos, spells: &[&str]) -> Unit {
    knowing(unit(id, faction, pos), spells)
}

fn cast_on(spell: &str, target: u32) -> UnitAction {
    UnitAction::Cast {
        spell: sid(spell),
        target: CastTarget::Unit(UnitId(target)),
    }
}

fn equip_spell(unit: u32, spell: &str) -> Command {
    Command::Equip {
        unit: UnitId(unit),
        equipped: Equipped::Spell(sid(spell)),
    }
}

fn uses(s: &BattleState, id: u32, spell: &str) -> u8 {
    s.unit(UnitId(id)).unwrap().spells.uses_left(&sid(spell))
}

fn equipped(s: &BattleState, id: u32) -> Option<Equipped> {
    s.unit(UnitId(id)).unwrap().loadout.equipped.clone()
}

/// The forecast of the combat in `events`.
fn combat(events: &[Event]) -> Forecast {
    let forecast = events.iter().find_map(|e| match e {
        Event::CombatResolved { forecast, .. } => Some(*forecast),
        _ => None,
    });
    forecast.unwrap_or_else(|| panic!("no combat in {events:?}"))
}

fn spell_used(unit: u32, spell: &str, uses_left: u8) -> Event {
    Event::SpellUsesChanged {
        unit: UnitId(unit),
        spell: sid(spell),
        uses_left,
    }
}

fn spell_cast(unit: u32, spell: &str, target: u32) -> Event {
    Event::SpellCast {
        unit: UnitId(unit),
        spell: sid(spell),
        target: CastTarget::Unit(UnitId(target)),
    }
}

/// Ends phases until unit `id`'s phase comes round again.
fn next_turn(s: &mut BattleState) {
    let turn = s.turn();
    while s.turn() == turn || s.phase() != Phase::Player {
        end(s);
    }
}

// ---- Uses per battle -------------------------------------------------------------

#[test]
fn uses_refill_at_battle_start() {
    let mut units = cast();
    // Stale uses from an earlier battle, and a learned spell the table
    // doesn't have.
    units[1] = knowing(units[1].clone(), &["fire", "heal", "ghost"]);
    units[1].spells.uses_left = BTreeMap::from([(sid("fire"), 0), (sid("mend"), 3)]);
    let arriving = mage(9, Faction::Enemy, p(5, 4), &["bolt"]);
    let s = start(BattleSetup {
        reinforcements: vec![reinforcement(2, arriving)],
        ..setup(units)
    });
    let u = s.unit(UnitId(2)).unwrap();
    assert_eq!(
        u.spells.uses_left,
        BTreeMap::from([(sid("fire"), 10), (sid("heal"), 8)])
    );
    assert_eq!(
        s.reinforcements()[0].unit.spells.uses_left,
        BTreeMap::from([(sid("bolt"), 2)])
    );
    // A unit with no spells has no uses.
    assert!(s.unit(UnitId(1)).unwrap().spells.uses_left.is_empty());
}

#[test]
fn battle_start_equips_the_first_attack_spell_of_an_unarmed_caster() {
    let units = vec![
        lord(1, p(0, 0)),
        // Id order: "fire" < "force"; the heal can't be equipped.
        mage(2, Faction::Player, p(0, 2), &["heal", "force", "fire"]),
        mage(3, Faction::Player, p(0, 4), &["heal"]),
        // A wieldable weapon comes first.
        knowing(unit(4, Faction::Enemy, p(7, 0)), &["fire"]),
        carrying(
            knowing(unit(5, Faction::Enemy, p(7, 2)), &["fire"]),
            &[weapon(1, 1, 3)],
        ),
    ];
    let mut units = units;
    // Already equipped with a spell: kept.
    units[0] = knowing(units[0].clone(), &["fire", "bolt"]);
    units[0].loadout.equipped = Some(Equipped::Spell(sid("bolt")));
    units[4].loadout.equipped = None;
    let s = start(setup(units));
    assert_eq!(equipped(&s, 1), Some(Equipped::Spell(sid("bolt"))));
    assert_eq!(equipped(&s, 2), Some(Equipped::Spell(sid("fire"))));
    assert_eq!(equipped(&s, 3), None);
    assert_eq!(equipped(&s, 4), Some(Equipped::Spell(sid("fire"))));
    assert_eq!(equipped(&s, 5), Some(Equipped::Weapon(0)));
}

#[test]
fn a_cast_spends_one_use_however_many_strikes() {
    let mut units = cast();
    units[1] = mage(2, Faction::Player, p(0, 3), &["bolt"]);
    // Spd 10 against 0: the mage strikes twice.
    units[1].stats.spd = 10;
    units[3].pos = p(2, 3);
    units[3].stats.hp = 20;
    units[3].hp = 20;
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 3), cast_on("bolt", 4));
    let [
        first,
        Event::CombatResolved {
            attacker,
            defender,
            forecast,
            outcome,
        },
        used,
        Event::UnitActed { unit: UnitId(2) },
    ] = events.as_slice()
    else {
        panic!("{events:?}");
    };
    // Already equipped at battle start, so no Equipped event.
    assert_eq!(*first, spell_cast(2, "bolt", 4));
    assert_eq!((*attacker, *defender), (UnitId(2), UnitId(4)));
    // Distance 2: the enemy's sword can't counter.
    assert_eq!(forecast.defender, None);
    assert_eq!(forecast.attacker.strikes, 2);
    assert_eq!(outcome.strikes.len(), 2);
    assert_eq!(outcome.defender_hp, 14);
    assert_eq!(*used, spell_used(2, "bolt", 1));
    assert_eq!(uses(&s, 2, "bolt"), 1);
    // Spells give no weapon EXP.
    assert!(s.unit(UnitId(2)).unwrap().weapon_exp.is_empty());
}

#[test]
fn casting_equips_the_spell_and_a_weapon_attack_equips_the_weapon_back() {
    let mut units = cast();
    units[1] = carrying(knowing(units[1].clone(), &["bolt"]), &[weapon(1, 1, 3)]);
    units[3].pos = p(1, 2);
    units[3].stats.hp = 30;
    units[3].hp = 30;
    let mut s = start(setup(units));
    assert_eq!(equipped(&s, 2), Some(Equipped::Weapon(0)));
    let events = act(&mut s, 2, p(0, 2), cast_on("bolt", 4));
    assert_eq!(
        events[..2],
        [
            Event::Equipped {
                unit: UnitId(2),
                equipped: Equipped::Spell(sid("bolt")),
            },
            spell_cast(2, "bolt", 4),
        ]
    );
    // The sword counters at distance 1: its wielder gains weapon EXP.
    assert!(events.contains(&spell_used(2, "bolt", 1)));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::WeaponExpGained {
            unit: UnitId(4),
            ..
        }
    )));
    assert_eq!(equipped(&s, 2), Some(Equipped::Spell(sid("bolt"))));
    // The enemy attacks: the mage counters with the spell and spends a use.
    end(&mut s);
    let events = act(&mut s, 4, p(1, 2), attack(2));
    let forecast = combat(&events);
    assert_eq!(forecast.defender.map(|d| d.damage), Some(3));
    assert!(events.contains(&spell_used(2, "bolt", 0)));
    // Back on the player's turn, a sword attack equips the sword again.
    next_turn(&mut s);
    let events = act(&mut s, 2, p(0, 2), attack(4));
    assert_eq!(
        events.first(),
        Some(&Event::Equipped {
            unit: UnitId(2),
            equipped: Equipped::Weapon(0),
        })
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::SpellUsesChanged { .. }))
    );
}

#[test]
fn a_spell_at_zero_uses_cant_be_cast_or_counter() {
    let mut units = cast();
    units[1] = mage(2, Faction::Player, p(0, 3), &["bolt"]);
    units[3].pos = p(2, 3);
    units[3].stats.hp = 30;
    units[3].hp = 30;
    let mut s = start(setup(units));
    act(&mut s, 2, p(0, 3), cast_on("bolt", 4));
    next_turn(&mut s);
    act(&mut s, 2, p(0, 3), cast_on("bolt", 4));
    assert_eq!(uses(&s, 2, "bolt"), 0);
    next_turn(&mut s);
    refused_act(
        &mut s,
        2,
        p(0, 3),
        cast_on("bolt", 4),
        CommandError::NoUsesLeft {
            unit: UnitId(2),
            spell: sid("bolt"),
        },
    );
    // Still equipped, but it can't counter.
    act(&mut s, 2, p(0, 3), UnitAction::Wait);
    end(&mut s);
    let events = act(&mut s, 4, p(1, 3), attack(2));
    let forecast = combat(&events);
    assert_eq!(forecast.defender, None);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::SpellUsesChanged { .. }))
    );
    assert_eq!(equipped(&s, 2), Some(Equipped::Spell(sid("bolt"))));
}

#[test]
fn a_defender_that_falls_before_striking_spends_nothing() {
    let mut units = cast();
    units[1] = mage(2, Faction::Player, p(0, 3), &["bolt"]);
    units[3] = mage(4, Faction::Enemy, p(2, 3), &["bolt"]);
    units[3].hp = 3;
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 3), cast_on("bolt", 4));
    let spent: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::SpellUsesChanged { .. }))
        .collect();
    assert_eq!(spent, [&spell_used(2, "bolt", 1)]);
    assert!(events.contains(&Event::UnitFell { unit: UnitId(4) }));
    // Its uses fall with it, untouched.
    assert_eq!(s.fallen()[0].spells.uses_left(&sid("bolt")), 2);
}

#[test]
fn an_attacker_that_falls_to_the_counter_still_spends_its_use() {
    let mut units = cast();
    units[1] = mage(2, Faction::Player, p(0, 3), &["bolt"]);
    units[1].hp = 3;
    units[3] = mage(4, Faction::Enemy, p(2, 3), &["bolt"]);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 3), cast_on("bolt", 4));
    let tail = &events[events.len() - 3..];
    assert_eq!(
        tail,
        [
            spell_used(2, "bolt", 1),
            spell_used(4, "bolt", 1),
            Event::UnitFell { unit: UnitId(2) },
        ]
    );
    assert_eq!(uses(&s, 4, "bolt"), 1);
}

// ---- Equipping spells --------------------------------------------------------------

#[test]
fn equipping_a_spell_is_free() {
    let mut units = cast();
    units[1] = carrying(
        knowing(units[1].clone(), &["fire", "force"]),
        &[weapon(1, 1, 3)],
    );
    let mut s = start(setup(units));
    assert_eq!(
        s.apply(&equip_spell(2, "force")),
        Ok(vec![Event::Equipped {
            unit: UnitId(2),
            equipped: Equipped::Spell(sid("force")),
        }])
    );
    assert_eq!(equipped(&s, 2), Some(Equipped::Spell(sid("force"))));
    assert!(!s.unit(UnitId(2)).unwrap().acted);
    assert!(s.apply(&equip(2, 0)).is_ok());
    assert_eq!(equipped(&s, 2), Some(Equipped::Weapon(0)));
}

// ---- Healing -------------------------------------------------------------------

/// A healer (Mag 6, knows Heal) at (1,1), with allies 5 at (1,2) and 6 at
/// (2,1), an Ally-faction unit 7 at (0,1), a Neutral 8 at (1,0) and an
/// enemy 9 at (3,1).
fn clinic() -> BattleState {
    let mut healer = mage(2, Faction::Player, p(1, 1), &["heal"]);
    healer.stats.mag = 6;
    let wounded = |id, faction, pos, hp| Unit {
        stats: Stats::from_growable([25, 0, 0, 0, 0, 0, 0], 3),
        hp,
        ..unit(id, faction, pos)
    };
    start(setup(vec![
        lord(1, p(7, 4)),
        healer,
        wounded(5, Faction::Player, p(1, 2), 10),
        wounded(6, Faction::Player, p(2, 1), 25),
        wounded(7, Faction::Ally, p(0, 1), 20),
        wounded(8, Faction::Neutral, p(1, 0), 5),
        wounded(9, Faction::Enemy, p(3, 1), 5),
    ]))
}

/// Example M3 of `magic.md`.
#[test]
fn heal_reproduces_example_m3() {
    let mut s = clinic();
    let events = act(&mut s, 2, p(1, 1), cast_on("heal", 5));
    assert_eq!(
        events,
        [
            spell_cast(2, "heal", 5),
            Event::Healed {
                target: UnitId(5),
                amount: 15,
            },
            spell_used(2, "heal", 7),
            Event::UnitActed { unit: UnitId(2) },
        ]
    );
    assert_eq!(s.unit(UnitId(5)).unwrap().hp, 25);
    // An ally at 20/25 gets only the 5 missing HP (an Ally-faction unit counts).
    next_turn(&mut s);
    let events = act(&mut s, 2, p(1, 1), cast_on("heal", 7));
    assert_eq!(
        events[1],
        Event::Healed {
            target: UnitId(7),
            amount: 5,
        }
    );
    assert_eq!(s.unit(UnitId(7)).unwrap().hp, 25);
    assert_eq!(uses(&s, 2, "heal"), 6);
}

#[test]
fn heal_target_errors_leave_the_state_unchanged() {
    let mut s = clinic();
    let heal = |target| cast_on("heal", target);
    // A full-HP ally, the caster itself, a neutral and an enemy.
    refused_act(&mut s, 2, p(1, 1), heal(6), CommandError::FullHp(UnitId(6)));
    refused_act(
        &mut s,
        2,
        p(1, 1),
        heal(2),
        CommandError::BadHealTarget(UnitId(2)),
    );
    refused_act(
        &mut s,
        2,
        p(1, 1),
        heal(8),
        CommandError::BadHealTarget(UnitId(8)),
    );
    refused_act(
        &mut s,
        2,
        p(2, 2),
        heal(9),
        CommandError::BadHealTarget(UnitId(9)),
    );
    // Range 1, counted from `dest`.
    refused_act(
        &mut s,
        2,
        p(1, 3),
        heal(7),
        CommandError::OutOfRange {
            target: UnitId(7),
            distance: 3,
        },
    );
    refused_act(
        &mut s,
        2,
        p(1, 1),
        heal(99),
        CommandError::UnknownUnit(UnitId(99)),
    );
    // From (0,0) unit 7 at (0,1) is in range.
    act(&mut s, 2, p(0, 0), heal(7));
}

// ---- Errors -----------------------------------------------------------------------

#[test]
fn cast_and_equip_errors_leave_the_state_unchanged() {
    let mut units = cast();
    units[1] = mage(2, Faction::Player, p(0, 2), &["bolt", "heal", "ghost"]);
    units[3].pos = p(3, 2);
    let mut s = start(setup(units));
    let not_known = |spell: &str| CommandError::SpellNotKnown {
        unit: UnitId(2),
        spell: sid(spell),
    };
    refused_act(&mut s, 2, p(0, 2), cast_on("fire", 4), not_known("fire"));
    refused_act(
        &mut s,
        2,
        p(0, 2),
        cast_on("ghost", 4),
        CommandError::UnknownSpell(sid("ghost")),
    );
    refused_act(
        &mut s,
        2,
        p(0, 2),
        cast_on("bolt", 1),
        CommandError::NotHostile(UnitId(1)),
    );
    refused_act(
        &mut s,
        2,
        p(0, 2),
        cast_on("bolt", 4),
        CommandError::OutOfRange {
            target: UnitId(4),
            distance: 3,
        },
    );
    refused_act(
        &mut s,
        2,
        p(0, 2),
        cast_on("bolt", 3),
        CommandError::OutOfRange {
            target: UnitId(3),
            distance: 9,
        },
    );
    refused(&mut s, &equip_spell(2, "fire"), not_known("fire"));
    refused(
        &mut s,
        &equip_spell(2, "ghost"),
        CommandError::UnknownSpell(sid("ghost")),
    );
    refused(
        &mut s,
        &equip_spell(2, "heal"),
        CommandError::NotAttackSpell(sid("heal")),
    );
    refused(
        &mut s,
        &equip_spell(4, "bolt"),
        CommandError::NotItsPhase {
            unit: UnitId(4),
            phase: Phase::Player,
        },
    );
    // Tables missing after loading: the spell is unknown.
    let text = ron::to_string(&s).unwrap();
    let mut loaded: BattleState = ron::from_str(&text).unwrap();
    loaded.restore_tables(
        Arc::new(s.terrain().clone()),
        Arc::new(s.classes().clone()),
        Arc::new(s.items().clone()),
        Arc::new(SpellTable::default()),
    );
    refused_act(
        &mut loaded,
        2,
        p(1, 2),
        cast_on("bolt", 4),
        CommandError::UnknownSpell(sid("bolt")),
    );
    act(&mut s, 2, p(1, 2), cast_on("bolt", 4));
}

#[test]
fn spell_error_messages() {
    let cases = [
        (
            CommandError::SpellNotKnown {
                unit: UnitId(2),
                spell: SpellId::new("fire"),
            },
            "unit 2 doesn't know \"fire\"",
        ),
        (
            CommandError::UnknownSpell(SpellId::new("x")),
            "unknown spell \"x\"",
        ),
        (
            CommandError::NoUsesLeft {
                unit: UnitId(2),
                spell: SpellId::new("fire"),
            },
            "unit 2 has no uses of \"fire\" left",
        ),
        (
            CommandError::NotAttackSpell(SpellId::new("heal")),
            "\"heal\" isn't an attack spell",
        ),
        (
            CommandError::BadHealTarget(UnitId(3)),
            "unit 3 can't be healed by it",
        ),
        (CommandError::FullHp(UnitId(3)), "unit 3 is at full HP"),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
}

// ---- Worked examples ------------------------------------------------------------------

/// Example M1 of `magic.md`: Fire into a Frost Elemental (weak to fire),
/// which counters with Frost; played through [`BattleState::apply`].
#[test]
fn example_m1_through_apply() {
    let mut m = mage(2, Faction::Player, p(1, 2), &["fire"]);
    m.stats = Stats::from_growable([18, 1, 9, 6, 7, 2, 6], 3);
    m.hp = 18;
    let mut e = mage(4, Faction::Enemy, p(3, 2), &["frost"]);
    e.class = ClassId("frost_elemental".into());
    e.stats = Stats::from_growable([30, 0, 8, 4, 5, 8, 4], 3);
    e.hp = 30;
    let mut s = start(setup(vec![lord(1, p(0, 0)), m, e]));
    let events = act(&mut s, 2, p(1, 2), cast_on("fire", 4));
    let Some(Event::CombatResolved { forecast, .. }) = events.get(1) else {
        panic!("{events:?}");
    };
    let side = |damage, hit, crit, effective| SideForecast {
        damage,
        followup_damage: damage,
        hit,
        crit,
        strikes: 1,
        effective,
        broken: false,
        affinity: None,
    };
    assert_eq!(
        forecast.attacker,
        SideForecast {
            affinity: Some(Affinity::Weak),
            ..side(20, 92, 2, true)
        }
    );
    assert_eq!(forecast.defender, Some(side(6, 89, 1, false)));
    // Afterwards: mage Fire 9/10, elemental Frost 9/10 (20 damage doesn't
    // stop the elemental's counter).
    assert_eq!(uses(&s, 2, "fire"), 9);
    assert_eq!(uses(&s, 4, "frost"), 9);
    assert!(events.contains(&spell_used(2, "fire", 9)));
    assert!(events.contains(&spell_used(4, "frost", 9)));
}

/// Example M4 of `magic.md`: a tier-3 mage (0 weapon slots) with Fire 0/10
/// and Force 3/8, Fire equipped, is attacked at distance 2: no counter.
/// After equipping Force it counters with Force and spends a use.
#[test]
fn example_m4_no_uses_left() {
    let mut m = mage(2, Faction::Player, p(0, 2), &["fire", "force"]);
    m.class = ClassId("sage".into());
    let mut dummy = unit(4, Faction::Enemy, p(2, 2));
    // A target that can take 15 casts and can't strike back at range 2.
    dummy.stats.hp = 99;
    dummy.stats.res = 99;
    dummy.hp = 99;
    let archer = carrying(unit(5, Faction::Enemy, p(0, 4)), &[weapon(2, 2, 0)]);
    let mut s = start(setup(vec![lord(1, p(7, 4)), m, dummy, archer]));
    assert_eq!(equipped(&s, 2), Some(Equipped::Spell(sid("fire"))));
    for spell in ["force"; 5].into_iter().chain(["fire"; 10]) {
        act(&mut s, 2, p(0, 2), cast_on(spell, 4));
        next_turn(&mut s);
    }
    assert_eq!((uses(&s, 2, "fire"), uses(&s, 2, "force")), (0, 3));
    assert_eq!(equipped(&s, 2), Some(Equipped::Spell(sid("fire"))));
    act(&mut s, 2, p(0, 2), UnitAction::Wait);
    end(&mut s);
    let events = act(&mut s, 5, p(0, 4), attack(2));
    let forecast = combat(&events);
    assert_eq!(forecast.defender, None);
    next_turn(&mut s);
    assert!(s.apply(&equip_spell(2, "force")).is_ok());
    act(&mut s, 2, p(0, 2), UnitAction::Wait);
    end(&mut s);
    let events = act(&mut s, 5, p(0, 4), attack(2));
    let forecast = combat(&events);
    assert!(forecast.defender.is_some());
    assert!(events.contains(&spell_used(2, "force", 2)));
}
