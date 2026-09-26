use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;
use crate::class::{ClassId, UnitTags, WeaponProficiency};
use crate::geom::Pos;
use crate::stats::Growths;
use crate::terrain::MovementTypeId;
use crate::unit::{Faction, UnitId};

fn id(s: &str) -> ItemId {
    ItemId::new(s)
}

fn weapon(kind: WeaponKind, rank: WeaponRank, range: (u32, u32)) -> ItemDef {
    ItemDef::Weapon(WeaponDef {
        name: "W".into(),
        kind,
        rank,
        might: 5,
        hit: 90,
        crit: 0,
        weight: 2,
        min_range: range.0,
        max_range: range.1,
        damage_type: DamageType::Physical,
        durability: 20,
        effective: vec![],
        price: 100,
    })
}

fn bonus(f: impl FnOnce(&mut Stats)) -> Stats {
    let mut s = Stats::default();
    f(&mut s);
    s
}

/// Swords (E and D), a bow, an axe, a vest, chain mail, a ring, a potion.
fn items() -> ItemTable {
    let entries = [
        ("sword", weapon(WeaponKind::Sword, WeaponRank::E, (1, 1))),
        (
            "steel_sword",
            weapon(WeaponKind::Sword, WeaponRank::D, (1, 1)),
        ),
        ("javelin", weapon(WeaponKind::Sword, WeaponRank::E, (1, 2))),
        ("bow", weapon(WeaponKind::Bow, WeaponRank::E, (2, 2))),
        ("axe", weapon(WeaponKind::Axe, WeaponRank::E, (1, 1))),
        (
            "vest",
            ItemDef::Armour(ArmourDef {
                name: "Vest".into(),
                weight_class: ArmourWeight::Light,
                bonus: bonus(|s| s.def = 1),
                weight: 0,
                price: 300,
            }),
        ),
        (
            "mail",
            ItemDef::Armour(ArmourDef {
                name: "Mail".into(),
                weight_class: ArmourWeight::Medium,
                bonus: bonus(|s| s.def = 3),
                weight: 2,
                price: 800,
            }),
        ),
        (
            "ring",
            ItemDef::Accessory(AccessoryDef {
                name: "Ring".into(),
                bonus: bonus(|s| {
                    s.spd = 2;
                    s.def = 4;
                }),
                price: 2000,
            }),
        ),
        (
            "potion",
            ItemDef::Consumable(ConsumableDef {
                name: "Potion".into(),
                effect: ConsumableEffect::Heal(10),
                price: 300,
            }),
        ),
    ];
    ItemTable {
        items: entries.into_iter().map(|(k, v)| (id(k), v)).collect(),
        traits: BTreeMap::from([(WeaponKind::Sword, WeaponTrait::SwordFollowUp)]),
        rules: WeaponRules::default(),
    }
}

fn class() -> ClassDef {
    ClassDef {
        id: ClassId("fencer".into()),
        name: "Fencer".into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 5,
        base: Stats::default(),
        caps: Stats::default(),
        growths: Growths::default(),
        weapons: vec![WeaponProficiency {
            kind: WeaponKind::Sword,
            start: WeaponRank::E,
            max: WeaponRank::C,
        }],
        armour: vec![ArmourWeight::Light],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: None,
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    }
}

fn classes_with(class: ClassDef) -> ClassTable {
    ClassTable {
        classes: BTreeMap::from([(class.id.clone(), class)]),
        hard_ceilings: Stats::from_growable([60, 30, 30, 30, 30, 30, 30], 15),
        ..ClassTable::default()
    }
}

fn classes() -> ClassTable {
    classes_with(class())
}

fn unit() -> Unit {
    let mut u = Unit::generic(
        UnitId(1),
        &ClassId("fencer".into()),
        &classes(),
        1,
        Faction::Player,
        Pos::new(0, 0),
    )
    .unwrap();
    u.stats = Stats::from_growable([20, 5, 0, 5, 7, 4, 1], 5);
    u
}

