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
    }
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
    let slot = |id: &str, pos| PlayerSlot {
        character: c(id),
        pos,
    };
    BattleDef {
        id: "test".into(),
        map: setup(vec![]).map,
        player_slots: vec![
            slot("lord", p(0, 0)),
            slot("ann", p(0, 2)),
            slot("cal", p(3, 3)),
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
        solo_stock: vec![],
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
        difficulty: Difficulty::Normal,
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
    let text = ron::to_string(&game).unwrap();
    let back: Campaign = ron::from_str(&text).unwrap();
    assert_eq!(back, game);
    assert_eq!(back.lead, LeadProfile::new("Mara", LeadGender::Female));
}
