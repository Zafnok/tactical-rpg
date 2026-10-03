//! Tests of the campaign rules, on the battle tests' fixtures: a lord
//! (`lord`) and a unit (`ann`) deployed, `ben` on the bench, an empty slot
//! for `cal` (not in the roster) and two enemies next to them.

use std::sync::Arc;

use super::*;
use crate::battle::tests::{
    armed, attack, classes, item, items_for, leveling, lord, p, setup, unit,
};
use crate::battle::{Command, TriggerWhen, UnitAction};
use crate::item::WeaponInstance;
use crate::lead::LeadGender;
use crate::support::{ByRank, PairDef, SupportRank, SupportRules};

fn c(id: &str) -> CharacterId {
    CharacterId(id.into())
}

/// `u` as character `id`.
fn named(u: Unit, id: &str) -> Unit {
    Unit {
        character: Some(c(id)),
        name: id.into(),
        ..u
    }
}

/// The game's tables, with levels on (EXP awards count) and every
/// weapon the units carry.
fn tables() -> GameTables {
    let s = setup(vec![]);
    let enemies = def().enemies;
    GameTables {
        terrain: s.terrain,
        classes: Arc::new(leveling(classes())),
        items: Arc::new(items_for(roster().iter().chain(&enemies))),
        spells: s.spells,
        skills: s.skills,
        arts: s.arts,
        supports: Arc::new(support_table()),
    }
}

/// The design's support rules, with two pairs: `lord`–`ann` and
/// `ann`–`ben`. (`lord`–`ben` have no support.)
fn support_table() -> SupportTable {
    let def = |a: &str, b: &str| PairDef {
        pair: pair(a, b),
        thresholds: None,
        starting_points: 0,
        conversations: ByRank {
            c: format!("{a}_{b}_c"),
            b: format!("{a}_{b}_b"),
            a: format!("{a}_{b}_a"),
        },
    };
    SupportTable::new(
        SupportRules::STARTING,
        [def("lord", "ann"), def("ann", "ben")],
    )
}

fn pair(a: &str, b: &str) -> SupportPair {
    SupportPair::new(c(a), c(b))
}

/// The roster: the lord (might 10), `ann` wearing a vest and a charm, and
/// `ben`. Ids and places are the battle's business.
fn roster() -> Vec<Unit> {
    let mut ann = named(unit(90, Faction::Player, p(7, 4)), "ann");
    ann.loadout.armour = Some(item("vest"));
    ann.loadout.accessory = Some(item("charm"));
    vec![
        named(armed(lord(91, p(7, 4)), 10), "lord"),
        ann,
        named(unit(92, Faction::Player, p(7, 4)), "ben"),
    ]
}

fn campaign(mode: GameMode) -> Campaign {
    let mut stock = Stock::default();
    stock.add(item("tonic"));
    Campaign::new_game(
        mode,
        LeadProfile::new("Mara", LeadGender::Female),
        "ch01",
        roster(),
        100,
        stock,
    )
}

/// Slots for the lord at (0,0), `ann` at (0,2) and `cal` at (3,3); enemy 4
/// at (1,0) and `rook` (5, might 20, joins if defeated) at (1,2); a pack of
/// two potions; Rout.
fn def() -> BattleDef {
    let slot = |id: &str, pos, n| PlayerSlot {
        character: c(id),
        pos,
        id: BattleDef::slot_id(n),
    };
    BattleDef {
        id: "test".into(),
        map: setup(vec![]).map,
        player_slots: vec![
            slot("lord", p(0, 0), 0),
            slot("ann", p(0, 2), 1),
            slot("cal", p(3, 3), 2),
        ],
        enemies: vec![
            unit(4, Faction::Enemy, p(1, 0)),
            Unit {
                role: Role::Boss,
                ..named(armed(unit(5, Faction::Enemy, p(1, 2)), 20), "rook")
            },
        ],
        reinforcements: vec![],
        preparations: false,
        pack_cap: 3,
        default_pack: vec![item("potion"), item("potion")],
        clear_gold: 500,
        objective: Objective::Rout { turn_limit: None },
        triggers: vec![Trigger {
            when: TriggerWhen::UnitFell {
                unit: c("rook"),
                mode: None,
                recruit: true,
            },
            scene: "rook_joins".into(),
            once: true,
        }],
        battle_notes: vec![BattleNote {
            text: "Rook: joins if defeated.".into(),
            units: vec![UnitId(5)],
        }],
        difficulty: Difficulty::Normal,
        music: BattleMusic::Pool("skirmish".into()),
        seed: 3,
    }
}

