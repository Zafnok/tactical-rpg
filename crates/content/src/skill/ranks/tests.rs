//! Tests of the rank rule: every number and every fundamental a rank is
//! compared on, and which ranks are compared.

use trpg_core::{Area, Condition, Stance, WeaponKind, WeaponReq};

use super::*;

fn skill(id: &str, family: &str, rank: u8, kind: SkillKind) -> SkillDef {
    SkillDef {
        id: SkillId(id.into()),
        name: id.into(),
        family: family.into(),
        rank,
        kind,
    }
}

/// What is wrong with `higher` as rank 2 of rank 1 `lower`.
fn problems(lower: SkillKind, higher: SkillKind) -> Vec<String> {
    compare(&skill("a", "f", 1, lower), &skill("b", "f", 2, higher))
}

fn mods(set: impl Fn(&mut CombatMods)) -> CombatMods {
    let mut m = CombatMods::default();
    set(&mut m);
    m
}

fn hit(n: StatValue) -> CombatMods {
    mods(|m| m.hit = n)
}

fn passive(effect: PassiveEffect) -> SkillKind {
    SkillKind::Passive(vec![effect])
}

fn combat(mods: CombatMods) -> SkillKind {
    passive(PassiveEffect::CombatMod {
        mods,
        when: Condition::Always,
    })
}

fn active(cost: SkillCost, effect: ActiveEffect) -> SkillKind {
    SkillKind::Active { cost, effect }
}

fn costing(effect: ActiveEffect) -> SkillKind {
    active(SkillCost::Durability(3), effect)
}

fn strike(set: impl Fn(&mut ActiveEffect)) -> SkillKind {
    let mut effect = ActiveEffect::Strike {
        with: WeaponReq::Any,
        mods: CombatMods::default(),
        range: 0,
        stance: None,
        post_move: 0,
        drain: false,
    };
    set(&mut effect);
    costing(effect)
}

fn timed(stats: &[(StatKind, StatValue)], combat: CombatMods) -> TimedMods {
    TimedMods {
        stats: stats.to_vec(),
        combat,
    }
}

fn stance(mods: &TimedMods, this_combat: bool) -> SkillKind {
    strike(|e| {
        if let ActiveEffect::Strike { stance, .. } = e {
            *stance = Some(Stance {
                mods: mods.clone(),
                this_combat,
            });
        }
    })
}

fn buff(area: Area, mods: TimedMods) -> SkillKind {
    costing(ActiveEffect::Buff { area, mods })
}

fn heal(radius: u32, power: StatValue) -> SkillKind {
    costing(ActiveEffect::Heal { radius, power })
}

fn push(collision: StatValue) -> SkillKind {
    costing(ActiveEffect::Push { collision })
}

type Build = Box<dyn Fn(StatValue) -> SkillKind>;

fn small(n: StatValue) -> u8 {
    u8::try_from(n).unwrap()
}

fn wide(n: StatValue) -> u32 {
    u32::try_from(n).unwrap()
}

/// The numbers of a passive a rank is compared on, by the name its problem
/// uses, with a skill that has it at `n`.
fn passive_numbers() -> Vec<(&'static str, Build)> {
    let always = Condition::Always;
    vec![
        ("hit", Box::new(|n| combat(hit(n)))),
        ("crit", Box::new(|n| combat(mods(|m| m.crit = n)))),
        ("might", Box::new(|n| combat(mods(|m| m.might = n)))),
        ("avoid", Box::new(|n| combat(mods(|m| m.avoid = n)))),
        (
            "attack speed",
            Box::new(|n| combat(mods(|m| m.attack_speed = n))),
        ),
        (
            "extra strikes",
            Box::new(|n| combat(mods(|m| m.extra_strikes = small(n)))),
        ),
        ("pierce", Box::new(|n| combat(mods(|m| m.pierce = n)))),
        (
            "amount",
            Box::new(move |n| {
                passive(PassiveEffect::StatWhile {
                    stat: StatKind::Def,
                    amount: n,
                    when: always,
                })
            }),
        ),
        (
            "heal bonus",
            Box::new(|n| passive(PassiveEffect::HealBonus(n))),
        ),
        (
            "spell might",
            Box::new(|n| passive(PassiveEffect::SpellMight(n))),
        ),
        (
            "tiles",
            Box::new(move |n| {
                passive(PassiveEffect::PostActionMove {
                    tiles: wide(n),
                    when: always,
                })
            }),
        ),
        (
            "hit",
            Box::new(|n| {
                passive(PassiveEffect::AllyAura {
                    radius: 2,
                    mods: hit(n),
                })
            }),
        ),
    ]
}

