//! Tests for `movement`. Scenarios are drawn in ASCII:
//!
//! - Terrain: `.` plain, `F` forest, `^` mountain, `A` peak, `~` river,
//!   `=` sea, `#` wall (costs as in `docs/design/terrain.md`).
//! - Units (standing on plain): `@` the mover, `p` player, `a` ally (green),
//!   `e` enemy, `n` neutral.
//! - Reach pictures: a digit is a stoppable tile's cost, `+` a tile the
//!   mover can pass through but not stop on, anything else is unreachable.
//! - Tile-set pictures: `*` is a member.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use super::*;
use crate::class::{ClassDef, UnitTags};
use crate::stats::{Growths, StatValue, Stats};
use crate::terrain::{TerrainId, TerrainRules};

const TERRAIN_CHARS: [char; 7] = ['.', 'F', '^', 'A', '~', '=', '#'];
const CLASSES: [&str; 4] = ["foot", "mounted", "armored", "flying"];
const MOVER: UnitId = UnitId(0);

fn terrain() -> TerrainTable {
    let rules = |name: &str, cost: [Option<u8>; 4]| TerrainRules {
        name: name.to_owned(),
        move_cost: cost.to_vec(),
        defense: 0,
        avoid: 0,
        heal_percent: 0,
    };
    TerrainTable {
        movement_types: CLASSES.iter().map(|&c| c.to_owned()).collect(),
        terrains: vec![
            rules("Plain", [Some(1); 4]),
            rules("Forest", [Some(2), Some(3), Some(2), Some(1)]),
            rules("Mountain", [Some(3), Some(5), None, Some(1)]),
            rules("Peak", [None, None, None, Some(3)]),
            rules("River", [Some(5), None, None, Some(1)]),
            rules("Sea", [None, None, None, Some(1)]),
            rules("Wall", [None; 4]),
        ],
    }
}

/// One class per movement type, named after it.
fn classes() -> ClassTable {
    let class = |i: u8, id: &str| ClassDef {
        id: ClassId(id.into()),
        name: id.into(),
        tier: 1,
        movement_type: MovementTypeId(i),
        move_points: 5,
        base: Stats::from_growable([10; 7], 5),
        caps: Stats::from_growable([50; 7], 5),
        growths: Growths([0; 7]),
        weapons: vec![],
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: None,
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    };
    ClassTable {
        classes: (0u8..)
            .zip(CLASSES)
            .map(|(i, id)| (ClassId(id.into()), class(i, id)))
            .collect(),
        ..ClassTable::default()
    }
}

fn unit(id: u32, class: &str, faction: Faction, pos: Pos, mov: StatValue) -> Unit {
    Unit {
        id: UnitId(id),
        character: None,
        name: format!("u{id}"),
        class: ClassId(class.into()),
        level: 1,
        exp: 0,
        class_records: BTreeMap::new(),
        stats: Stats::from_growable([10; 7], mov),
        hp: 10,
        faction,
        pos,
        acted: false,
        is_lord: false,
        weapon_ranks: BTreeMap::new(),
        map_label: format!("u{id}"),
        weapon: None,
    }
}

fn at(i: usize) -> i32 {
    i32::try_from(i).unwrap()
}

#[derive(Debug, Clone)]
struct Scene {
    rows: Vec<String>,
    map: BattleMap,
    terrain: TerrainTable,
    classes: ClassTable,
    units: Vec<Unit>,
}

impl Scene {
    fn reach(&self, id: UnitId) -> Result<Reach, MoveError> {
        reachable(&self.map, &self.terrain, &self.classes, &self.units, id)
    }

    fn path_cost(&self, path: &[Pos]) -> Result<u32, PathError> {
        path_cost(
            &self.map,
            &self.terrain,
            &self.classes,
            &self.units,
            MOVER,
            path,
        )
    }

    fn threat(&self, id: UnitId, ranges: &[AttackRange]) -> Result<TileSet, MoveError> {
        threat_area(
            &self.map,
            &self.terrain,
            &self.classes,
            &self.units,
            id,
            ranges,
        )
    }

    /// Draws `reach` over the scene (see the module docs).
    fn render(&self, reach: &Reach) -> Vec<String> {
        self.picture(|pos, ch| match reach.cost(pos) {
            Some(c) if reach.is_stoppable(pos) => char::from_digit(c, 36).unwrap_or('?'),
            Some(_) => '+',
            None => ch,
        })
    }

