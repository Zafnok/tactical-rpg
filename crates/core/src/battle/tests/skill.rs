//! Skills in battle (ticket 0311): combat actives, non-combat actives,
//! passives and auras in the forecast, timed effects, moves after an attack
//! and Shove. The test skills are [`skills`].

use super::*;
use crate::skill::{CostError, TimedEffect, TimedMods};
use crate::stats::StatKind;

fn sk(id: &str) -> SkillId {
    SkillId::new(id)
}

/// `u` in the class whose active is `skill`.
fn with_skill(u: Unit, skill: &str) -> Unit {
    Unit {
        class: skill_class(skill),
        ..u
    }
}

/// `u` having learned `skills` (passives).
fn learned(u: Unit, skills: &[&str]) -> Unit {
    Unit {
        learned_skills: skills.iter().map(|s| sk(s)).collect(),
        ..u
    }
}

/// `u` with only this weapon, at `durability`.
fn wielding(u: Unit, weapon: ItemId, durability: u32) -> Unit {
    let mut u = carrying(u, &[weapon]);
    if let Some(Some(w)) = u.loadout.weapons.first_mut() {
        w.durability_left = durability;
    }
    u
}

/// `u` knowing `spells`, with no weapons.
fn mage(u: Unit, spells: &[&str]) -> Unit {
    Unit {
        learned: spells.iter().map(|s| SpellId::new(s)).collect(),
        loadout: Loadout::default(),
        ..u
    }
}

fn attack_with(target: u32, active: &str) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
        active: Some(sk(active)),
        then_move: None,
    }
}

fn attack_then(target: u32, active: Option<&str>, to: Pos) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
        active: active.map(sk),
        then_move: Some(to),
    }
}

fn cast_with(spell: &str, target: u32, active: &str) -> UnitAction {
    UnitAction::Cast {
        spell: SpellId::new(spell),
        target: CastTarget::Unit(UnitId(target)),
        active: Some(sk(active)),
    }
}

fn use_skill(skill: &str, target: Option<u32>) -> UnitAction {
    UnitAction::UseSkill {
        skill: sk(skill),
        target: target.map(UnitId),
    }
}

/// The variant names of `events`, for checking their order.
fn names(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|e| {
            let text = format!("{e:?}");
            text.split([' ', '{', '(']).next().unwrap_or("").to_owned()
        })
        .collect()
}

