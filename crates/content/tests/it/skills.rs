//! Every class skill in `assets/data/skills.ron`, used in a battle built from
//! the real content tables (ticket 0311): one test per skill, each showing
//! its effect on a forecast or on the battle state, against the numbers in
//! `docs/design/progression.md`, and the uses per battle of the non-attack
//! actives against `docs/design/combat-arts.md` (ticket 0316).
//!
//! Units are generic units of the skill's class with flat stats (HP 30, Str,
//! Mag, Dex and Spd 5, Def and Res 2) and one iron weapon, on an all-plain
//! 8×5 map. Most tests compare the same attack with and without the skill.

use std::sync::{Arc, OnceLock};

use trpg_content::Content;
use trpg_core::{
    BattleMap, BattlePack, BattleSetup, BattleState, CastTarget, ClassId, ClassRecord, Command,
    CommandError, CostError, Event, Faction, Forecast, Grid, ItemId, LoadoutDef, Objective, Phase,
    Pos, SkillId, SpellId, StatValue, Stats, Stock, Unit, UnitAction, UnitId,
};

fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| trpg_content::load_embedded().unwrap_or_else(|e| panic!("{e}")))
}

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// A unit of `class` with the flat stats, carrying `weapon` (if any).
fn unit(id: u32, class: &str, faction: Faction, pos: Pos, weapon: Option<&str>) -> Unit {
    let c = content();
    // Lord-only classes: made as a Swordsman, then moved in.
    let lord_only = c
        .classes
        .get(&ClassId(class.into()))
        .is_some_and(|d| d.lord_only);
    let made_as = if lord_only { "swordsman" } else { class };
    let mut u = Unit::generic(
        UnitId(id),
        &ClassId(made_as.into()),
        &c.classes,
        1,
        faction,
        pos,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    if lord_only {
        u.class = ClassId(class.into());
        u.is_lord = true;
        u.class_records
            .insert(ClassId(class.into()), trpg_core::ClassRecord::UNLOCKED);
        u.refresh_spells(&c.classes);
    }
    let mov = u.stats.mov;
    u.stats = Stats::from_growable([30, 5, 5, 5, 5, 2, 2], mov);
    u.hp = 30;
    let loadout = LoadoutDef {
        weapons: weapon.map(ItemId::new).into_iter().collect(),
        ..LoadoutDef::default()
    };
    u.with_loadout(&loadout, &c.classes, &c.items)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn player(id: u32, class: &str, pos: Pos, weapon: &str) -> Unit {
    unit(id, class, Faction::Player, pos, Some(weapon))
}

fn enemy(id: u32, pos: Pos) -> Unit {
    unit(id, "swordsman", Faction::Enemy, pos, Some("iron_sword"))
}

/// `u` with `f` applied.
fn with(mut u: Unit, f: impl FnOnce(&mut Unit)) -> Unit {
    f(&mut u);
    u
}

/// `u` having learned `skill`.
fn knowing(u: Unit, skill: &str) -> Unit {
    let skills = &content().skills;
    with(u, |u| {
        assert!(u.learn_skill(&SkillId::new(skill), skills), "{skill}");
    })
}

/// A battle on an 8×5 plain map with forests at `forests`.
fn battle_with(units: Vec<Unit>, forests: &[Pos]) -> BattleState {
    let c = content();
    let id = |t: &str| {
        c.terrain
            .display
            .id_of(t)
            .unwrap_or_else(|| panic!("no terrain {t}"))
    };
    let mut tiles = Grid::filled(8, 5, id("plain"));
    for &f in forests {
        if let Some(t) = tiles.get_mut(f) {
            *t = id("forest");
        }
    }
    BattleState::new(BattleSetup {
        map: BattleMap::new("Skills", tiles),
        terrain: Arc::new(c.terrain.rules.clone()),
        classes: Arc::new(c.classes.clone()),
        items: Arc::new(c.items.clone()),
        spells: Arc::new(c.spells.clone()),
        skills: Arc::new(c.skills.clone()),
        arts: Arc::new(c.arts.clone()),
        pack: BattlePack::default(),
        gold: 0,
        stock: Stock::default(),
        units,
        reinforcements: vec![],
        objective: Objective::Rout { turn_limit: None },
        rewind_charges: 0,
        seed: 1,
        triggers: vec![],
        mode: trpg_core::GameMode::Classic,
        battle_notes: vec![],
    })
    .0
}

fn battle(units: Vec<Unit>) -> BattleState {
    battle_with(units, &[])
}

fn act(s: &mut BattleState, id: u32, dest: Pos, action: UnitAction) -> Vec<Event> {
    s.apply(&Command::Act {
        unit: UnitId(id),
        dest,
        action,
    })
    .unwrap_or_else(|e| panic!("{e}"))
}

fn try_act(
    s: &BattleState,
    id: u32,
    dest: Pos,
    action: UnitAction,
) -> Result<Vec<Event>, CommandError> {
    s.clone().apply(&Command::Act {
        unit: UnitId(id),
        dest,
        action,
    })
}

fn attack(target: u32, active: Option<&str>) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
        active: active.map(SkillId::new),
        art: None,
    }
}

fn cast(spell: &str, target: u32, active: Option<&str>) -> UnitAction {
    UnitAction::Cast {
        spell: SpellId::new(spell),
        target: CastTarget::Unit(UnitId(target)),
        active: active.map(SkillId::new),
    }
}

