//! Tests for combat maths: the worked examples of the design docs, one test
//! per rule, property tests and the 2RN statistics.

use super::*;
use crate::rng::{ScriptedRng, SimRng};

use proptest::prelude::*;

static PLAIN: TerrainRules = terrain(0, 0);
static FOREST: TerrainRules = terrain(1, 20);
static FORT: TerrainRules = terrain(2, 20);

const fn terrain(defense: i8, avoid: i8) -> TerrainRules {
    TerrainRules {
        name: String::new(),
        move_cost: Vec::new(),
        defense,
        avoid,
        heal_percent: 0,
    }
}

fn rules() -> CombatRules {
    CombatRules::default()
}

/// Stats in design-table order: HP, Str, Mag, Dex, Spd, Def, Res.
fn stats(v: [StatValue; 7]) -> Stats {
    Stats::from_growable(v, 5)
}

/// The typeless placeholder weapons of `stats-and-combat.md`.
fn typeless(
    damage_type: DamageType,
    might: StatValue,
    hit: StatValue,
    crit: StatValue,
    range: (u32, u32),
) -> WeaponStats {
    WeaponStats {
        kind: None,
        trait_: WeaponTrait::None,
        might,
        hit,
        crit,
        weight: 0,
        min_range: range.0,
        max_range: range.1,
        damage_type,
        effective: Vec::new(),
        broken: false,
        element: Element::None,
    }
}

/// A starter weapon from `weapons-and-items.md`.
fn weapon(kind: WeaponKind) -> WeaponStats {
    let (might, hit, crit, weight, range) = match kind {
        WeaponKind::Sword => (5, 90, 0, 2, (1, 1)),
        WeaponKind::Spear => (7, 80, 0, 4, (1, 1)),
        WeaponKind::Axe => (8, 75, 0, 6, (1, 1)),
        WeaponKind::Bow => (6, 85, 0, 3, (2, 2)),
        WeaponKind::Gauntlet => (3, 95, 5, 1, (1, 1)),
    };
    WeaponStats {
        kind: Some(kind),
        trait_: rules().type_trait(kind),
        weight,
        ..typeless(DamageType::Physical, might, hit, crit, range)
    }
}

/// A starter attack spell from `magic.md`.
fn spell(element: Element, might: StatValue, hit: StatValue) -> WeaponStats {
    WeaponStats {
        element,
        ..typeless(DamageType::Magical, might, hit, 0, (1, 2))
    }
}

fn unit(s: Stats, weapon: Option<WeaponStats>, terrain: &TerrainRules) -> CombatantInput<'_> {
    CombatantInput {
        stats: s,
        tags: UnitTags::default(),
        affinities: Vec::new(),
        weapon,
        weapon_rank: WeaponRank::E,
        armour_weight: 0,
        terrain,
    }
}

fn numbers(
    damage: StatValue,
    followup_damage: StatValue,
    hit: u8,
    crit: u8,
    strikes: u8,
) -> SideForecast {
    SideForecast {
        damage,
        followup_damage,
        hit,
        crit,
        strikes,
        effective: false,
        broken: false,
        affinity: None,
    }
}

fn hp(current: StatValue, max: StatValue) -> CombatHp {
    CombatHp { current, max }
}

fn fc(rules: &CombatRules, a: &CombatantInput, d: &CombatantInput, distance: u32) -> Forecast {
    forecast(rules, a, d, distance).expect("attacker can attack")
}

// ---------------------------------------------------------------------------
// Worked examples.

fn ex1_attacker() -> CombatantInput<'static> {
    let w = typeless(DamageType::Physical, 5, 90, 0, (1, 1));
    unit(stats([22, 8, 0, 7, 9, 5, 2]), Some(w), &PLAIN)
}

fn ex1_defender() -> CombatantInput<'static> {
    let w = typeless(DamageType::Physical, 8, 75, 0, (1, 1));
    unit(stats([20, 9, 0, 3, 5, 4, 1]), Some(w), &PLAIN)
}

fn ex2_attacker() -> CombatantInput<'static> {
    let w = typeless(DamageType::Magical, 5, 85, 0, (1, 2));
    unit(stats([18, 1, 9, 6, 7, 2, 6]), Some(w), &PLAIN)
}

fn ex2_defender() -> CombatantInput<'static> {
    let w = typeless(DamageType::Physical, 7, 80, 0, (1, 1));
    unit(stats([25, 10, 0, 4, 3, 11, 2]), Some(w), &FOREST)
}

fn ex3_attacker(spd: StatValue) -> CombatantInput<'static> {
    let w = typeless(DamageType::Physical, 3, 100, 10, (1, 1));
    unit(stats([34, 20, 0, 22, spd, 10, 8]), Some(w), &PLAIN)
}

fn ex3_defender(spd: StatValue) -> CombatantInput<'static> {
    let w = typeless(DamageType::Physical, 8, 75, 0, (1, 1));
    unit(stats([40, 18, 0, 10, spd, 12, 5]), Some(w), &FORT)
}

fn w1() -> (CombatantInput<'static>, CombatantInput<'static>) {
    (
        unit(
            stats([22, 8, 0, 7, 9, 5, 2]),
            Some(weapon(WeaponKind::Sword)),
            &PLAIN,
        ),
        unit(
            stats([20, 9, 0, 3, 5, 4, 1]),
            Some(weapon(WeaponKind::Axe)),
            &PLAIN,
        ),
    )
}