/// The forecast of the combat in `events`.
fn combat(events: &[Event]) -> Forecast {
    events
        .iter()
        .find_map(|e| match e {
            Event::CombatResolved { forecast, .. } => Some(*forecast),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no combat in {events:?}"))
}

/// The forecast of unit `id` doing `action` from `dest`, without changing
/// `s`.
fn forecast_of(s: &BattleState, id: u32, dest: Pos, action: UnitAction) -> Forecast {
    let mut copy = s.clone();
    combat(&act(&mut copy, id, dest, action))
}

fn durability(s: &BattleState, id: u32) -> u32 {
    s.unit(UnitId(id))
        .and_then(|u| u.loadout.weapon(0))
        .map_or(0, |w| w.durability_left)
}

fn hp(s: &BattleState, id: u32) -> StatValue {
    s.unit(UnitId(id)).map_or(0, |u| u.hp)
}

fn applied(unit: u32, skill: &str, until: Phase) -> Event {
    Event::EffectApplied {
        unit: UnitId(unit),
        skill: sk(skill),
        until,
    }
}

fn spent(unit: u32, item: ItemId, amount: u32, left: u32) -> Event {
    Event::DurabilitySpent {
        unit: UnitId(unit),
        slot: 0,
        item,
        amount,
        left,
    }
}

fn used(unit: u32, skill: &str) -> Event {
    Event::SkillUsed {
        unit: UnitId(unit),
        skill: sk(skill),
    }
}

fn not_usable(unit: u32, skill: &str) -> CommandError {
    CommandError::SkillNotUsable {
        unit: UnitId(unit),
        skill: sk(skill),
    }
}

fn cannot_pay(skill: &str, error: CostError) -> CommandError {
    CommandError::CannotPay {
        skill: sk(skill),
        error,
    }
}

/// A hit-50 test sword, so hit bonuses show.
fn blade() -> ItemId {
    weapon_id(1, 1, 3, 50, 0)
}

// ---- Combat actives ---------------------------------------------------------

#[test]
fn keen_edge_raises_hit_and_crit_and_costs_the_attacking_weapon() {
    let mut s = start(setup(vec![
        with_skill(wielding(lord(1, p(0, 0)), blade(), 20), "keen"),
        unit(3, Faction::Enemy, p(2, 0)),
    ]));
    let plain = forecast_of(&s, 1, p(1, 0), attack(3));
    assert_eq!((plain.attacker.hit, plain.attacker.crit), (50, 0));
    let events = act(&mut s, 1, p(1, 0), attack_with(3, "keen"));
    let f = combat(&events);
    assert_eq!((f.attacker.hit, f.attacker.crit), (80, 10));
    assert_eq!(f.defender, plain.defender);
    assert_eq!(events[1], used(1, "keen"));
    assert_eq!(events[2], spent(1, blade(), 3, 17));
    assert_eq!(
        names(&events),
        [
            "UnitMoved",
            "SkillUsed",
            "DurabilitySpent",
            "CombatResolved",
            "WeaponExpGained",
            "WeaponExpGained",
            "UnitActed"
        ]
    );
    assert_eq!(durability(&s, 1), 17);
}

#[test]
fn an_active_is_refused_when_it_cant_be_used_or_paid() {
    let cast = vec![
        with_skill(wielding(lord(1, p(0, 0)), blade(), 2), "keen"),
        with_skill(
            wielding(unit(2, Faction::Player, p(0, 2)), blade(), 0),
            "keen",
        ),
        with_skill(unit(5, Faction::Player, p(0, 4)), "brace"),
        learned(unit(6, Faction::Player, p(4, 2)), &["focus"]),
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(2, 2)),
    ];
    let mut s = start(setup(cast));
    let refuse = |s: &mut BattleState, id, dest, action, err| refused_act(s, id, dest, action, err);
    refuse(
        &mut s,
        1,
        p(1, 0),
        attack_with(3, "keen"),
        cannot_pay("keen", CostError::NotEnoughDurability { left: 2, cost: 3 }),
    );
    refuse(
        &mut s,
        2,
        p(1, 2),
        attack_with(4, "keen"),
        cannot_pay("keen", CostError::WeaponBroken),
    );
    refuse(
        &mut s,
        1,
        p(1, 0),
        attack_with(3, "flurry"),
        not_usable(1, "flurry"),
    );
    refuse(
        &mut s,
        1,
        p(1, 0),
        attack_with(3, "nope"),
        CommandError::UnknownSkill(sk("nope")),
    );
    // A non-combat active isn't an attack option; a passive isn't an active.
    refuse(
        &mut s,
        5,
        p(2, 3),
        attack_with(4, "brace"),
        CommandError::WrongSkillKind(sk("brace")),
    );
    refuse(
        &mut s,
        6,
        p(3, 2),
        attack_with(4, "focus"),
        not_usable(6, "focus"),
    );
    // Only the active of its own class.
    refuse(
        &mut s,
        5,
        p(2, 3),
        attack_with(4, "keen"),
        not_usable(5, "keen"),
    );
}

#[test]
fn a_weapon_brought_to_zero_breaks_after_the_combat() {
    let mut s = start(setup(vec![
        with_skill(wielding(lord(1, p(0, 0)), weapon(1, 1, 3), 3), "keen"),
        Unit {
            hp: 3,
            ..unit(3, Faction::Enemy, p(2, 0))
        },
    ]));
    let events = act(&mut s, 1, p(1, 0), attack_with(3, "keen"));
    // The combat used the unbroken weapon.
    assert!(!combat(&events).attacker.broken);
    assert_eq!(combat(&events).attacker.damage, 3);
    assert_eq!(
        names(&events),
        [
            "UnitMoved",
            "SkillUsed",
            "DurabilitySpent",
            "CombatResolved",
            "WeaponExpGained",
            "ItemBroke",
            "UnitFell",
            "UnitActed",
            "BattleEnded"
        ]
    );
    assert_eq!(
        events[5],
        Event::ItemBroke {
            unit: UnitId(1),
            item: weapon(1, 1, 3),
        }
    );
    assert_eq!(durability(&s, 1), 0);
}

#[test]
fn flurry_adds_a_strike() {
    let s = start(setup(vec![
        with_skill(lord(1, p(0, 0)), "flurry"),
        unit(3, Faction::Enemy, p(2, 0)),
    ]));
    let f = forecast_of(&s, 1, p(1, 0), attack_with(3, "flurry"));
    assert_eq!(f.attacker.strikes, 2);
    assert_eq!(f.defender.map(|d| d.strikes), Some(1));
}

#[test]
fn long_shot_needs_a_bow_and_reaches_further() {
    let archer = |id, pos, skill| {
        with_skill(
            carrying(unit(id, Faction::Player, pos), &[item("flier_bow")]),
            skill,
        )
    };
    let mut s = start(setup(vec![
        Unit {
            is_lord: true,
            ..archer(1, p(0, 0), "long_shot")
        },
        archer(2, p(0, 4), "long_shot_2"),
        with_skill(unit(5, Faction::Player, p(7, 4)), "long_shot"),
        unit(3, Faction::Enemy, p(3, 0)),
        unit(4, Faction::Enemy, p(5, 4)),
    ]));
    refused_act(
        &mut s,
        1,
        p(0, 0),
        attack(3),
        CommandError::OutOfRange {
            target: UnitId(3),
            distance: 3,
        },
    );
    // Range 1–2, +2: 4 reaches, 5 doesn't.
    refused_act(
        &mut s,
        2,
        p(0, 4),
        attack_with(4, "long_shot_2"),
        CommandError::OutOfRange {
            target: UnitId(4),
            distance: 5,
        },
    );
    refused_act(
        &mut s,
        5,
        p(7, 4),
        attack_with(4, "long_shot"),
        CommandError::WrongWeaponForSkill(sk("long_shot")),
    );
    let events = act(&mut s, 1, p(0, 0), attack_with(3, "long_shot"));
    // The defender (range 1) can't counter at 3.
    assert_eq!(combat(&events).defender, None);
    let events = act(&mut s, 2, p(1, 4), attack_with(4, "long_shot_2"));
    assert_eq!(combat(&events).defender, None);
}

#[test]
fn a_stance_rider_counts_in_its_combat_and_lasts_until_the_next_own_phase() {
    let mut s = start(setup(vec![
        with_skill(lord(1, p(0, 0)), "guarding"),
        armed(unit(3, Faction::Enemy, p(2, 0)), 5),
    ]));
    let events = act(&mut s, 1, p(1, 0), attack_with(3, "guarding"));
    // The enemy's 5 might meets Def 3.
    assert_eq!(combat(&events).defender.map(|d| d.damage), Some(2));
    let at = names(&events).iter().position(|n| n == "EffectApplied");
    assert_eq!(
        at.map(|i| events[i].clone()),
        Some(applied(1, "guarding", Phase::Player))
    );
    assert!(at > names(&events).iter().position(|n| n == "CombatResolved"));
    assert_eq!(names(&events).last().map(String::as_str), Some("UnitActed"));
    end(&mut s);
    // Enemy phase: still on.
    let events = act(&mut s, 3, p(2, 0), attack(1));
    assert_eq!(combat(&events).attacker.damage, 2);
    assert_eq!(
        end(&mut s),
        [
            Event::EffectExpired {
                unit: UnitId(1),
                skill: sk("guarding"),
            },
            started(2, Phase::Player),
        ]
    );
    assert!(s.unit(UnitId(1)).is_some_and(|u| u.effects.is_empty()));
}

#[test]
fn a_stance_rider_is_not_applied_if_the_user_falls_and_counts_once() {
    let mut s = start(setup(vec![
        lord(1, p(0, 4)),
        Unit {
            hp: 1,
            ..with_skill(unit(2, Faction::Player, p(0, 0)), "guarding")
        },
        armed(unit(3, Faction::Enemy, p(2, 0)), 5),
    ]));
    let events = act(&mut s, 2, p(1, 0), attack_with(3, "guarding"));
    assert!(!names(&events).contains(&"EffectApplied".to_owned()));
    assert_eq!(s.unit(UnitId(2)), None);
    // A unit already under the same effect doesn't get it twice.
    let mut effect = TimedEffect {
        source: sk("guarding"),
        mods: TimedMods {
            stats: vec![(StatKind::Def, 3)],
            combat: CombatMods::default(),
        },
        until: Phase::Enemy,
    };
    let under = Unit {
        effects: vec![effect.clone()],
        ..with_skill(lord(1, p(0, 0)), "guarding")
    };
    let mut s = start(setup(vec![
        under,
        armed(unit(3, Faction::Enemy, p(2, 0)), 7),
    ]));
    let events = act(&mut s, 1, p(1, 0), attack_with(3, "guarding"));
    assert_eq!(combat(&events).defender.map(|d| d.damage), Some(4));
    effect.until = Phase::Player;
    assert_eq!(
        s.unit(UnitId(1)).map(|u| u.effects.clone()),
        Some(vec![effect])
    );
}

// ---- Moving after an attack --------------------------------------------------

#[test]
fn swoop_moves_one_tile_after_the_attack() {
    let mut s = start(setup(vec![
        with_skill(lord(1, p(0, 0)), "swoop"),
        with_skill(unit(2, Faction::Player, p(0, 3)), "swoop"),
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(2, 3)),
        unit(5, Faction::Enemy, p(1, 4)),
    ]));
    let err = |pos| CommandError::CannotMoveAfter(pos);
    let swoop = |to| attack_then(3, Some("swoop"), to);
    // Not without a skill, not 2 tiles, not onto a unit or the tile itself.
    refused_act(
        &mut s,
        1,
        p(1, 0),
        attack_then(3, None, p(1, 1)),
        err(p(1, 1)),
    );
    refused_act(&mut s, 1, p(1, 0), swoop(p(0, 1)), err(p(0, 1)));
    refused_act(&mut s, 1, p(1, 0), swoop(p(2, 0)), err(p(2, 0)));
    refused_act(&mut s, 1, p(1, 0), swoop(p(1, 0)), err(p(1, 0)));
    // Its old tile is free once it has moved.
    let events = act(&mut s, 1, p(1, 0), swoop(p(0, 0)));
    assert_eq!(
        events[events.len() - 2..],
        [
            Event::UnitMoved {
                unit: UnitId(1),
                path: vec![p(1, 0), p(0, 0)],
            },
            Event::UnitActed { unit: UnitId(1) },
        ]
    );
    assert_eq!(s.unit(UnitId(1)).map(|u| u.pos), Some(p(0, 0)));
    // Into a wall: refused.
    let mut walled = s.clone();
    if let Some(t) = walled.map.tiles.get_mut(p(1, 2)) {
        *t = TerrainId(2);
    }
    refused_act(
        &mut walled,
        2,
        p(1, 3),
        UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: Some(sk("swoop")),
            then_move: Some(p(1, 2)),
        },
        err(p(1, 2)),
    );
}