fn act(s: &mut BattleState, id: u32, dest: Pos, action: UnitAction) {
    let cmd = Command::Act {
        unit: UnitId(id),
        dest,
        action,
    };
    s.apply(&cmd).unwrap_or_else(|e| panic!("{e}"));
}

fn end(s: &mut BattleState) {
    s.apply(&Command::EndPhase)
        .unwrap_or_else(|e| panic!("{e}"));
}

/// The battle won: the lord kills enemy 4; `rook` fells `ann` in the enemy
/// phase; the lord kills `rook` on turn 2 (it joins).
fn won(campaign: &Campaign, def: &BattleDef) -> BattleState {
    let (mut s, _) = BattleState::new(campaign.battle_setup(def, &tables()));
    act(&mut s, 1, p(0, 0), attack(4));
    end(&mut s);
    act(&mut s, 5, p(1, 2), attack(2));
    assert!(s.unit(UnitId(2)).is_none(), "ann fell");
    end(&mut s);
    act(&mut s, 1, p(1, 1), attack(5));
    assert_eq!(s.outcome(), Some(Outcome::Victory));
    s
}

#[test]
fn difficulty_sets_the_rewind_charges() {
    let charges = Difficulty::ALL.map(Difficulty::rewind_charges);
    assert_eq!(charges, [2, 3, 5, 8]);
    for d in Difficulty::ALL {
        let def = BattleDef {
            difficulty: d,
            ..def()
        };
        let setup = campaign(GameMode::Classic).battle_setup(&def, &tables());
        assert_eq!(setup.rewind_charges, d.rewind_charges());
    }
    assert_eq!(Difficulty::default(), Difficulty::Normal);
}

#[test]
fn new_game_names_the_lead_and_rests_the_roster() {
    let mut units = roster();
    units[0].character = Some(c(LEAD_ID));
    units[1].hp = 1;
    units[1].acted = true;
    units[1].effects.push(crate::skill::TimedEffect {
        source: crate::skill::SkillId::new("keen").into(),
        mods: crate::skill::TimedMods::default(),
        until: crate::battle::Phase::Player,
    });
    let game = Campaign::new_game(
        GameMode::Casual,
        LeadProfile::new("Mara", LeadGender::Female),
        "ch01",
        units,
        5,
        Stock::default(),
    );
    assert_eq!(game.roster[0].name, "Mara");
    assert_eq!(game.roster[0].map_label, "Ma");
    assert_eq!(game.roster[1].name, "ann", "only the lead is renamed");
    assert_eq!(game.roster[1].hp, game.roster[1].stats.hp);
    assert!(!game.roster[1].acted);
    assert!(game.roster[1].effects.is_empty());
    assert_eq!(game.mode, GameMode::Casual);
    assert_eq!(game.chapter, "ch01");
    assert_eq!(game.gold, 5);
    assert_eq!(game.playtime_s, 0);
    assert!(game.flags.is_empty());
}

