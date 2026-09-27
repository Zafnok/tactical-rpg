//! Terrain magic (`magic.md`, "Terrain magic"): Fire burns a forest for one
//! round, then leaves it burnt; Frost freezes water and sea for good.

use super::*;

fn sid(id: &str) -> SpellId {
    SpellId::new(id)
}

/// A test unit knowing `spells`, with no weapons.
fn mage(id: u32, faction: Faction, pos: Pos, spells: &[&str]) -> Unit {
    Unit {
        learned: spells.iter().map(|s| sid(s)).collect(),
        loadout: Loadout::default(),
        ..unit(id, faction, pos)
    }
}

fn on_tile(spell: &str, pos: Pos) -> UnitAction {
    UnitAction::Cast {
        spell: sid(spell),
        target: CastTarget::Tile(pos),
    }
}

fn tile(s: &BattleState, pos: Pos) -> TerrainId {
    *s.map().tiles.get(pos).unwrap()
}

fn changed(pos: Pos, from: TerrainId, to: TerrainId) -> Event {
    Event::TerrainChanged { pos, from, to }
}

/// Whether unit `id` could move onto (or through) `pos` now.
fn passable(s: &BattleState, id: u32, pos: Pos) -> bool {
    reachable(s.map(), s.terrain(), s.classes(), s.units(), UnitId(id))
        .unwrap()
        .is_passable(pos)
}

/// The forest at (2,2) of [`OPEN`].
const WOOD: Pos = Pos { x: 2, y: 2 };

/// Player mage 1 (lord) at (0,2), next to the forest; enemy 3 at (3,3) and
/// ally 5 at (2,4), both next to it; enemy 4 far away.
fn burn_cast() -> Vec<Unit> {
    vec![
        Unit {
            is_lord: true,
            ..mage(
                1,
                Faction::Player,
                p(0, 2),
                &["fire", "frost", "force", "heal"],
            )
        },
        unit(2, Faction::Player, p(0, 0)),
        unit(3, Faction::Enemy, p(3, 3)),
        unit(4, Faction::Enemy, p(7, 0)),
        unit(5, Faction::Ally, p(2, 4)),
    ]
}

#[test]
fn a_player_fire_burns_until_the_next_player_phase() {
    let mut units = burn_cast();
    units[0].loadout.equipped = Some(Equipped::Spell(sid("force")));
    let mut s = start(setup(units));
    assert!(passable(&s, 3, WOOD));
    let events = act(&mut s, 1, p(0, 2), on_tile("fire", WOOD));
    assert_eq!(
        events,
        [
            Event::SpellCast {
                unit: UnitId(1),
                spell: sid("fire"),
                target: CastTarget::Tile(WOOD),
            },
            changed(WOOD, FOREST, BURNING),
            Event::SpellUsesChanged {
                unit: UnitId(1),
                spell: sid("fire"),
                uses_left: 9,
            },
            Event::UnitActed { unit: UnitId(1) },
        ]
    );
    assert_eq!(tile(&s, WOOD), BURNING);
    assert_eq!(
        s.burning(),
        [Burning {
            pos: WOOD,
            phase: Phase::Player,
            then: BURNT,
        }]
    );
    // No equip: a tile cast isn't an attack.
    assert_eq!(
        s.unit(UnitId(1)).unwrap().loadout.equipped,
        Some(Equipped::Spell(sid("force")))
    );
    // Impassable for everyone through the Enemy and Other phases.
    assert!(!passable(&s, 1, WOOD));
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    assert!(!passable(&s, 3, WOOD));
    refused_act(
        &mut s,
        3,
        WOOD,
        UnitAction::Wait,
        CommandError::CannotStop(WOOD),
    );
    assert_eq!(end(&mut s), [started(1, Phase::Other)]);
    assert!(!passable(&s, 5, WOOD));
    assert_eq!(tile(&s, WOOD), BURNING);
    // Burnt, and walkable, from the start of the next Player phase.
    assert_eq!(
        end(&mut s),
        [changed(WOOD, BURNING, BURNT), started(2, Phase::Player)]
    );
    assert_eq!(tile(&s, WOOD), BURNT);
    assert!(s.burning().is_empty());
    assert!(passable(&s, 1, WOOD));
    act(&mut s, 1, WOOD, UnitAction::Wait);
    // …for good.
    walk(&mut s, 3);
    assert_eq!(tile(&s, WOOD), BURNT);
}

