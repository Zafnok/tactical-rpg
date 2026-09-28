//! Tests of the EXP, level-up and class-point rules. The Swordsman and the
//! numbers are `progression.md`'s.

use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;
use crate::class::{ClassId, UnitTags};
use crate::geom::Pos;
use crate::rng::{ScriptedRng, SimRng};
use crate::skill::{SkillDef, SkillId, SkillKind};
use crate::spell::SpellId;
use crate::stats::{Growths, Stats};
use crate::terrain::MovementTypeId;
use crate::unit::{Faction, UnitId};

use CombatResult::{Damaged, Killed, NoDamage};

/// `progression.md`'s tier-1 Swordsman (growths 70 40 10 55 60 25 20).
fn swordsman() -> ClassDef {
    ClassDef {
        id: ClassId("swordsman".into()),
        name: "Swordsman".into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 5,
        base: Stats::from_growable([18, 5, 0, 7, 8, 3, 1], 5),
        caps: Stats::from_growable([40, 20, 10, 24, 25, 18, 15], 5),
        growths: Growths([70, 40, 10, 55, 60, 25, 20]),
        weapons: vec![],
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: Some(SkillId::new("keen_edge")),
        passives: vec![SkillId::new("sword_focus_1"), SkillId::new("parry")],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![(1, SpellId::new("spark")), (5, SpellId::new("frost"))],
        affinities: vec![],
    }
}

/// A class like the Swordsman, of `tier`, with these growths and caps.
fn custom(id: &str, tier: Tier, growths: [GrowthValue; 7], caps: [StatValue; 7]) -> ClassDef {
    ClassDef {
        id: ClassId(id.into()),
        tier,
        growths: Growths(growths),
        caps: Stats::from_growable(caps, 5),
        spells: vec![],
        ..swordsman()
    }
}

/// The swordsman and a tier-3 copy (`adept`), with the design's tier tables.
fn table() -> ClassTable {
    let adept = ClassDef {
        id: ClassId("adept".into()),
        tier: 3,
        ..swordsman()
    };
    ClassTable {
        classes: [swordsman(), adept]
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect(),
        min_gains: vec![2, 2, 3],
        cp_per_class_level: vec![10, 17, 25],
        class_level_cap: 10,
        level_cap: 99,
        hard_ceilings: Stats::from_growable([80, 50, 50, 50, 50, 50, 50], 15),
    }
}

fn skills() -> SkillTable {
    let passive = |id: &str, family: &str, rank| SkillDef {
        id: SkillId::new(id),
        name: id.into(),
        family: family.into(),
        rank,
        kind: SkillKind::Passive(vec![]),
    };
    SkillTable {
        skills: [
            passive("sword_focus_1", "sword_focus", 1),
            passive("sword_focus_2", "sword_focus", 2),
            passive("parry", "parry", 1),
        ]
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect(),
    }
}

/// A level-1 generic unit of `class` with the given stats.
fn unit_in(class: &str, stats: [StatValue; 7]) -> Unit {
    let mut u = Unit::generic(
        UnitId(1),
        &ClassId(class.into()),
        &table(),
        1,
        Faction::Player,
        Pos::new(0, 0),
    )
    .unwrap();
    u.stats = Stats::from_growable(stats, 5);
    u.hp = u.stats.hp;
    u
}

fn fresh() -> Unit {
    unit_in("swordsman", [18, 5, 0, 7, 8, 3, 1])
}

/// A [`RandomSource`] that replays a script and records every `roll_below`
/// bound, so tests can check the safety net's totals.
struct Recorder {
    inner: ScriptedRng,
    totals: Vec<u32>,
}

impl Recorder {
    fn new(rolls: [u8; 7], below: &[u32]) -> Self {
        Self {
            inner: ScriptedRng::new(rolls).with_below(below.to_vec()),
            totals: Vec::new(),
        }
    }
}

impl RandomSource for Recorder {
    fn roll_percent(&mut self) -> u8 {
        self.inner.roll_percent()
    }