fn loadout(weapons: &[&str], armour: Option<&str>, accessory: Option<&str>) -> LoadoutDef {
    LoadoutDef {
        weapons: weapons.iter().map(|w| id(w)).collect(),
        armour: armour.map(id),
        accessory: accessory.map(id),
    }
}

fn equipped(weapons: &[&str], armour: Option<&str>, accessory: Option<&str>) -> Unit {
    unit()
        .with_loadout(&loadout(weapons, armour, accessory), &classes(), &items())
        .unwrap()
}

// ---- Weapon ranks and EXP -------------------------------------------------

#[test]
fn rank_thresholds() {
    let r = WeaponRules::default();
    let cases = [
        (0, WeaponRank::E),
        (29, WeaponRank::E),
        (30, WeaponRank::D),
        (69, WeaponRank::D),
        (70, WeaponRank::C),
        (120, WeaponRank::B),
        (179, WeaponRank::B),
        (180, WeaponRank::A),
        (249, WeaponRank::A),
        (250, WeaponRank::S),
        (u32::MAX, WeaponRank::S),
    ];
    for (exp, rank) in cases {
        assert_eq!(r.rank_for(exp), rank, "{exp}");
    }
    let thresholds: Vec<u32> = RANKS.iter().map(|&k| r.threshold(k)).collect();
    assert_eq!(thresholds, [0, 30, 70, 120, 180, 250]);
}

/// The design's examples (`weapons-and-items.md`, Weapon ranks).
#[test]
fn weapon_exp_formula() {
    let r = WeaponRules::default();
    assert_eq!(r.weapon_exp(2, 2, 19, false), 5);
    assert_eq!(r.weapon_exp(1, 1, 3, false), 2);
    assert_eq!(r.weapon_exp(2, 2, 19, true), 7);
    assert_eq!(r.weapon_exp(3, 0, 0, false), 1);
    assert_eq!(r.weapon_exp(3, 0, 0, true), 2);
    assert_eq!(r.weapon_exp(0, 0, 0, false), 0);
    assert_eq!(r.weapon_exp(0, 0, 50, true), 0);
    assert_eq!(r.weapon_exp(1, 1, 4, false), 2);
    assert_eq!(r.weapon_exp(1, 1, 5, false), 3);
    assert_eq!(r.weapon_exp(1, 1, -5, false), 2);
}

#[test]
fn weapon_exp_counts_from_the_rank_and_ranks_up() {
    let r = WeaponRules::default();
    let mut u = unit();
    u.weapon_ranks.insert(WeaponKind::Sword, WeaponRank::D);
    // Rank D without EXP counts as 30.
    assert_eq!(
        u.gain_weapon_exp(WeaponKind::Sword, 5, WeaponRank::C, &r),
        None
    );
    assert_eq!(u.weapon_exp[&WeaponKind::Sword], 35);
    assert_eq!(
        u.gain_weapon_exp(WeaponKind::Sword, 35, WeaponRank::C, &r),
        Some(WeaponRank::C)
    );
    assert_eq!(u.rank(WeaponKind::Sword), WeaponRank::C);
    // Capped at the class max rank and its threshold.
    assert_eq!(
        u.gain_weapon_exp(WeaponKind::Sword, 500, WeaponRank::C, &r),
        None
    );
    assert_eq!(u.weapon_exp[&WeaponKind::Sword], 70);
    assert_eq!(u.rank(WeaponKind::Sword), WeaponRank::C);
    // A new kind starts at E; jumping two ranks reports the final one.
    assert_eq!(u.rank(WeaponKind::Axe), WeaponRank::E);
    assert_eq!(
        u.gain_weapon_exp(WeaponKind::Axe, 75, WeaponRank::S, &r),
        Some(WeaponRank::C)
    );
    // A rank above the class max (from another class) is never lowered.
    u.weapon_ranks.insert(WeaponKind::Bow, WeaponRank::A);
    assert_eq!(
        u.gain_weapon_exp(WeaponKind::Bow, 3, WeaponRank::D, &r),
        None
    );
    assert_eq!(u.rank(WeaponKind::Bow), WeaponRank::A);
    assert_eq!(u.weapon_exp[&WeaponKind::Bow], 180);
}

// ---- Durability -------------------------------------------------------------