/// W2; `soldier_def` is the soldier's gear-adjusted Def (5 + Leather Vest 1).
fn w2(soldier_def: StatValue) -> (CombatantInput<'static>, CombatantInput<'static>) {
    let soldier = unit(
        stats([20, 7, 0, 5, 6, soldier_def, 0]),
        Some(weapon(WeaponKind::Spear)),
        &PLAIN,
    );
    let mut cavalier = unit(
        stats([24, 8, 0, 6, 7, 7, 0]),
        Some(weapon(WeaponKind::Axe)),
        &PLAIN,
    );
    cavalier.weapon_rank = WeaponRank::C;
    cavalier.tags = UnitTags::from_tags(&[UnitTag::Mounted]);
    (soldier, cavalier)
}

fn w3() -> (CombatantInput<'static>, CombatantInput<'static>) {
    let archer = unit(
        stats([18, 6, 0, 8, 7, 4, 0]),
        Some(weapon(WeaponKind::Bow)),
        &PLAIN,
    );
    let mut flier = unit(
        stats([18, 7, 0, 7, 12, 4, 0]),
        Some(weapon(WeaponKind::Spear)),
        &FOREST,
    );
    flier.tags = UnitTags::from_tags(&[UnitTag::Flying]);
    (archer, flier)
}

fn w4() -> (CombatantInput<'static>, CombatantInput<'static>) {
    let axe = WeaponStats {
        broken: true,
        ..weapon(WeaponKind::Axe)
    };
    (
        unit(stats([26, 10, 0, 5, 6, 5, 0]), Some(axe), &PLAIN),
        unit(
            stats([22, 7, 0, 8, 10, 3, 0]),
            Some(weapon(WeaponKind::Gauntlet)),
            &PLAIN,
        ),
    )
}

fn mage(attack: WeaponStats) -> CombatantInput<'static> {
    unit(stats([18, 1, 9, 6, 7, 2, 6]), Some(attack), &PLAIN)
}

fn frost_elemental(ice: Affinity) -> CombatantInput<'static> {
    let mut e = unit(
        stats([30, 0, 8, 4, 5, 8, 4]),
        Some(spell(Element::Ice, 4, 95)),
        &PLAIN,
    );
    e.affinities = vec![(Element::Fire, Affinity::Weak), (Element::Ice, ice)];
    e
}