    fn roll_below(&mut self, n: u32) -> u32 {
        self.totals.push(n);
        self.inner.roll_below(n)
    }
}

/// Levels `unit` up in `class` with `min` and the scripted draws; checks
/// every roll and draw was used, and returns the gains and the totals.
fn roll(
    unit: &Unit,
    class: &ClassDef,
    min: u8,
    rolls: [u8; 7],
    below: &[u32],
) -> (StatGains, Vec<u32>) {
    let mut rng = Recorder::new(rolls, below);
    let gains = level_up(unit, class, min, &mut rng);
    assert_eq!(rng.inner.consumed(), 7, "always 7 rolls");
    assert_eq!(rng.inner.consumed_below(), below.len(), "net draws");
    (gains, rng.totals)
}

// ---- EXP formulas ----------------------------------------------------------------

#[test]
fn exp_matches_the_design_examples() {
    // (unit level, target level, result, boss, EXP): the 8 rows.
    let rows = [
        (10, 10, Damaged, false, 20),
        (10, 15, Damaged, false, 24),
        (10, 10, Killed, false, 60),
        (10, 15, Killed, false, 94),
        (10, 5, Killed, false, 26),
        (20, 10, Killed, false, 14),
        (10, 10, Killed, true, 100),
        (10, 30, NoDamage, true, 2),
    ];
    for (me, them, result, boss, exp) in rows {
        assert_eq!(
            exp_for_combat(me, them, result, boss),
            exp,
            "{me} vs {them} {result:?} boss {boss}"
        );
    }
}

#[test]
fn exp_is_clamped_to_2_and_100() {
    // Far below: 2 × ((31 − 40) / 3) = −6 → 2.
    assert_eq!(exp_for_combat(50, 10, Damaged, false), 2);
    assert_eq!(exp_for_combat(50, 10, Killed, false), 2);
    assert_eq!(exp_for_combat(10, 50, Damaged, false), 46);
    assert_eq!(exp_for_combat(10, 50, Killed, false), 100);
    // Huge levels don't overflow.
    assert_eq!(exp_for_combat(Level::MAX, 1, Killed, true), 2);
    // A boss 10 levels below: 2 × (7 + 0) + 40 = 54.
    assert_eq!(exp_for_combat(20, 10, Killed, true), 54);
    assert_eq!(exp_for_combat(1, Level::MAX, Killed, false), 100);
    assert_eq!(exp_for_combat(1, Level::MAX, NoDamage, false), 2);
    // `/` rounds toward zero: d = −32 gives (−1) / 3 = 0, not −1.
    assert_eq!(exp_for_combat(33, 1, Damaged, false), 2);
    // d = −1: 30 / 3 = 10; kill 2 × (10 + 17) = 54.
    assert_eq!(exp_for_combat(6, 5, Killed, false), 54);
}

#[test]
fn other_awards() {
    assert_eq!(exp_for_heal(), 24);
    assert_eq!(exp_for_tile_cast(), 24);
    assert_eq!(exp_for_active_skill(), 20);
    assert_eq!(cp_for_combat(false), 2);
    assert_eq!(cp_for_combat(true), 4);
    assert_eq!(ACTION_CP, 2);
}

// ---- Growths ---------------------------------------------------------------------

#[test]
fn growth_adds_20_to_the_talent_only() {
    let class = swordsman();
    let mut u = fresh();
    assert_eq!(growth(&u, &class, StatKind::Spd), 60);
    u.talent = Some(StatKind::Spd);
    assert_eq!(growth(&u, &class, StatKind::Spd), 80);
    assert_eq!(growth(&u, &class, StatKind::Str), 40);
    assert_eq!(growth(&u, &class, StatKind::Mov), 0);
}

#[test]
fn stat_gains_accessors() {
    let g = StatGains([1, 0, 2, 0, 0, 0, 1]);
    assert_eq!(g.get(StatKind::Hp), 1);
    assert_eq!(g.get(StatKind::Mag), 2);
    assert_eq!(g.get(StatKind::Res), 1);
    assert_eq!(g.get(StatKind::Mov), 0);
    assert_eq!(g.count(), 3);
}