#[test]
fn durability_breaks_once() {
    let mut w = WeaponInstance {
        def: id("sword"),
        durability_left: 3,
    };
    assert!(!w.is_broken());
    assert!(!w.spend_durability(2));
    assert_eq!(w.durability_left, 1);
    assert!(w.spend_durability(5));
    assert_eq!(w.durability_left, 0);
    assert!(w.is_broken());
    assert!(!w.spend_durability(1));
    assert!(!w.spend_durability(0));
}

#[test]
fn unit_spend_durability_reports_item_broke() {
    let mut u = equipped(&["sword", "javelin"], None, None);
    assert_eq!(u.spend_durability(1, 19), None);
    assert_eq!(
        u.spend_durability(1, 1),
        Some(Event::ItemBroke {
            unit: UnitId(1),
            item: id("javelin"),
        })
    );
    assert_eq!(u.spend_durability(1, 1), None);
    assert_eq!(u.spend_durability(2, 1), None);
    assert_eq!(u.spend_durability(9, 1), None);
    assert_eq!(u.loadout.weapon(0).map(|w| w.durability_left), Some(20));
}

#[test]
fn weapon_stats_of_a_copy() {
    let t = items();
    let mut copy = t.new_weapon(&id("sword")).unwrap();
    let stats = t.weapon_stats(&copy).unwrap();
    assert_eq!(stats.kind, Some(WeaponKind::Sword));
    assert_eq!(stats.trait_, WeaponTrait::SwordFollowUp);
    assert_eq!((stats.might, stats.hit, stats.weight), (5, 90, 2));
    assert!(!stats.broken);
    copy.durability_left = 0;
    assert!(t.weapon_stats(&copy).unwrap().broken);
    // A kind without a trait entry has none.
    let axe = t.new_weapon(&id("axe")).unwrap();
    assert_eq!(t.weapon_stats(&axe).unwrap().trait_, WeaponTrait::None);
    assert_eq!(t.new_weapon(&id("potion")), None);
    let not_weapon = WeaponInstance {
        def: id("vest"),
        durability_left: 1,
    };
    assert_eq!(t.weapon_stats(&not_weapon), None);
}

#[test]
fn item_lookups_by_kind() {
    let t = items();
    assert!(t.weapon(&id("sword")).is_some());
    assert!(t.weapon(&id("vest")).is_none());
    assert!(t.armour(&id("vest")).is_some());
    assert!(t.armour(&id("ring")).is_none());
    assert!(t.accessory(&id("ring")).is_some());
    assert!(t.accessory(&id("potion")).is_none());
    assert!(t.consumable(&id("potion")).is_some());
    assert!(t.consumable(&id("sword")).is_none());
    assert!(t.weapon(&id("nope")).is_none());
    let names: Vec<(&str, u32)> = ["sword", "vest", "ring", "potion"]
        .iter()
        .map(|k| {
            let d = t.get(&id(k)).unwrap();
            (d.name(), d.price())
        })
        .collect();
    assert_eq!(
        names,
        [("W", 100), ("Vest", 300), ("Ring", 2000), ("Potion", 300)]
    );
}

#[test]
fn combat_rules_come_from_the_table() {
    let mut t = items();
    assert_eq!(t.combat_rules(), CombatRules::default());
    t.rules.rank_speed = [1, 2, 3, 4, 5, 6];
    t.rules.broken_hit_penalty = 7;
    t.rules.broken_might_divisor = 3;
    let r = t.combat_rules();
    assert_eq!(r.rank_speed, [1, 2, 3, 4, 5, 6]);
    assert_eq!((r.broken_hit_penalty, r.broken_might_divisor), (7, 3));
}

// ---- Wielding, gear-adjusted stats, ranges ------------------------------------