#[test]
#[allow(clippy::too_many_lines)]
fn worked_examples() {
    let effective = |s: SideForecast| SideForecast {
        effective: true,
        ..s
    };
    let with = |s: SideForecast, affinity| SideForecast {
        affinity: Some(affinity),
        ..s
    };
    let cases: Vec<(&str, CombatantInput, CombatantInput, u32, Forecast)> = vec![
        (
            "stats 1: plain melee, attacker doubles at the threshold",
            ex1_attacker(),
            ex1_defender(),
            1,
            Forecast {
                attacker: numbers(9, 9, 94, 3, 2),
                defender: Some(numbers(12, 12, 63, 0, 1)),
            },
        ),
        (
            "stats 1 boundary: defender Spd 6",
            ex1_attacker(),
            unit(stats([20, 9, 0, 3, 6, 4, 1]), ex1_defender().weapon, &PLAIN),
            1,
            Forecast {
                attacker: numbers(9, 9, 92, 3, 1),
                defender: Some(numbers(12, 12, 63, 0, 1)),
            },
        ),
        (
            "stats 2: magic at range into a forest, no counter",
            ex2_attacker(),
            ex2_defender(),
            2,
            Forecast {
                attacker: numbers(11, 11, 71, 2, 2),
                defender: None,
            },
        ),
        (
            "stats 2b: same units adjacent, the knight counters",
            ex2_attacker(),
            ex2_defender(),
            1,
            Forecast {
                attacker: numbers(11, 11, 71, 2, 2),
                defender: Some(numbers(15, 15, 74, 1, 1)),
            },
        ),
        (
            "stats 3: the rare triple",
            ex3_attacker(30),
            ex3_defender(16),
            1,
            Forecast {
                attacker: numbers(9, 9, 92, 19, 3),
                defender: Some(numbers(16, 16, 35, 0, 1)),
            },
        ),
        (
            "stats 3 boundary: defender Spd 17 → 2 strikes",
            ex3_attacker(30),
            ex3_defender(17),
            1,
            Forecast {
                attacker: numbers(9, 9, 90, 19, 2),
                defender: Some(numbers(16, 16, 35, 0, 1)),
            },
        ),
        (
            "stats 3 boundary: attacker Spd 40 → 4 strikes",
            ex3_attacker(40),
            ex3_defender(16),
            1,
            Forecast {
                attacker: numbers(9, 9, 92, 19, 4),
                defender: Some(numbers(16, 16, 15, 0, 1)),
            },
        ),
        (
            "W1: sword follow-up vs. axe",
            w1().0,
            w1().1,
            1,
            Forecast {
                attacker: numbers(9, 10, 94, 3, 2),
                defender: Some(numbers(12, 12, 63, 0, 1)),
            },
        ),
        (
            "W2: spear vs. cavalry",
            w2(6).0,
            w2(6).1,
            1,
            Forecast {
                attacker: effective(numbers(14, 14, 76, 1, 1)),
                defender: Some(numbers(10, 10, 75, 2, 1)),
            },
        ),
        (
            "W2b: axe minimum damage",
            w2(20).0,
            w2(20).1,
            1,
            Forecast {
                attacker: effective(numbers(14, 14, 76, 1, 1)),
                defender: Some(numbers(5, 5, 75, 2, 1)),
            },
        ),
        (
            "W3: bow vs. flyer at range, no counter",
            w3().0,
            w3().1,
            2,
            Forecast {
                attacker: effective(numbers(20, 20, 77, 3, 1)),
                defender: None,
            },
        ),
        (
            "W4: gauntlet avoid, broken weapon",
            w4().0,
            w4().1,
            1,
            Forecast {
                attacker: SideForecast {
                    broken: true,
                    ..numbers(11, 11, 30, 0, 1)
                },
                defender: Some(numbers(5, 5, 99, 8, 2)),
            },
        ),
        (
            "M1: Fire into a weak Frost Elemental",
            mage(spell(Element::Fire, 5, 90)),
            frost_elemental(Affinity::Absorb),
            2,
            Forecast {
                attacker: with(effective(numbers(20, 20, 92, 2, 1)), Affinity::Weak),
                defender: Some(numbers(6, 6, 89, 1, 1)),
            },
        ),
        (
            "M2: Frost into the elemental (absorb)",
            mage(spell(Element::Ice, 4, 95)),
            frost_elemental(Affinity::Absorb),
            2,
            Forecast {
                attacker: with(numbers(9, 9, 97, 0, 1), Affinity::Absorb),
                defender: Some(numbers(6, 6, 89, 1, 1)),
            },
        ),
        (
            "M2 variant: Ice Resist",
            mage(spell(Element::Ice, 4, 95)),
            frost_elemental(Affinity::Resist),
            2,
            Forecast {
                attacker: with(numbers(4, 4, 97, 2, 1), Affinity::Resist),
                defender: Some(numbers(6, 6, 89, 1, 1)),
            },
        ),
    ];
    let rules = rules();
    for (name, a, d, distance, expected) in &cases {
        assert_eq!(
            forecast(&rules, a, d, *distance).as_ref(),
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn example_1_scripted_trace() {
    let rules = rules();
    let f = fc(&rules, &ex1_attacker(), &ex1_defender(), 1);
    let mut rng = ScriptedRng::new([10, 20, 50, 70, 80, 0, 0, 99]);
    let out = resolve(&rules, &f, hp(22, 22), hp(20, 20), &mut rng);
    let strike = |by, hit, damage, target_hp_after| Strike {
        by,
        hit,
        crit: false,
        damage,
        healed: false,
        target_hp_after,
    };
    assert_eq!(
        out,
        CombatOutcome {
            strikes: vec![
                strike(Side::Attacker, true, 9, 11),
                strike(Side::Defender, false, 0, 22),
                strike(Side::Attacker, true, 9, 2),
            ],
            attacker_hp: 22,
            defender_hp: 2,
        }
    );
    assert_eq!(rng.consumed(), 8);
}

#[test]
fn w1_second_strike_crit_deals_30() {
    let rules = rules();
    let (a, d) = w1();
    let f = fc(&rules, &a, &d, 1);
    // Attacker hits, no crit; defender misses; attacker hits and crits.
    let mut rng = ScriptedRng::new([0, 0, 99, 99, 99, 0, 0, 0]);
    let out = resolve(&rules, &f, hp(22, 22), hp(40, 40), &mut rng);
    let damages: Vec<_> = out
        .strikes
        .iter()
        .map(|s| (s.by, s.damage, s.crit))
        .collect();
    assert_eq!(
        damages,
        [
            (Side::Attacker, 9, false),
            (Side::Defender, 0, false),
            (Side::Attacker, 30, true)
        ]
    );
    assert_eq!(out.defender_hp, 40 - 9 - 30);
}

#[test]
fn m2_absorb_heals_up_to_max() {
    let rules = rules();
    let f = fc(
        &rules,
        &mage(spell(Element::Ice, 4, 95)),
        &frost_elemental(Affinity::Absorb),
        2,
    );
    for (start, after) in [(20, 29), (25, 30), (30, 30)] {
        // Mage hits (crit roll 0 still can't crit), elemental misses.
        let mut rng = ScriptedRng::new([0, 0, 0, 99, 99]);
        let out = resolve(&rules, &f, hp(18, 18), hp(start, 30), &mut rng);
        let first = out.strikes[0];
        assert!(first.hit && first.healed && !first.crit);
        assert_eq!(first.damage, 9);
        assert_eq!(first.target_hp_after, after);
        assert_eq!(out.defender_hp, after);
    }
}

// ---------------------------------------------------------------------------
// Weapon traits.

/// Everything hits, nothing crits (crit rolls 99 ≥ any crit below 100).
fn all_hits(strikes: usize) -> ScriptedRng {
    ScriptedRng::new([0, 0, 99].repeat(strikes))
}

#[test]
fn sword_followup_only_on_strikes_after_the_first() {
    let rules = rules();
    let (a, d) = w1();
    let f = fc(&rules, &a, &d, 1);
    let out = resolve(&rules, &f, hp(22, 22), hp(40, 40), &mut all_hits(3));
    let damages: Vec<_> = out.strikes.iter().map(|s| (s.by, s.damage)).collect();
    assert_eq!(
        damages,
        [
            (Side::Attacker, 9),
            (Side::Defender, 12),
            (Side::Attacker, 10)
        ]
    );
}

#[test]
fn sword_followup_is_six_fifths_rounded_down() {
    let rules = rules();
    let mut sword = unit(
        stats([20, 20, 0, 0, 10, 0, 0]),
        Some(weapon(WeaponKind::Sword)),
        &PLAIN,
    );
    let target = unit(stats([20, 0, 0, 0, 0, 5, 0]), None, &PLAIN);
    // 20 + 5 − 5 = 20 → 24; 23 → 27 (27.6 rounded down).
    let f = fc(&rules, &sword, &target, 1).attacker;
    assert_eq!((f.damage, f.followup_damage), (20, 24));
    sword.stats.str = 23;
    let f = fc(&rules, &sword, &target, 1).attacker;
    assert_eq!((f.damage, f.followup_damage), (23, 27));
}

#[test]
fn defender_sword_followup_too() {
    let rules = rules();
    let (sword, axe) = w1();
    // The swordfighter defends against the slower axe fighter.
    let f = fc(&rules, &axe, &sword, 1);
    assert_eq!(
        f.defender.map(|d| (d.damage, d.followup_damage, d.strikes)),
        Some((9, 10, 2))
    );
    let out = resolve(&rules, &f, hp(40, 40), hp(22, 22), &mut all_hits(3));
    let damages: Vec<_> = out.strikes.iter().map(|s| (s.by, s.damage)).collect();
    assert_eq!(
        damages,
        [
            (Side::Attacker, 12),
            (Side::Defender, 9),
            (Side::Defender, 10)
        ]
    );
}

#[test]
fn spear_is_effective_against_mounted_but_not_flying() {
    let rules = rules();
    let spear = unit(
        stats([20, 7, 0, 5, 6, 5, 0]),
        Some(weapon(WeaponKind::Spear)),
        &PLAIN,
    );
    let mut target = unit(stats([20, 0, 0, 0, 0, 7, 0]), None, &PLAIN);
    // Plain: 7 + 7 − 7.
    assert_eq!(fc(&rules, &spear, &target, 1).attacker.damage, 7);
    target.tags = UnitTags::from_tags(&[UnitTag::Mounted]);
    let f = fc(&rules, &spear, &target, 1);
    assert_eq!((f.attacker.damage, f.attacker.effective), (14, true));
    target.tags = UnitTags::from_tags(&[UnitTag::Flying]);
    let f = fc(&rules, &spear, &target, 1);
    assert_eq!((f.attacker.damage, f.attacker.effective), (7, false));
}

#[test]
fn bow_is_effective_against_flying() {
    let rules = rules();
    let bow = unit(
        stats([20, 6, 0, 8, 7, 4, 0]),
        Some(weapon(WeaponKind::Bow)),
        &PLAIN,
    );
    let mut target = unit(stats([20, 0, 0, 0, 0, 4, 0]), None, &PLAIN);
    assert_eq!(fc(&rules, &bow, &target, 2).attacker.damage, 8);
    target.tags = UnitTags::from_tags(&[UnitTag::Mounted]);
    assert_eq!(fc(&rules, &bow, &target, 2).attacker.damage, 8);
    target.tags = UnitTags::from_tags(&[UnitTag::Flying]);
    assert_eq!(fc(&rules, &bow, &target, 2).attacker.damage, 20);
}

#[test]
fn bow_cannot_attack_or_counter_adjacent() {
    let rules = rules();
    let (archer, flier) = w3();
    assert_eq!(forecast(&rules, &archer, &flier, 1), None);
    let f = fc(&rules, &flier, &archer, 1);
    assert_eq!(f.defender, None);
}

#[test]
fn largest_effective_multiplier_wins_and_they_do_not_stack() {
    let rules = rules();
    let mut w = weapon(WeaponKind::Spear); // (Mounted, 2) from the trait
    w.effective = vec![(UnitTag::Armored, 3), (UnitTag::Flying, 5)];
    let a = unit(stats([20, 0, 0, 0, 0, 0, 0]), Some(w), &PLAIN);
    let mut target = unit(stats([20, 0, 0, 0, 0, 0, 0]), None, &PLAIN);
    let damage = |t: &CombatantInput| fc(&rules, &a, t, 1).attacker.damage;
    assert_eq!(damage(&target), 7);
    target.tags = UnitTags::from_tags(&[UnitTag::Mounted]);
    assert_eq!(damage(&target), 14);
    target.tags = UnitTags::from_tags(&[UnitTag::Mounted, UnitTag::Armored]);
    assert_eq!(damage(&target), 21);
    target.tags = UnitTags::from_tags(&[UnitTag::Armored]);
    assert_eq!(damage(&target), 21);
}

#[test]
fn trait_multiplier_wins_over_a_smaller_listed_one() {
    let rules = rules();
    let mut w = weapon(WeaponKind::Bow); // (Flying, 3)
    w.effective = vec![(UnitTag::Flying, 2)];
    let a = unit(stats([20, 0, 0, 0, 0, 0, 0]), Some(w), &PLAIN);
    let mut target = unit(stats([20, 0, 0, 0, 0, 0, 0]), None, &PLAIN);
    target.tags = UnitTags::from_tags(&[UnitTag::Flying]);
    assert_eq!(fc(&rules, &a, &target, 2).attacker.damage, 18);
}

#[test]
fn axe_minimum_only_on_a_hit_and_before_crit() {
    let rules = rules();
    let (soldier, cavalier) = w2(20);
    let f = fc(&rules, &cavalier, &soldier, 1);
    assert_eq!(f.attacker.damage, 5);
    // Miss (and the soldier misses back): the minimum doesn't apply.
    let mut rng = ScriptedRng::new([99, 99, 99, 99]);
    let out = resolve(&rules, &f, hp(24, 24), hp(20, 20), &mut rng);
    assert_eq!((out.strikes[0].hit, out.strikes[0].damage), (false, 0));
    assert_eq!(out.defender_hp, 20);
    // Crit: the minimum 5 is tripled.
    let mut rng = ScriptedRng::new([0, 0, 0, 99, 99]);
    let out = resolve(&rules, &f, hp(24, 24), hp(20, 20), &mut rng);
    assert_eq!((out.strikes[0].crit, out.strikes[0].damage), (true, 15));
    assert_eq!(out.defender_hp, 5);
}

#[test]
fn axe_minimum_does_not_lower_bigger_damage() {
    let rules = rules();
    let (fighter, brawler) = w4();
    assert_eq!(fc(&rules, &fighter, &brawler, 1).attacker.damage, 11);
}

#[test]
fn gauntlet_avoid_only_when_equipped() {
    let rules = rules();
    let (fighter, mut brawler) = w4();
    assert_eq!(fc(&rules, &fighter, &brawler, 1).attacker.hit, 30);
    brawler.weapon = Some(weapon(WeaponKind::Sword));
    assert_eq!(fc(&rules, &fighter, &brawler, 1).attacker.hit, 45);
    brawler.weapon = None;
    assert_eq!(fc(&rules, &fighter, &brawler, 1).attacker.hit, 45);
}

#[test]
fn broken_weapon_halves_might_and_loses_20_hit() {
    let rules = rules();
    let (mut fighter, brawler) = w4();
    let broken = fc(&rules, &fighter, &brawler, 1).attacker;
    fighter.weapon = Some(weapon(WeaponKind::Axe));
    let whole = fc(&rules, &fighter, &brawler, 1).attacker;
    assert_eq!((whole.damage, whole.hit, whole.broken), (15, 50, false));
    assert_eq!((broken.damage, broken.hit, broken.broken), (11, 30, true));
}

#[test]
fn broken_might_rounds_down() {
    let rules = rules();
    let mut w = weapon(WeaponKind::Sword); // Mt 5 → 2
    w.broken = true;
    let a = unit(stats([20, 0, 0, 0, 0, 0, 0]), Some(w), &PLAIN);
    let d = unit(stats([20, 0, 0, 0, 0, 0, 0]), None, &PLAIN);
    assert_eq!(fc(&rules, &a, &d, 1).attacker.damage, 2);
}

// ---------------------------------------------------------------------------
// Affinities.

#[test]
fn weak_is_times_three_unless_a_tag_multiplier_is_larger() {
    let rules = rules();
    let mut fire = spell(Element::Fire, 5, 90);
    let mut target = frost_elemental(Affinity::Absorb);
    target.stats.res = 0;
    // Weak ×3: 9 + 15.
    assert_eq!(
        fc(&rules, &mage(fire.clone()), &target, 2).attacker.damage,
        24
    );
    target.tags = UnitTags::from_tags(&[UnitTag::Mounted]);
    fire.effective = vec![(UnitTag::Mounted, 2)];
    assert_eq!(
        fc(&rules, &mage(fire.clone()), &target, 2).attacker.damage,
        24
    );
    fire.effective = vec![(UnitTag::Mounted, 4)];
    let f = fc(&rules, &mage(fire), &target, 2).attacker;
    assert_eq!(
        (f.damage, f.effective, f.affinity),
        (29, true, Some(Affinity::Weak))
    );
}

#[test]
fn resist_halves_before_crit() {
    let rules = rules();
    let f = fc(
        &rules,
        &mage(spell(Element::Ice, 4, 95)),
        &frost_elemental(Affinity::Resist),
        2,
    );
    assert_eq!(f.attacker.damage, 4);
    let mut rng = ScriptedRng::new([0, 0, 0, 99, 99]);
    let out = resolve(&rules, &f, hp(18, 18), hp(30, 30), &mut rng);
    assert_eq!((out.strikes[0].crit, out.strikes[0].damage), (true, 12));
    assert_eq!(out.defender_hp, 18);
}

#[test]
fn absorb_forces_crit_to_zero() {
    let rules = rules();
    let mut frost = spell(Element::Ice, 4, 95);
    frost.crit = 60;
    let f = fc(&rules, &mage(frost), &frost_elemental(Affinity::Absorb), 2);
    assert_eq!(
        (f.attacker.crit, f.attacker.affinity),
        (0, Some(Affinity::Absorb))
    );
}

#[test]
fn absorb_heal_on_a_miss_does_nothing() {
    let rules = rules();
    let f = fc(
        &rules,
        &mage(spell(Element::Ice, 4, 95)),
        &frost_elemental(Affinity::Absorb),
        2,
    );
    let mut rng = ScriptedRng::new([99, 99, 99, 99]);
    let out = resolve(&rules, &f, hp(18, 18), hp(20, 30), &mut rng);
    assert_eq!((out.strikes[0].hit, out.strikes[0].damage), (false, 0));
    assert_eq!(out.defender_hp, 20);
}

#[test]
fn physical_attacks_ignore_affinities() {
    let rules = rules();
    let mut w = typeless(DamageType::Physical, 5, 90, 0, (1, 2));
    w.element = Element::Fire;
    let a = unit(stats([20, 9, 0, 6, 7, 2, 6]), Some(w), &PLAIN);
    let target = frost_elemental(Affinity::Absorb);
    let f = fc(&rules, &a, &target, 2).attacker;
    assert_eq!((f.damage, f.effective, f.affinity), (6, false, None));
}

#[test]
fn element_none_spells_ignore_affinities() {
    let rules = rules();
    let mut target = frost_elemental(Affinity::Absorb);
    target.affinities.push((Element::None, Affinity::Absorb));
    let f = fc(&rules, &mage(spell(Element::None, 6, 80)), &target, 2).attacker;
    assert_eq!((f.damage, f.affinity), (11, None));
}

#[test]
fn magic_uses_mag_and_res() {
    let rules = rules();
    let a = unit(
        stats([20, 30, 9, 0, 0, 0, 0]),
        Some(spell(Element::None, 5, 90)),
        &PLAIN,
    );
    let d = unit(stats([20, 0, 0, 0, 0, 30, 4]), None, &FOREST);
    // 9 + 5 − (4 + 1): Str and Def don't count.
    assert_eq!(fc(&rules, &a, &d, 1).attacker.damage, 9);
}

// ---------------------------------------------------------------------------
// Attack speed.

fn speed(s: Stats, w: Option<WeaponStats>, rank: WeaponRank, armour: StatValue) -> StatValue {
    let mut u = unit(s, w, &PLAIN);
    u.weapon_rank = rank;
    u.armour_weight = armour;
    u.attack_speed(&rules())
}

#[test]
fn burden_boundaries() {
    let sword = || Some(weapon(WeaponKind::Sword)); // weight 2
    let with_str = |str_| stats([20, str_, 0, 0, 10, 0, 0]);
    // Str 10 carries exactly 2 weight.
    assert_eq!(speed(with_str(10), sword(), WeaponRank::E, 0), 10);
    // Str 9 carries only 1.
    assert_eq!(speed(with_str(9), sword(), WeaponRank::E, 0), 9);
    // Extra Str never makes you faster.
    assert_eq!(speed(with_str(40), sword(), WeaponRank::E, 0), 10);
    // Armour weight adds to the burden.
    assert_eq!(speed(with_str(10), sword(), WeaponRank::E, 2), 8);
    assert_eq!(speed(with_str(0), sword(), WeaponRank::E, 3), 5);
    // No weapon: weight 0, but armour still counts.
    assert_eq!(speed(with_str(0), None, WeaponRank::S, 3), 7);
}

#[test]
fn rank_speed_table() {
    let s = stats([20, 50, 0, 0, 10, 0, 0]);
    let got: Vec<_> = [
        WeaponRank::E,
        WeaponRank::D,
        WeaponRank::C,
        WeaponRank::B,
        WeaponRank::A,
        WeaponRank::S,
    ]
    .iter()
    .map(|&r| speed(s, Some(weapon(WeaponKind::Sword)), r, 0))
    .collect();
    assert_eq!(got, [10, 10, 11, 12, 13, 14]);
}

#[test]
fn spells_have_no_rank_speed_and_no_weight() {
    let s = stats([20, 5, 9, 0, 7, 0, 0]);
    assert_eq!(
        speed(s, Some(spell(Element::Fire, 5, 90)), WeaponRank::S, 0),
        7
    );
    // Heavy armour still slows a caster: 5 − 5/5.
    assert_eq!(
        speed(s, Some(spell(Element::Fire, 5, 90)), WeaponRank::S, 5),
        3
    );
}

#[test]
fn weapons_and_items_attack_speed_illustration() {
    // The illustration table of `weapons-and-items.md` (Str 10, rank B,
    // enemy AS 10).
    let rules = rules();
    let brawler = |spd| stats([20, 10, 0, 0, spd, 0, 0]);
    let cases = [
        (brawler(22), WeaponKind::Gauntlet, 0, 24, 3),
        (brawler(20), WeaponKind::Gauntlet, 0, 22, 2),
        (brawler(20), WeaponKind::Axe, 2, 16, 2),
    ];
    for (s, kind, armour, as_, strikes) in cases {
        let got = speed(s, Some(weapon(kind)), WeaponRank::B, armour);
        assert_eq!(got, as_, "{kind:?}");
        assert_eq!(rules.strikes(got - 10), strikes, "{kind:?}");
    }
    let steel_axe = WeaponStats {
        weight: 8,
        ..weapon(WeaponKind::Axe)
    };
    assert_eq!(speed(brawler(20), Some(steel_axe), WeaponRank::B, 2), 14);
}

// ---------------------------------------------------------------------------
// Strikes and order.

#[test]
fn strike_count_thresholds() {
    let rules = rules();
    let got: Vec<u8> = [-30, 0, 3, 4, 13, 14, 23, 24, 100]
        .iter()
        .map(|&d| rules.strikes(d))
        .collect();
    assert_eq!(got, [1, 1, 1, 2, 2, 3, 3, 4, 4]);
    assert_eq!(rules.max_strikes(), 4);
}

#[test]
fn thresholds_are_data() {
    let rules = CombatRules {
        strike_thresholds: vec![2],
        ..CombatRules::default()
    };
    assert_eq!(rules.strikes(2), 2);
    assert_eq!(rules.strikes(50), 2);
    assert_eq!(rules.max_strikes(), 2);
}

#[test]
fn triple_strike_order_is_a_d_a_a() {
    let rules = rules();
    let f = fc(&rules, &ex3_attacker(30), &ex3_defender(16), 1);
    let mut rng = ScriptedRng::new([99; 8]);
    let out = resolve(&rules, &f, hp(34, 34), hp(40, 40), &mut rng);
    let order: Vec<Side> = out.strikes.iter().map(|s| s.by).collect();
    assert_eq!(
        order,
        [
            Side::Attacker,
            Side::Defender,
            Side::Attacker,
            Side::Attacker
        ]
    );
    assert!(out.strikes.iter().all(|s| !s.hit));
    assert_eq!(rng.consumed(), 8);
}

#[test]
fn faster_defender_gets_the_extra_strikes() {
    let rules = rules();
    let (fighter, brawler) = w4();
    let f = fc(&rules, &fighter, &brawler, 1);
    let mut rng = ScriptedRng::new([99; 6]);
    let out = resolve(&rules, &f, hp(26, 26), hp(22, 22), &mut rng);
    let order: Vec<Side> = out.strikes.iter().map(|s| s.by).collect();
    assert_eq!(order, [Side::Attacker, Side::Defender, Side::Defender]);
}

#[test]
fn no_counter_means_only_attacker_strikes() {
    let rules = rules();
    let f = fc(&rules, &ex2_attacker(), &ex2_defender(), 2);
    let out = resolve(&rules, &f, hp(18, 18), hp(25, 25), &mut all_hits(2));
    let order: Vec<Side> = out.strikes.iter().map(|s| s.by).collect();
    assert_eq!(order, [Side::Attacker, Side::Attacker]);
    assert_eq!(out.defender_hp, 25 - 22);
}

#[test]
fn lethal_first_strike_ends_combat() {
    let rules = rules();
    let f = fc(&rules, &ex1_attacker(), &ex1_defender(), 1);
    let mut rng = all_hits(1);
    let out = resolve(&rules, &f, hp(22, 22), hp(9, 20), &mut rng);
    assert_eq!(out.strikes.len(), 1);
    assert_eq!((out.strikes[0].target_hp_after, out.defender_hp), (0, 0));
    assert_eq!(rng.consumed(), 3);
}

#[test]
fn lethal_counter_ends_combat() {
    let rules = rules();
    let f = fc(&rules, &ex1_attacker(), &ex1_defender(), 1);
    // Attacker misses; the counter kills.
    let mut rng = ScriptedRng::new([99, 99, 0, 0, 99]);
    let out = resolve(&rules, &f, hp(12, 22), hp(20, 20), &mut rng);
    assert_eq!(out.strikes.len(), 2);
    assert_eq!((out.attacker_hp, out.defender_hp), (0, 20));
    assert_eq!(rng.consumed(), 5);
}

#[test]
fn overkill_never_goes_below_zero() {
    let rules = rules();
    let f = fc(&rules, &ex1_attacker(), &ex1_defender(), 1);
    let out = resolve(&rules, &f, hp(22, 22), hp(3, 20), &mut all_hits(1));
    assert_eq!((out.strikes[0].damage, out.defender_hp), (9, 0));
}

#[test]
fn crit_rolls_only_on_a_hit_and_is_strictly_below() {
    let rules = rules();
    // Crit 3: roll 2 crits, roll 3 doesn't.
    let f = fc(&rules, &ex1_attacker(), &ex1_defender(), 1);
    let mut rng = ScriptedRng::new([0, 0, 2, 99, 99, 99, 99]);
    let out = resolve(&rules, &f, hp(22, 22), hp(40, 40), &mut rng);
    assert_eq!((out.strikes[0].crit, out.strikes[0].damage), (true, 27));
    let mut rng = ScriptedRng::new([0, 0, 3, 99, 99, 99, 99]);
    let out = resolve(&rules, &f, hp(22, 22), hp(40, 40), &mut rng);
    assert_eq!((out.strikes[0].crit, out.strikes[0].damage), (false, 9));
}

#[test]
fn no_attack_without_a_weapon_or_out_of_range() {
    let rules = rules();
    let unarmed = unit(stats([20, 9, 0, 3, 5, 4, 1]), None, &PLAIN);
    assert_eq!(forecast(&rules, &unarmed, &ex1_defender(), 1), None);
    assert_eq!(forecast(&rules, &ex1_attacker(), &ex1_defender(), 2), None);
    assert_eq!(forecast(&rules, &ex1_attacker(), &ex1_defender(), 0), None);
    assert_eq!(fc(&rules, &ex1_attacker(), &unarmed, 1).defender, None);
}

#[test]
fn fliers_get_no_terrain_bonus() {
    let rules = rules();
    let (archer, mut flier) = w3();
    flier.tags = UnitTags::default();
    let f = fc(&rules, &archer, &flier, 2).attacker;
    // Not flying: forest Def +1 and Avoid +20 apply (and no effectiveness).
    assert_eq!((f.damage, f.hit), (6 + 6 - (4 + 1), 85 + 16 - (24 + 20)));
}

// ---------------------------------------------------------------------------
// 2RN.

#[test]
fn two_rn_exact_probabilities() {
    let hits_of = |hit: u8| {
        let mut n = 0;
        for r1 in 0..100u8 {
            for r2 in 0..100u8 {
                let mut rng = ScriptedRng::new([r1, r2]);
                if roll_hit(&mut rng, hit) {
                    n += 1;
                }
                assert_eq!(rng.consumed(), 2);
            }
        }
        n
    };
    // The design's "true chances" (per 10 000).
    assert_eq!(hits_of(0), 0);
    assert_eq!(hits_of(10), 210);
    assert_eq!(hits_of(20), 820);
    assert_eq!(hits_of(50), 5050);
    assert_eq!(hits_of(80), 9220);
    assert_eq!(hits_of(90), 9810);
    assert_eq!(hits_of(100), 10_000);
}

#[test]
fn displayed_80_hits_about_92_percent() {
    let mut rng = SimRng::new(80);
    let hits = (0..100_000).filter(|_| roll_hit(&mut rng, 80)).count();
    // 92.2% ± 1%.
    assert!((91_200..=93_200).contains(&hits), "{hits} of 100 000");
}

// ---------------------------------------------------------------------------
// Properties.

#[derive(Debug, Clone)]
struct Spec {
    stats: [StatValue; 7],
    kind: Option<WeaponKind>,
    magical: bool,
    element: Element,
    might: StatValue,
    hit: StatValue,
    crit: StatValue,
    broken: bool,
    tags: UnitTags,
    affinities: Vec<(Element, Affinity)>,
    rank: WeaponRank,
    armour: StatValue,
    terrain: usize,
    armed: bool,
}

fn spec() -> impl Strategy<Value = Spec> {
    let kinds = prop_oneof![
        Just(None),
        Just(Some(WeaponKind::Sword)),
        Just(Some(WeaponKind::Spear)),
        Just(Some(WeaponKind::Axe)),
        Just(Some(WeaponKind::Bow)),
        Just(Some(WeaponKind::Gauntlet)),
    ];
    let element = prop_oneof![Just(Element::Fire), Just(Element::Ice), Just(Element::None)];
    let affinity = prop_oneof![
        Just(Affinity::Weak),
        Just(Affinity::Resist),
        Just(Affinity::Absorb)
    ];
    let rank = prop_oneof![
        Just(WeaponRank::E),
        Just(WeaponRank::D),
        Just(WeaponRank::C),
        Just(WeaponRank::B),
        Just(WeaponRank::A),
        Just(WeaponRank::S),
    ];
    (
        (1..=80, prop::array::uniform6(0..=50)),
        (kinds, any::<bool>(), element.clone()),
        (0..=25, 0..=130, 0..=60, any::<bool>()),
        (
            prop::array::uniform3(any::<bool>()),
            prop::collection::vec((element, affinity), 0..3),
        ),
        (rank, 0..=5, 0..3usize, prop::bool::weighted(0.9)),
    )
        .prop_map(
            |(
                (hp, s),
                (kind, magical, element),
                (might, hit, crit, broken),
                (t, affinities),
                (rank, armour, terrain, armed),
            )| Spec {
                stats: [hp, s[0], s[1], s[2], s[3], s[4], s[5]],
                kind,
                magical,
                element,
                might,
                hit,
                crit,
                broken,
                tags: UnitTags {
                    mounted: t[0],
                    flying: t[1],
                    armored: t[2],
                },
                affinities,
                rank,
                armour,
                terrain,
                armed,
            },
        )
}

fn build(s: &Spec) -> CombatantInput<'static> {
    let terrain = [&PLAIN, &FOREST, &FORT][s.terrain];
    let weapon = s.armed.then(|| match s.kind {
        Some(kind) => WeaponStats {
            might: s.might,
            hit: s.hit,
            crit: s.crit,
            broken: s.broken,
            min_range: 1,
            max_range: 2,
            ..weapon(kind)
        },
        None => WeaponStats {
            element: s.element,
            ..typeless(
                if s.magical {
                    DamageType::Magical
                } else {
                    DamageType::Physical
                },
                s.might,
                s.hit,
                s.crit,
                (1, 2),
            )
        },
    });
    CombatantInput {
        stats: stats(s.stats),
        tags: s.tags,
        affinities: s.affinities.clone(),
        weapon,
        weapon_rank: s.rank,
        armour_weight: s.armour,
        terrain,
    }
}

fn check_side(rules: &CombatRules, side: &SideForecast) -> Result<(), TestCaseError> {
    prop_assert!(side.hit <= 100 && side.crit <= 100);
    prop_assert!(side.damage >= 0 && side.followup_damage >= side.damage);
    prop_assert!((1..=rules.max_strikes()).contains(&side.strikes));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn forecast_and_resolve_invariants(a in spec(), d in spec(), distance in 1..=2u32, seed in any::<u64>(), hp_a in 1..=80i32, hp_d in 1..=80i32) {
        let rules = rules();
        let (a, d) = (build(&a), build(&d));
        let Some(f) = forecast(&rules, &a, &d, distance) else {
            prop_assert!(a.weapon.is_none());
            return Ok(());
        };
        check_side(&rules, &f.attacker)?;
        if let Some(def) = &f.defender {
            check_side(&rules, def)?;
            prop_assert!(f.attacker.strikes == 1 || def.strikes == 1);
        } else {
            prop_assert!(d.weapon.is_none());
        }

        let (ha, hd) = (hp(hp_a.min(a.stats.hp), a.stats.hp), hp(hp_d.min(d.stats.hp), d.stats.hp));
        let out = resolve(&rules, &f, ha, hd, &mut SimRng::new(seed));
        let max_strikes = usize::from(f.attacker.strikes) + f.defender.map_or(0, |d| usize::from(d.strikes));
        prop_assert!(!out.strikes.is_empty() && out.strikes.len() <= max_strikes);
        prop_assert!((0..=ha.max).contains(&out.attacker_hp));
        prop_assert!((0..=hd.max).contains(&out.defender_hp));
        for s in &out.strikes {
            prop_assert!(s.target_hp_after >= 0);
            prop_assert!(s.hit || (!s.crit && s.damage == 0));
        }
        // Only the last strike may leave a unit at 0.
        let last = out.strikes.len() - 1;
        prop_assert!(out.strikes[..last].iter().all(|s| s.target_hp_after > 0));

        // All-zero rolls hit whenever hit > 0.
        let mut zeros = ScriptedRng::new(vec![0; 3 * max_strikes]);
        let out = resolve(&rules, &f, ha, hd, &mut zeros);
        for s in &out.strikes {
            let side = match s.by {
                Side::Attacker => f.attacker,
                Side::Defender => f.defender.expect("defender struck"),
            };
            prop_assert_eq!(s.hit, side.hit > 0);
        }
    }
}