// ---- Level up ------------------------------------------------------------------

#[test]
fn worked_example_no_natural_gains_then_the_net_picks_two() {
    let u = fresh();
    // 0 gains; the first pick is over all 7 (total 280): draw 0 → HP. The
    // second is over the 6 left (total 210): Str 40, Mag 50, Dex 105,
    // Spd 165, so draw 105 → Spd.
    let (gains, totals) = roll(&u, &swordsman(), 2, [85, 72, 50, 91, 77, 30, 40], &[0, 105]);
    assert_eq!(gains, StatGains([1, 0, 0, 0, 1, 0, 0]));
    assert_eq!(totals, [280, 210]);
}

#[test]
fn worked_example_three_natural_gains_skip_the_net() {
    let u = fresh();
    let (gains, totals) = roll(&u, &swordsman(), 2, [10, 20, 50, 30, 77, 30, 40], &[]);
    assert_eq!(gains, StatGains([1, 1, 0, 1, 0, 0, 0]));
    assert!(totals.is_empty());
}

#[test]
fn a_roll_below_the_growth_gains_and_one_at_it_doesnt() {
    let u = fresh();
    // Growths 70 40 10 55 60 25 20: growth − 1 gains, growth doesn't.
    let (gains, _) = roll(&u, &swordsman(), 0, [69, 40, 9, 55, 59, 25, 19], &[]);
    assert_eq!(gains, StatGains([1, 0, 1, 0, 1, 0, 1]));
}

#[test]
fn net_walk_picks_the_first_stat_whose_running_sum_passes_the_draw() {
    let u = fresh();
    let no_gain = [99; 7];
    // Running sums over all 7: 70, 110, 120, 175, 235, 260, 280.
    for (draw, stat) in [(69, 0), (70, 1), (119, 2), (120, 3), (259, 5), (279, 6)] {
        let (gains, _) = roll(&u, &swordsman(), 1, no_gain, &[draw]);
        let mut expected = [0; 7];
        expected[stat] = 1;
        assert_eq!(gains, StatGains(expected), "draw {draw}");
    }
}

#[test]
fn capped_and_zero_growth_stats_never_gain_and_are_not_net_candidates() {
    // Str at its cap (20), Mag growth 0, everything else rolls 99.
    let class = custom(
        "c",
        1,
        [70, 40, 0, 55, 60, 25, 20],
        [40, 20, 10, 24, 25, 18, 15],
    );
    let u = unit_in("swordsman", [18, 20, 0, 7, 8, 3, 1]);
    let (gains, totals) = roll(&u, &class, 0, [0, 0, 0, 99, 99, 99, 99], &[]);
    assert_eq!(gains, StatGains([1, 0, 0, 0, 0, 0, 0]));
    assert!(totals.is_empty());
    // With the net, Str and Mag are left out of the total: 280 − 40 − 10.
    // 70 → Dex (70 HP, then Dex's 55 brings it to 125); then HP.
    let (gains, totals) = roll(&u, &class, 2, [99; 7], &[70, 0]);
    assert_eq!(totals, [230, 175]);
    assert_eq!(gains, StatGains([1, 0, 0, 1, 0, 0, 0]));
}

#[test]
fn a_stat_above_its_cap_after_a_reclass_never_grows() {
    let u = unit_in("swordsman", [50, 5, 0, 7, 8, 3, 1]);
    let (gains, totals) = roll(&u, &swordsman(), 2, [0; 7], &[]);
    assert_eq!(gains, StatGains([0, 1, 1, 1, 1, 1, 1]));
    assert!(totals.is_empty());
}