#[test]
fn wielding_needs_the_class_kind_and_the_rank() {
    let t = items();
    let c = class();
    let mut u = unit();
    let def = |k: &str| t.weapon(&id(k)).unwrap();
    assert!(u.can_wield(def("sword"), &c));
    assert!(!u.can_wield(def("steel_sword"), &c));
    assert!(!u.can_wield(def("axe"), &c));
    u.weapon_ranks.insert(WeaponKind::Sword, WeaponRank::D);
    assert!(u.can_wield(def("steel_sword"), &c));
    // Rank without the class kind isn't enough.
    u.weapon_ranks.insert(WeaponKind::Axe, WeaponRank::S);
    assert!(!u.can_wield(def("axe"), &c));
}

#[test]
fn effective_stats_add_gear_up_to_the_hard_ceiling() {
    let u = equipped(&[], Some("vest"), Some("ring"));
    let s = u.effective_stats(&classes(), &items());
    assert_eq!(s, Stats::from_growable([20, 5, 0, 5, 9, 9, 1], 5));
    // Ceiling Def 6: raised to 6, not 9.
    let mut c = classes();
    c.hard_ceilings.def = 6;
    assert_eq!(u.effective_stats(&c, &items()).def, 6);
    // A stat already above the ceiling is kept, not lowered.
    c.hard_ceilings.def = 2;
    assert_eq!(u.effective_stats(&c, &items()).def, 4);
    // No gear: permanent stats.
    assert_eq!(unit().effective_stats(&classes(), &items()), unit().stats);
}

#[test]
fn armour_weight() {
    let mut c = class();
    c.armour.push(ArmourWeight::Medium);
    let t = items();
    let mail = unit()
        .with_loadout(&loadout(&[], Some("mail"), None), &classes_with(c), &t)
        .unwrap();
    assert_eq!(mail.armour_weight(&t), 2);
    assert_eq!(unit().armour_weight(&t), 0);
}

#[test]
fn attack_ranges_of_usable_weapons() {
    let u = equipped(&["sword", "javelin", "steel_sword"], None, None);
    // The steel sword needs rank D: skipped. (1,1) once.
    assert_eq!(u.attack_ranges(&classes(), &items()), [(1, 1), (1, 2)]);
    let u = equipped(&["axe"], None, None);
    assert!(u.attack_ranges(&classes(), &items()).is_empty());
    assert!(unit().attack_ranges(&classes(), &items()).is_empty());
    let mut lost = equipped(&["sword"], None, None);
    lost.class = ClassId("nope".into());
    assert!(lost.attack_ranges(&classes(), &items()).is_empty());
}

#[test]
fn combat_input_from_the_loadout() {
    let terrain = TerrainRules {
        name: "Plain".into(),
        move_cost: vec![Some(1)],
        defense: 0,
        avoid: 0,
        heal_percent: 0,
    };
    let t = items();
    let (c, cs) = (class(), classes());
    let mut u = equipped(&["javelin", "sword"], Some("vest"), None);
    u.weapon_ranks.insert(WeaponKind::Sword, WeaponRank::B);
    let input = u.combat_input(&c, &cs, &t, None, &terrain);
    assert_eq!(input.stats.def, 5);
    assert_eq!(input.weapon.as_ref().map(|w| w.max_range), Some(2));
    assert_eq!(input.weapon_rank, WeaponRank::B);
    assert_eq!(input.armour_weight, 0);
    let input = u.combat_input(&c, &cs, &t, Some(1), &terrain);
    assert_eq!(input.weapon.as_ref().map(|w| w.max_range), Some(1));
    // An empty slot, or nothing equipped: no weapon, rank E.
    let input = u.combat_input(&c, &cs, &t, Some(2), &terrain);
    assert_eq!((input.weapon, input.weapon_rank), (None, WeaponRank::E));
    u.loadout.equipped = None;
    assert_eq!(u.combat_input(&c, &cs, &t, None, &terrain).weapon, None);
    // A weapon the unit can't wield gives no weapon.
    let axe = equipped(&["axe"], None, None);
    assert_eq!(
        axe.combat_input(&c, &cs, &t, Some(0), &terrain).weapon,
        None
    );
}

// ---- Loadouts ---------------------------------------------------------------