/// The numbers of an active a rank is compared on, like
/// [`passive_numbers`].
fn active_numbers() -> Vec<(&'static str, Build)> {
    let def = StatKind::Def;
    vec![
        (
            "hit",
            Box::new(|n| {
                strike(|e| {
                    if let ActiveEffect::Strike { mods, .. } = e {
                        *mods = hit(n);
                    }
                })
            }),
        ),
        (
            "range",
            Box::new(|n| {
                strike(|e| {
                    if let ActiveEffect::Strike { range, .. } = e {
                        *range = wide(n);
                    }
                })
            }),
        ),
        (
            "move after",
            Box::new(|n| {
                strike(|e| {
                    if let ActiveEffect::Strike { post_move, .. } = e {
                        *post_move = wide(n);
                    }
                })
            }),
        ),
        (
            "Def",
            Box::new(move |n| stance(&timed(&[(def, n)], hit(0)), true)),
        ),
        (
            "avoid",
            Box::new(|n| stance(&timed(&[], mods(|m| m.avoid = n)), true)),
        ),
        (
            "Def",
            Box::new(move |n| buff(Area::Own, timed(&[(def, n)], hit(0)))),
        ),
        ("hit", Box::new(|n| buff(Area::Own, timed(&[], hit(n))))),
        ("power", Box::new(|n| heal(1, n))),
        ("collision damage", Box::new(push)),
        (
            "uses",
            Box::new(|n| {
                active(
                    SkillCost::Uses(small(n)),
                    ActiveEffect::Push { collision: 5 },
                )
            }),
        ),
    ]
}

#[test]
fn no_number_may_be_lower_and_one_must_be_higher() {
    let numbers = passive_numbers().into_iter().chain(active_numbers());
    for (i, (what, build)) in numbers.enumerate() {
        let at = format!("{i}: {what}");
        assert_eq!(
            problems(build(2), build(1)),
            [format!("its {what} is 1, less than 2")],
            "{at}"
        );
        assert_eq!(problems(build(2), build(3)), Vec::<String>::new(), "{at}");
        assert_eq!(
            problems(build(2), build(2)),
            ["none of its numbers is higher"],
            "{at}"
        );
        // A number may go from 0 to more (Bow Focus 2 adds crit).
        assert_eq!(problems(build(0), build(1)), Vec::<String>::new(), "{at}");
    }
}