#[test]
fn no_move_after_the_attack_if_the_attacker_falls() {
    let mut s = start(setup(vec![
        lord(1, p(0, 4)),
        Unit {
            hp: 1,
            ..with_skill(unit(2, Faction::Player, p(0, 0)), "swoop")
        },
        unit(3, Faction::Enemy, p(2, 0)),
    ]));
    let events = act(&mut s, 2, p(1, 0), attack_then(3, Some("swoop"), p(1, 1)));
    assert_eq!(names(&events).last().map(String::as_str), Some("UnitFell"));
    assert!(!names(&events)[1..].contains(&"UnitMoved".to_owned()));
}

#[test]
fn skirmish_moves_after_a_bow_attack_only() {
    let archer = learned(
        carrying(lord(1, p(0, 0)), &[item("flier_bow"), weapon(1, 1, 3)]),
        &["skirmish"],
    );
    let mut s = start(setup(vec![archer, unit(3, Faction::Enemy, p(3, 0))]));
    refused_act(
        &mut s,
        1,
        p(2, 0),
        UnitAction::Attack {
            target: UnitId(3),
            slot: 1,
            active: None,
            then_move: Some(p(2, 1)),
        },
        CommandError::CannotMoveAfter(p(2, 1)),
    );
    let events = act(&mut s, 1, p(1, 0), attack_then(3, None, p(1, 1)));
    assert_eq!(s.unit(UnitId(1)).map(|u| u.pos), Some(p(1, 1)));
    assert!(names(&events).contains(&"UnitMoved".to_owned()));
}

// ---- Spell actives -------------------------------------------------------------

#[test]
fn overcast_costs_an_extra_spell_use_and_adds_might() {
    let mut s = start(setup(vec![
        with_skill(
            mage(lord(1, p(0, 0)), &["bolt", "heal", "fire"]),
            "overcast",
        ),
        unit(3, Faction::Enemy, p(2, 0)),
        Unit {
            hp: 5,
            ..unit(2, Faction::Player, p(0, 1))
        },
    ]));
    let plain = forecast_of(&s, 1, p(1, 0), cast_on("bolt", 3));
    let events = act(&mut s, 1, p(1, 0), cast_with("bolt", 3, "overcast"));
    assert_eq!(combat(&events).attacker.damage, plain.attacker.damage + 5);
    let uses = |left| Event::SpellUsesChanged {
        unit: UnitId(1),
        spell: SpellId::new("bolt"),
        uses_left: left,
    };
    assert_eq!(events[1..3], [used(1, "overcast"), uses(1)]);
    assert_eq!(
        names(&events),
        [
            "UnitMoved",
            "SkillUsed",
            "SpellUsesChanged",
            "SpellCast",
            "CombatResolved",
            "SpellUsesChanged",
            "WeaponExpGained",
            "UnitActed"
        ]
    );
    assert!(events.contains(&uses(0)));
}

fn cast_on(spell: &str, target: u32) -> UnitAction {
    UnitAction::Cast {
        spell: SpellId::new(spell),
        target: CastTarget::Unit(UnitId(target)),
        active: None,
    }
}