    /// Draws `set` over the scene: `*` for members.
    fn draw(&self, set: &TileSet) -> Vec<String> {
        self.picture(|pos, ch| if set.contains(pos) { '*' } else { ch })
    }

    fn picture(&self, cell: impl Fn(Pos, char) -> char) -> Vec<String> {
        (0..)
            .zip(&self.rows)
            .map(|(y, row)| {
                (0..)
                    .zip(row.chars())
                    .map(|(x, ch)| cell(Pos::new(x, y), ch))
                    .collect()
            })
            .collect()
    }
}

/// Parses an ASCII scene. Every unit is of `class` with Mov `mov`; `@` (the
/// mover, [`MOVER`]) is of faction `mover`; other units get ids from 1 in
/// reading order.
fn scene_as(rows: &[&str], class: &str, mov: StatValue, mover: Faction) -> Scene {
    let height = u16::try_from(rows.len()).unwrap();
    let width = u16::try_from(rows[0].chars().count()).unwrap();
    let mut tiles = Vec::new();
    let mut units = Vec::new();
    let mut next_id = 1;
    for (y, row) in rows.iter().enumerate() {
        assert_eq!(row.chars().count(), usize::from(width), "ragged scene");
        for (x, ch) in row.chars().enumerate() {
            let pos = Pos::new(at(x), at(y));
            let faction = match ch {
                '@' => Some(mover),
                'p' => Some(Faction::Player),
                'a' => Some(Faction::Ally),
                'e' => Some(Faction::Enemy),
                'n' => Some(Faction::Neutral),
                _ => None,
            };
            let terrain = if let Some(f) = faction {
                let id = if ch == '@' { 0 } else { next_id };
                next_id += u32::from(ch != '@');
                units.push(unit(id, class, f, pos, mov));
                0
            } else {
                TERRAIN_CHARS.iter().position(|&c| c == ch).unwrap()
            };
            tiles.push(TerrainId(u16::try_from(terrain).unwrap()));
        }
    }
    Scene {
        rows: rows.iter().map(|&r| r.to_owned()).collect(),
        map: BattleMap {
            name: "test".into(),
            tiles: Grid::from_cells(width, height, tiles).unwrap(),
        },
        terrain: terrain(),
        classes: classes(),
        units,
    }
}

fn scene(rows: &[&str], class: &str, mov: StatValue) -> Scene {
    scene_as(rows, class, mov, Faction::Player)
}

/// Asserts the reach picture of `@` in `rows`.
fn check_as(rows: &[&str], class: &str, mov: StatValue, mover: Faction, expected: &[&str]) {
    let s = scene_as(rows, class, mov, mover);
    let reach = s.reach(MOVER).unwrap();
    assert_eq!(s.render(&reach), expected, "{class}, Mov {mov}, {mover:?}");
}

fn check(rows: &[&str], class: &str, mov: StatValue, expected: &[&str]) {
    check_as(rows, class, mov, Faction::Player, expected);
}

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

// ---------------------------------------------------------------- scenarios

#[test]
fn wall_blocks() {
    check(
        &["@.#..", ".##..", "....."],
        "foot",
        4,
        &["01#..", "1##..", "234.."],
    );
}

#[test]
fn forest_costs_two_on_foot_three_mounted() {
    check(&["@F.", "..."], "foot", 2, &["02.", "12."]);
    check(&["@F.", "..."], "mounted", 3, &["03.", "123"]);
    check(&["@F.", "..."], "armored", 2, &["02.", "12."]);
}

#[test]
fn mountains_by_movement_type() {
    check(&["@^."], "foot", 4, &["034"]);
    check(&["@^."], "mounted", 6, &["056"]);
    check(&["@^."], "armored", 9, &["0^."]);
    check(&["@^."], "flying", 2, &["012"]);
}

#[test]
fn sea_is_impassable_for_foot() {
    check(&["@.=."], "foot", 9, &["01=."]);
    check(&["@.=."], "mounted", 9, &["01=."]);
}

#[test]
fn rivers_are_wadeable_on_foot_only() {
    check(&["@~."], "foot", 6, &["056"]);
    check(&["@~."], "foot", 4, &["0~."]);
    check(&["@~."], "mounted", 9, &["0~."]);
    check(&["@~."], "armored", 9, &["0~."]);
}