#[test]
fn talent_makes_a_zero_growth_stat_grow() {
    let class = custom(
        "c",
        1,
        [70, 40, 0, 55, 60, 25, 20],
        [40, 20, 10, 24, 25, 18, 15],
    );
    let mut u = fresh();
    let (gains, _) = roll(&u, &class, 0, [99, 99, 19, 99, 99, 99, 99], &[]);
    assert_eq!(gains, StatGains::default());
    u.talent = Some(StatKind::Mag);
    let (gains, _) = roll(&u, &class, 0, [99, 99, 19, 99, 99, 99, 99], &[]);
    assert_eq!(gains, StatGains([0, 0, 1, 0, 0, 0, 0]));
    // Talent +20 on a stat: 40 → 60, so a roll of 59 now gains.
    u.talent = Some(StatKind::Str);
    let (gains, _) = roll(&u, &class, 0, [99, 59, 99, 99, 99, 99, 99], &[]);
    assert_eq!(gains, StatGains([0, 1, 0, 0, 0, 0, 0]));
}

#[test]
fn growths_above_100_give_a_sure_point_and_a_chance_of_another() {
    // HP 120, Str 200, Mag 150 (one below its cap).
    let class = custom(
        "c",
        1,
        [120, 200, 150, 0, 0, 0, 0],
        [60, 40, 10, 24, 25, 18, 15],
    );
    let u = unit_in("swordsman", [18, 5, 9, 7, 8, 3, 1]);
    // HP: r 19 < 20 → +2; Str: 200 % 100 = 0 → +2 whatever the roll; Mag:
    // r 0 < 50 → +2, clamped to +1 by the cap.
    let (gains, _) = roll(&u, &class, 0, [19, 99, 0, 0, 0, 0, 0], &[]);
    assert_eq!(gains, StatGains([2, 2, 1, 0, 0, 0, 0]));
    // HP: r 20 → +1 only; Str: +2 even with roll 0.
    let (gains, _) = roll(&u, &class, 0, [20, 0, 50, 0, 0, 0, 0], &[]);
    assert_eq!(gains, StatGains([1, 2, 1, 0, 0, 0, 0]));
}

#[test]
fn net_with_one_natural_gain_at_tiers_1_and_3() {
    let u = fresh();
    let one = [0, 99, 99, 99, 99, 99, 99];
    // Tier 1 (2): one more pick, over the 6 others (210).
    let (gains, totals) = roll(&u, &swordsman(), 2, one, &[0]);
    assert_eq!(gains, StatGains([1, 1, 0, 0, 0, 0, 0]));
    assert_eq!(totals, [210]);
    // Tier 3 (3): two picks, 210 then 170 (Str gone).
    let (gains, totals) = roll(&u, &swordsman(), 3, one, &[0, 169]);
    assert_eq!(gains, StatGains([1, 1, 0, 0, 0, 0, 1]));
    assert_eq!(totals, [210, 170]);
}

#[test]
fn net_with_no_natural_gains_at_tier_3_picks_three() {
    let u = fresh();
    let (gains, totals) = roll(&u, &swordsman(), 3, [99; 7], &[279, 259, 0]);
    assert_eq!(gains, StatGains([1, 0, 0, 0, 0, 1, 1]));
    assert_eq!(totals, [280, 260, 235]);
}

#[test]
fn net_is_limited_by_the_eligible_stats() {
    // Only Res can grow: the net wants 3 but gives 1.
    let class = custom("c", 3, [0, 0, 0, 0, 0, 0, 20], [40, 20, 10, 24, 25, 18, 15]);
    let u = fresh();
    let (gains, totals) = roll(&u, &class, 3, [99; 7], &[5]);
    assert_eq!(gains, StatGains([0, 0, 0, 0, 0, 0, 1]));
    assert_eq!(totals, [20]);
    // Nothing can grow: no gains, no draws.
    let capped = unit_in("swordsman", [40, 20, 10, 24, 25, 18, 15]);
    let (gains, totals) = roll(&capped, &swordsman(), 3, [0; 7], &[]);
    assert_eq!(gains, StatGains::default());
    assert!(totals.is_empty());
}