#[test]
fn spell_actives_need_two_uses_and_an_attack_spell() {
    let caster = carrying(
        with_skill(
            mage(lord(1, p(0, 0)), &["bolt", "heal", "fire"]),
            "overcast",
        ),
        &[weapon(1, 1, 3)],
    );
    let mut s = start(setup(vec![
        caster,
        Unit {
            hp: 5,
            ..unit(5, Faction::Player, p(0, 1))
        },
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(7, 4)),
    ]));
    if let Some(u) = s.unit_mut(UnitId(1)) {
        u.spells.uses_left.insert(SpellId::new("bolt"), 1);
    }
    refused_act(
        &mut s,
        1,
        p(1, 0),
        cast_with("bolt", 3, "overcast"),
        cannot_pay("overcast", CostError::NotEnoughUses { left: 1 }),
    );
    refused_act(
        &mut s,
        1,
        p(1, 0),
        attack_with(3, "overcast"),
        CommandError::WrongWeaponForSkill(sk("overcast")),
    );
    refused_act(
        &mut s,
        1,
        p(0, 0),
        cast_with("heal", 5, "overcast"),
        CommandError::WrongSkillKind(sk("overcast")),
    );
    refused_act(
        &mut s,
        1,
        p(1, 1),
        UnitAction::Cast {
            spell: SpellId::new("fire"),
            target: CastTarget::Tile(p(2, 2)),
            active: Some(sk("overcast")),
        },
        CommandError::WrongSkillKind(sk("overcast")),
    );
    // A durability active can't ride on a spell.
    let mut s = start(setup(vec![
        with_skill(mage(lord(1, p(0, 0)), &["bolt"]), "keen"),
        unit(3, Faction::Enemy, p(2, 0)),
    ]));
    refused_act(
        &mut s,
        1,
        p(1, 0),
        cast_with("bolt", 3, "keen"),
        CommandError::WrongWeaponForSkill(sk("keen")),
    );
}

#[test]
fn siphon_heals_half_the_hp_its_strikes_removed() {
    let caster = |hp, mag| {
        let mut u = with_skill(mage(lord(1, p(0, 0)), &["bolt"]), "siphon");
        u.hp = hp;
        u.stats.mag = mag;
        u
    };
    // Bolt: 3 might + Mag 3 = 6 damage; heals 3.
    let mut s = start(setup(vec![caster(4, 3), unit(3, Faction::Enemy, p(2, 0))]));
    let events = act(&mut s, 1, p(1, 0), cast_with("bolt", 3, "siphon"));
    let healed = Event::Healed {
        target: UnitId(1),
        amount: 3,
    };
    let at = events.iter().position(|e| *e == healed);
    assert!(at > names(&events).iter().rposition(|n| n == "SpellUsesChanged"));
    // 4 − 3 (the counter) + 3.
    assert_eq!(hp(&s, 1), 4);
    // Only the HP removed counts (target at 2 HP: heals 1), and never above
    // max HP.
    let low = Unit {
        hp: 2,
        ..unit(3, Faction::Enemy, p(2, 0))
    };
    let mut s = start(setup(vec![caster(4, 3), low]));
    let events = act(&mut s, 1, p(1, 0), cast_with("bolt", 3, "siphon"));
    assert!(events.contains(&Event::Healed {
        target: UnitId(1),
        amount: 1
    }));
    let mut s = start(setup(vec![caster(10, 3), unit(3, Faction::Enemy, p(3, 0))]));
    let events = act(&mut s, 1, p(1, 0), cast_with("bolt", 3, "siphon"));
    assert!(!names(&events).contains(&"Healed".to_owned()));
    assert_eq!(hp(&s, 1), 10);
    let mut s = start(setup(vec![caster(9, 3), unit(3, Faction::Enemy, p(3, 0))]));
    let events = act(&mut s, 1, p(1, 0), cast_with("bolt", 3, "siphon"));
    assert!(events.contains(&Event::Healed {
        target: UnitId(1),
        amount: 1
    }));
}

// ---- Non-combat actives ------------------------------------------------------

#[test]
fn brace_is_paid_from_the_equipped_weapon_and_lasts_until_the_next_own_phase() {
    let mut s = start(setup(vec![
        with_skill(
            carrying(lord(1, p(0, 0)), &[weapon(1, 1, 3), weapon(1, 1, 4)]),
            "brace",
        ),
        armed(unit(3, Faction::Enemy, p(2, 0)), 7),
    ]));
    s.apply(&Command::Equip {
        unit: UnitId(1),
        equipped: Equipped::Weapon(1),
    })
    .unwrap();
    refused_act(
        &mut s,
        1,
        p(1, 0),
        use_skill("brace", Some(3)),
        CommandError::BadSkillTarget(sk("brace")),
    );
    let events = act(&mut s, 1, p(1, 0), use_skill("brace", None));
    assert_eq!(
        events[1..],
        [
            used(1, "brace"),
            Event::DurabilitySpent {
                unit: UnitId(1),
                slot: 1,
                item: weapon(1, 1, 4),
                amount: 3,
                left: 17,
            },
            applied(1, "brace", Phase::Player),
            Event::UnitActed { unit: UnitId(1) },
        ]
    );
    assert_eq!(durability(&s, 1), 20);
    end(&mut s);
    let events = act(&mut s, 3, p(2, 0), attack(1));
    assert_eq!(combat(&events).attacker.damage, 2);
}

#[test]
fn non_combat_actives_need_an_equipped_weapon_and_are_actions() {
    let mut s = start(setup(vec![
        with_skill(mage(lord(1, p(0, 0)), &["bolt"]), "brace"),
        with_skill(unit(2, Faction::Player, p(0, 2)), "keen"),
        with_skill(
            wielding(unit(4, Faction::Player, p(0, 4)), weapon(1, 1, 3), 2),
            "brace",
        ),
        learned(unit(5, Faction::Player, p(7, 4)), &["focus"]),
        unit(3, Faction::Enemy, p(7, 0)),
    ]));
    refused_act(
        &mut s,
        1,
        p(0, 0),
        use_skill("brace", None),
        cannot_pay("brace", CostError::NoWeapon),
    );
    refused_act(
        &mut s,
        2,
        p(0, 2),
        use_skill("keen", None),
        CommandError::WrongSkillKind(sk("keen")),
    );
    refused_act(
        &mut s,
        4,
        p(0, 4),
        use_skill("brace", None),
        cannot_pay("brace", CostError::NotEnoughDurability { left: 2, cost: 3 }),
    );
    refused_act(
        &mut s,
        5,
        p(7, 4),
        use_skill("focus", None),
        not_usable(5, "focus"),
    );
    refused_act(
        &mut s,
        5,
        p(7, 4),
        use_skill("zap", None),
        CommandError::UnknownSkill(sk("zap")),
    );
}