#[test]
fn flying_pays_one_everywhere_but_peaks_and_walls() {
    check(&["@F^~=A#."], "flying", 12, &["012347#."]);
    // On foot: forest 2, mountain 3, river 5 (total 10 = "a"), sea blocks.
    check(&["@F^~=A#."], "foot", 12, &["025a=A#."]);
}

#[test]
fn passes_through_friends_but_cant_stop_on_them() {
    let rows = ["#####", "@pan.", "#####"];
    check(&rows, "foot", 4, &["#####", "0+++4", "#####"]);
    // Out of move on a friend's tile: passable, not stoppable.
    check(&["@p"], "foot", 3, &["0+"]);
}

#[test]
fn blocked_by_enemies() {
    check(&["@e.."], "foot", 5, &["0e.."]);
    check(&["@e.", "..."], "foot", 3, &["0e.", "123"]);
    check(&["@e.", "..."], "foot", 4, &["0e4", "123"]);
}

#[test]
fn hostility_depends_on_the_movers_faction() {
    // An enemy passes neutrals, is blocked by players and allies.
    check_as(&["@n.p."], "foot", 4, Faction::Enemy, &["0+2p."]);
    check_as(&["@n.a."], "foot", 4, Faction::Enemy, &["0+2a."]);
    // A green ally passes players, is blocked by enemies.
    check_as(&["@p.e."], "foot", 4, Faction::Ally, &["0+2e."]);
    // A neutral passes everyone.
    check_as(&["@e.p."], "foot", 4, Faction::Neutral, &["0+2+4"]);
}

#[test]
fn zero_or_negative_mov_only_keeps_the_start() {
    check(&["@.."], "foot", 0, &["0.."]);
    check(&["@.."], "foot", -1, &["0.."]);
}

#[test]
fn exact_budget_is_reachable() {
    check(&["@...."], "foot", 3, &["0123."]);
}

#[test]
fn ties_prefer_the_smaller_y_x_key() {
    let s = scene(&["@..", "...", "..."], "foot", 4);
    let reach = s.reach(MOVER).unwrap();
    // (1,0) is expanded before (0,1), so paths go right first.
    assert_eq!(
        reach.path_to(p(1, 1)),
        Some(vec![p(0, 0), p(1, 0), p(1, 1)])
    );
    assert_eq!(
        reach.path_to(p(2, 2)),
        Some(vec![p(0, 0), p(1, 0), p(2, 0), p(2, 1), p(2, 2)])
    );
    assert_eq!(
        reach.path_to(p(0, 2)),
        Some(vec![p(0, 0), p(0, 1), p(0, 2)])
    );
}

#[test]
fn paths_take_the_cheapest_route() {
    let s = scene(&["@F.", "..."], "foot", 3);
    let reach = s.reach(MOVER).unwrap();
    // Through the forest (2 + 1) beats around it (4).
    assert_eq!(
        reach.path_to(p(2, 0)),
        Some(vec![p(0, 0), p(1, 0), p(2, 0)])
    );
    assert_eq!(reach.cost(p(2, 0)), Some(3));
    let s = scene(&["@F.", "..."], "mounted", 4);
    let reach = s.reach(MOVER).unwrap();
    // Mounted: through (3 + 1) and around (4) tie.
    assert_eq!(reach.cost(p(2, 0)), Some(4));
    let s = scene(&["@^.", "..."], "foot", 4);
    let reach = s.reach(MOVER).unwrap();
    // On foot, climbing (3) beats walking round to the mountain (1 + 1 + 3).
    assert_eq!(reach.cost(p(1, 0)), Some(3));
    let s = scene(&["@^.", "..."], "mounted", 9);
    let reach = s.reach(MOVER).unwrap();
    // Mounted: 1 + 1 + 1 + 1 around beats 5 + 1 through.
    assert_eq!(
        reach.path_to(p(2, 0)),
        Some(vec![p(0, 0), p(0, 1), p(1, 1), p(2, 1), p(2, 0)])
    );
    assert_eq!(reach.cost(p(1, 0)), Some(5));
}