#[test]
fn an_enemy_fire_burns_until_the_next_enemy_phase() {
    let mut units = burn_cast();
    units[2] = mage(3, Faction::Enemy, p(3, 3), &["fire"]);
    let mut s = start(setup(units));
    end(&mut s);
    act(&mut s, 3, p(3, 3), on_tile("fire", WOOD));
    assert_eq!(s.burning()[0].phase, Phase::Enemy);
    assert_eq!(end(&mut s), [started(1, Phase::Other)]);
    assert_eq!(end(&mut s), [started(2, Phase::Player)]);
    assert!(!passable(&s, 1, WOOD));
    assert_eq!(
        end(&mut s),
        [changed(WOOD, BURNING, BURNT), started(2, Phase::Enemy)]
    );
    assert!(passable(&s, 3, WOOD));
}

#[test]
fn a_burning_tile_burns_out_even_if_its_phase_is_skipped() {
    let units = vec![
        lord(1, p(0, 2)),
        armed(unit(2, Faction::Player, p(4, 1)), 10),
        mage(3, Faction::Enemy, p(4, 2), &["fire"]),
    ];
    let mut s = with_objective(units, Objective::Survive { turns: 5 });
    end(&mut s);
    act(&mut s, 3, p(4, 2), on_tile("fire", WOOD));
    end(&mut s);
    act(&mut s, 2, p(4, 1), attack(3));
    assert!(s.unit(UnitId(3)).is_none());
    // The Enemy phase has no units left: skipped, but the fire goes out.
    assert_eq!(
        end(&mut s),
        [changed(WOOD, BURNING, BURNT), started(3, Phase::Player)]
    );
}

#[test]
fn a_reinforcement_waits_while_its_tile_burns() {
    let arriving = unit(9, Faction::Enemy, WOOD);
    let mut s = start(BattleSetup {
        reinforcements: vec![reinforcement(1, arriving)],
        ..setup(burn_cast())
    });
    act(&mut s, 1, p(0, 2), on_tile("fire", WOOD));
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    assert!(s.unit(UnitId(9)).is_none());
    assert_eq!(end(&mut s), [started(1, Phase::Other)]);
    assert_eq!(
        end(&mut s),
        [changed(WOOD, BURNING, BURNT), started(2, Phase::Player)]
    );
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(9)]
            },
            started(2, Phase::Enemy)
        ]
    );
}

#[test]
fn frost_freezes_sea_for_good() {
    let rows = [
        "...s....", //
        "...s....", //
        "...s....", //
        "...s....", //
        "...s....", //
    ];
    let units = vec![
        lord(1, p(2, 1)),
        mage(2, Faction::Player, p(2, 2), &["frost"]),
        unit(3, Faction::Enemy, p(7, 0)),
    ];
    let mut s = start(BattleSetup {
        map: map(&rows),
        ..setup(units)
    });
    let sea = p(3, 2);
    assert!(!passable(&s, 1, sea));
    let events = act(&mut s, 2, p(2, 2), on_tile("frost", sea));
    assert_eq!(events[1], changed(sea, SEA, ICE));
    assert!(s.burning().is_empty());
    // A foot unit crosses at once…
    act(&mut s, 1, p(4, 2), UnitAction::Wait);
    // …and on every later turn.
    for _ in 0..3 {
        walk(&mut s, 2);
        assert_eq!(tile(&s, sea), ICE);
        let back = if s.unit(UnitId(1)).unwrap().pos.x > 3 {
            p(2, 1)
        } else {
            p(4, 2)
        };
        act(&mut s, 1, back, UnitAction::Wait);
    }
}

#[test]
fn frost_freezes_water() {
    let rows = ["..~.....", "........", "........", "........", "........"];
    let units = vec![
        mage(1, Faction::Player, p(0, 0), &["frost"]),
        unit(3, Faction::Enemy, p(7, 0)),
    ];
    let mut s = start(BattleSetup {
        map: map(&rows),
        ..setup(units)
    });
    act(&mut s, 1, p(1, 0), on_tile("frost", p(2, 0)));
    assert_eq!(tile(&s, p(2, 0)), ICE);
}

#[test]
fn a_cast_at_a_unit_never_changes_terrain() {
    let mut units = burn_cast();
    units[2].pos = WOOD;
    let mut s = start(setup(units));
    act(
        &mut s,
        1,
        p(0, 2),
        UnitAction::Cast {
            spell: sid("fire"),
            target: CastTarget::Unit(UnitId(3)),
        },
    );
    assert_eq!(tile(&s, WOOD), FOREST);
    assert!(s.burning().is_empty());
}