#[test]
fn war_cry_buffs_adjacent_allies_only_and_refreshes() {
    let mut s = start(setup(vec![
        with_skill(lord(1, p(1, 1)), "war_cry"),
        with_skill(unit(2, Faction::Player, p(3, 1)), "war_cry"),
        unit(5, Faction::Player, p(2, 1)),
        unit(6, Faction::Ally, p(1, 2)),
        unit(3, Faction::Enemy, p(1, 0)),
        unit(4, Faction::Enemy, p(6, 4)),
        unit(7, Faction::Player, p(5, 4)),
    ]));
    let events = act(&mut s, 1, p(1, 1), use_skill("war_cry", None));
    assert_eq!(
        events[2..],
        [
            applied(5, "war_cry", Phase::Player),
            applied(6, "war_cry", Phase::Player),
            Event::UnitActed { unit: UnitId(1) },
        ]
    );
    act(&mut s, 2, p(3, 1), use_skill("war_cry", None));
    let effects = |s: &BattleState, id| s.unit(UnitId(id)).map_or(0, |u| u.effects.len());
    assert_eq!([1, 2, 5, 6, 3].map(|id| effects(&s, id)), [0, 0, 1, 1, 0]);
    // Str +2: 3 + 2 damage.
    let events = act(&mut s, 5, p(2, 0), attack(3));
    assert_eq!(combat(&events).attacker.damage, 5);
    // Nobody in reach: refused.
    refused_act(
        &mut s,
        7,
        p(5, 4),
        use_skill("war_cry", None),
        not_usable(7, "war_cry"),
    );
    let mut lone = start(setup(vec![
        with_skill(lord(1, p(0, 0)), "war_cry"),
        unit(3, Faction::Enemy, p(1, 0)),
    ]));
    refused_act(
        &mut lone,
        1,
        p(0, 0),
        use_skill("war_cry", None),
        CommandError::NoSkillTargets(sk("war_cry")),
    );
}

#[test]
fn inspire_gives_allies_within_two_tiles_hit_and_avoid() {
    let mut s = start(setup(vec![
        with_skill(lord(1, p(0, 0)), "inspire"),
        wielding(unit(2, Faction::Player, p(2, 0)), blade(), 20),
        unit(5, Faction::Player, p(0, 3)),
        wielding(unit(3, Faction::Enemy, p(4, 0)), blade(), 20),
    ]));
    let events = act(&mut s, 1, p(0, 0), use_skill("inspire", None));
    assert_eq!(
        names(&events)
            .iter()
            .filter(|n| *n == "EffectApplied")
            .count(),
        1
    );
    let events = act(&mut s, 2, p(3, 0), attack(3));
    let f = combat(&events);
    assert_eq!((f.attacker.hit, f.defender.map(|d| d.hit)), (60, Some(40)));
}

#[test]
fn sanctuary_heals_wounded_allies_in_reach() {
    let wounded = |id, faction, pos, hp| Unit {
        hp,
        ..unit(id, faction, pos)
    };
    let cleric = |skill| {
        let mut u = with_skill(lord(1, p(1, 1)), skill);
        u.stats.mag = 2;
        u.hp = 3;
        u
    };
    let others = || {
        vec![
            wounded(2, Faction::Player, p(1, 0), 4),
            wounded(5, Faction::Player, p(2, 1), 9),
            wounded(6, Faction::Ally, p(1, 2), 10),
            wounded(7, Faction::Player, p(3, 1), 1),
            wounded(3, Faction::Enemy, p(0, 1), 1),
            unit(4, Faction::Enemy, p(7, 4)),
        ]
    };
    let mut s = start(setup([vec![cleric("sanctuary")], others()].concat()));
    let events = act(&mut s, 1, p(1, 1), use_skill("sanctuary", None));
    // Mag 2 + 5 = 7: unit 2 gets 6 (to max), unit 5 gets 1; unit 6 is full,
    // unit 7 is out of reach, the enemy and the user are left out.
    assert_eq!(
        events[2..4],
        [
            Event::Healed {
                target: UnitId(2),
                amount: 6
            },
            Event::Healed {
                target: UnitId(5),
                amount: 1
            },
        ]
    );
    assert_eq!([1, 2, 5, 7, 3].map(|id| hp(&s, id)), [3, 10, 10, 1, 1]);
    let mut s = start(setup([vec![cleric("sanctuary_2")], others()].concat()));
    act(&mut s, 1, p(1, 1), use_skill("sanctuary_2", None));
    assert_eq!(hp(&s, 7), 8);
    // Nobody wounded in reach, or a target given: refused.
    let mut s = start(setup(vec![
        cleric("sanctuary"),
        unit(2, Faction::Player, p(1, 0)),
        unit(3, Faction::Enemy, p(7, 4)),
    ]));
    refused_act(
        &mut s,
        1,
        p(1, 1),
        use_skill("sanctuary", None),
        CommandError::NoSkillTargets(sk("sanctuary")),
    );
    refused_act(
        &mut s,
        1,
        p(1, 1),
        use_skill("sanctuary", Some(2)),
        CommandError::BadSkillTarget(sk("sanctuary")),
    );
}

// ---- Shove -------------------------------------------------------------------

fn shover(pos: Pos) -> Unit {
    with_skill(lord(1, pos), "shove")
}

