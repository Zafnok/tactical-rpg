//! Tests of the skill rules: conditions, bonuses, costs, learning and the
//! usable skill list.

use super::*;
use crate::art::ArtId;
use crate::class::{ClassDef, ClassId, UnitTags};
use crate::geom::Pos;
use crate::item::WeaponInstance;
use crate::item::{Equipped, ItemId};
use crate::stats::Growths;
use crate::terrain::MovementTypeId;
use crate::unit::{ClassRecord, Faction, UnitId};

fn sid(id: &str) -> SkillId {
    SkillId::new(id)
}

fn passive(id: &str, family: &str, rank: u8, effects: Vec<PassiveEffect>) -> SkillDef {
    SkillDef {
        id: sid(id),
        name: id.into(),
        family: family.into(),
        rank,
        kind: SkillKind::Passive(effects),
    }
}

fn active(id: &str, family: &str, rank: u8, effect: ActiveEffect) -> SkillDef {
    SkillDef {
        id: sid(id),
        name: id.into(),
        family: family.into(),
        rank,
        kind: SkillKind::Active {
            cost: SkillCost::Durability(3),
            effect,
        },
    }
}

/// `def` costing `uses` uses per battle.
fn with_uses(def: SkillDef, uses: u8) -> SkillDef {
    let SkillKind::Active { effect, .. } = def.kind else {
        return def;
    };
    SkillDef {
        kind: SkillKind::Active {
            cost: SkillCost::Uses(uses),
            effect,
        },
        ..def
    }
}

fn strike(mods: CombatMods) -> ActiveEffect {
    ActiveEffect::Strike {
        with: WeaponReq::Any,
        mods,
        range: 0,
        stance: None,
        post_move: 0,
        drain: false,
    }
}

fn hit(n: StatValue) -> CombatMods {
    CombatMods {
        hit: n,
        ..CombatMods::default()
    }
}

/// White Magic 1/2, Long Shot 1/2, Keen Edge, Brace (3 uses per battle),
/// Rally 1/2 (1 / 2 uses) and Swoop.
fn table() -> SkillTable {
    let own = || ActiveEffect::Buff {
        area: Area::Own,
        mods: TimedMods::default(),
    };
    let all = [
        passive(
            "white_magic_1",
            "white_magic",
            1,
            vec![PassiveEffect::HealBonus(2)],
        ),
        passive(
            "white_magic_2",
            "white_magic",
            2,
            vec![PassiveEffect::HealBonus(4)],
        ),
        passive("evasion", "evasion", 1, vec![]),
        active("long_shot", "long_shot", 1, strike(hit(1))),
        active("long_shot_2", "long_shot", 2, strike(hit(2))),
        active("keen_edge", "keen_edge", 1, strike(hit(30))),
        with_uses(active("brace", "brace", 1, own()), 3),
        with_uses(active("rally", "rally", 1, own()), 1),
        with_uses(active("rally_2", "rally", 2, own()), 2),
        active("swoop", "swoop", 1, strike(CombatMods::default())),
    ];
    SkillTable {
        skills: all.into_iter().map(|s| (s.id.clone(), s)).collect(),
    }
}

fn class(id: &str, active: Option<&str>) -> ClassDef {
    ClassDef {
        id: ClassId(id.into()),
        name: id.into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 5,
        base: Stats::default(),
        growths: Growths::default(),
        weapons: vec![],
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: active.map(sid),
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    }
}

/// `archer` and `ranger` (Long Shot), `marksman` (Long Shot 2), `swordsman` (Keen
/// Edge), `guard` (Brace), `exile` (Rally), `commander` (Rally 2), `plain`
/// (no active), `ghost` (an active missing from the skill table). Mastery
/// at class level 10.
fn classes() -> ClassTable {
    let all = [
        class("archer", Some("long_shot")),
        class("ranger", Some("long_shot")),
        class("marksman", Some("long_shot_2")),
        class("swordsman", Some("keen_edge")),
        class("guard", Some("brace")),
        class("exile", Some("rally")),
        class("commander", Some("rally_2")),
        class("plain", None),
        class("ghost", Some("missing")),
    ];
    ClassTable {
        classes: all.into_iter().map(|c| (c.id.clone(), c)).collect(),
        class_level_cap: 10,
        ..ClassTable::default()
    }
}