#[test]
fn with_loadout_fills_slots_and_equips_the_first_usable_weapon() {
    let u = equipped(&["axe", "steel_sword", "sword"], Some("vest"), Some("ring"));
    assert_eq!(u.loadout.equipped, Some(2));
    assert_eq!(u.loadout.weapon_count(), 3);
    assert_eq!(
        u.loadout.equipped_weapon(),
        Some(&WeaponInstance {
            def: id("sword"),
            durability_left: 20,
        })
    );
    assert_eq!(u.loadout.armour, Some(id("vest")));
    assert_eq!(u.loadout.accessory, Some(id("ring")));
    // Nothing usable: nothing equipped (carrying is allowed).
    let u = equipped(&["axe"], None, None);
    assert_eq!(u.loadout.equipped, None);
    assert_eq!(u.loadout.equipped_weapon(), None);
    assert!(u.validate_loadout(&classes(), &items()).is_ok());
}

#[test]
fn loadout_errors() {
    let t = items();
    let try_with =
        |def: LoadoutDef, classes: &ClassTable| unit().with_loadout(&def, classes, &t).map(|_| ());
    let cs = classes();
    assert_eq!(
        try_with(loadout(&["sword"; 4], None, None), &cs),
        Err(LoadoutError::TooManyWeapons { count: 4, slots: 3 })
    );
    assert_eq!(
        try_with(loadout(&["nope"], None, None), &cs),
        Err(LoadoutError::UnknownItem(id("nope")))
    );
    assert_eq!(
        try_with(loadout(&["potion"], None, None), &cs),
        Err(LoadoutError::WrongSlot(id("potion")))
    );
    assert_eq!(
        try_with(loadout(&[], Some("ring"), None), &cs),
        Err(LoadoutError::WrongSlot(id("ring")))
    );
    assert_eq!(
        try_with(loadout(&[], None, Some("vest")), &cs),
        Err(LoadoutError::WrongSlot(id("vest")))
    );
    assert_eq!(
        try_with(loadout(&[], Some("nope"), None), &cs),
        Err(LoadoutError::UnknownItem(id("nope")))
    );
    assert_eq!(
        try_with(loadout(&[], Some("mail"), None), &cs),
        Err(LoadoutError::ArmourNotAllowed(id("mail")))
    );
    // Magic classes from tier 3: no weapon slots.
    let mut mage = class();
    mage.weapon_slots = 0;
    let mages = classes_with(mage);
    assert_eq!(
        try_with(loadout(&["sword"], None, None), &mages),
        Err(LoadoutError::TooManyWeapons { count: 1, slots: 0 })
    );
    assert!(try_with(loadout(&[], Some("vest"), Some("ring")), &mages).is_ok());
    let mut two = class();
    two.weapon_slots = 2;
    assert_eq!(
        try_with(
            loadout(&["sword", "axe", "sword"], None, None),
            &classes_with(two)
        ),
        Err(LoadoutError::TooManyWeapons { count: 3, slots: 2 })
    );
}

#[test]
fn validate_checks_the_equipped_slot_and_class() {
    let t = items();
    let cs = classes();
    let mut u = equipped(&["sword", "axe"], None, None);
    u.loadout.equipped = Some(2);
    assert_eq!(
        u.validate_loadout(&cs, &t),
        Err(LoadoutError::EquippedEmpty(2))
    );
    u.loadout.equipped = Some(1);
    assert_eq!(
        u.validate_loadout(&cs, &t),
        Err(LoadoutError::CannotWield(id("axe")))
    );
    u.loadout.equipped = Some(0);
    assert!(u.validate_loadout(&cs, &t).is_ok());
    u.class = ClassId("nope".into());
    assert_eq!(
        u.validate_loadout(&cs, &t),
        Err(LoadoutError::UnknownClass("nope".into()))
    );
}

#[test]
fn loadout_error_messages() {
    let cases = [
        (
            LoadoutError::UnknownClass("x".into()),
            "unknown class \"x\"",
        ),
        (LoadoutError::UnknownItem(id("x")), "unknown item \"x\""),
        (
            LoadoutError::WrongSlot(id("x")),
            "\"x\" doesn't go in that slot",
        ),
        (
            LoadoutError::TooManyWeapons { count: 4, slots: 3 },
            "4 weapons, but the class has 3 weapon slots",
        ),
        (
            LoadoutError::ArmourNotAllowed(id("x")),
            "the class can't wear \"x\"",
        ),
        (LoadoutError::EquippedEmpty(2), "equipped slot 2 is empty"),
        (LoadoutError::CannotWield(id("x")), "can't wield \"x\""),
    ];
    for (e, msg) in cases {
        assert_eq!(e.to_string(), msg);
    }
}