#[test]
fn battle_setup_places_the_roster_in_its_slots() {
    let game = campaign(GameMode::Casual);
    let def = BattleDef {
        reinforcements: vec![Reinforcement {
            turn: 2,
            unit: unit(6, Faction::Enemy, p(7, 4)),
        }],
        ..def()
    };
    let tables = tables();
    let setup = game.battle_setup(&def, &tables);
    let placed: Vec<_> = setup
        .units
        .iter()
        .map(|u| (u.id.0, u.name.as_str(), u.pos, u.faction))
        .collect();
    // `cal` isn't in the roster (slot 3 stays empty); `ben` has no slot.
    assert_eq!(
        placed,
        [
            (1, "lord", p(0, 0), Faction::Player),
            (2, "ann", p(0, 2), Faction::Player),
            (4, "u4", p(1, 0), Faction::Enemy),
            (5, "rook", p(1, 2), Faction::Enemy),
        ]
    );
    assert_eq!(BattleDef::first_enemy_id(def.player_slots.len()), 4);
    assert_eq!(setup.pack.items, def.default_pack);
    assert_eq!(setup.pack.cap, 3);
    assert_eq!(setup.gold, 100);
    assert_eq!(setup.stock, game.stock);
    assert_eq!(setup.mode, GameMode::Casual);
    assert_eq!(setup.rewind_charges, 3);
    assert_eq!(setup.seed, 3);
    assert_eq!(setup.triggers, def.triggers);
    assert_eq!(setup.battle_notes, def.battle_notes);
    // The battle keeps them, for the notes panel and the Objective page.
    let (state, _) = BattleState::new(setup.clone());
    assert_eq!(state.battle_notes(), def.battle_notes);
    assert_eq!(setup.objective, def.objective);
    assert_eq!(setup.map, def.map);
    assert!(Arc::ptr_eq(&setup.terrain, &tables.terrain));
    assert!(Arc::ptr_eq(&setup.classes, &tables.classes));
    assert!(Arc::ptr_eq(&setup.items, &tables.items));
    assert!(Arc::ptr_eq(&setup.spells, &tables.spells));
    assert!(Arc::ptr_eq(&setup.skills, &tables.skills));
    assert!(Arc::ptr_eq(&setup.arts, &tables.arts));
    assert_eq!(setup.reinforcements, def.reinforcements);
    // The roster itself is untouched.
    assert_eq!(game.roster[0].id, UnitId(91));
    // A slot numbered after the enemies comes after them, in id order.
    let mut late = def.clone();
    late.player_slots[1].id = UnitId(6);
    let ids: Vec<u32> = game
        .battle_setup(&late, &tables)
        .units
        .iter()
        .map(|u| u.id.0)
        .collect();
    assert_eq!(ids, [1, 4, 5, 6]);
}

#[test]
fn classic_death_leaves_the_roster_and_its_gear_goes_to_the_stock() {
    let mut game = campaign(GameMode::Classic);
    let def = def();
    let s = won(&game, &def);
    let rewards = game.apply_result(&def, &s, 0).unwrap();
    let names: Vec<_> = game.roster.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, ["lord", "ben", "rook"]);
    assert_eq!(rewards.lost, [c("ann")]);
    assert_eq!(rewards.joined, [c("rook")]);
    // Ann's sword (with its durability), vest and charm; the tonic the
    // campaign had; the two unused potions.
    let sword = roster()[1].loadout.weapons[0].clone();
    assert_eq!(game.stock.weapons, Vec::from_iter(sword));
    assert!(
        game.stock
            .weapons
            .iter()
            .all(|w: &WeaponInstance| w.durability_left == 20)
    );
    for (id, n) in [("vest", 1), ("charm", 1), ("tonic", 1), ("potion", 2)] {
        assert_eq!(game.stock.count(&item(id)), n, "{id}");
    }
    assert_eq!(game.gold, 600);
}

#[test]
fn casual_retreat_stays_in_the_roster_at_full_hp() {
    let mut game = campaign(GameMode::Casual);
    let def = def();
    let s = won(&game, &def);
    let rewards = game.apply_result(&def, &s, 0).unwrap();
    let names: Vec<_> = game.roster.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, ["lord", "ann", "ben", "rook"]);
    assert!(rewards.lost.is_empty());
    let ann = &game.roster[1];
    assert_eq!(ann.hp, ann.stats.hp);
    assert_eq!(game.stock.weapons, []);
    assert_eq!(game.stock.count(&item("vest")), 0);
}

#[test]
fn the_roster_takes_what_the_battle_changed_and_rests() {
    let mut game = campaign(GameMode::Casual);
    let def = def();
    let s = won(&game, &def);
    let lord = s.unit(UnitId(1)).unwrap().clone();
    assert!(!lord.weapon_exp.is_empty());
    assert!(lord.acted);
    game.apply_result(&def, &s, 0).unwrap();
    let after = &game.roster[0];
    assert_eq!(after.weapon_exp, lord.weapon_exp);
    assert_eq!(after.exp, lord.exp);
    assert_eq!(after.level, lord.level);
    assert!(!after.acted);
    assert_eq!(after.hp, after.stats.hp);
    // `ben` sat it out.
    assert_eq!(game.roster[2], campaign(GameMode::Casual).roster[2]);
}