#[test]
fn the_casters_old_tile_can_be_burnt_once_it_moves_off() {
    let mut units = burn_cast();
    units[0].pos = WOOD;
    let mut s = start(setup(units));
    act(&mut s, 1, p(2, 0), on_tile("fire", WOOD));
    assert_eq!(tile(&s, WOOD), BURNING);
}

#[test]
fn invalid_tile_casts_change_nothing() {
    let rows = [
        "........", //
        "........", //
        "..f.....", //
        "..f.....", //
        "~.......", //
    ];
    let mut units = burn_cast();
    units[2].pos = p(2, 3); // An enemy on the second forest.
    units.push(mage(6, Faction::Player, p(4, 0), &["fire"]));
    let mut s = start(BattleSetup {
        map: map(&rows),
        ..setup(units)
    });
    let at = p(1, 2);
    let bad = |s: &mut BattleState, spell: &str, pos: Pos, err: CommandError| {
        refused_act(s, 1, at, on_tile(spell, pos), err);
    };
    bad(&mut s, "fire", p(2, 3), CommandError::TileOccupied(p(2, 3)));
    bad(&mut s, "fire", p(1, 1), CommandError::WrongTerrain(p(1, 1)));
    bad(&mut s, "frost", WOOD, CommandError::WrongTerrain(WOOD));
    bad(
        &mut s,
        "fire",
        p(4, 2),
        CommandError::TileOutOfRange {
            pos: p(4, 2),
            distance: 3,
        },
    );
    // Its own tile, once it has moved there.
    bad(
        &mut s,
        "fire",
        at,
        CommandError::TileOutOfRange {
            pos: at,
            distance: 0,
        },
    );
    bad(&mut s, "fire", p(-1, 2), CommandError::TileOffMap(p(-1, 2)));
    bad(
        &mut s,
        "force",
        WOOD,
        CommandError::NoTerrainEffect(sid("force")),
    );
    bad(
        &mut s,
        "heal",
        WOOD,
        CommandError::NoTerrainEffect(sid("heal")),
    );
    bad(
        &mut s,
        "bolt",
        WOOD,
        CommandError::SpellNotKnown {
            unit: UnitId(1),
            spell: sid("bolt"),
        },
    );
    // Already burning.
    act(&mut s, 1, at, on_tile("fire", WOOD));
    refused_act(
        &mut s,
        6,
        p(3, 1),
        on_tile("fire", WOOD),
        CommandError::WrongTerrain(WOOD),
    );
    // No uses left.
    let mut s = start(setup(burn_cast()));
    s.units[0].spells.uses_left.insert(sid("fire"), 0);
    bad(
        &mut s,
        "fire",
        WOOD,
        CommandError::NoUsesLeft {
            unit: UnitId(1),
            spell: sid("fire"),
        },
    );
}

#[test]
fn error_messages() {
    let cases = [
        (
            CommandError::NoTerrainEffect(sid("force")),
            "\"force\" can't be cast on a tile",
        ),
        (
            CommandError::TileOffMap(p(-1, 2)),
            "(-1, 2) is outside the map",
        ),
        (
            CommandError::TileOutOfRange {
                pos: p(3, 2),
                distance: 3,
            },
            "(3, 2) is out of range (3 tiles)",
        ),
        (CommandError::TileOccupied(p(2, 3)), "(2, 3) is occupied"),
        (
            CommandError::WrongTerrain(p(1, 2)),
            "the spell can't change the terrain at (1, 2)",
        ),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
}

#[test]
fn terrain_events_and_burning_state_round_trip_through_ron() {
    let mut s = start(setup(burn_cast()));
    let events = act(&mut s, 1, p(0, 2), on_tile("fire", WOOD));
    let text = ron::to_string(&events).unwrap();
    assert_eq!(ron::from_str::<Vec<Event>>(&text).unwrap(), events);
    let text = ron::to_string(&s).unwrap();
    let mut loaded: BattleState = ron::from_str(&text).unwrap();
    loaded.restore_tables(
        Arc::new(terrain()),
        Arc::new(classes()),
        s.tables.items.clone(),
        Arc::new(spells()),
    );
    assert_eq!(loaded, s);
}