#[test]
fn tier_tables_fall_back_to_the_highest_tier() {
    let t = table();
    assert_eq!(min_gains(&t, 1), 2);
    assert_eq!(min_gains(&t, 3), 3);
    assert_eq!(min_gains(&t, 6), 3);
    assert_eq!(cp_per_class_level(&t, 2), 17);
    assert_eq!(cp_per_class_level(&t, 9), 25);
    let empty = ClassTable::default();
    assert_eq!(min_gains(&empty, 1), 0);
    assert_eq!(cp_per_class_level(&empty, 1), 0);
}

#[test]
fn apply_gains_raises_stats_and_current_hp() {
    let mut u = fresh();
    u.hp = 10;
    apply_gains(&mut u, &StatGains([2, 1, 0, 0, 0, 0, 1]));
    assert_eq!(u.stats, Stats::from_growable([20, 6, 0, 7, 8, 3, 2], 5));
    assert_eq!(u.hp, 12);
}

// ---- grant_exp -----------------------------------------------------------------

#[test]
fn exp_below_100_levels_nothing() {
    let mut u = fresh();
    let mut rng = ScriptedRng::new([]);
    let events = grant_exp(&mut u, 60, &table(), &mut rng);
    assert_eq!(
        events,
        [Event::ExpGained {
            unit: UnitId(1),
            amount: 60
        }]
    );
    assert_eq!((u.level, u.exp), (1, 60));
}

#[test]
fn crossing_100_levels_up_and_carries_over() {
    let mut u = fresh();
    u.exp = 60;
    u.hp = 10;
    let mut rng = ScriptedRng::new([10, 20, 50, 30, 77, 30, 40]);
    let events = grant_exp(&mut u, 60, &table(), &mut rng);
    let gains = StatGains([1, 1, 0, 1, 0, 0, 0]);
    assert_eq!(
        events,
        [
            Event::ExpGained {
                unit: UnitId(1),
                amount: 60
            },
            Event::LeveledUp {
                unit: UnitId(1),
                level: 2,
                gains
            },
        ]
    );
    assert_eq!((u.level, u.exp), (2, 20));
    assert_eq!(u.stats, Stats::from_growable([19, 6, 0, 8, 8, 3, 1], 5));
    assert_eq!(u.hp, 11);
}