#[test]
fn shove_pushes_an_adjacent_enemy_one_tile_away() {
    let mut s = start(setup(vec![
        shover(p(0, 0)),
        unit(3, Faction::Enemy, p(2, 1)),
    ]));
    let events = act(&mut s, 1, p(1, 1), use_skill("shove", Some(3)));
    assert_eq!(
        events,
        [
            Event::UnitMoved {
                unit: UnitId(1),
                path: vec![p(0, 0), p(1, 0), p(1, 1)],
            },
            used(1, "shove"),
            spent(1, weapon(1, 1, 3), 3, 17),
            Event::Pushed {
                unit: UnitId(3),
                from: p(2, 1),
                to: p(3, 1),
                collided: None,
                damage: 0,
            },
            Event::UnitActed { unit: UnitId(1) },
        ]
    );
    assert_eq!(
        s.unit(UnitId(3)).map(|u| (u.pos, u.hp)),
        Some((p(3, 1), 10))
    );
    // Upwards and leftwards too.
    for (from, target, to) in [(p(1, 4), p(1, 3), p(1, 2)), (p(3, 0), p(2, 0), p(1, 0))] {
        let mut s = start(setup(vec![shover(from), unit(3, Faction::Enemy, target)]));
        act(&mut s, 1, from, use_skill("shove", Some(3)));
        assert_eq!(s.unit(UnitId(3)).map(|u| u.pos), Some(to));
    }
}

/// Shover 1 (from (0,0), Mov `mov`) at `dest` shoves enemy 3 at
/// `target_pos` (with `hp`), with `other` also on the map: the events, and
/// enemy 3's tile and HP after.
fn shove_at(
    mov: StatValue,
    target_pos: Pos,
    hp: StatValue,
    other: Option<Unit>,
    dest: Pos,
) -> (Vec<Event>, Option<(Pos, StatValue)>, BattleState) {
    let mut at = shover(p(0, 0));
    at.stats.mov = mov;
    let mut units = vec![
        at,
        Unit {
            hp,
            ..unit(3, Faction::Enemy, target_pos)
        },
    ];
    units.extend(other);
    let mut s = start(setup(units));
    let events = act(&mut s, 1, dest, use_skill("shove", Some(3)));
    let after = s.unit(UnitId(3)).map(|u| (u.pos, u.hp));
    (events, after, s)
}

fn pushed(from: Pos, to: Pos, collided: Option<Pos>, damage: StatValue) -> Event {
    Event::Pushed {
        unit: UnitId(3),
        from,
        to,
        collided,
        damage,
    }
}

#[test]
fn a_blocked_shove_is_a_collision() {
    // Off the map: it stays and takes 5.
    let (events, after, _) = shove_at(3, p(0, 1), 10, None, p(1, 1));
    assert!(events.contains(&pushed(p(0, 1), p(0, 1), Some(p(-1, 1)), 5)));
    assert_eq!(after, Some((p(0, 1), 5)));
    // Into a unit: both are hurt, whoever it is.
    let hit = |unit: u32, damage| Event::CollisionDamage {
        unit: UnitId(unit),
        by: UnitId(3),
        damage,
    };
    let (events, after, s) = shove_at(
        3,
        p(2, 1),
        10,
        Some(unit(2, Faction::Player, p(3, 1))),
        p(1, 1),
    );
    let at = |e: &Event| events.iter().position(|x| x == e);
    let push = pushed(p(2, 1), p(2, 1), Some(p(3, 1)), 5);
    assert!(at(&push).is_some());
    assert_eq!(at(&hit(2, 5)), at(&push).map(|i| i + 1));
    assert_eq!(after, Some((p(2, 1), 5)));
    assert_eq!(hp(&s, 2), 5);
    // Collisions can kill both, the pushed unit falling first.
    let (events, after, s) = shove_at(
        3,
        p(2, 1),
        3,
        Some(Unit {
            hp: 2,
            ..unit(4, Faction::Enemy, p(3, 1))
        }),
        p(1, 1),
    );
    assert!(events.contains(&pushed(p(2, 1), p(2, 1), Some(p(3, 1)), 3)));
    assert!(events.contains(&hit(4, 2)));
    assert_eq!(after, None);
    let fell: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::UnitFell { .. }))
        .collect();
    assert_eq!(
        fell,
        [
            &Event::UnitFell { unit: UnitId(3) },
            &Event::UnitFell { unit: UnitId(4) }
        ]
    );
    assert_eq!(s.fallen().len(), 2);
    assert_eq!(s.outcome(), Some(Outcome::Victory));
    // Off the map, too.
    let (events, after, _) = shove_at(3, p(0, 1), 3, None, p(1, 1));
    assert!(events.contains(&pushed(p(0, 1), p(0, 1), Some(p(-1, 1)), 3)));
    assert_eq!(after, None);
    // The shover's own old tile is free once it has moved: no collision.
    let (events, after, _) = shove_at(5, p(1, 0), 10, None, p(2, 0));
    assert!(events.contains(&pushed(p(1, 0), p(0, 0), None, 0)));
    assert_eq!(after, Some((p(0, 0), 10)));
    // Into a wall.
    let mut s = start(setup(vec![
        shover(p(0, 0)),
        unit(3, Faction::Enemy, p(2, 1)),
    ]));
    if let Some(t) = s.map.tiles.get_mut(p(3, 1)) {
        *t = TerrainId(2);
    }
    let events = act(&mut s, 1, p(1, 1), use_skill("shove", Some(3)));
    assert!(events.contains(&pushed(p(2, 1), p(2, 1), Some(p(3, 1)), 5)));
}

#[test]
fn shove_is_refused_when_badly_aimed() {
    // Not adjacent, not hostile, no target, no such unit.
    let mut s = start(setup(vec![
        shover(p(0, 0)),
        unit(2, Faction::Player, p(1, 2)),
        unit(3, Faction::Enemy, p(3, 1)),
    ]));
    let bad = CommandError::BadSkillTarget(sk("shove"));
    refused_act(&mut s, 1, p(1, 1), use_skill("shove", Some(3)), bad.clone());
    refused_act(&mut s, 1, p(1, 1), use_skill("shove", Some(2)), bad.clone());
    refused_act(&mut s, 1, p(1, 1), use_skill("shove", None), bad);
    refused_act(
        &mut s,
        1,
        p(1, 1),
        use_skill("shove", Some(9)),
        CommandError::UnknownUnit(UnitId(9)),
    );
}