/// A rank 1, a rank 2 whose numbers are no lower, and what is wrong with it.
type Case = (SkillKind, SkillKind, &'static str);

/// The fundamentals of a passive a rank is compared on.
fn passive_fundamentals() -> Vec<Case> {
    let stat = |stat, amount, when| passive(PassiveEffect::StatWhile { stat, amount, when });
    let moving = |tiles, when| passive(PassiveEffect::PostActionMove { tiles, when });
    let aura = |radius, n| {
        passive(PassiveEffect::AllyAura {
            radius,
            mods: hit(n),
        })
    };
    let when = |n, when| passive(PassiveEffect::CombatMod { mods: hit(n), when });
    let (always, half) = (Condition::Always, Condition::HpAtMostHalf);
    let sword = Condition::WeaponKindEquipped(WeaponKind::Sword);
    let bow = Condition::WeaponKindEquipped(WeaponKind::Bow);
    vec![
        (
            stat(StatKind::Def, 2, always),
            stat(StatKind::Res, 4, always),
            "its stat is Res, not Def",
        ),
        (
            stat(StatKind::Def, 2, always),
            stat(StatKind::Def, 4, half),
            "its condition is HpAtMostHalf, not Always",
        ),
        (
            when(10, sword),
            when(20, bow),
            "its condition is WeaponKindEquipped(Bow), not WeaponKindEquipped(Sword)",
        ),
        (
            when(10, Condition::MovedAtLeast(4)),
            when(20, Condition::MovedAtLeast(3)),
            "its condition is MovedAtLeast(3), not MovedAtLeast(4)",
        ),
        (
            moving(1, always),
            moving(2, half),
            "its condition is HpAtMostHalf, not Always",
        ),
        (aura(2, 10), aura(3, 20), "its radius is 3, not 2"),
        (
            passive(PassiveEffect::HealBonus(2)),
            passive(PassiveEffect::SpellMight(4)),
            "it has another kind of effect",
        ),
        (
            combat(hit(10)),
            SkillKind::Passive(vec![
                PassiveEffect::CombatMod {
                    mods: hit(20),
                    when: always,
                },
                PassiveEffect::HealBonus(2),
            ]),
            "it has 2 effects, not 1",
        ),
    ]
}

/// The flags of combat bonuses a rank is compared on.
fn flag_fundamentals() -> Vec<Case> {
    vec![
        (
            combat(hit(10)),
            combat(mods(|m| {
                m.hit = 20;
                m.single_strike = true;
            })),
            "its single_strike is true, not false",
        ),
        (
            combat(hit(10)),
            combat(mods(|m| {
                m.hit = 20;
                m.double_crit = true;
            })),
            "its double_crit is true, not false",
        ),
        (
            combat(mods(|m| {
                m.hit = 10;
                m.ignore_terrain = true;
            })),
            combat(hit(20)),
            "its ignore_terrain is false, not true",
        ),
        (
            combat(hit(10)),
            combat(mods(|m| {
                m.hit = 20;
                m.sword_followup = Some((3, 2));
            })),
            "its sword_followup is Some((3, 2)), not None",
        ),
    ]
}

/// The fundamentals of an active a rank is compared on.
fn active_fundamentals() -> Vec<Case> {
    let with = |with: WeaponReq, n| {
        strike(|e| {
            if let ActiveEffect::Strike { with: w, mods, .. } = e {
                *w = with;
                *mods = hit(n);
            }
        })
    };
    let draining = |on: bool, n| {
        strike(|e| {
            if let ActiveEffect::Strike { drain, mods, .. } = e {
                *drain = on;
                *mods = hit(n);
            }
        })
    };
    let def = |n| timed(&[(StatKind::Def, n)], hit(0));
    let allies = |radius| Area::Allies { radius };
    vec![
        (
            with(WeaponReq::Any, 10),
            with(WeaponReq::Kind(WeaponKind::Bow), 20),
            "its weapon is Kind(Bow), not Any",
        ),
        (
            draining(false, 10),
            draining(true, 20),
            "its drain is true, not false",
        ),
        (
            stance(&def(3), false),
            stance(&def(5), true),
            "its this_combat is true, not false",
        ),
        (
            strike(|_| {}),
            stance(&def(5), true),
            "only one of them has a stance",
        ),
        (
            stance(&def(0), true),
            with(WeaponReq::Any, 10),
            "only one of them has a stance",
        ),
        (
            buff(Area::Own, def(3)),
            buff(allies(2), def(5)),
            "its area is Allies { radius: 2 }, not Own",
        ),
        (
            buff(allies(1), def(3)),
            buff(allies(2), def(5)),
            "its area is Allies { radius: 2 }, not Allies { radius: 1 }",
        ),
        // Sanctuary 2, as it was: a wider reach is a skill of its own.
        (heal(1, 5), heal(2, 5), "its radius is 2, not 1"),
        (heal(1, 5), heal(2, 9), "its radius is 2, not 1"),
        (heal(1, 5), push(9), "it has another kind of effect"),
        (
            heal(1, 5),
            buff(Area::Own, def(9)),
            "it has another kind of effect",
        ),
        (
            combat(hit(10)),
            with(WeaponReq::Any, 20),
            "one is a passive and the other an active",
        ),
        (
            push(5),
            passive(PassiveEffect::HealBonus(9)),
            "one is a passive and the other an active",
        ),
        (
            active(SkillCost::Uses(3), ActiveEffect::Push { collision: 5 }),
            push(9),
            "its cost is Durability(3), not like Uses(3)",
        ),
        (
            push(5),
            active(
                SkillCost::ExtraSpellUse,
                ActiveEffect::Push { collision: 9 },
            ),
            "its cost is ExtraSpellUse, not like Durability(3)",
        ),
    ]
}

#[test]
fn a_rank_keeps_the_fundamentals_of_the_rank_below() {
    let cases = passive_fundamentals()
        .into_iter()
        .chain(flag_fundamentals())
        .chain(active_fundamentals());
    for (i, (lower, higher, problem)) in cases.enumerate() {
        assert_eq!(problems(lower, higher), [problem], "{i}");
    }
}

#[test]
fn every_effect_of_a_passive_is_compared_with_its_pair() {
    let two = |crit, heal| {
        SkillKind::Passive(vec![
            PassiveEffect::CombatMod {
                mods: mods(|m| m.crit = crit),
                when: Condition::Always,
            },
            PassiveEffect::HealBonus(heal),
        ])
    };
    assert_eq!(problems(two(10, 2), two(10, 4)), Vec::<String>::new());
    assert_eq!(
        problems(two(10, 2), two(5, 1)),
        [
            "its crit is 5, less than 10",
            "its heal bonus is 1, less than 2"
        ]
    );
}

#[test]
fn a_cost_in_durability_may_be_anything_and_is_no_bigger_number() {
    let pushing = |cost, collision| {
        active(
            SkillCost::Durability(cost),
            ActiveEffect::Push { collision },
        )
    };
    assert_eq!(problems(pushing(5, 5), pushing(1, 9)), Vec::<String>::new());
    assert_eq!(problems(pushing(1, 5), pushing(9, 9)), Vec::<String>::new());
    assert_eq!(
        problems(pushing(1, 5), pushing(9, 5)),
        ["none of its numbers is higher"]
    );
    let spell = |might| {
        active(
            SkillCost::ExtraSpellUse,
            ActiveEffect::Strike {
                with: WeaponReq::Spell,
                mods: mods(|m| m.might = might),
                range: 0,
                stance: None,
                post_move: 0,
                drain: false,
            },
        )
    };
    assert_eq!(problems(spell(5), spell(8)), Vec::<String>::new());
}

#[test]
fn timed_stats_are_compared_stat_by_stat() {
    use StatKind::{Def, Res};
    let own = |stats: &[(StatKind, StatValue)]| buff(Area::Own, timed(stats, hit(0)));
    // A stat the lower rank lacks counts as 0; one listed twice adds up.
    assert_eq!(
        problems(own(&[(Def, 5)]), own(&[(Res, 5), (Def, 5)])),
        Vec::<String>::new()
    );
    assert_eq!(
        problems(own(&[(Def, 1), (Def, 1)]), own(&[(Def, 2)])),
        ["none of its numbers is higher"]
    );
    assert_eq!(
        problems(own(&[(Def, 5)]), own(&[(Def, 4), (Res, 9)])),
        ["its Def is 4, less than 5"]
    );
    assert_eq!(
        problems(own(&[(Def, 5), (Res, 5)]), own(&[(Def, 8)])),
        ["its Res is 0, less than 5"]
    );
}

/// The two replacements Nick kept (`progression.md` → *Superseding*): a
/// rank 2 may add another bonus of the same kind.
#[test]
fn bow_focus_2_and_leadership_2_pass() {
    let bow = Condition::WeaponKindEquipped(WeaponKind::Bow);
    let focus = |mods| passive(PassiveEffect::CombatMod { mods, when: bow });
    let both = mods(|m| {
        m.hit = 10;
        m.crit = 5;
    });
    assert_eq!(problems(focus(hit(5)), focus(both)), Vec::<String>::new());
    let aura = |mods| passive(PassiveEffect::AllyAura { radius: 2, mods });
    let led = mods(|m| {
        m.hit = 10;
        m.avoid = 10;
    });
    assert_eq!(problems(aura(hit(10)), aura(led)), Vec::<String>::new());
}

fn table(skills: Vec<SkillDef>) -> BTreeMap<SkillId, SkillDef> {
    skills.into_iter().map(|s| (s.id.clone(), s)).collect()
}

#[test]
fn each_rank_is_compared_with_the_rank_below_it_in_its_family() {
    let heal = |n| passive(PassiveEffect::HealBonus(n));
    // Family `w`: 2 → 4 is fine, 4 → 3 isn't (though 3 is above rank
    // 1's 2). Family `g` has a gap: rank 3 is compared with rank 1.
    // `x` and `y` are families of one.
    let skills = table(vec![
        skill("w3", "w", 3, heal(3)),
        skill("w1", "w", 1, heal(2)),
        skill("w2", "w", 2, heal(4)),
        skill("g3", "g", 3, heal(1)),
        skill("g1", "g", 1, heal(2)),
        skill("x", "x", 1, heal(9)),
        skill("y", "y", 2, heal(1)),
    ]);
    let found = rank_problems(&skills);
    let expected = [
        (
            "g3",
            "skill \"g3\" (rank 3) must be \"g1\" (rank 1) with bigger numbers and nothing else: its heal bonus is 1, less than 2",
        ),
        (
            "w3",
            "skill \"w3\" (rank 3) must be \"w2\" (rank 2) with bigger numbers and nothing else: its heal bonus is 3, less than 4",
        ),
    ];
    let found: Vec<(&str, &str)> = found
        .iter()
        .map(|(id, problem)| (id.0.as_str(), problem.as_str()))
        .collect();
    assert_eq!(found, expected);
    assert_eq!(rank_problems(&BTreeMap::new()), []);
}

#[test]
fn two_skills_of_one_rank_are_not_compared_with_each_other() {
    // The loader reports the duplicate rank; each is still compared
    // with the rank below.
    let heal = |n| passive(PassiveEffect::HealBonus(n));
    let skills = table(vec![
        skill("a", "w", 1, heal(2)),
        skill("b", "w", 2, heal(4)),
        skill("c", "w", 2, heal(3)),
    ]);
    assert_eq!(rank_problems(&skills), []);
    let skills = table(vec![
        skill("a", "w", 1, heal(2)),
        skill("b", "w", 2, heal(1)),
        skill("c", "w", 2, heal(1)),
    ]);
    let ids: Vec<String> = rank_problems(&skills)
        .into_iter()
        .map(|(id, _)| id.0)
        .collect();
    assert_eq!(ids, ["b"]);
}