// ---- Pack and stock -----------------------------------------------------------

#[test]
fn pack_cap_limits_only_what_is_brought() {
    assert_eq!(BattlePack::bring(vec![id("potion"); 4], 3), None);
    let mut pack = BattlePack::bring(vec![id("potion"); 3], 3).unwrap();
    pack.gain(id("elixir"));
    assert_eq!(pack.items.len(), 4);
    assert_eq!(pack.cap, 3);
}

#[test]
fn stock_counts_and_packs() {
    let mut stock = Stock::default();
    for _ in 0..3 {
        stock.add(id("potion"));
    }
    stock.add(id("elixir"));
    assert_eq!(stock.count(&id("potion")), 3);
    assert!(stock.take(&id("elixir")));
    assert!(!stock.take(&id("elixir")));
    assert_eq!(stock.count(&id("elixir")), 0);
    assert!(!stock.items.contains_key(&id("elixir")));
    // Not enough in stock, or over the cap: nothing moves.
    let before = stock.clone();
    assert_eq!(stock.pack(vec![id("potion"); 4], 6), None);
    assert_eq!(stock.pack(vec![id("potion"); 3], 2), None);
    assert_eq!(stock, before);
    let pack = stock.pack(vec![id("potion"); 2], 2).unwrap();
    assert_eq!(pack.items, vec![id("potion"); 2]);
    assert_eq!(stock.count(&id("potion")), 1);
    stock.unpack(pack);
    assert_eq!(stock.count(&id("potion")), 3);
}

// ---- Properties -------------------------------------------------------------

fn arb_bonus() -> impl Strategy<Value = Stats> {
    (prop::array::uniform7(-5..40), -2..4).prop_map(|(v, mov)| Stats::from_growable(v, mov))
}

proptest! {
    #[test]
    fn effective_stats_stay_under_the_hard_ceilings(
        base in prop::array::uniform7(0..40),
        armour in arb_bonus(),
        accessory in arb_bonus(),
        ceilings in prop::array::uniform7(0..45),
    ) {
        let mut t = items();
        if let Some(ItemDef::Armour(a)) = t.items.get_mut(&id("vest")) {
            a.bonus = armour;
        }
        if let Some(ItemDef::Accessory(a)) = t.items.get_mut(&id("ring")) {
            a.bonus = accessory;
        }
        let mut cs = classes();
        cs.hard_ceilings = Stats::from_growable(ceilings, 15);
        let mut u = unit().with_loadout(&loadout(&[], Some("vest"), Some("ring")), &cs, &t).unwrap();
        u.stats = Stats::from_growable(base, 5);
        let s = u.effective_stats(&cs, &t);
        for kind in StatKind::ALL {
            let limit = u.stats.get(kind).max(cs.hard_ceilings.get(kind));
            prop_assert!(s.get(kind) <= limit, "{:?}", kind);
            if kind != StatKind::Mov {
                prop_assert!(s.get(kind) <= cs.hard_ceilings.get(kind) || s.get(kind) <= u.stats.get(kind));
            }
        }
    }

    #[test]
    fn a_valid_loadout_never_exceeds_the_weapon_slots(
        slots in 0u8..=5,
        picks in prop::collection::vec(prop::sample::select(vec!["sword", "axe", "bow", "potion"]), 0..6),
    ) {
        let mut c = class();
        c.weapon_slots = slots;
        let cs = classes_with(c);
        if let Ok(u) = unit().with_loadout(&loadout(&picks, None, None), &cs, &items()) {
            prop_assert!(u.loadout.weapon_count() <= usize::from(slots).min(WEAPON_SLOTS));
            prop_assert!(u.validate_loadout(&cs, &items()).is_ok());
        }
    }
}