/// Sets `pos` burning, with a burn-out damage (9) unlike Shove's collision
/// (5).
fn set_burning(s: &mut BattleState, pos: Pos) {
    if let Some(t) = s.map.tiles.get_mut(pos) {
        *t = BURNING;
    }
    s.burning.push(Burning {
        pos,
        phase: Phase::Player,
        then: BURNT,
        damage: 9,
    });
}

#[test]
fn shove_into_a_burning_tile_burns_and_lands_next_to_it() {
    let mut s = start(setup(vec![
        shover(p(0, 0)),
        unit(3, Faction::Enemy, p(2, 1)),
        unit(4, Faction::Enemy, p(4, 1)),
    ]));
    set_burning(&mut s, p(3, 1));
    let events = act(&mut s, 1, p(1, 1), use_skill("shove", Some(3)));
    // Next to (3,1) in Dir order: (4,1) is taken, (3,2) is free.
    assert!(events.contains(&Event::Pushed {
        unit: UnitId(3),
        from: p(2, 1),
        to: p(3, 2),
        collided: Some(p(3, 1)),
        damage: 5,
    }));
    assert_eq!(s.unit(UnitId(3)).map(|u| (u.pos, u.hp)), Some((p(3, 2), 5)));
    // It may land back where it was, and the fire can kill.
    let mut s = start(setup(vec![
        shover(p(0, 0)),
        Unit {
            hp: 3,
            ..unit(3, Faction::Enemy, p(0, 3))
        },
        unit(4, Faction::Enemy, p(1, 4)),
    ]));
    set_burning(&mut s, p(0, 4));
    let events = act(&mut s, 1, p(0, 2), use_skill("shove", Some(3)));
    assert!(events.contains(&Event::Pushed {
        unit: UnitId(3),
        from: p(0, 3),
        to: p(0, 3),
        collided: Some(p(0, 4)),
        damage: 3,
    }));
    assert!(events.contains(&Event::UnitFell { unit: UnitId(3) }));
    assert_eq!(s.unit(UnitId(3)), None);
}

// ---- Passives and auras in the forecast ---------------------------------------

#[test]
fn weapon_and_hp_conditioned_passives() {
    let s = start(setup(vec![
        learned(lord(1, p(0, 0)), &["focus", "fury"]),
        learned(
            carrying(unit(2, Faction::Player, p(0, 4)), &[item("axe")]),
            &["focus"],
        ),
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(2, 4)),
    ]));
    let crit =
        |s: &BattleState, id, dest, target| forecast_of(s, id, dest, attack(target)).attacker.crit;
    assert_eq!(crit(&s, 1, p(1, 0), 3), 10);
    assert_eq!(crit(&s, 2, p(1, 4), 4), 0);
    let mut hurt = s.clone();
    if let Some(u) = hurt.unit_mut(UnitId(1)) {
        u.hp = 5;
    }
    assert_eq!(crit(&hurt, 1, p(1, 0), 3), 25);
    if let Some(u) = hurt.unit_mut(UnitId(1)) {
        u.hp = 6;
    }
    assert_eq!(crit(&hurt, 1, p(1, 0), 3), 10);
}

#[test]
fn steadfast_helps_only_outside_its_own_phase() {
    let mut s = start(setup(vec![
        learned(lord(1, p(0, 0)), &["steadfast"]),
        armed(learned(unit(3, Faction::Enemy, p(2, 0)), &["steadfast"]), 5),
    ]));
    let f = forecast_of(&s, 1, p(1, 0), attack(3));
    // The lord (own phase) takes 5; the enemy (Def +2) takes 1.
    assert_eq!(
        (f.attacker.damage, f.defender.map(|d| d.damage)),
        (1, Some(5))
    );
    end(&mut s);
    let f = forecast_of(&s, 3, p(1, 0), attack(1));
    assert_eq!(
        (f.attacker.damage, f.defender.map(|d| d.damage)),
        (3, Some(3))
    );
}

#[test]
fn charge_counts_tiles_moved_before_attacking() {
    let rider = |mov| {
        let mut u = learned(lord(1, p(0, 0)), &["charge"]);
        u.stats.mov = mov;
        u
    };
    let s = start(setup(vec![rider(6), unit(3, Faction::Enemy, p(5, 0))]));
    assert_eq!(forecast_of(&s, 1, p(4, 0), attack(3)).attacker.damage, 5);
    let s = start(setup(vec![rider(6), unit(3, Faction::Enemy, p(4, 0))]));
    assert_eq!(forecast_of(&s, 1, p(3, 0), attack(3)).attacker.damage, 3);
    // Counters never charge.
    let mut s = start(setup(vec![rider(6), unit(3, Faction::Enemy, p(1, 0))]));
    end(&mut s);
    let f = forecast_of(&s, 3, p(1, 0), attack(1));
    assert_eq!(f.defender.map(|d| d.damage), Some(3));
}

#[test]
fn sky_dodge_works_against_bows_only() {
    let s = start(setup(vec![
        carrying(lord(1, p(0, 0)), &[weapon_id(1, 2, 3, 50, 0)]),
        carrying(unit(2, Faction::Player, p(0, 4)), &[item("flier_bow")]),
        learned(unit(3, Faction::Enemy, p(2, 0)), &["sky_dodge"]),
        learned(unit(4, Faction::Enemy, p(2, 4)), &["sky_dodge"]),
    ]));
    assert_eq!(forecast_of(&s, 1, p(1, 0), attack(3)).attacker.hit, 50);
    assert_eq!(forecast_of(&s, 2, p(1, 4), attack(4)).attacker.hit, 90);
}