#[test]
fn unused_charges_give_exp_to_every_deployed_unit() {
    for mode in [GameMode::Classic, GameMode::Casual] {
        let mut game = campaign(mode);
        let def = def();
        let s = won(&game, &def);
        let rewards = game.apply_result(&def, &s, 3).unwrap();
        assert_eq!(rewards.bonus_exp, 21, "7% of 100 per charge");
        assert_eq!(rewards.unused_charges, 3);
        assert_eq!(rewards.clear_gold, 500);
        let gained: Vec<_> = rewards
            .events
            .iter()
            .filter_map(|e| match e {
                Event::ExpGained { unit, amount } => Some((unit.0, *amount)),
                _ => None,
            })
            .collect();
        // Ann retreated in Casual and gets it too; in Classic she's gone.
        // Ben (not deployed) and the recruit get nothing.
        let expected: &[(u32, u32)] = match mode {
            GameMode::Classic => &[(1, 21)],
            GameMode::Casual => &[(1, 21), (2, 21)],
        };
        assert_eq!(gained, expected, "{mode:?}");
        // The results screen's rows: each deployed unit before the bonus,
        // and what it got.
        let rows: Vec<_> = rewards
            .deployed
            .iter()
            .map(|u| (u.id.0, rewards.exp_gained(u.id)))
            .collect();
        assert_eq!(rows, expected, "{mode:?}");
        assert_eq!(rewards.deployed[0], *s.unit(UnitId(1)).unwrap());
        assert_eq!(rewards.exp_gained(UnitId(3)), 0, "nobody's id");
        let lord = s.unit(UnitId(1)).unwrap();
        let total = |u: &Unit| u.level * EXP_PER_LEVEL + u.exp;
        assert_eq!(total(&game.roster[0]), total(lord) + 21);
        let ben = game.member(&c("ben")).unwrap();
        assert_eq!(ben.exp, 0);
        if mode == GameMode::Casual {
            let ann = s.fallen().iter().find(|u| u.id == UnitId(2)).unwrap();
            assert_eq!(total(&game.roster[1]), total(ann) + 21);
        }
    }
}

/// Every charge used: the gold, the deployed units, and no EXP for anyone.
#[test]
fn no_unused_charges_give_gold_and_no_exp() {
    let mut game = campaign(GameMode::Casual);
    let def = def();
    let s = won(&game, &def);
    let rewards = game.apply_result(&def, &s, 0).unwrap();
    assert_eq!((rewards.clear_gold, rewards.bonus_exp), (500, 0));
    assert_eq!(rewards.events, []);
    assert_eq!(rewards.deployed.len(), 2);
    assert!(
        rewards
            .deployed
            .iter()
            .all(|u| rewards.exp_gained(u.id) == 0)
    );
    assert_eq!(game.gold, s.gold() + 500);
}

/// A unit at the level cap gets nothing from the bonus.
#[test]
fn a_unit_at_the_level_cap_gets_no_bonus() {
    let mut game = campaign(GameMode::Classic);
    let def = def();
    game.roster[0].level = tables().classes.level_cap;
    let s = won(&game, &def);
    let rewards = game.apply_result(&def, &s, 3).unwrap();
    assert_eq!(rewards.bonus_exp, 21);
    assert_eq!(rewards.exp_gained(UnitId(1)), 0);
}

/// No cap on the bonus (Nick): past a level, the rest carries over.
#[test]
fn the_bonus_is_one_award_with_no_cap() {
    let mut game = campaign(GameMode::Casual);
    let def = def();
    let s = won(&game, &def);
    let lord = s.unit(UnitId(1)).unwrap().clone();
    let rewards = game.apply_result(&def, &s, 20).unwrap();
    assert_eq!(rewards.bonus_exp, 140);
    let total = |u: &Unit| u.level * EXP_PER_LEVEL + u.exp;
    assert_eq!(total(&game.roster[0]), total(&lord) + 140);
}

#[test]
fn recruits_join_after_a_victory_but_not_after_a_defeat() {
    let def = def();
    // Won: rook joins, as a ready player unit at full HP.
    let mut game = campaign(GameMode::Classic);
    let s = won(&game, &def);
    assert_eq!(s.recruited().len(), 1);
    game.apply_result(&def, &s, 0).unwrap();
    let rook = game.member(&c("rook")).unwrap();
    assert_eq!(rook.faction, Faction::Player);
    assert_eq!(rook.role, Role::Regular);
    assert_eq!(rook.hp, rook.stats.hp);
    // A second copy of the same recruit doesn't join twice.
    let before = game.roster.len();
    game.apply_result(&def, &s, 0).unwrap();
    assert_eq!(game.roster.len(), before);
    // Lost: the lord fells rook, then the turn limit runs out.
    let mut game = campaign(GameMode::Classic);
    let limited = BattleDef {
        objective: Objective::Rout {
            turn_limit: Some(1),
        },
        ..def
    };
    let (mut s, _) = BattleState::new(game.battle_setup(&limited, &tables()));
    act(&mut s, 1, p(1, 1), attack(5));
    assert_eq!(s.recruited().len(), 1);
    end(&mut s);
    end(&mut s);
    assert_eq!(s.outcome(), Some(Outcome::Defeat));
    let unchanged = game.clone();
    assert_eq!(game.apply_result(&limited, &s, 3), Err(ApplyError::NotWon));
    assert_eq!(game, unchanged);
    assert!(game.member(&c("rook")).is_none());
    // Not over yet: not won either.
    let (fresh, _) = BattleState::new(game.battle_setup(&limited, &tables()));
    assert_eq!(
        game.apply_result(&limited, &fresh, 3),
        Err(ApplyError::NotWon)
    );
    assert_eq!(ApplyError::NotWon.to_string(), "the battle wasn't won");
}