#[test]
fn reach_accessors() {
    let s = scene(&["#@p.", "...."], "foot", 2);
    let reach = s.reach(MOVER).unwrap();
    assert_eq!(reach.origin(), p(1, 0));
    assert_eq!(reach.budget(), 2);
    assert_eq!(s.render(&reach), ["#0+2", "212."]);
    assert!(reach.is_passable(p(2, 0)));
    assert!(!reach.is_stoppable(p(2, 0)));
    assert!(reach.is_stoppable(p(1, 0)));
    assert!(!reach.is_passable(p(0, 0)));
    assert!(!reach.is_passable(p(3, 1)));
    assert_eq!(reach.cost(p(9, 9)), None);
    assert_eq!(reach.cost(p(-1, 0)), None);
    assert_eq!(s.draw(&reach.passable()), ["#***", "***."]);
    assert_eq!(s.draw(reach.stoppable()), ["#*p*", "***."]);
    assert_eq!(reach.path_to(p(1, 0)), Some(vec![p(1, 0)]));
    assert_eq!(reach.path_to(p(3, 1)), None);
    assert_eq!(reach.path_to(p(0, 0)), None);
    assert_eq!(
        reach.path_to(p(3, 0)),
        Some(vec![p(1, 0), p(2, 0), p(3, 0)])
    );
}

#[test]
fn reachable_errors() {
    let mut s = scene(&["@.."], "foot", 3);
    assert_eq!(
        s.reach(UnitId(9)).unwrap_err(),
        MoveError::UnknownUnit(UnitId(9))
    );
    s.units[0].class = ClassId("boat".into());
    assert_eq!(
        s.reach(MOVER).unwrap_err(),
        MoveError::UnknownClass(ClassId("boat".into()))
    );
    for off in [p(-1, 0), p(3, 0), p(0, 1), p(0, -1)] {
        let mut s = scene(&["@.."], "foot", 3);
        s.units[0].pos = off;
        assert_eq!(s.reach(MOVER).unwrap_err(), MoveError::OffMap(MOVER));
    }
}

// ---------------------------------------------------------------- path_cost

#[test]
fn path_cost_accepts_legal_paths() {
    let s = scene(&["@F.e", "..#."], "foot", 4);
    assert_eq!(s.path_cost(&[p(0, 0)]), Ok(0));
    assert_eq!(s.path_cost(&[p(0, 0), p(1, 0), p(2, 0)]), Ok(3));
    // Back and forth is legal, and costs every step.
    assert_eq!(s.path_cost(&[p(0, 0), p(0, 1), p(0, 0), p(0, 1)]), Ok(3));
    // Exactly the budget.
    assert_eq!(s.path_cost(&[p(0, 0), p(0, 1), p(1, 1), p(1, 0)]), Ok(4));
    // Through a friend.
    let s = scene(&["@p."], "foot", 2);
    assert_eq!(s.path_cost(&[p(0, 0), p(1, 0), p(2, 0)]), Ok(2));
}

#[test]
fn path_cost_rejects_illegal_paths() {
    let s = scene(&["@F.e", "..#."], "foot", 4);
    assert_eq!(s.path_cost(&[]), Err(PathError::Empty));
    assert_eq!(s.path_cost(&[p(1, 0)]), Err(PathError::WrongStart));
    assert_eq!(
        s.path_cost(&[p(0, 0), p(1, 1)]),
        Err(PathError::NotAdjacent { index: 1 })
    );
    assert_eq!(
        s.path_cost(&[p(0, 0), p(0, 0)]),
        Err(PathError::NotAdjacent { index: 1 })
    );
    assert_eq!(
        s.path_cost(&[p(0, 0), p(0, 1), p(0, 3)]),
        Err(PathError::NotAdjacent { index: 2 })
    );
    assert_eq!(
        s.path_cost(&[p(0, 0), p(0, 1), p(1, 1), p(2, 1)]),
        Err(PathError::Impassable { index: 3 })
    );
    assert_eq!(
        s.path_cost(&[p(0, 0), p(0, -1)]),
        Err(PathError::Impassable { index: 1 })
    );
    assert_eq!(
        s.path_cost(&[p(0, 0), p(1, 0), p(2, 0), p(3, 0)]),
        Err(PathError::Hostile { index: 3 })
    );
    assert_eq!(
        s.path_cost(&[p(0, 0), p(0, 1), p(1, 1), p(1, 0), p(2, 0)]),
        Err(PathError::OverBudget { cost: 5, budget: 4 })
    );
    assert_eq!(
        path_cost(&s.map, &s.terrain, &s.classes, &s.units, UnitId(7), &[]),
        Err(PathError::Mover(MoveError::UnknownUnit(UnitId(7))))
    );
}