#[test]
fn black_and_white_magic_boost_spells_only() {
    let mut s = start(setup(vec![
        learned(
            mage(lord(1, p(0, 0)), &["bolt", "heal"]),
            &["black_magic", "white_magic_2"],
        ),
        learned(unit(2, Faction::Player, p(0, 4)), &["black_magic"]),
        Unit {
            hp: 1,
            ..unit(5, Faction::Player, p(0, 1))
        },
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(2, 4)),
    ]));
    assert_eq!(
        forecast_of(&s, 1, p(1, 0), cast_on("bolt", 3))
            .attacker
            .damage,
        4
    );
    assert_eq!(forecast_of(&s, 2, p(1, 4), attack(4)).attacker.damage, 3);
    // Heal 10 + Mag 0 + 4, capped at the 9 missing.
    let events = act(&mut s, 1, p(0, 0), cast_on("heal", 5));
    assert!(events.contains(&Event::Healed {
        target: UnitId(5),
        amount: 9
    }));
    // A bigger wound shows the whole bonus.
    let mut s = start(setup(vec![
        learned(mage(lord(1, p(0, 0)), &["heal"]), &["white_magic_1"]),
        Unit {
            hp: 1,
            stats: Stats::from_growable([30, 0, 0, 0, 0, 0, 0], 3),
            ..unit(5, Faction::Player, p(0, 1))
        },
        unit(3, Faction::Enemy, p(7, 4)),
    ]));
    let events = act(&mut s, 1, p(0, 0), cast_on("heal", 5));
    assert!(events.contains(&Event::Healed {
        target: UnitId(5),
        amount: 12
    }));
}

#[test]
fn leadership_reaches_other_allies_within_two_tiles_once() {
    let armed_at = |id, faction, pos| wielding(unit(id, faction, pos), blade(), 20);
    let mut s = start(setup(vec![
        learned(wielding(lord(1, p(0, 0)), blade(), 20), &["leadership"]),
        armed_at(2, Faction::Player, p(0, 2)),
        armed_at(5, Faction::Player, p(0, 3)),
        armed_at(6, Faction::Ally, p(2, 0)),
        armed_at(10, Faction::Player, p(5, 2)),
        armed_at(3, Faction::Enemy, p(1, 2)),
        armed_at(4, Faction::Enemy, p(1, 3)),
        learned(armed_at(7, Faction::Enemy, p(7, 0)), &["leadership"]),
        armed_at(8, Faction::Enemy, p(6, 1)),
        armed_at(9, Faction::Enemy, p(6, 3)),
    ]));
    let hits = |s: &BattleState, id, dest, target| {
        let f = forecast_of(s, id, dest, attack(target));
        (f.attacker.hit, f.defender.map_or(0, |d| d.hit))
    };
    // Unit 2 is 2 tiles from the lord, unit 5 is 3; the enemies don't get it.
    assert_eq!(hits(&s, 2, p(0, 2), 3), (60, 50));
    assert_eq!(hits(&s, 5, p(0, 3), 4), (50, 50));
    // Not the lord itself.
    assert_eq!(hits(&s, 1, p(1, 1), 3), (50, 50));
    // The enemy leader reaches unit 8 (2 tiles), not unit 9 (4 tiles).
    assert_eq!(hits(&s, 10, p(5, 1), 8), (50, 60));
    assert_eq!(hits(&s, 10, p(5, 3), 9), (50, 50));
    // Two sources of the same aura count once.
    let mut two = s.clone();
    if let Some(u) = two.unit_mut(UnitId(5)) {
        u.learned_skills.insert(sk("leadership"));
    }
    assert_eq!(hits(&two, 2, p(0, 2), 3), (60, 50));
    // Green allies get it too.
    end(&mut s);
    end(&mut s);
    assert_eq!(s.phase(), Phase::Other);
    assert_eq!(hits(&s, 6, p(1, 1), 3), (60, 50));
}

// ---- Timed effects -------------------------------------------------------------

#[test]
fn effects_last_until_their_users_next_phase_whoever_has_them() {
    let mut s = start(setup(vec![
        with_skill(lord(1, p(1, 1)), "war_cry"),
        unit(5, Faction::Player, p(2, 1)),
        unit(6, Faction::Ally, p(1, 2)),
        unit(3, Faction::Enemy, p(7, 4)),
    ]));
    act(&mut s, 1, p(1, 1), use_skill("war_cry", None));
    let effects =
        |s: &BattleState| [5, 6].map(|id| s.unit(UnitId(id)).map_or(0, |u| u.effects.len()));
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    assert_eq!(effects(&s), [1, 1]);
    // The green ally keeps it through its own phase…
    assert_eq!(end(&mut s), [started(1, Phase::Other)]);
    assert_eq!(effects(&s), [1, 1]);
    // …and loses it when the user's side's phase starts.
    let expired = |id| Event::EffectExpired {
        unit: UnitId(id),
        skill: sk("war_cry"),
    };
    assert_eq!(
        end(&mut s),
        [expired(5), expired(6), started(2, Phase::Player)]
    );
    assert_eq!(effects(&s), [0, 0]);
}

#[test]
fn skill_error_messages() {
    let cases = [
        (CommandError::UnknownSkill(sk("x")), "unknown skill \"x\""),
        (not_usable(4, "x"), "unit 4 can't use \"x\" now"),
        (
            CommandError::WrongSkillKind(sk("x")),
            "\"x\" can't be used that way",
        ),
        (
            CommandError::WrongWeaponForSkill(sk("x")),
            "\"x\" can't be used with this attack",
        ),
        (
            cannot_pay("x", CostError::WeaponBroken),
            "can't pay for \"x\": the weapon is broken",
        ),
        (
            CommandError::BadSkillTarget(sk("x")),
            "\"x\" can't target that",
        ),
        (
            CommandError::NoSkillTargets(sk("x")),
            "\"x\" would reach nobody",
        ),
        (
            CommandError::CannotMoveAfter(p(1, 2)),
            "can't move to (1, 2) after attacking",
        ),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
}

#[test]
fn spell_might_counts_only_for_a_spell_counter() {
    // Black Magic on the defender: its Bolt counter gets +1, its sword
    // counter doesn't.
    let caster = |spell: bool| {
        let u = learned(unit(3, Faction::Enemy, p(1, 0)), &["black_magic"]);
        if spell {
            Unit {
                loadout: Loadout {
                    equipped: Some(Equipped::Spell(SpellId::new("bolt"))),
                    ..Loadout::default()
                },
                learned: [SpellId::new("bolt")].into(),
                ..u
            }
        } else {
            u
        }
    };
    let counter = |spell| {
        let s = start(setup(vec![lord(1, p(0, 0)), caster(spell)]));
        forecast_of(&s, 1, p(0, 0), attack(3))
            .defender
            .map(|d| d.damage)
    };
    assert_eq!((counter(false), counter(true)), (Some(3), Some(4)));
}