#[test]
fn mode_goes_from_classic_to_casual_only() {
    let mut game = campaign(GameMode::Classic);
    assert!(game.downgrade_mode());
    assert_eq!(game.mode, GameMode::Casual);
    assert!(!game.downgrade_mode());
    assert_eq!(game.mode, GameMode::Casual);
}

#[test]
fn campaign_round_trips_through_ron() {
    let mut game = campaign(GameMode::Casual);
    game.flags.insert("met_bors".into(), true);
    game.playtime_s = 1234;
    let def = def();
    let s = won(&game, &def);
    game.apply_result(&def, &s, 2).unwrap();
    game.supports
        .gain(&pair("lord", "ann"), 20, &support_table());
    game.view_support(&c("lord"), &c("ann"), &support_table())
        .unwrap();
    game.supports.gain(&pair("ann", "ben"), 7, &support_table());
    let text = ron::to_string(&game).unwrap();
    let back: Campaign = ron::from_str(&text).unwrap();
    assert_eq!(back, game);
    assert_eq!(
        back.supports.rank(&pair("lord", "ann")),
        Some(SupportRank::C)
    );
    assert_eq!(back.lead, LeadProfile::new("Mara", LeadGender::Female));
}

// ---- Supports ----------------------------------------------------------------

/// The battle won with the lord and `ann` side by side: `ann` steps next
/// to the lord, who fights beside her twice (3 points each) and they end
/// one player phase adjacent (1 point): 7 points for the pair.
fn won_together(campaign: &Campaign, def: &BattleDef) -> BattleState {
    let (mut s, _) = BattleState::new(campaign.battle_setup(def, &tables()));
    act(&mut s, 2, p(0, 1), UnitAction::Wait);
    act(&mut s, 1, p(0, 0), attack(4));
    end(&mut s);
    end(&mut s);
    act(&mut s, 1, p(1, 1), attack(5));
    assert_eq!(s.outcome(), Some(Outcome::Victory));
    s
}

fn support_points(game: &Campaign, a: &str, b: &str) -> u32 {
    let state = game.supports.state(&pair(a, b), &support_table());
    state.map_or(0, |s| s.points())
}

#[test]
fn support_points_carry_from_battle_to_battle() {
    let mut game = campaign(GameMode::Casual);
    assert_eq!(game.supports, SupportBook::default());
    game.supports
        .gain(&pair("lord", "ann"), 5, &support_table());
    let def = def();
    // The battle starts with the campaign's supports and its table.
    let (start, _) = BattleState::new(game.battle_setup(&def, &tables()));
    assert_eq!(start.supports(), &game.supports);
    assert_eq!(start.support_table(), &support_table());
    let s = won_together(&game, &def);
    game.apply_result(&def, &s, 0).unwrap();
    assert_eq!(support_points(&game, "lord", "ann"), 12);
    // Every battle counts, a skirmish after the story battle too.
    let s = won_together(&game, &def);
    game.apply_result(&def, &s, 0).unwrap();
    assert_eq!(support_points(&game, "lord", "ann"), 19);
    assert_eq!(support_points(&game, "ann", "ben"), 0);
}

#[test]
fn a_battle_that_wasnt_won_gives_no_support_points() {
    let mut game = campaign(GameMode::Casual);
    let def = def();
    let (mut s, _) = BattleState::new(game.battle_setup(&def, &tables()));
    act(&mut s, 2, p(0, 1), UnitAction::Wait);
    end(&mut s);
    let in_battle = s.supports().state(&pair("lord", "ann"), &support_table());
    assert_eq!(in_battle.map(|s| s.points()), Some(1));
    assert_eq!(game.apply_result(&def, &s, 0), Err(ApplyError::NotWon));
    assert_eq!(game.supports, SupportBook::default());
}