#[test]
fn a_big_award_gives_several_level_ups_each_rolled() {
    let mut u = fresh();
    let mut rng = ScriptedRng::new([0; 21]);
    let events = grant_exp(&mut u, 250, &table(), &mut rng);
    let levels: Vec<Level> = events
        .iter()
        .filter_map(|e| match e {
            Event::LeveledUp { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, [2, 3]);
    assert_eq!((u.level, u.exp), (3, 50));
    assert_eq!(rng.consumed(), 14);
}

#[test]
fn the_level_cap_stops_exp() {
    let mut t = table();
    t.level_cap = 5;
    let mut u = fresh();
    u.level = 4;
    u.exp = 70;
    let mut rng = ScriptedRng::new([99; 7]).with_below([0, 0]);
    let events = grant_exp(&mut u, 80, &t, &mut rng);
    // Cut to the 30 that reaching level 5 takes; EXP is then 0 (`--`).
    assert_eq!(
        events.first(),
        Some(&Event::ExpGained {
            unit: UnitId(1),
            amount: 30
        })
    );
    assert_eq!(events.len(), 2);
    assert_eq!((u.level, u.exp), (5, 0));
    // At the cap: nothing, not even an event.
    assert_eq!(grant_exp(&mut u, 50, &t, &mut rng), []);
    assert_eq!((u.level, u.exp), (5, 0));
    // A zero award and an unknown class give nothing either.
    let mut v = fresh();
    assert_eq!(grant_exp(&mut v, 0, &t, &mut rng), []);
    v.class = ClassId("nope".into());
    assert_eq!(grant_exp(&mut v, 50, &t, &mut rng), []);
    assert_eq!(v.exp, 0);
}

#[test]
fn an_award_below_the_cap_is_not_cut() {
    let mut t = table();
    t.level_cap = 5;
    let mut u = fresh();
    u.level = 3;
    u.exp = 50;
    let mut rng = ScriptedRng::new([0; 7]);
    // 150 EXP of room: all 100 count.
    let events = grant_exp(&mut u, 100, &t, &mut rng);
    assert_eq!(
        events[0],
        Event::ExpGained {
            unit: UnitId(1),
            amount: 100
        }
    );
    assert_eq!((u.level, u.exp), (4, 50));
}

#[test]
fn a_huge_award_near_a_huge_cap_does_not_overflow() {
    let mut t = table();
    t.level_cap = Level::MAX;
    let mut u = fresh();
    u.level = Level::MAX - 1;
    u.exp = 99;
    let mut rng = ScriptedRng::new([99; 7]).with_below([0, 0]);
    let events = grant_exp(&mut u, u32::MAX, &t, &mut rng);
    assert_eq!(events.len(), 2);
    assert_eq!((u.level, u.exp), (Level::MAX, 0));
}

#[test]
fn a_level_up_teaches_personal_spells() {
    let mut u = fresh();
    u.personal_spells = vec![(2, SpellId::new("gale"))];
    // A spell from outside the lists (chapter data) is kept.
    u.learned.insert(SpellId::new("secret"));
    let mut rng = ScriptedRng::new([0; 7]);
    let events = grant_exp(&mut u, 100, &table(), &mut rng);
    assert_eq!(
        events.last(),
        Some(&Event::SpellLearned {
            unit: UnitId(1),
            spell: SpellId::new("gale")
        })
    );
    assert!(u.learned.contains(&SpellId::new("gale")));
    assert!(u.learned.contains(&SpellId::new("secret")));
    assert!(u.learned.contains(&SpellId::new("spark")));
}

// ---- Class points ----------------------------------------------------------------

fn class_level(u: &Unit, class: &str) -> Option<(ClassLevel, ClassPoints)> {
    u.class_records
        .get(&ClassId(class.into()))
        .map(|r| (r.class_level, r.class_points))
}

#[test]
fn class_points_go_to_the_current_class_only() {
    let mut u = fresh();
    u.class_records.insert(
        ClassId("adept".into()),
        ClassRecord {
            class_level: 2,
            class_points: 30,
        },
    );
    let events = grant_class_points(&mut u, 4, &table(), &skills());
    assert_eq!(
        events,
        [Event::ClassPointsGained {
            unit: UnitId(1),
            class: ClassId("swordsman".into()),
            amount: 4
        }]
    );
    assert_eq!(class_level(&u, "swordsman"), Some((1, 4)));
    assert_eq!(class_level(&u, "adept"), Some((2, 30)));
}

#[test]
fn class_levels_follow_the_tier_table_and_teach_spells() {
    let mut u = fresh();
    let t = table();
    // 10 CP per level at tier 1: 9 CP is still level 1; 10 is level 2.
    grant_class_points(&mut u, 9, &t, &skills());
    assert_eq!(class_level(&u, "swordsman"), Some((1, 9)));
    let events = grant_class_points(&mut u, 1, &t, &skills());
    assert_eq!(
        events[1],
        Event::ClassLeveledUp {
            unit: UnitId(1),
            class: ClassId("swordsman".into()),
            class_level: 2
        }
    );
    // 40 more: 50 CP = level 6, passing level 5's spell (Frost).
    let events = grant_class_points(&mut u, 40, &t, &skills());
    let summary: Vec<String> = events.iter().map(|e| format!("{e:?}")).collect();
    assert_eq!(events.len(), 1 + 4 + 1, "{summary:#?}");
    assert_eq!(
        events[4],
        Event::SpellLearned {
            unit: UnitId(1),
            spell: SpellId::new("frost")
        }
    );
    assert_eq!(class_level(&u, "swordsman"), Some((6, 50)));
    // Tier 3 needs 25 per level.
    let mut a = unit_in("adept", [18, 5, 0, 7, 8, 3, 1]);
    grant_class_points(&mut a, 49, &t, &skills());
    assert_eq!(class_level(&a, "adept"), Some((2, 49)));
}

#[test]
fn mastery_teaches_the_passives_and_stops_the_points() {
    let mut u = fresh();
    u.learned_skills.insert(SkillId::new("sword_focus_2"));
    let t = table();
    let events = grant_class_points(&mut u, 500, &t, &skills());
    // Cut at mastery's 90 CP; levels 2..=10, then mastery. Sword Focus 1
    // is below the known rank 2, so only Parry is learned.
    assert_eq!(
        events.first(),
        Some(&Event::ClassPointsGained {
            unit: UnitId(1),
            class: ClassId("swordsman".into()),
            amount: 90
        })
    );
    let tail = &events[events.len() - 2..];
    assert_eq!(
        tail,
        [
            Event::ClassMastered {
                unit: UnitId(1),
                class: ClassId("swordsman".into())
            },
            Event::SkillLearned {
                unit: UnitId(1),
                skill: SkillId::new("parry")
            },
        ]
    );
    assert_eq!(class_level(&u, "swordsman"), Some((10, 90)));
    assert!(u.learned_skills.contains(&SkillId::new("parry")));
    // Mastered: nothing more.
    assert_eq!(grant_class_points(&mut u, 5, &t, &skills()), []);
    assert_eq!(class_level(&u, "swordsman"), Some((10, 90)));
}

#[test]
fn mastery_learns_every_passive_when_none_is_known() {
    let mut u = fresh();
    let events = grant_class_points(&mut u, 90, &table(), &skills());
    let learned: Vec<&SkillId> = events
        .iter()
        .filter_map(|e| match e {
            Event::SkillLearned { skill, .. } => Some(skill),
            _ => None,
        })
        .collect();
    assert_eq!(
        learned,
        [&SkillId::new("sword_focus_1"), &SkillId::new("parry")]
    );
}

#[test]
fn class_points_with_nothing_to_give() {
    let t = table();
    let mut u = fresh();
    assert_eq!(grant_class_points(&mut u, 0, &t, &skills()), []);
    // A class missing from its records is unlocked first.
    u.class_records.clear();
    grant_class_points(&mut u, 3, &t, &skills());
    assert_eq!(class_level(&u, "swordsman"), Some((1, 3)));
    // No CP table: nothing.
    let mut empty = t.clone();
    empty.cp_per_class_level.clear();
    assert_eq!(grant_class_points(&mut u, 3, &empty, &skills()), []);
    // Unknown class: nothing.
    u.class = ClassId("nope".into());
    assert_eq!(grant_class_points(&mut u, 3, &t, &skills()), []);
}

// ---- Properties and statistics ---------------------------------------------------

fn arb_class() -> impl Strategy<Value = ClassDef> {
    (
        1..=4u8,
        prop::array::uniform7(0..=250u16),
        prop::array::uniform7(1..=60i32),
    )
        .prop_map(|(tier, growths, caps)| custom("c", tier, growths, caps))
}

proptest! {
    #[test]
    fn level_ups_respect_caps_growths_and_the_net(
        class in arb_class(),
        start in prop::array::uniform7(0..=70i32),
        talent in prop::option::of(0..7usize),
        seed in any::<u64>(),
        levels in 1..40usize,
    ) {
        let mut t = table();
        t.classes = BTreeMap::from([(class.id.clone(), class.clone())]);
        t.level_cap = 30;
        let mut u = unit_in("swordsman", start);
        u.class = class.id.clone();
        u.talent = talent.map(|i| StatKind::GROWABLE[i]);
        let mut rng = SimRng::new(seed);
        let floor = min_gains(&t, class.tier);
        for _ in 0..levels {
            let before = u.stats;
            let events = grant_exp(&mut u, 100, &t, &mut rng);
            let gained = events.iter().find_map(|e| match e {
                Event::LeveledUp { gains, .. } => Some(*gains),
                _ => None,
            });
            prop_assert!(u.level <= t.level_cap);
            let Some(gains) = gained else {
                prop_assert_eq!(u.level, t.level_cap);
                continue;
            };
            let mut eligible = 0;
            for (i, &kind) in StatKind::GROWABLE.iter().enumerate() {
                let g = growth(&u, &class, kind);
                let cap = class.caps.get(kind);
                let (old, new) = (before.get(kind), u.stats.get(kind));
                prop_assert_eq!(new - old, gains.0[i]);
                prop_assert!(gains.0[i] >= 0);
                if old >= cap || g == 0 {
                    prop_assert_eq!(new, old, "{:?} can't grow", kind);
                } else {
                    eligible += 1;
                    prop_assert!(new <= cap, "{:?} past its cap", kind);
                    prop_assert!(gains.0[i] <= StatValue::from(g.div_ceil(100)));
                }
            }
            prop_assert!(gains.count() >= usize::from(floor).min(eligible));
        }
    }

    #[test]
    fn class_levels_never_pass_the_cap(
        awards in prop::collection::vec(0..60u32, 1..40),
        tier in 1..=5u8,
    ) {
        let mut t = table();
        if let Some(c) = t.classes.get_mut(&ClassId("swordsman".into())) {
            c.tier = tier;
        }
        let mut u = fresh();
        let per = cp_per_class_level(&t, tier);
        for cp in awards {
            grant_class_points(&mut u, cp, &t, &skills());
            let (level, points) = class_level(&u, "swordsman").unwrap_or((0, 0));
            prop_assert!(level <= t.class_level_cap);
            prop_assert!(points <= per * 9);
            let expected = ClassLevel::try_from(1 + points / per).unwrap_or(10).min(10);
            prop_assert_eq!(level, expected);
        }
    }
}

/// Average gain per stat over `n` level ups of `unit` in `class` (never
/// applied, so caps don't interfere).
fn average_gains(unit: &Unit, class: &ClassDef, min: u8, n: u32) -> [f64; 7] {
    let mut rng = SimRng::new(20_260_928);
    let mut totals: [StatValue; 7] = [0; 7];
    for _ in 0..n {
        let gains = level_up(unit, class, min, &mut rng);
        for (t, g) in totals.iter_mut().zip(gains.0) {
            *t += g;
        }
    }
    totals.map(|t| f64::from(t) / f64::from(n))
}

#[test]
fn without_a_net_average_gains_match_the_growths() {
    let class = custom("c", 1, [70, 40, 10, 55, 130, 25, 0], [99; 7]);
    let u = unit_in("swordsman", [1; 7]);
    let avg = average_gains(&u, &class, 0, 10_000);
    for (i, &kind) in StatKind::GROWABLE.iter().enumerate() {
        let expected = f64::from(growth(&u, &class, kind)) / 100.0;
        assert!(
            (avg[i] - expected).abs() <= 0.02,
            "{kind:?}: {} vs {expected}",
            avg[i]
        );
    }
}

#[test]
fn with_the_real_net_averages_are_at_least_the_growths() {
    for tier in [1, 3] {
        let class = ClassDef {
            tier,
            ..swordsman()
        };
        let u = unit_in("swordsman", [1; 7]);
        let min = min_gains(&table(), tier);
        let avg = average_gains(&u, &class, min, 10_000);
        let mut total = 0.0;
        for (i, &kind) in StatKind::GROWABLE.iter().enumerate() {
            let g = f64::from(growth(&u, &class, kind)) / 100.0;
            assert!(
                avg[i] >= g - 0.02,
                "tier {tier} {kind:?}: {} vs {g}",
                avg[i]
            );
            total += avg[i];
        }
        // The net lifts the total above the growths' 2.8.
        assert!(total > 2.8 + 0.02, "tier {tier}: {total}");
    }
}