fn use_skill(skill: &str, target: Option<u32>) -> UnitAction {
    UnitAction::UseSkill {
        skill: SkillId::new(skill),
        target: target.map(UnitId),
    }
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

/// The forecast of `id` doing `action` from `dest` (the state is kept).
fn forecast(s: &BattleState, id: u32, dest: Pos, action: UnitAction) -> Forecast {
    combat(&try_act(s, id, dest, action).unwrap_or_else(|e| panic!("{e}")))
}

/// The defender's numbers (it must counter).
fn counter(f: &Forecast) -> trpg_core::SideForecast {
    f.defender.unwrap_or_else(|| panic!("no counter in {f:?}"))
}

fn hp(s: &BattleState, id: u32) -> StatValue {
    s.unit(UnitId(id)).map_or(0, |u| u.hp)
}

fn durability(s: &BattleState, id: u32) -> u32 {
    s.unit(UnitId(id))
        .and_then(|u| u.loadout.weapon(0))
        .map_or(0, |w| w.durability_left)
}

/// The uses unit `id` has left this battle of the non-attack active
/// `skill`.
fn uses(s: &BattleState, id: u32, skill: &str) -> u8 {
    s.unit(UnitId(id))
        .map_or(0, |u| u.skill_uses.uses_left(&SkillId::new(skill)))
}

/// Ends phases until the Player phase starts again.
fn next_turn(s: &mut BattleState) {
    let mut end = || {
        s.apply(&Command::EndPhase)
            .unwrap_or_else(|e| panic!("{e}"));
        s.phase()
    };
    while end() != Phase::Player {}
}

fn healed(events: &[Event], id: u32) -> StatValue {
    events
        .iter()
        .map(|e| match e {
            Event::Healed { target, amount } if *target == UnitId(id) => *amount,
            _ => 0,
        })
        .sum()
}

/// The active attack of `attacker` (class `class`, `weapon`) on an enemy
/// next to it, with and without `skill`: `(without, with)`.
fn duel(class: &str, weapon: &str, skill: &str) -> (Forecast, Forecast) {
    let s = battle(vec![player(1, class, p(0, 0), weapon), enemy(3, p(1, 0))]);
    (
        forecast(&s, 1, p(0, 0), attack(3, None)),
        forecast(&s, 1, p(0, 0), attack(3, Some(skill))),
    )
}

/// The attack of `a` on an enemy next to it, with and without the passive
/// `skill` on `a` (or on the enemy with `on_enemy`): `(without, with)`.
fn passive(a: &Unit, skill: &str, on_enemy: bool) -> (Forecast, Forecast) {
    let pos = a.pos;
    let target = |learned: bool| {
        let e = enemy(3, p(pos.x + 1, pos.y));
        if learned && on_enemy {
            knowing(e, skill)
        } else {
            e
        }
    };
    let attacker = |learned: bool| {
        if learned && !on_enemy {
            knowing(a.clone(), skill)
        } else {
            a.clone()
        }
    };
    let f = |learned| {
        let s = battle(vec![attacker(learned), target(learned)]);
        forecast(&s, a.id.0, pos, attack(3, None))
    };
    (f(false), f(true))
}

// ---- Tier 1 --------------------------------------------------------------------

#[test]
fn keen_edge_hit_30_crit_10_for_3_durability() {
    let (plain, keen) = duel("swordsman", "iron_sword", "keen_edge");
    assert_eq!(keen.attacker.crit, plain.attacker.crit + 10);
    assert_eq!(keen.attacker.hit, (plain.attacker.hit + 30).min(100));
    let mut s = battle(vec![
        player(1, "swordsman", p(0, 0), "iron_sword"),
        enemy(3, p(1, 0)),
    ]);
    act(&mut s, 1, p(0, 0), attack(3, Some("keen_edge")));
    assert_eq!(durability(&s, 1), 17);
}

#[test]
fn sword_focus_1_crit_10_with_swords() {
    let (plain, focus) = passive(
        &player(1, "swordsman", p(0, 0), "iron_sword"),
        "sword_focus_1",
        false,
    );
    assert_eq!(focus.attacker.crit, plain.attacker.crit + 10);
    // Not with another weapon.
    let (plain, focus) = passive(
        &player(1, "brawler", p(0, 0), "iron_gauntlets"),
        "sword_focus_1",
        false,
    );
    assert_eq!(focus, plain);
}

#[test]
fn flurry_one_more_strike_for_5_durability() {
    let (plain, flurry) = duel("brawler", "iron_gauntlets", "flurry");
    assert_eq!((plain.attacker.strikes, flurry.attacker.strikes), (1, 2));
    let mut s = battle(vec![
        player(1, "brawler", p(0, 0), "iron_gauntlets"),
        enemy(3, p(1, 0)),
    ]);
    act(&mut s, 1, p(0, 0), attack(3, Some("flurry")));
    assert_eq!(durability(&s, 1), 15);
}

/// A gauntlet attacker against an enemy of Spd `spd`: strikes without and
/// with the passive `skill`.
fn light_feet(skill: &str, spd: StatValue) -> (u8, u8) {
    let a = player(1, "brawler", p(0, 0), "iron_gauntlets");
    let strikes = |learned: bool| {
        let a = if learned {
            knowing(a.clone(), skill)
        } else {
            a.clone()
        };
        let e = with(enemy(3, p(1, 0)), |u| u.stats.spd = spd);
        forecast(&battle(vec![a, e]), 1, p(0, 0), attack(3, None))
            .attacker
            .strikes
    };
    (strikes(false), strikes(true))
}

#[test]
fn light_feet_1_attack_speed_2_with_gauntlets() {
    // Brawler AS 5, enemy AS 3 (Spd 4, sword burden 1): a gap of 2, then 4.
    assert_eq!(light_feet("light_feet_1", 4), (1, 2));
    assert_eq!(light_feet("light_feet_1", 5), (1, 1));
}

#[test]
fn heavy_blow_might_5_and_one_strike() {
    let fast = with(player(1, "raider", p(0, 0), "iron_axe"), |u| {
        u.stats.spd = 13;
    });
    let s = battle(vec![fast, enemy(3, p(1, 0))]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let heavy = forecast(&s, 1, p(0, 0), attack(3, Some("heavy_blow")));
    assert_eq!(plain.attacker.strikes, 2);
    assert_eq!(heavy.attacker.strikes, 1);
    assert_eq!(heavy.attacker.damage, plain.attacker.damage + 5);
}

#[test]
fn axe_focus_1_hit_10_with_axes() {
    let (plain, focus) = passive(
        &player(1, "raider", p(0, 0), "iron_axe"),
        "axe_focus_1",
        false,
    );
    assert_eq!(focus.attacker.hit, plain.attacker.hit + 10);
}

#[test]
fn vault_move_1_after_a_bow_attack_for_1_durability() {
    let archer = player(1, "archer", p(0, 0), "iron_bow");
    let mut s = battle(vec![archer, enemy(3, p(2, 0))]);
    // A plain attack offers no move.
    let events = try_act(&s, 1, p(0, 0), attack(3, None)).unwrap_or_default();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::MoveAfterOffered { .. }))
    );
    let events = act(&mut s, 1, p(0, 0), attack(3, Some("vault")));
    assert!(events.contains(&Event::MoveAfterOffered {
        unit: UnitId(1),
        tiles: 1,
    }));
    move_after(&mut s, 1, p(0, 1));
    assert_eq!(s.unit(UnitId(1)).map(|u| u.pos), Some(p(0, 1)));
    assert_eq!(durability(&s, 1), 19);
}