#[test]
fn error_messages() {
    let cases: [(Box<dyn std::error::Error>, &str); 10] = [
        (
            Box::new(MoveError::UnknownUnit(UnitId(3))),
            "no unit with id 3",
        ),
        (
            Box::new(MoveError::UnknownClass(ClassId("boat".into()))),
            "unknown class \"boat\"",
        ),
        (
            Box::new(MoveError::OffMap(UnitId(4))),
            "unit 4 is outside the map",
        ),
        (
            Box::new(PathError::Mover(MoveError::OffMap(UnitId(4)))),
            "unit 4 is outside the map",
        ),
        (Box::new(PathError::Empty), "the path is empty"),
        (
            Box::new(PathError::WrongStart),
            "the path doesn't start at the unit",
        ),
        (
            Box::new(PathError::NotAdjacent { index: 2 }),
            "step 2 isn't to a neighbouring tile",
        ),
        (
            Box::new(PathError::Impassable { index: 3 }),
            "step 3 is impassable",
        ),
        (
            Box::new(PathError::Hostile { index: 4 }),
            "step 4 enters an enemy's tile",
        ),
        (
            Box::new(PathError::OverBudget { cost: 7, budget: 5 }),
            "the path costs 7 but the unit has 5 move",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}

// ------------------------------------------------------------- attack tiles

fn attack_picture(rows: &[&str], mov: StatValue, min: u32, max: u32) -> Vec<String> {
    let s = scene(rows, "foot", mov);
    s.draw(&attack_tiles(&s.reach(MOVER).unwrap(), min, max))
}

const OPEN_5: [&str; 5] = [".....", ".....", "..@..", ".....", "....."];

#[test]
fn attack_tiles_range_1() {
    assert_eq!(
        attack_picture(&OPEN_5, 0, 1, 1),
        [".....", "..*..", ".*@*.", "..*..", "....."]
    );
}

#[test]
fn attack_tiles_range_2() {
    assert_eq!(
        attack_picture(&OPEN_5, 0, 2, 2),
        ["..*..", ".*.*.", "*.@.*", ".*.*.", "..*.."]
    );
}

#[test]
fn attack_tiles_range_1_to_2() {
    assert_eq!(
        attack_picture(&OPEN_5, 0, 1, 2),
        ["..*..", ".***.", "**@**", ".***.", "..*.."]
    );
}

#[test]
fn attack_tiles_surround_the_move_area() {
    // Mov 1: stoppable is a plus; range 1 rings it.
    assert_eq!(
        attack_picture(&OPEN_5, 1, 1, 1),
        ["..*..", ".*.*.", "*.@.*", ".*.*.", "..*.."]
    );
    // Range 2 from the plus: the tiles at distance 2 and 3 from the centre.
    assert_eq!(
        attack_picture(&OPEN_5, 1, 2, 2),
        [".***.", "**.**", "*.@.*", "**.**", ".***."]
    );
}

#[test]
fn attack_tiles_include_friends_and_walls_and_clip_at_edges() {
    // The friend's tile is passable but not stoppable, so it is red.
    assert_eq!(attack_picture(&["@p."], 2, 1, 1), ["@*."]);
    // Walls are in range too.
    assert_eq!(attack_picture(&["@#."], 0, 1, 1), ["@*."]);
    // Corner: only the in-map tiles.
    assert_eq!(
        attack_picture(&["@..", "...", "..."], 0, 1, 2),
        ["@**", "**.", "*.."]
    );
}

#[test]
fn attack_tiles_degenerate_ranges() {
    assert_eq!(attack_picture(&OPEN_5, 0, 2, 1), OPEN_5);
    // Range 0 is the unit's own tile, which is stoppable.
    assert_eq!(attack_picture(&OPEN_5, 0, 0, 0), OPEN_5);
    // Huge ranges cover the whole map.
    assert_eq!(
        attack_picture(&["@..", "..."], 0, 1, u32::MAX),
        ["@**", "***"]
    );
    assert_eq!(
        attack_picture(&["@..", "..."], 0, 3, u32::MAX),
        ["@..", "..*"]
    );
}

// ------------------------------------------------------ threat, danger zone

#[test]
fn threat_area_is_stoppable_plus_attack() {
    let s = scene(&OPEN_5, "foot", 1);
    assert_eq!(
        s.draw(&s.threat(MOVER, &[(1, 1)]).unwrap()),
        ["..*..", ".***.", "*****", ".***.", "..*.."]
    );
    // Two weapons: the union of both rings.
    let s = scene(&OPEN_5, "foot", 0);
    assert_eq!(
        s.draw(&s.threat(MOVER, &[(1, 1), (2, 2)]).unwrap()),
        ["..*..", ".***.", "*****", ".***.", "..*.."]
    );
    // A bow only: its own tile and the ring at 2.
    assert_eq!(
        s.draw(&s.threat(MOVER, &[(2, 2)]).unwrap()),
        ["..*..", ".*.*.", "*.*.*", ".*.*.", "..*.."]
    );
}

#[test]
fn threat_area_without_weapons_is_empty() {
    let s = scene(&OPEN_5, "foot", 3);
    assert!(s.threat(MOVER, &[]).unwrap().is_empty());
    assert_eq!(
        s.threat(UnitId(5), &[]),
        Err(MoveError::UnknownUnit(UnitId(5)))
    );
}

#[test]
fn danger_zone_unions_hostile_threats() {
    let s = scene(&["e...p...e"], "foot", 1);
    // Enemies: the left one has a sword (1), the right one a bow (2).
    let ranges = |u: &Unit| vec![if u.pos.x == 0 { (1, 1) } else { (2, 2) }];
    let zone = |f| danger_zone(&s.map, &s.terrain, &s.classes, &s.units, f, ranges).unwrap();
    assert_eq!(s.draw(&zone(Faction::Player)), ["***.p****"]);
    assert_eq!(s.draw(&zone(Faction::Ally)), ["***.p****"]);
    // The player (bow, Mov 1) threatens 1–7 for the enemies.
    assert_eq!(s.draw(&zone(Faction::Enemy)), ["e*******e"]);
    assert!(zone(Faction::Neutral).is_empty());
}

#[test]
fn danger_zone_reports_bad_units() {
    let mut s = scene(&["e.p"], "foot", 1);
    s.units[0].class = ClassId("boat".into());
    let r = danger_zone(
        &s.map,
        &s.terrain,
        &s.classes,
        &s.units,
        Faction::Player,
        |_| vec![(1, 1)],
    );
    assert_eq!(r, Err(MoveError::UnknownClass(ClassId("boat".into()))));
}

// ------------------------------------------------------------------ TileSet

#[test]
fn tile_set_basics() {
    let mut set = TileSet::new(10, 10);
    assert_eq!((set.width(), set.height()), (10, 10));
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(set.insert(p(4, 6)));
    assert!(!set.insert(p(4, 6)));
    assert!(set.insert(p(3, 6)));
    assert!(set.insert(p(9, 9)));
    assert!(set.insert(p(0, 0)));
    for outside in [p(-1, 0), p(10, 0), p(0, 10), p(0, -1), p(10, 10)] {
        assert!(!set.insert(outside));
        assert!(!set.contains(outside));
    }
    assert!(!set.is_empty());
    assert_eq!(set.len(), 4);
    assert!(set.contains(p(4, 6)));
    assert!(set.contains(p(3, 6)));
    assert!(!set.contains(p(5, 6)));
    assert!(!set.contains(p(6, 4)));
    // Row-major, across the 64-bit word boundary (index 63 | 64).
    let members: Vec<Pos> = set.iter().collect();
    assert_eq!(members, [p(0, 0), p(3, 6), p(4, 6), p(9, 9)]);
}

#[test]
fn tile_set_word_boundaries() {
    // 3 × 43 = 129 tiles: three words, the last with one bit.
    let mut set = TileSet::new(3, 43);
    let all: Vec<Pos> = (0..43).flat_map(|y| (0..3).map(move |x| p(x, y))).collect();
    for &pos in &all {
        assert!(set.insert(pos));
    }
    assert_eq!(set.len(), 129);
    assert_eq!(set.iter().collect::<Vec<_>>(), all);
    assert!(!set.contains(p(0, 43)));
    assert!(TileSet::new(0, 0).is_empty());
    assert_eq!(TileSet::new(0, 5).iter().count(), 0);
}

#[test]
fn tile_set_union() {
    let mut a = TileSet::new(3, 2);
    a.insert(p(0, 0));
    let mut b = TileSet::new(4, 4);
    b.insert(p(2, 1));
    b.insert(p(3, 3)); // Outside `a`'s map: dropped.
    b.insert(p(0, 0));
    a.union_with(&b);
    assert_eq!(a.iter().collect::<Vec<_>>(), [p(0, 0), p(2, 1)]);
    assert_eq!(a.len(), 2);
}

// ------------------------------------------------------------------ perf

/// `cargo test -p trpg-core --release -- --ignored reachable_is_fast`
#[test]
#[ignore = "timing; run in release"]
fn reachable_is_fast() {
    let rows: Vec<String> = (0..64)
        .map(|y| {
            (0..64)
                .map(|x| if (x, y) == (32, 32) { '@' } else { '.' })
                .collect()
        })
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let s = scene(&rows, "foot", 10);
    let runs = 1000;
    let start = std::time::Instant::now();
    for _ in 0..runs {
        let reach = s.reach(MOVER).unwrap();
        assert_eq!(reach.stoppable().len(), 221);
    }
    let per_run = start.elapsed() / runs;
    assert!(
        per_run < std::time::Duration::from_millis(1),
        "{per_run:?} per run"
    );
}

// ------------------------------------------------------------- properties

/// A random map (plain-heavy) with 1–8 units on distinct tiles; the first is
/// the mover.
fn arb_scene() -> impl Strategy<Value = Scene> {
    (1u16..=10, 1u16..=10)
        .prop_flat_map(|(w, h)| {
            let n = usize::from(w) * usize::from(h);
            let tile = prop_oneof![4 => Just(0u16), 3 => 0u16..7];
            let faction = prop_oneof![
                Just(Faction::Player),
                Just(Faction::Enemy),
                Just(Faction::Ally),
                Just(Faction::Neutral),
            ];
            (
                Just((w, h)),
                prop::collection::vec(tile, n),
                prop::collection::vec((0..n, faction, 0usize..4), 1..=8),
                -1..=12,
            )
        })
        .prop_map(|((w, h), tiles, specs, mov)| {
            let mut seen = BTreeSet::new();
            let units = specs
                .into_iter()
                .filter(|&(i, _, _)| seen.insert(i))
                .zip(0..)
                .map(|((i, faction, class), id)| {
                    let pos = Pos::new(at(i % usize::from(w)), at(i / usize::from(w)));
                    unit(id, CLASSES[class], faction, pos, mov)
                })
                .collect();
            let tiles = tiles.into_iter().map(TerrainId).collect();
            let rows = (0..h)
                .map(|_| ".".repeat(usize::from(w)))
                .collect::<Vec<_>>();
            Scene {
                rows,
                map: BattleMap {
                    name: "random".into(),
                    tiles: Grid::from_cells(w, h, tiles).unwrap(),
                },
                terrain: terrain(),
                classes: classes(),
                units,
            }
        })
}

impl Scene {
    fn mover(&self) -> &Unit {
        &self.units[0]
    }

    fn movement_type(&self) -> MovementTypeId {
        self.classes.get(&self.mover().class).unwrap().movement_type
    }

    /// The cost for the mover to enter `pos`, or `None` if it can't (off the
    /// map, impassable, or a hostile unit is there).
    fn step_cost(&self, pos: Pos) -> Option<u32> {
        let hostile = self
            .units
            .iter()
            .any(|u| u.pos == pos && u.faction.is_hostile_to(self.mover().faction));
        if hostile {
            return None;
        }
        let t = *self.map.tiles.get(pos)?;
        self.terrain
            .move_cost(t, self.movement_type())
            .map(u32::from)
    }

    /// Cheapest costs by brute-force relaxation (Bellman–Ford), within Mov.
    fn oracle_costs(&self) -> BTreeMap<Pos, u32> {
        let budget = u32::try_from(self.mover().stats.mov).unwrap_or(0);
        let mut best = BTreeMap::from([(self.mover().pos, 0)]);
        loop {
            let mut changed = false;
            for (&pos, &c) in &best.clone() {
                for next in self.map.tiles.neighbors4(pos) {
                    let Some(step) = self.step_cost(next) else {
                        continue;
                    };
                    let nc = c + step;
                    if nc <= budget && best.get(&next).is_none_or(|&old| nc < old) {
                        best.insert(next, nc);
                        changed = true;
                    }
                }
            }
            if !changed {
                return best;
            }
        }
    }

    fn occupied_by_other(&self, pos: Pos) -> bool {
        self.units[1..].iter().any(|u| u.pos == pos)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn costs_are_optimal_and_within_budget(s in arb_scene()) {
        let reach = s.reach(MOVER).unwrap();
        let oracle = s.oracle_costs();
        let budget = u32::try_from(s.mover().stats.mov).unwrap_or(0);
        prop_assert_eq!(reach.budget(), budget);
        for pos in s.map.tiles.positions() {
            prop_assert_eq!(reach.cost(pos), oracle.get(&pos).copied(), "{:?}", pos);
            if let Some(c) = reach.cost(pos) {
                prop_assert!(c <= budget);
            }
        }
    }

    #[test]
    fn paths_are_contiguous_and_sum_to_the_cost(s in arb_scene()) {
        let reach = s.reach(MOVER).unwrap();
        for t in reach.passable().iter() {
            let path = reach.path_to(t).unwrap();
            prop_assert_eq!(path.first(), Some(&s.mover().pos));
            prop_assert_eq!(path.last(), Some(&t));
            let mut sum = 0;
            for pair in path.windows(2) {
                prop_assert_eq!(Pos::manhattan(pair[0], pair[1]), 1);
                sum += s.step_cost(pair[1]).unwrap();
            }
            prop_assert_eq!(Some(sum), reach.cost(t));
        }
    }

    #[test]
    fn paths_never_enter_hostile_or_impassable_tiles(s in arb_scene()) {
        let reach = s.reach(MOVER).unwrap();
        let mt = s.movement_type();
        for t in reach.passable().iter() {
            for &pos in &reach.path_to(t).unwrap()[1..] {
                let hostile = s.units.iter()
                    .any(|u| u.pos == pos && u.faction.is_hostile_to(s.mover().faction));
                prop_assert!(!hostile, "{:?}", pos);
                let terrain = *s.map.tiles.get(pos).unwrap();
                prop_assert!(s.terrain.move_cost(terrain, mt).is_some(), "{:?}", pos);
            }
        }
    }

    #[test]
    fn stoppable_is_passable_and_unoccupied(s in arb_scene()) {
        let reach = s.reach(MOVER).unwrap();
        prop_assert!(reach.is_stoppable(s.mover().pos));
        for pos in s.map.tiles.positions() {
            let expected = reach.is_passable(pos)
                && (pos == s.mover().pos || !s.occupied_by_other(pos));
            prop_assert_eq!(reach.is_stoppable(pos), expected, "{:?}", pos);
        }
        prop_assert!(reach.stoppable().iter().all(|t| reach.passable().contains(t)));
    }

    #[test]
    fn more_move_never_removes_a_tile(s in arb_scene(), extra in 1i32..6) {
        let reach = s.reach(MOVER).unwrap();
        let mut more = s;
        more.units[0].stats.mov += extra;
        let bigger = more.reach(MOVER).unwrap();
        for t in reach.passable().iter() {
            prop_assert!(bigger.is_passable(t), "{:?}", t);
            prop_assert!(bigger.cost(t) <= reach.cost(t));
        }
        for t in reach.stoppable().iter() {
            prop_assert!(bigger.is_stoppable(t), "{:?}", t);
        }
    }

    #[test]
    fn path_cost_of_path_to_is_the_cost(s in arb_scene()) {
        let reach = s.reach(MOVER).unwrap();
        for t in reach.passable().iter() {
            let path = reach.path_to(t).unwrap();
            prop_assert_eq!(s.path_cost(&path), Ok(reach.cost(t).unwrap()));
        }
    }

    #[test]
    fn attack_tiles_match_brute_force(s in arb_scene(), min in 0u32..4, len in 0u32..4) {
        let reach = s.reach(MOVER).unwrap();
        let max = min + len;
        let attack = attack_tiles(&reach, min, max);
        for pos in s.map.tiles.positions() {
            let expected = !reach.is_stoppable(pos)
                && reach.stoppable().iter().any(|from| {
                    (min..=max).contains(&Pos::manhattan(from, pos))
                });
            prop_assert_eq!(attack.contains(pos), expected, "{:?}", pos);
        }
    }

    #[test]
    fn tile_set_matches_a_btree_set(
        w in 0u16..20,
        h in 0u16..20,
        ops in prop::collection::vec((-2i32..22, -2i32..22), 0..60),
    ) {
        let mut set = TileSet::new(w, h);
        let mut model = BTreeSet::new();
        for (x, y) in ops {
            let pos = p(x, y);
            let inside = x >= 0 && y >= 0 && x < i32::from(w) && y < i32::from(h);
            let added = inside && model.insert((y, x));
            prop_assert_eq!(set.insert(pos), added);
        }
        prop_assert_eq!(set.len(), model.len());
        prop_assert_eq!(set.is_empty(), model.is_empty());
        let members: Vec<(i32, i32)> = set.iter().map(|q| (q.y, q.x)).collect();
        prop_assert_eq!(members, model.into_iter().collect::<Vec<_>>());
    }
}