#[test]
fn a_pair_gains_one_rank_per_camp_visit_however_many_battles_it_fights() {
    let mut game = campaign(GameMode::Casual);
    let table = support_table();
    game.supports.gain(&pair("lord", "ann"), 19, &table);
    let def = def();
    // The points wait at C's threshold, battle after battle.
    for _ in 0..3 {
        let s = won_together(&game, &def);
        game.apply_result(&def, &s, 0).unwrap();
        assert_eq!(support_points(&game, "lord", "ann"), 20);
        assert_eq!(game.supports.rank(&pair("lord", "ann")), None);
        assert_eq!(
            game.supports.unlocked(&table),
            [(pair("lord", "ann"), SupportRank::C)]
        );
    }
    // Viewing the conversation at camp gains the rank, either way round.
    assert_eq!(
        game.view_support(&c("ann"), &c("lord"), &table),
        Ok(SupportViewed {
            rank: SupportRank::C,
            conversation: "lord_ann_c".into()
        })
    );
    assert_eq!(
        game.supports.rank(&pair("lord", "ann")),
        Some(SupportRank::C)
    );
    assert_eq!(game.supports.unlocked(&table), []);
    // Then the next battle's points count again.
    let s = won_together(&game, &def);
    game.apply_result(&def, &s, 0).unwrap();
    assert_eq!(support_points(&game, "lord", "ann"), 27);
}

#[test]
fn a_support_conversation_needs_a_pair_in_the_army_with_one_unlocked() {
    let mut game = campaign(GameMode::Casual);
    let table = support_table();
    let unchanged = game.clone();
    let mut view = |a: &str, b: &str| game.view_support(&c(a), &c(b), &table);
    assert_eq!(view("lord", "ann"), Err(SupportError::NothingUnlocked));
    assert_eq!(view("lord", "ben"), Err(SupportError::NoSuchPair));
    assert_eq!(view("rook", "ann"), Err(SupportError::NotInArmy(c("rook"))));
    assert_eq!(view("ann", "rook"), Err(SupportError::NotInArmy(c("rook"))));
    assert_eq!(game.roster, unchanged.roster);
}

/// A campaign where `lord`–`ann` have C unlocked, and `ann`–`ben` are at
/// rank C with B unlocked.
fn bonded(mode: GameMode) -> Campaign {
    let mut game = campaign(mode);
    let table = support_table();
    game.supports.gain(&pair("lord", "ann"), 20, &table);
    game.supports.gain(&pair("ann", "ben"), 20, &table);
    game.view_support(&c("ann"), &c("ben"), &table).unwrap();
    game.supports.gain(&pair("ann", "ben"), 60, &table);
    game
}

#[test]
fn a_classic_death_ends_the_units_supports() {
    let mut game = bonded(GameMode::Classic);
    let table = support_table();
    assert_eq!(game.supports.unlocked(&table).len(), 2);
    let def = def();
    let s = won(&game, &def);
    let rewards = game.apply_result(&def, &s, 0).unwrap();
    assert_eq!(rewards.lost, [c("ann")]);
    // Her unviewed conversations are gone; the viewed one stays seen.
    assert_eq!(game.supports.unlocked(&table), []);
    let state = |a, b| game.supports.state(&pair(a, b), &table).unwrap();
    assert!(state("lord", "ann").ended());
    assert_eq!(state("lord", "ann").rank(), None);
    assert!(state("ann", "ben").ended());
    assert_eq!(
        state("ann", "ben").seen().collect::<Vec<_>>(),
        [SupportRank::C]
    );
    assert_eq!(
        game.view_support(&c("lord"), &c("ann"), &table),
        Err(SupportError::NotInArmy(c("ann")))
    );
}

#[test]
fn a_casual_retreat_keeps_the_units_supports() {
    let mut game = bonded(GameMode::Casual);
    let table = support_table();
    let before = game.supports.clone();
    let def = def();
    let s = won(&game, &def);
    game.apply_result(&def, &s, 0).unwrap();
    assert_eq!(game.supports, before);
    assert_eq!(game.supports.unlocked(&table).len(), 2);
    assert_eq!(
        game.view_support(&c("lord"), &c("ann"), &table)
            .map(|v| v.rank),
        Ok(SupportRank::C)
    );
}