/// Unit `id`, offered a move after its attack, moves to `to`.
fn move_after(s: &mut BattleState, id: u32, to: Pos) {
    s.apply(&Command::MoveAfter {
        unit: UnitId(id),
        to: Some(to),
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// The enemy's damage attacking `defender` (at (0,0)) in the enemy phase,
/// after `setup` ran in the player phase.
fn enemy_damage(defender: Unit, setup: impl FnOnce(&mut BattleState)) -> StatValue {
    let mut s = battle(vec![defender, enemy(3, p(1, 0))]);
    setup(&mut s);
    s.apply(&Command::EndPhase)
        .unwrap_or_else(|e| panic!("{e}"));
    forecast(&s, 3, p(1, 0), attack(1, None)).attacker.damage
}

#[test]
fn brace_def_and_res_5_until_its_next_phase_3_uses_a_battle() {
    let guard = player(1, "guard", p(0, 0), "iron_spear");
    // Str 5 + Iron Sword 5 − Def 2 = 8.
    assert_eq!(enemy_damage(guard.clone(), |_| {}), 8);
    let mut left = (0, 0);
    let braced = enemy_damage(guard, |s| {
        assert_eq!(uses(s, 1, "brace"), 3);
        act(s, 1, p(0, 0), use_skill("brace", None));
        left = (uses(s, 1, "brace"), durability(s, 1));
    });
    // No durability is spent (Nick, 2026-10-01).
    assert_eq!((braced, left), (3, (2, 20)));
}

#[test]
fn a_guard_with_no_weapon_can_brace() {
    let guard = unit(1, "guard", Faction::Player, p(0, 0), None);
    let braced = enemy_damage(guard, |s| {
        act(s, 1, p(0, 0), use_skill("brace", None));
    });
    assert_eq!(braced, 3);
}

#[test]
fn steadfast_1_def_2_outside_its_own_phase() {
    let guard = player(1, "guard", p(0, 0), "iron_spear");
    assert_eq!(
        enemy_damage(knowing(guard.clone(), "steadfast_1"), |_| {}),
        6
    );
    let (plain, steady) = passive(&guard, "steadfast_1", false);
    assert_eq!(steady, plain);
}

#[test]
fn lance_rush_might_5_multiplied_by_effectiveness() {
    let (plain, rush) = duel("rider", "iron_spear", "lance_rush");
    assert_eq!(rush.attacker.damage, plain.attacker.damage + 5);
    // Against a mounted unit the spear's ×2 doubles it too (Nick).
    let cavalry = unit(3, "rider", Faction::Enemy, p(1, 0), Some("iron_spear"));
    let s = battle(vec![player(1, "rider", p(0, 0), "iron_spear"), cavalry]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let rush = forecast(&s, 1, p(0, 0), attack(3, Some("lance_rush")));
    assert!(plain.attacker.effective);
    assert_eq!(rush.attacker.damage, plain.attacker.damage + 10);
}

/// A rider (Mov 7) attacking an enemy at (x, 0) from (x − 1, 0), with and
/// without the passive `skill`: damage `(without, with)`.
fn charge(skill: &str, x: i32) -> (StatValue, StatValue) {
    let rider = player(1, "rider", p(0, 0), "iron_spear");
    let damage = |learned: bool| {
        let r = if learned {
            knowing(rider.clone(), skill)
        } else {
            rider.clone()
        };
        let s = battle(vec![r, enemy(3, p(x, 0))]);
        forecast(&s, 1, p(x - 1, 0), attack(3, None))
            .attacker
            .damage
    };
    (damage(false), damage(true))
}

#[test]
fn charge_1_damage_2_after_moving_4() {
    let (plain, charged) = charge("charge_1", 5);
    assert_eq!(charged, plain + 2);
    let (plain, charged) = charge("charge_1", 4);
    assert_eq!(charged, plain);
}

#[test]
fn overcast_spell_might_5_for_an_extra_use() {
    let mut s = battle(vec![
        unit(1, "mage", Faction::Player, p(0, 0), None),
        enemy(3, p(2, 0)),
    ]);
    let plain = forecast(&s, 1, p(0, 0), cast("fire", 3, None));
    let events = act(&mut s, 1, p(0, 0), cast("fire", 3, Some("overcast")));
    assert_eq!(combat(&events).attacker.damage, plain.attacker.damage + 5);
    let uses = s
        .unit(UnitId(1))
        .map(|u| u.spells.uses_left(&SpellId::new("fire")));
    assert_eq!(uses, Some(8));
}

/// The fire damage of a caster of `class`, without and with `skill`.
fn spell_damage(class: &str, skill: &str) -> (StatValue, StatValue) {
    let caster = unit(1, class, Faction::Player, p(0, 0), None);
    let damage = |learned: bool| {
        let c = if learned {
            knowing(caster.clone(), skill)
        } else {
            caster.clone()
        };
        let s = battle(vec![c, enemy(3, p(2, 0))]);
        forecast(&s, 1, p(0, 0), cast("fire", 3, None))
            .attacker
            .damage
    };
    (damage(false), damage(true))
}

#[test]
fn black_magic_1_attack_spells_might_1() {
    let (plain, boosted) = spell_damage("mage", "black_magic_1");
    assert_eq!(boosted, plain + 1);
}

#[test]
fn sanctuary_heals_adjacent_allies_by_mag_plus_5_3_uses_a_battle() {
    let mut s = battle(vec![
        player(1, "cleric", p(1, 1), "iron_gauntlets"),
        with(player(2, "swordsman", p(1, 0), "iron_sword"), |u| u.hp = 1),
        with(player(4, "swordsman", p(1, 3), "iron_sword"), |u| u.hp = 1),
        enemy(3, p(7, 4)),
    ]);
    assert_eq!(uses(&s, 1, "sanctuary"), 3);
    let events = act(&mut s, 1, p(1, 1), use_skill("sanctuary", None));
    assert_eq!((healed(&events, 2), healed(&events, 4)), (10, 0));
    assert_eq!((uses(&s, 1, "sanctuary"), durability(&s, 1)), (2, 20));
}

/// A caster of `class` healing a wounded ally with `spell`, without and
/// with the passive `skill`: HP restored.
fn heal_amount(class: &str, spell: &str, skill: &str) -> (StatValue, StatValue) {
    let caster = unit(1, class, Faction::Player, p(0, 0), None);
    let amount = |learned: bool| {
        let c = if learned {
            knowing(caster.clone(), skill)
        } else {
            caster.clone()
        };
        let ally = with(player(2, "swordsman", p(1, 0), "iron_sword"), |u| u.hp = 1);
        let mut s = battle(vec![c, ally, enemy(3, p(7, 4))]);
        healed(&act(&mut s, 1, p(0, 0), cast(spell, 2, None)), 2)
    };
    (amount(false), amount(true))
}

#[test]
fn white_magic_1_heal_spells_2_more() {
    let (plain, boosted) = heal_amount("cleric", "heal", "white_magic_1");
    assert_eq!((plain, boosted), (15, 17));
}

/// Hit and avoid of ally 2 attacking after the lord's `setup`, lord at
/// (0,0), ally at (0,2), its target at (1,2): (hit, enemy's hit).
fn lord_help(lord: Unit, setup: impl FnOnce(&mut BattleState)) -> (u8, u8) {
    let mut s = battle(vec![
        lord,
        player(2, "swordsman", p(0, 2), "iron_sword"),
        with(enemy(3, p(1, 2)), |u| u.stats.dex = 0),
    ]);
    setup(&mut s);
    let f = forecast(&s, 2, p(0, 2), attack(3, None));
    (f.attacker.hit, counter(&f).hit)
}

fn exile() -> Unit {
    with(player(1, "exile", p(0, 0), "iron_sword"), |u| {
        u.stats.dex = 0;
    })
}

#[test]
fn inspire_allies_within_2_hit_and_avoid_10_2_uses_a_battle() {
    let plain = lord_help(exile(), |_| {});
    let mut left = (0, 0);
    let inspired = lord_help(exile(), |s| {
        assert_eq!(uses(s, 1, "inspire"), 2);
        act(s, 1, p(0, 0), use_skill("inspire", None));
        left = (uses(s, 1, "inspire"), durability(s, 1));
    });
    assert_eq!(inspired, (plain.0 + 10, plain.1 - 10));
    assert_eq!(left, (1, 20));
}

#[test]
fn leadership_1_allies_within_2_hit_10() {
    let plain = lord_help(exile(), |_| {});
    let led = lord_help(knowing(exile(), "leadership_1"), |_| {});
    assert_eq!(led, (plain.0 + 10, plain.1));
}

// ---- Tier 2 --------------------------------------------------------------------

#[test]
fn blade_flurry_one_more_strike_with_a_sword() {
    let (plain, flurry) = duel("duelist", "iron_sword", "blade_flurry");
    assert_eq!((plain.attacker.strikes, flurry.attacker.strikes), (1, 2));
}

#[test]
fn sword_focus_2_crit_20_and_supersedes_1() {
    let a = player(1, "duelist", p(0, 0), "iron_sword");
    let (plain, focus) = passive(&knowing(a.clone(), "sword_focus_1"), "sword_focus_2", false);
    assert_eq!(focus.attacker.crit, plain.attacker.crit + 10);
    let (plain, _) = passive(&a, "sword_focus_2", false);
    assert_eq!(focus.attacker.crit, plain.attacker.crit + 20);
}

#[test]
fn deadly_blow_doubles_crit() {
    let sharp = with(player(1, "shadowblade", p(0, 0), "iron_sword"), |u| {
        u.stats.dex = 20;
    });
    let s = battle(vec![sharp, enemy(3, p(1, 0))]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let deadly = forecast(&s, 1, p(0, 0), attack(3, Some("deadly_blow")));
    assert!(plain.attacker.crit > 0);
    assert_eq!(deadly.attacker.crit, plain.attacker.crit * 2);
}

#[test]
fn evasion_1_avoid_10() {
    let (plain, evasive) = passive(
        &player(1, "swordsman", p(0, 0), "iron_sword"),
        "evasion_1",
        true,
    );
    assert_eq!(evasive.attacker.hit, plain.attacker.hit - 10);
}

#[test]
fn hundred_fists_one_more_strike_and_hit_10_with_gauntlets() {
    let (plain, fists) = duel("striker", "iron_gauntlets", "hundred_fists");
    assert_eq!((plain.attacker.strikes, fists.attacker.strikes), (1, 2));
    assert_eq!(fists.attacker.hit, (plain.attacker.hit + 10).min(100));
    let dodgy = with(enemy(3, p(1, 0)), |u| u.stats.spd = 10);
    let s = battle(vec![player(1, "striker", p(0, 0), "iron_gauntlets"), dodgy]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let fists = forecast(&s, 1, p(0, 0), attack(3, Some("hundred_fists")));
    assert_eq!(fists.attacker.hit, plain.attacker.hit + 10);
}

#[test]
fn light_feet_2_attack_speed_4_and_supersedes_1() {
    // Enemy AS 5: a gap of 0, then 4.
    assert_eq!(light_feet("light_feet_2", 6), (1, 2));
    assert_eq!(light_feet("light_feet_1", 6), (1, 1));
}

#[test]
fn shove_pushes_an_adjacent_enemy_8_uses_a_battle() {
    let mut s = battle(vec![
        player(1, "grappler", p(0, 0), "iron_gauntlets"),
        enemy(3, p(1, 0)),
    ]);
    assert_eq!(uses(&s, 1, "shove"), 8);
    act(&mut s, 1, p(0, 0), use_skill("shove", Some(3)));
    assert_eq!(s.unit(UnitId(3)).map(|u| u.pos), Some(p(2, 0)));
    assert_eq!((uses(&s, 1, "shove"), durability(&s, 1)), (7, 20));
}

#[test]
fn a_blocked_shove_deals_5_collision_damage_to_both_units() {
    let mut s = battle(vec![
        player(1, "grappler", p(0, 0), "iron_gauntlets"),
        enemy(3, p(1, 0)),
        enemy(4, p(2, 0)),
    ]);
    act(&mut s, 1, p(0, 0), use_skill("shove", Some(3)));
    assert_eq!(
        s.unit(UnitId(3)).map(|u| (u.pos, u.hp)),
        Some((p(1, 0), 25))
    );
    // The unit it hit takes the same (Nick).
    assert_eq!(hp(&s, 4), 25);
}

#[test]
fn iron_grip_def_3_with_gauntlets() {
    let (plain, grip) = passive(
        &player(1, "grappler", p(0, 0), "iron_gauntlets"),
        "iron_grip",
        false,
    );
    assert_eq!(counter(&grip).damage, counter(&plain).damage - 3);
}

#[test]
fn rampage_might_8_and_its_avoid_20_lower() {
    let quick = with(player(1, "berserker", p(0, 0), "iron_axe"), |u| {
        u.stats.spd = 15;
    });
    let s = battle(vec![quick, enemy(3, p(1, 0))]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let rampage = forecast(&s, 1, p(0, 0), attack(3, Some("rampage")));
    assert_eq!(rampage.attacker.damage, plain.attacker.damage + 8);
    assert_eq!(counter(&rampage).hit, counter(&plain).hit + 20);
}

#[test]
fn fury_crit_15_at_half_hp_or_less() {
    let hurt = |hp| with(player(1, "berserker", p(0, 0), "iron_axe"), |u| u.hp = hp);
    let (plain, fury) = passive(&hurt(15), "fury", false);
    assert_eq!(fury.attacker.crit, plain.attacker.crit + 15);
    let (plain, fury) = passive(&hurt(16), "fury", false);
    assert_eq!(fury, plain);
}

#[test]
fn war_cry_adjacent_allies_str_2_2_uses_a_battle() {
    let mut s = battle(vec![
        player(1, "vanguard", p(0, 1), "iron_axe"),
        player(2, "swordsman", p(0, 2), "iron_sword"),
        enemy(3, p(1, 2)),
    ]);
    let plain = forecast(&s, 2, p(0, 2), attack(3, None));
    assert_eq!(uses(&s, 1, "war_cry"), 2);
    act(&mut s, 1, p(0, 1), use_skill("war_cry", None));
    let cried = forecast(&s, 2, p(0, 2), attack(3, None));
    assert_eq!(cried.attacker.damage, plain.attacker.damage + 2);
    assert_eq!((uses(&s, 1, "war_cry"), durability(&s, 1)), (1, 20));
}

#[test]
fn axe_focus_2_hit_20_with_axes() {
    let (plain, focus) = passive(
        &player(1, "vanguard", p(0, 0), "iron_axe"),
        "axe_focus_2",
        false,
    );
    assert_eq!(focus.attacker.hit, plain.attacker.hit + 20);
}

#[test]
fn long_shot_range_2_more_with_a_bow() {
    let s = battle(vec![
        player(1, "marksman", p(0, 0), "iron_bow"),
        enemy(3, p(4, 0)),
    ]);
    assert!(try_act(&s, 1, p(0, 0), attack(3, Some("long_shot"))).is_ok());
    let far = battle(vec![
        player(1, "marksman", p(0, 0), "iron_bow"),
        enemy(3, p(5, 0)),
    ]);
    assert!(matches!(
        try_act(&far, 1, p(0, 0), attack(3, Some("long_shot"))),
        Err(CommandError::OutOfRange { distance: 5, .. })
    ));
}

/// A Marksman's bow attack: (hit, crit) without and with the passives
/// `skills`, learned in order.
fn bow_focus(skills: &[&str]) -> ((u8, u8), (u8, u8)) {
    let archer = player(1, "marksman", p(0, 0), "iron_bow");
    let f = |a: Unit| {
        let f = forecast(
            &battle(vec![a, enemy(3, p(2, 0))]),
            1,
            p(0, 0),
            attack(3, None),
        );
        (f.attacker.hit, f.attacker.crit)
    };
    let learned = skills.iter().fold(archer.clone(), |u, sk| knowing(u, sk));
    (f(archer), f(learned))
}

#[test]
fn bow_focus_1_hit_5_with_bows() {
    let ((hit, crit), focus) = bow_focus(&["bow_focus_1"]);
    assert_eq!(focus, (hit + 5, crit));
}

#[test]
fn bow_focus_2_hit_10_crit_5_with_bows_and_supersedes_1() {
    let ((hit, crit), focus) = bow_focus(&["bow_focus_1", "bow_focus_2"]);
    assert_eq!(focus, (hit + 10, crit + 5));
}

#[test]
fn actives_are_locked_to_their_weapon_kind() {
    // Nick: Lance Rush with spears, Deadly Blow with swords, Trample with
    // axes.
    let wrong = |class: &str, weapon: &str, skill: &str| {
        let s = battle(vec![player(1, class, p(0, 0), weapon), enemy(3, p(1, 0))]);
        try_act(&s, 1, p(0, 0), attack(3, Some(skill))).err()
    };
    for (class, weapon, skill) in [
        ("rider", "iron_sword", "lance_rush"),
        ("shadowblade", "iron_gauntlets", "deadly_blow"),
        ("iron_rider", "iron_spear", "trample"),
        ("iron_rider", "iron_sword", "trample"),
    ] {
        assert_eq!(
            wrong(class, weapon, skill),
            Some(CommandError::WrongWeaponForSkill(SkillId::new(skill))),
            "{skill} with {weapon}"
        );
    }
}

#[test]
fn volley_one_more_strike_with_a_bow() {
    let s = battle(vec![
        player(1, "outrider", p(0, 0), "iron_bow"),
        enemy(3, p(2, 0)),
    ]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let volley = forecast(&s, 1, p(0, 0), attack(3, Some("volley")));
    assert_eq!((plain.attacker.strikes, volley.attacker.strikes), (1, 2));
}

#[test]
fn fortify_def_and_res_8_until_its_next_phase_2_uses_a_battle() {
    let bulwark = player(1, "bulwark", p(0, 0), "iron_spear");
    let mut left = (0, 0);
    let fortified = enemy_damage(bulwark, |s| {
        assert_eq!(uses(s, 1, "fortify"), 2);
        act(s, 1, p(0, 0), use_skill("fortify", None));
        left = (uses(s, 1, "fortify"), durability(s, 1));
    });
    assert_eq!((fortified, left), (0, (1, 20)));
}

#[test]
fn steadfast_2_def_4_outside_its_own_phase() {
    let bulwark = player(1, "bulwark", p(0, 0), "iron_spear");
    assert_eq!(enemy_damage(knowing(bulwark, "steadfast_2"), |_| {}), 4);
}

#[test]
fn trample_might_4_and_ignores_the_targets_terrain() {
    let s = battle_with(
        vec![
            player(1, "iron_rider", p(0, 0), "iron_axe"),
            enemy(3, p(1, 0)),
        ],
        &[p(1, 0)],
    );
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let trample = forecast(&s, 1, p(0, 0), attack(3, Some("trample")));
    // The forest gives Def +1 and avoid +20.
    assert_eq!(trample.attacker.damage, plain.attacker.damage + 4 + 1);
    assert_eq!(trample.attacker.hit, (plain.attacker.hit + 20).min(100));
}

#[test]
fn piercing_lance_ignores_5_def_with_a_spear() {
    let armoured = with(enemy(3, p(1, 0)), |u| u.stats.def = 8);
    let s = battle(vec![player(1, "lancer", p(0, 0), "iron_spear"), armoured]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let pierce = forecast(&s, 1, p(0, 0), attack(3, Some("piercing_lance")));
    assert_eq!(pierce.attacker.damage, plain.attacker.damage + 5);
}

#[test]
fn charge_2_damage_4_after_moving_4() {
    let (plain, charged) = charge("charge_2", 5);
    assert_eq!(charged, plain + 4);
}

#[test]
fn black_magic_2_attack_spells_might_3() {
    // Sorcerers start with Force, not Fire: the Mystic's Fire shows it.
    let (plain, boosted) = spell_damage("mystic", "black_magic_2");
    assert_eq!(boosted, plain + 3);
}

#[test]
fn siphon_heals_half_the_damage_dealt_for_an_extra_use() {
    let mystic = with(unit(1, "mystic", Faction::Player, p(0, 0), None), |u| {
        u.stats.dex = 20;
        u.stats.spd = 1;
        u.hp = 10;
    });
    let sure = with(enemy(3, p(2, 0)), |u| {
        u.stats.spd = 0;
        u.stats.dex = 40;
    });
    let mut s = battle(vec![mystic, sure]);
    let events = act(&mut s, 1, p(0, 0), cast("fire", 3, Some("siphon")));
    // Mag 5 + Fire 5 − Res 2 = 8 dealt by one sure strike: heals 4.
    assert_eq!(combat(&events).attacker.hit, 100);
    assert_eq!(healed(&events, 1), 4);
    assert_eq!(hp(&s, 1), 14);
}

#[test]
fn benediction_heals_allies_within_2_once_a_battle() {
    let mut s = battle(vec![
        player(1, "priest", p(1, 1), "iron_gauntlets"),
        with(player(4, "swordsman", p(1, 3), "iron_sword"), |u| u.hp = 1),
        enemy(3, p(7, 4)),
    ]);
    assert_eq!(uses(&s, 1, "benediction"), 1);
    let events = act(&mut s, 1, p(1, 1), use_skill("benediction", None));
    assert_eq!(healed(&events, 4), 10);
    assert_eq!((uses(&s, 1, "benediction"), durability(&s, 1)), (0, 20));
    // The ally is still wounded next turn, but the one use is spent.
    next_turn(&mut s);
    assert_eq!(
        try_act(&s, 1, p(1, 1), use_skill("benediction", None)),
        Err(CommandError::CannotPay {
            skill: SkillId::new("benediction"),
            error: CostError::NoUsesLeft,
        })
    );
    // A Priest that was never a Cleric has no Sanctuary.
    assert_eq!(uses(&s, 1, "sanctuary"), 0);
    assert!(try_act(&s, 1, p(1, 1), use_skill("sanctuary", None)).is_err());
}

/// Benediction is a skill of its own, not a rank of Sanctuary (Nick,
/// `combat-arts.md`): a Priest promoted from a mastered Cleric keeps
/// Sanctuary (3 uses, adjacent allies) and has Benediction too (1 use,
/// allies within 2 tiles), and White Magic adds to both.
#[test]
fn a_priest_promoted_from_a_mastered_cleric_has_sanctuary_and_benediction() {
    let c = content();
    let mastered = ClassRecord {
        class_level: c.classes.class_level_cap,
        class_points: 0,
    };
    let priest = with(player(1, "priest", p(1, 1), "iron_gauntlets"), |u| {
        u.class_records.insert(ClassId("cleric".into()), mastered);
    });
    // Mastering Cleric taught White Magic 1 (heals +2).
    let priest = knowing(priest, "white_magic_1");
    let actives: Vec<&str> = priest
        .usable_skills(&c.classes, &c.skills)
        .into_iter()
        .filter(|s| s.is_active())
        .map(|s| s.id.0.as_str())
        .collect();
    assert_eq!(actives, ["benediction", "sanctuary"]);
    let wounded = |id, pos| with(player(id, "swordsman", pos, "iron_sword"), |u| u.hp = 1);
    let mut s = battle(vec![
        priest,
        wounded(2, p(1, 0)),
        wounded(4, p(1, 3)),
        enemy(3, p(7, 4)),
    ]);
    assert_eq!(
        (uses(&s, 1, "sanctuary"), uses(&s, 1, "benediction")),
        (3, 1)
    );
    // Sanctuary: the adjacent ally only, by Mag 5 + 5 + 2.
    let events = act(&mut s, 1, p(1, 1), use_skill("sanctuary", None));
    assert_eq!((healed(&events, 2), healed(&events, 4)), (12, 0));
    assert_eq!(
        (uses(&s, 1, "sanctuary"), uses(&s, 1, "benediction")),
        (2, 1)
    );
    // Benediction: both allies, by the same.
    next_turn(&mut s);
    let events = act(&mut s, 1, p(1, 1), use_skill("benediction", None));
    assert_eq!((healed(&events, 2), healed(&events, 4)), (12, 12));
    assert_eq!(
        (uses(&s, 1, "sanctuary"), uses(&s, 1, "benediction")),
        (2, 0)
    );
}

#[test]
fn white_magic_2_heal_spells_4_more() {
    let (plain, boosted) = heal_amount("mystic", "heal", "white_magic_2");
    assert_eq!(boosted, plain + 4);
}

#[test]
fn crest_strike_might_4_hit_15_with_a_sword() {
    let heir = with(player(1, "blade_heir", p(0, 0), "iron_sword"), |u| {
        u.stats.dex = 0;
    });
    let s = battle(vec![heir, enemy(3, p(1, 0))]);
    let plain = forecast(&s, 1, p(0, 0), attack(3, None));
    let crest = forecast(&s, 1, p(0, 0), attack(3, Some("crest_strike")));
    assert_eq!(crest.attacker.damage, plain.attacker.damage + 4);
    assert_eq!(crest.attacker.hit, plain.attacker.hit + 15);
}

#[test]
fn resolve_str_and_spd_3_at_half_hp_or_less() {
    let heir = |hp| {
        with(player(1, "blade_heir", p(0, 0), "iron_sword"), |u| {
            u.hp = hp;
        })
    };
    let (plain, resolved) = passive(&heir(15), "resolve", false);
    assert_eq!(resolved.attacker.damage, plain.attacker.damage + 3);
    assert_eq!(counter(&resolved).hit, counter(&plain).hit - 6);
    let (plain, resolved) = passive(&heir(16), "resolve", false);
    assert_eq!(resolved, plain);
}

fn commander() -> Unit {
    with(player(1, "commander", p(0, 0), "iron_sword"), |u| {
        u.stats.dex = 0;
    })
}

#[test]
fn rally_allies_within_2_str_and_def_3_once_a_battle() {
    let mut s = battle(vec![
        commander(),
        player(2, "swordsman", p(0, 2), "iron_sword"),
        enemy(3, p(1, 2)),
    ]);
    let plain = forecast(&s, 2, p(0, 2), attack(3, None));
    assert_eq!(uses(&s, 1, "rally"), 1);
    act(&mut s, 1, p(0, 0), use_skill("rally", None));
    let rallied = forecast(&s, 2, p(0, 2), attack(3, None));
    assert_eq!(rallied.attacker.damage, plain.attacker.damage + 3);
    assert_eq!(counter(&rallied).damage, counter(&plain).damage - 3);
    assert_eq!((uses(&s, 1, "rally"), durability(&s, 1)), (0, 20));
}

#[test]
fn leadership_2_allies_within_2_hit_and_avoid_10_and_supersedes_1() {
    let plain = lord_help(commander(), |_| {});
    let led = lord_help(
        knowing(knowing(commander(), "leadership_1"), "leadership_2"),
        |_| {},
    );
    assert_eq!(led, (plain.0 + 10, plain.1 - 10));
}

// ---- The Flier (tier 3) -------------------------------------------------------------

#[test]
fn swoop_move_1_after_the_attack_for_3_durability() {
    let mut s = battle(vec![
        player(1, "flier", p(0, 0), "iron_spear"),
        enemy(3, p(1, 0)),
    ]);
    act(&mut s, 1, p(0, 0), attack(3, Some("swoop")));
    move_after(&mut s, 1, p(0, 1));
    assert_eq!(s.unit(UnitId(1)).map(|u| u.pos), Some(p(0, 1)));
    assert_eq!(durability(&s, 1), 17);
}

#[test]
fn sky_dodge_1_avoid_10_against_bows() {
    let flier = |learned: bool| {
        let f = with(
            unit(3, "flier", Faction::Enemy, p(2, 0), Some("iron_spear")),
            |u| {
                u.stats.spd = 10;
            },
        );
        if learned {
            knowing(f, "sky_dodge_1")
        } else {
            f
        }
    };
    let hit = |weapon: &str, dest, learned| {
        let s = battle(vec![player(1, "archer", p(0, 0), weapon), flier(learned)]);
        forecast(&s, 1, dest, attack(3, None)).attacker.hit
    };
    assert_eq!(
        hit("iron_bow", p(0, 0), true),
        hit("iron_bow", p(0, 0), false) - 10
    );
    let sword = |learned| {
        let s = battle(vec![
            player(1, "swordsman", p(0, 0), "iron_sword"),
            flier(learned),
        ]);
        forecast(&s, 1, p(1, 0), attack(3, None)).attacker.hit
    };
    assert_eq!(sword(true), sword(false));
}

#[test]
fn every_skill_has_a_test_here() {
    let source = include_str!("skills.rs");
    for id in content().skills.skills.keys() {
        assert!(
            source.contains(&format!("\"{}\"", id.0)),
            "no test uses {}",
            id.0
        );
    }
}