fn unit_in(class: &str) -> Unit {
    let classes = classes();
    Unit::generic(
        UnitId(1),
        &ClassId(class.into()),
        &classes,
        1,
        Faction::Player,
        Pos::new(0, 0),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Records `level` in `class`.
fn record(u: &mut Unit, class: &str, level: u8) {
    u.class_records.insert(
        ClassId(class.into()),
        ClassRecord {
            class_level: level,
            class_points: 0,
        },
    );
}

fn usable(u: &Unit) -> Vec<String> {
    let skills = table();
    u.usable_skills(&classes(), &skills)
        .into_iter()
        .map(|s| s.id.0.clone())
        .collect()
}

// ---- Conditions ------------------------------------------------------------

fn ctx() -> SkillContext {
    SkillContext {
        weapon: Some(WeaponKind::Sword),
        spell: false,
        opponent_weapon: Some(WeaponKind::Bow),
        own_phase: true,
        hp: 10,
        max_hp: 20,
        moved: 4,
    }
}

#[test]
fn conditions() {
    let c = ctx();
    assert!(Condition::Always.holds(&SkillContext::default()));
    assert!(Condition::WeaponKindEquipped(WeaponKind::Sword).holds(&c));
    assert!(!Condition::WeaponKindEquipped(WeaponKind::Bow).holds(&c));
    assert!(Condition::AgainstWeaponKind(WeaponKind::Bow).holds(&c));
    assert!(!Condition::AgainstWeaponKind(WeaponKind::Sword).holds(&c));
    assert!(!Condition::NotOwnPhase.holds(&c));
    assert!(Condition::NotOwnPhase.holds(&SkillContext {
        own_phase: false,
        ..c
    }));
    assert!(Condition::MovedAtLeast(4).holds(&c));
    assert!(!Condition::MovedAtLeast(5).holds(&c));
    // HP ≤ half: 10 of 20 and 10 of 21 are, 11 of 21 isn't.
    assert!(Condition::HpAtMostHalf.holds(&c));
    let at = |hp, max_hp| Condition::HpAtMostHalf.holds(&SkillContext { hp, max_hp, ..c });
    assert!(at(10, 21));
    assert!(!at(11, 21));
    assert!(!at(11, 20));
    assert!(!at(StatValue::MAX, 20));
}

#[test]
fn weapon_requirements() {
    use WeaponKind::{Bow, Sword};
    assert!(WeaponReq::Any.allows(Some(Sword), false));
    assert!(!WeaponReq::Any.allows(None, true));
    assert!(!WeaponReq::Any.allows(None, false));
    assert!(!WeaponReq::Any.allows(Some(Sword), true));
    assert!(WeaponReq::Kind(Bow).allows(Some(Bow), false));
    assert!(!WeaponReq::Kind(Bow).allows(Some(Sword), false));
    assert!(!WeaponReq::Kind(Bow).allows(Some(Bow), true));
    assert!(WeaponReq::Spell.allows(None, true));
    assert!(!WeaponReq::Spell.allows(Some(Sword), false));
}

#[test]
fn skill_kinds() {
    let t = table();
    let get = |id: &str| t.get(&sid(id)).unwrap_or_else(|| panic!("{id}"));
    assert!(get("keen_edge").is_active() && get("keen_edge").is_combat());
    assert!(get("brace").is_active() && !get("brace").is_combat());
    assert!(!get("white_magic_1").is_active() && !get("white_magic_1").is_combat());
    assert_eq!(
        get("white_magic_1").passive_effects(),
        [PassiveEffect::HealBonus(2)]
    );
    assert!(get("keen_edge").passive_effects().is_empty());
    assert!(t.get(&sid("nope")).is_none());
}

// ---- Bonuses ---------------------------------------------------------------

fn stat(kind: StatKind, amount: StatValue) -> Stats {
    let mut s = Stats::default();
    s.set(kind, amount);
    s
}

#[test]
fn bonuses_add_and_apply() {
    let mut b = Bonuses::default();
    b.add_timed(&TimedMods {
        stats: vec![(StatKind::Def, 5), (StatKind::Res, 5), (StatKind::Def, 1)],
        combat: hit(10),
    });
    assert_eq!(b.stats, Stats::from_growable([0, 0, 0, 0, 0, 6, 5], 0));
    assert_eq!(b.combat, hit(10));
    b.add(&Bonuses {
        stats: Stats::from_growable([1, 2, 3, 4, 5, 6, 7], 8),
        combat: hit(1),
    });
    assert_eq!(b.stats, Stats::from_growable([1, 2, 3, 4, 5, 12, 12], 8));
    assert_eq!(b.combat, hit(11));
    let base = Stats::from_growable([10, 10, 10, 10, 10, 10, 10], 5);
    assert_eq!(
        b.apply(base),
        Stats::from_growable([11, 12, 13, 14, 15, 22, 22], 13)
    );
}

#[test]
fn passive_bonuses_follow_their_conditions() {
    let sword = Condition::WeaponKindEquipped(WeaponKind::Sword);
    let axe = Condition::WeaponKindEquipped(WeaponKind::Axe);
    let skills = [
        passive(
            "a",
            "a",
            1,
            vec![
                PassiveEffect::StatWhile {
                    stat: StatKind::Def,
                    amount: 2,
                    when: sword,
                },
                PassiveEffect::StatWhile {
                    stat: StatKind::Spd,
                    amount: 3,
                    when: axe,
                },
                PassiveEffect::CombatMod {
                    mods: hit(10),
                    when: sword,
                },
                PassiveEffect::CombatMod {
                    mods: hit(100),
                    when: axe,
                },
            ],
        ),
        passive(
            "b",
            "b",
            1,
            vec![
                PassiveEffect::SpellMight(3),
                PassiveEffect::HealBonus(9),
                PassiveEffect::PostActionMove {
                    tiles: 1,
                    when: Condition::Always,
                },
                PassiveEffect::AllyAura {
                    radius: 2,
                    mods: hit(50),
                },
            ],
        ),
    ];
    let refs: Vec<&SkillDef> = skills.iter().collect();
    let b = passive_bonuses(&refs, &ctx());
    assert_eq!(b.stats, stat(StatKind::Def, 2));
    assert_eq!(b.combat, hit(10));
    let spell = SkillContext {
        weapon: None,
        spell: true,
        ..ctx()
    };
    let b = passive_bonuses(&refs, &spell);
    assert_eq!(b.stats, Stats::default());
    assert_eq!(
        b.combat,
        CombatMods {
            might: 3,
            ..CombatMods::default()
        }
    );
}

#[test]
fn effect_bonuses_add_every_effect() {
    let effect = |source: &str, def| TimedEffect {
        source: EffectSource::Skill(sid(source)),
        mods: TimedMods {
            stats: vec![(StatKind::Def, def)],
            combat: hit(1),
        },
        until: Phase::Player,
    };
    let b = effect_bonuses(&[effect("brace", 5), effect("rally", 3)]);
    assert_eq!(b.stats, stat(StatKind::Def, 8));
    assert_eq!(b.combat, hit(2));
    assert_eq!(effect_bonuses(&[]), Bonuses::default());
}

#[test]
fn heal_bonus_post_move_and_auras() {
    let bow = Condition::WeaponKindEquipped(WeaponKind::Bow);
    let skills = [
        passive("w", "w", 1, vec![PassiveEffect::HealBonus(2)]),
        passive(
            "x",
            "x",
            1,
            vec![
                PassiveEffect::HealBonus(4),
                PassiveEffect::SpellMight(7),
                PassiveEffect::PostActionMove {
                    tiles: 1,
                    when: bow,
                },
            ],
        ),
        passive(
            "y",
            "y",
            1,
            vec![
                PassiveEffect::PostActionMove {
                    tiles: 3,
                    when: Condition::MovedAtLeast(9),
                },
                PassiveEffect::PostActionMove {
                    tiles: 2,
                    when: Condition::Always,
                },
                PassiveEffect::AllyAura {
                    radius: 2,
                    mods: hit(10),
                },
            ],
        ),
        active("z", "z", 1, strike(hit(1))),
    ];
    let refs: Vec<&SkillDef> = skills.iter().collect();
    assert_eq!(heal_bonus(&refs), 6);
    assert_eq!(heal_bonus(&[]), 0);
    let with_bow = SkillContext {
        weapon: Some(WeaponKind::Bow),
        ..ctx()
    };
    assert_eq!(post_move_tiles(&refs[..2], &with_bow), 1);
    assert_eq!(post_move_tiles(&refs[..2], &ctx()), 0);
    assert_eq!(post_move_tiles(&refs, &ctx()), 2);
    assert_eq!(post_move_tiles(&[], &ctx()), 0);
    let found: Vec<(&SkillId, u32, &CombatMods)> = auras(&refs).collect();
    assert_eq!(found, [(&sid("y"), 2, &hit(10))]);
}

// ---- Costs -----------------------------------------------------------------

fn armed(durability: u32) -> Unit {
    let mut u = unit_in("plain");
    u.loadout.weapons[0] = Some(WeaponInstance {
        def: ItemId::new("iron"),
        durability_left: durability,
    });
    u.loadout.equipped = Some(Equipped::Weapon(0));
    u.spells.uses_left.insert(SpellId::new("fire"), 2);
    u.spells.uses_left.insert(SpellId::new("frost"), 1);
    u
}

#[test]
fn check_cost_rules() {
    let weapon = CostSource::Weapon(0);
    let fire = CostSource::Spell(SpellId::new("fire"));
    let frost = CostSource::Spell(SpellId::new("frost"));
    let dur = SkillCost::Durability;
    let u = armed(5);
    assert_eq!(check_cost(&u, dur(5), &weapon), Ok(()));
    assert_eq!(check_cost(&u, dur(1), &weapon), Ok(()));
    // More than is left is fine: the rest is spent (Nick, 0414).
    assert_eq!(check_cost(&u, dur(6), &weapon), Ok(()));
    assert_eq!(
        check_cost(&u, dur(1), &CostSource::Weapon(1)),
        Err(CostError::NoWeapon)
    );
    assert_eq!(
        check_cost(&armed(0), dur(0), &weapon),
        Err(CostError::WeaponBroken)
    );
    assert_eq!(check_cost(&u, dur(1), &fire), Err(CostError::WrongSource));
    assert_eq!(check_cost(&u, SkillCost::ExtraSpellUse, &fire), Ok(()));
    assert_eq!(
        check_cost(&u, SkillCost::ExtraSpellUse, &frost),
        Err(CostError::NotEnoughUses { left: 1 })
    );
    assert_eq!(
        check_cost(
            &u,
            SkillCost::ExtraSpellUse,
            &CostSource::Spell(SpellId::new("heal"))
        ),
        Err(CostError::NotEnoughUses { left: 0 })
    );
    assert_eq!(
        check_cost(&u, SkillCost::ExtraSpellUse, &weapon),
        Err(CostError::WrongSource)
    );
}

fn own(skill: &str) -> CostSource {
    CostSource::Own(sid(skill))
}

#[test]
fn a_cost_in_uses_needs_a_use_left_and_nothing_else() {
    let uses = SkillCost::Uses(3);
    // No weapon at all, and one broken: only the uses count.
    let mut bare = unit_in("plain");
    bare.skill_uses.uses_left.insert(sid("brace"), 1);
    bare.skill_uses.uses_left.insert(sid("shove"), 0);
    let mut broken = armed(0);
    broken.skill_uses = bare.skill_uses.clone();
    for u in [&bare, &broken] {
        assert_eq!(check_cost(u, uses, &own("brace")), Ok(()));
        assert_eq!(
            check_cost(u, uses, &own("shove")),
            Err(CostError::NoUsesLeft)
        );
        // A skill missing from the unit's uses has none.
        assert_eq!(
            check_cost(u, uses, &own("rally")),
            Err(CostError::NoUsesLeft)
        );
    }
    // Uses are never paid from a weapon or a spell, nor anything else from
    // a skill's uses.
    let u = armed(5);
    let wrong = Err(CostError::WrongSource);
    assert_eq!(check_cost(&u, uses, &CostSource::Weapon(0)), wrong);
    let fire = CostSource::Spell(SpellId::new("fire"));
    assert_eq!(check_cost(&u, uses, &fire), wrong);
    assert_eq!(
        check_cost(&bare, SkillCost::Durability(1), &own("brace")),
        wrong
    );
    assert_eq!(
        check_cost(&bare, SkillCost::ExtraSpellUse, &own("brace")),
        wrong
    );
}

#[test]
fn pay_cost_spends_one_use_and_no_durability() {
    let mut u = armed(5);
    u.skill_uses.uses_left.insert(sid("brace"), 2);
    let paid = |u: &mut Unit| pay_cost(u, SkillCost::Uses(3), &own("brace"));
    let changed = |left| {
        Ok(Paid {
            events: vec![Event::SkillUsesChanged {
                unit: UnitId(1),
                skill: sid("brace"),
                uses_left: left,
            }],
            broke: None,
        })
    };
    assert_eq!(paid(&mut u), changed(1));
    assert_eq!(paid(&mut u), changed(0));
    assert_eq!(u.skill_uses.uses_left(&sid("brace")), 0);
    assert_eq!(u.loadout.weapon(0).map(|w| w.durability_left), Some(5));
    // Refused: nothing changes.
    let before = u.clone();
    assert_eq!(paid(&mut u), Err(CostError::NoUsesLeft));
    assert_eq!(u, before);
}

#[test]
fn skill_uses_fill_spend_and_never_go_below_zero() {
    let t = table();
    let get = |id: &str| t.get(&sid(id)).unwrap_or_else(|| panic!("{id}"));
    assert_eq!(get("brace").uses_per_battle(), Some(3));
    assert_eq!(get("keen_edge").uses_per_battle(), None);
    assert_eq!(get("white_magic_1").uses_per_battle(), None);
    // Only the skills that cost uses are counted.
    let mut uses = SkillUses::full(&[get("brace"), get("keen_edge"), get("white_magic_1")]);
    assert_eq!(uses.uses_left, [(sid("brace"), 3)].into());
    assert_eq!(uses.uses_left(&sid("keen_edge")), 0);
    assert_eq!(uses.spend(&sid("keen_edge")), None);
    assert_eq!(
        [(); 4].map(|()| uses.spend(&sid("brace"))),
        [Some(2), Some(1), Some(0), None]
    );
    assert_eq!(uses.uses_left(&sid("brace")), 0);
    assert_eq!(SkillUses::full(&[]), SkillUses::default());
}

#[test]
fn preparing_for_battle_refills_the_uses_of_the_usable_actives() {
    let (classes, skills) = (classes(), table());
    let prepare = |u: &mut Unit| {
        u.prepare_for_battle(
            &classes,
            &crate::item::ItemTable::default(),
            &crate::spell::SpellTable::default(),
            &skills,
        );
    };
    let left = |u: &Unit| -> Vec<(String, u8)> {
        let uses = u.skill_uses.uses_left.iter();
        uses.map(|(id, n)| (id.0.clone(), *n)).collect()
    };
    // A Guard: 3 Braces, whatever it had left.
    let mut u = unit_in("guard");
    assert_eq!(left(&u), []);
    u.skill_uses.uses_left.insert(sid("brace"), 0);
    u.skill_uses.uses_left.insert(sid("gone"), 5);
    prepare(&mut u);
    assert_eq!(left(&u), [("brace".to_owned(), 3)]);
    // A mastered class's active counts too; a combat active has no uses.
    record(&mut u, "guard", 10);
    u.class = ClassId("exile".into());
    prepare(&mut u);
    assert_eq!(left(&u), [("brace".to_owned(), 3), ("rally".to_owned(), 1)]);
    u.class = ClassId("swordsman".into());
    prepare(&mut u);
    assert_eq!(left(&u), [("brace".to_owned(), 3)]);
    // Only the highest rank of a family is usable, with its own uses.
    let mut v = unit_in("exile");
    record(&mut v, "exile", 10);
    v.class = ClassId("commander".into());
    prepare(&mut v);
    assert_eq!(left(&v), [("rally_2".to_owned(), 2)]);
}

#[test]
fn pay_cost_spends_durability_and_breaks_at_zero() {
    // More than is left: the rest is spent and the weapon breaks.
    let mut short = armed(2);
    let paid = pay_cost(&mut short, SkillCost::Durability(3), &CostSource::Weapon(0)).unwrap();
    assert_eq!(
        paid.events,
        [Event::DurabilitySpent {
            unit: UnitId(1),
            slot: 0,
            item: ItemId::new("iron"),
            amount: 2,
            left: 0,
        }]
    );
    assert!(paid.broke.is_some());
    let weapon = CostSource::Weapon(0);
    let mut u = armed(5);
    let paid = pay_cost(&mut u, SkillCost::Durability(3), &weapon);
    assert_eq!(
        paid,
        Ok(Paid {
            events: vec![Event::DurabilitySpent {
                unit: UnitId(1),
                slot: 0,
                item: ItemId::new("iron"),
                amount: 3,
                left: 2,
            }],
            broke: None,
        })
    );
    let paid = pay_cost(&mut u, SkillCost::Durability(2), &weapon);
    assert_eq!(
        paid.map(|p| p.broke),
        Ok(Some(Event::ItemBroke {
            unit: UnitId(1),
            item: ItemId::new("iron"),
        }))
    );
    assert_eq!(u.loadout.weapon(0).map(|w| w.durability_left), Some(0));
    // Refused: nothing changes.
    let before = u.clone();
    assert_eq!(
        pay_cost(&mut u, SkillCost::Durability(1), &weapon),
        Err(CostError::WeaponBroken)
    );
    assert_eq!(u, before);
}

#[test]
fn pay_cost_spends_one_extra_spell_use() {
    let fire = SpellId::new("fire");
    let mut u = armed(5);
    let paid = pay_cost(
        &mut u,
        SkillCost::ExtraSpellUse,
        &CostSource::Spell(fire.clone()),
    );
    assert_eq!(
        paid,
        Ok(Paid {
            events: vec![Event::SpellUsesChanged {
                unit: UnitId(1),
                spell: fire.clone(),
                uses_left: 1,
            }],
            broke: None,
        })
    );
    assert_eq!(u.spells.uses_left(&fire), 1);
    let before = u.clone();
    assert_eq!(
        pay_cost(&mut u, SkillCost::ExtraSpellUse, &CostSource::Spell(fire)),
        Err(CostError::NotEnoughUses { left: 1 })
    );
    assert_eq!(u, before);
}

#[test]
fn cost_error_messages() {
    let cases = [
        (CostError::WrongSource, "it can't be paid that way"),
        (CostError::NoWeapon, "no weapon to pay with"),
        (CostError::WeaponBroken, "the weapon is broken"),
        (
            CostError::NotEnoughUses { left: 1 },
            "it needs 2 spell uses and the spell has 1",
        ),
        (CostError::NoUsesLeft, "it has no uses left this battle"),
    ];
    for (e, text) in cases {
        assert_eq!(e.to_string(), text);
    }
}

// ---- Learning and usable skills ----------------------------------------------

#[test]
fn a_higher_rank_supersedes_and_a_lower_one_changes_nothing() {
    let skills = table();
    let mut u = unit_in("plain");
    assert!(u.learn_skill(&sid("white_magic_1"), &skills));
    assert!(u.learn_skill(&sid("evasion"), &skills));
    assert!(u.learn_skill(&sid("white_magic_2"), &skills));
    let learned = |u: &Unit| {
        u.learned_skills
            .iter()
            .map(|s| s.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(learned(&u), ["evasion", "white_magic_2"]);
    let before = u.clone();
    assert!(!u.learn_skill(&sid("white_magic_1"), &skills));
    assert!(!u.learn_skill(&sid("white_magic_2"), &skills));
    assert!(!u.learn_skill(&sid("evasion"), &skills));
    assert!(!u.learn_skill(&sid("unknown"), &skills));
    assert_eq!(u, before);
    // Learning 1 after 2, from scratch, keeps 2.
    let mut v = unit_in("plain");
    v.learn_skill(&sid("white_magic_2"), &skills);
    v.learn_skill(&sid("white_magic_1"), &skills);
    assert_eq!(learned(&v), ["white_magic_2"]);
}

#[test]
fn the_current_class_active_is_usable_from_class_level_1() {
    let u = unit_in("archer");
    assert_eq!(
        u.class_records
            .get(&ClassId("archer".into()))
            .map(|r| r.class_level),
        Some(1)
    );
    assert_eq!(usable(&u), ["long_shot"]);
    assert!(usable(&unit_in("plain")).is_empty());
    // An active missing from the table is skipped.
    assert!(usable(&unit_in("ghost")).is_empty());
}

#[test]
fn an_unmastered_class_active_stays_behind_after_a_reclass() {
    let mut u = unit_in("swordsman");
    record(&mut u, "swordsman", 9);
    u.class = ClassId("guard".into());
    record(&mut u, "guard", 1);
    assert_eq!(usable(&u), ["brace"]);
    // Coming back brings it back.
    u.class = ClassId("swordsman".into());
    assert_eq!(usable(&u), ["keen_edge"]);
}

#[test]
fn a_mastered_class_active_is_usable_in_any_class() {
    let mut u = unit_in("swordsman");
    record(&mut u, "swordsman", 10);
    u.class = ClassId("guard".into());
    record(&mut u, "guard", 1);
    assert_eq!(usable(&u), ["brace", "keen_edge"]);
    // Masteries of missing classes are skipped.
    record(&mut u, "nowhere", 10);
    assert_eq!(usable(&u), ["brace", "keen_edge"]);
}

#[test]
fn only_the_highest_rank_of_a_family_is_usable() {
    let skills = table();
    // A mastered archer promoted to marksman: Long Shot 2 only.
    let mut u = unit_in("archer");
    record(&mut u, "archer", 10);
    u.class = ClassId("marksman".into());
    record(&mut u, "marksman", 1);
    u.learn_skill(&sid("white_magic_1"), &skills);
    assert_eq!(usable(&u), ["long_shot_2", "white_magic_1"]);
    // The other way round (rank 2 mastered, rank 1 current) too.
    let mut v = unit_in("marksman");
    record(&mut v, "marksman", 10);
    v.class = ClassId("archer".into());
    record(&mut v, "archer", 1);
    assert_eq!(usable(&v), ["long_shot_2"]);
    let classes = classes();
    assert!(
        v.usable_active(&sid("long_shot"), &classes, &skills)
            .is_none()
    );
    assert_eq!(
        v.usable_active(&sid("long_shot_2"), &classes, &skills)
            .map(|s| s.rank),
        Some(2)
    );
    // The same active from two classes is listed once.
    let mut w = unit_in("archer");
    record(&mut w, "archer", 10);
    w.class = ClassId("ranger".into());
    record(&mut w, "ranger", 1);
    assert_eq!(usable(&w), ["long_shot"]);
    // A passive is never a usable active.
    assert!(
        u.usable_active(&sid("white_magic_1"), &classes, &skills)
            .is_none()
    );
}

// ---- Timed effects -----------------------------------------------------------

#[test]
fn the_same_effect_refreshes_and_effects_expire_by_phase() {
    let mut u = unit_in("plain");
    let effect = |source: &str, def, until| TimedEffect {
        source: EffectSource::Skill(sid(source)),
        mods: TimedMods {
            stats: vec![(StatKind::Def, def)],
            combat: CombatMods::default(),
        },
        until,
    };
    u.add_effect(effect("brace", 5, Phase::Player));
    u.add_effect(effect("rally", 3, Phase::Enemy));
    u.add_effect(effect("war_cry", 2, Phase::Player));
    u.add_effect(effect("brace", 5, Phase::Other));
    assert_eq!(
        u.effects,
        [
            effect("rally", 3, Phase::Enemy),
            effect("war_cry", 2, Phase::Player),
            effect("brace", 5, Phase::Other),
        ]
    );
    assert_eq!(effect_bonuses(&u.effects).stats, stat(StatKind::Def, 10));
    assert_eq!(
        u.expire_effects(Phase::Player),
        [EffectSource::Skill(sid("war_cry"))]
    );
    assert_eq!(u.expire_effects(Phase::Player), Vec::<EffectSource>::new());
    assert_eq!(
        u.expire_effects(Phase::Other),
        [EffectSource::Skill(sid("brace"))]
    );
    assert_eq!(u.effects, [effect("rally", 3, Phase::Enemy)]);
}

#[test]
fn a_negative_bonus_never_takes_a_stat_below_zero() {
    let base = Stats::from_growable([20, 5, 0, 4, 2, 3, -1], 5);
    let bonuses = Bonuses {
        stats: Stats::from_growable([0, -3, 0, 2, -3, -3, -2], -7),
        ..Bonuses::default()
    };
    // Str 5 − 3, Dex 4 + 2, Spd 2 − 3 → 0, Def 3 − 3, Res −1 stays −1, Mov
    // 5 − 7 → 0.
    assert_eq!(
        bonuses.apply(base),
        Stats::from_growable([20, 2, 0, 6, 0, 0, -1], 0)
    );
}

#[test]
fn art_effects_are_keyed_by_art_and_mov_effects_change_move_points() {
    let mut u = unit_in("plain");
    assert_eq!(u.move_points(), 5);
    let pin = |until| TimedEffect {
        source: EffectSource::Art(ArtId::new("pinning_shot")),
        mods: TimedMods {
            stats: vec![(StatKind::Mov, -3)],
            combat: CombatMods::default(),
        },
        until,
    };
    u.add_effect(pin(Phase::Player));
    assert!(u.has_effect(&EffectSource::Art(ArtId::new("pinning_shot"))));
    assert!(!u.has_effect(&EffectSource::Skill(sid("pinning_shot"))));
    assert_eq!(u.move_points(), 2);
    // Refreshed, not stacked.
    u.add_effect(pin(Phase::Other));
    assert_eq!((u.effects.len(), u.move_points()), (1, 2));
    // Never below 0.
    u.add_effect(TimedEffect {
        source: EffectSource::Art(ArtId::new("deeper")),
        ..pin(Phase::Other)
    });
    assert_eq!(u.move_points(), 0);
    assert_eq!(u.stats.mov, 5);
}

#[test]
fn a_debuff_lasts_until_the_end_of_the_targets_next_phase() {
    assert_eq!(TimedEffect::debuff_until(Phase::Enemy), Phase::Other);
    assert_eq!(TimedEffect::debuff_until(Phase::Other), Phase::Player);
    assert_eq!(TimedEffect::debuff_until(Phase::Player), Phase::Enemy);
}

#[test]
fn effect_sources_come_from_skill_and_art_ids() {
    assert_eq!(
        EffectSource::from(sid("brace")),
        EffectSource::Skill(sid("brace"))
    );
    assert_eq!(
        EffectSource::from(ArtId::new("sidestep")),
        EffectSource::Art(ArtId::new("sidestep"))
    );
}
