//! Tests of the battle screen's map scene (ADR-0038, ticket 0432): what each
//! mode puts on the map, as data, and that the view is as big as the map
//! skin says. (The skill and item target modes are in `skill_tests.rs` and
//! `item_tests.rs`, the rewind screen's map in `rewind_tests.rs`.)

use std::rc::Rc;

use proptest::prelude::*;
use trpg_core::{BattleMap, Grid, Pos, TerrainId, UnitId};

use super::ai_phase::{AiAction, PACING};
use super::layout::MAP_VIEW;
use super::mode::Mode;
use super::testing::{battle, quick_units, skirmish, vaulted};
use super::{BattleScreen, quick_battle};
use crate::color::{Rgb, UiColor};
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{Cell, GlyphBuffer, PxRect, Rect};
use crate::input::Action;
use crate::map_view::{CursorStyle, GlyphSkin, MapScene, MapSkin, RangeKind};
use crate::screen::tests::ctx;
use crate::screen::{Ctx, FrameInput, Screen};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn quick() -> BattleScreen {
    BattleScreen::new(quick_battle(&ctx().content).unwrap())
}

/// One frame of no time with `actions`.
fn step(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
    s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
}

/// One frame of `seconds` with no keys.
fn wait(s: &mut BattleScreen, c: &mut Ctx, seconds: f32) {
    s.update(c, &FrameInput::new(vec![], seconds, vec![]));
}

/// `tiles` row by row, as [`MapScene::tinted`] lists them.
fn rows(tiles: impl IntoIterator<Item = Pos>) -> Vec<Pos> {
    let mut tiles: Vec<Pos> = tiles.into_iter().collect();
    tiles.sort_by_key(|p| (p.y, p.x));
    tiles
}

/// Every range on the scene, as `(kind, how many tiles)`.
fn ranges(scene: &MapScene) -> Vec<(RangeKind, usize)> {
    use RangeKind::{Attack, Danger, Heal, Move};
    [Danger, Move, Attack, Heal]
        .into_iter()
        .map(|k| (k, scene.tinted(k).len()))
        .filter(|&(_, n)| n > 0)
        .collect()
}

#[test]
fn browsing_shows_the_terrain_the_units_and_the_cursor() {
    let mut c = ctx();
    let s = quick();
    let scene = s.scene(&c);
    // The game's clock, in milliseconds, for a skin's moving marks.
    assert_eq!(scene.clock_ms, 0);
    c.clock_s = 1.5;
    assert_eq!(s.scene(&c).clock_ms, 1500);
    c.clock_s = -3.0;
    assert_eq!(s.scene(&c).clock_ms, 0);
    c.clock_s = 0.0;
    // `test_small` (14 × 8) centred in the glyph skin's 35 × 30 tiles.
    assert_eq!((scene.origin, scene.size), (p(-10, -11), (35, 30)));
    assert_eq!(scene.tiles.len(), 35 * 30);
    let map = &s.state().map().tiles;
    for pos in (-11..19).flat_map(|y| (-10..25).map(move |x| p(x, y))) {
        let tile = scene.tile(pos).unwrap();
        assert_eq!(tile.terrain, map.get(pos).copied(), "{pos:?}");
        assert!(tile.tints.is_empty(), "{pos:?}");
    }
    let sea = c.content.terrain.display.id_of("sea");
    assert!(sea.is_some());
    assert_eq!(scene.tile(p(0, 0)).unwrap().terrain, sea);
    assert_eq!(scene.tile(p(-1, 0)).unwrap().terrain, None);
    assert_eq!(scene.tile(p(14, 7)).unwrap().terrain, None);
    assert!(scene.tile(p(13, 7)).unwrap().terrain.is_some());
    // Every unit, in the battle's order, where it stands.
    let shown: Vec<_> = scene
        .units
        .iter()
        .map(|u| (u.id, u.pos, u.label.clone(), u.faction, u.hp))
        .collect();
    let units: Vec<_> = s
        .state()
        .units()
        .iter()
        .map(|u| {
            (
                u.id,
                u.pos,
                u.map_label.clone(),
                u.faction,
                (u.hp, u.stats.hp),
            )
        })
        .collect();
    assert_eq!(shown, units);
    assert_eq!(shown.len(), 8);
    for u in &scene.units {
        assert!(!u.acted && !u.has_effect() && u.fade.abs() < f32::EPSILON);
    }
    // The cursor on the lord, at the start of its pulse.
    let cursor = scene.cursor.unwrap();
    assert_eq!((cursor.pos, cursor.style), (p(3, 5), CursorStyle::Corners));
    assert!((cursor.brightness - 1.0).abs() < 1e-6);
    assert!(scene.path.is_empty());
}

#[test]
fn the_cursor_has_the_players_style_and_its_pulse() {
    let mut c = ctx();
    c.cursor_style = CursorStyle::TileGlow;
    let mut s = quick();
    wait(&mut s, &mut c, 0.5);
    let cursor = s.scene(&c).cursor.unwrap();
    assert_eq!(cursor.style, CursorStyle::TileGlow);
    assert!((cursor.brightness - s.cursor().brightness()).abs() < 1e-6);
    assert!((cursor.brightness - 0.5).abs() < 1e-6);
}

#[test]
fn a_selected_unit_shows_its_ranges_and_its_path() {
    let mut c = ctx();
    let mut s = quick();
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::Selected(sel) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let scene = s.scene(&c);
    assert_eq!(scene.tinted(RangeKind::Move), rows(sel.moves.iter()));
    assert_eq!(scene.tinted(RangeKind::Attack), rows(sel.attack.iter()));
    assert!(!sel.moves.is_empty() && !sel.attack.is_empty());
    assert_eq!(ranges(&scene).len(), 2);
    // On the unit: the cursor, and a path of its own tile only.
    assert_eq!(scene.cursor.map(|c| c.pos), Some(p(3, 5)));
    assert_eq!(scene.path, [p(3, 5)]);
    // Away from it: the path, and no cursor where its arrowhead points.
    step(&mut s, &mut c, &[Action::CursorRight, Action::CursorRight]);
    let scene = s.scene(&c);
    assert_eq!(scene.path, [p(3, 5), p(4, 5), p(5, 5)]);
    assert_eq!(scene.cursor, None);
    assert_eq!(scene.unit(UnitId(1)).map(|u| u.pos), Some(p(3, 5)));
    // Past its reach: the path stops short and the cursor shows.
    step(&mut s, &mut c, &[Action::CursorRight; 8]);
    let scene = s.scene(&c);
    assert_eq!(scene.path.last(), Some(&p(7, 5)));
    assert_eq!(scene.cursor.map(|c| c.pos), Some(p(13, 5)));
}

#[test]
fn the_danger_zone_lies_under_the_modes_ranges() {
    let mut c = ctx();
    let mut s = quick();
    step(&mut s, &mut c, &[Action::DangerZone]);
    let zone = s.danger().unwrap().clone();
    let scene = s.scene(&c);
    assert_eq!(scene.tinted(RangeKind::Danger), rows(zone.iter()));
    assert_eq!(ranges(&scene), [(RangeKind::Danger, zone.iter().count())]);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::Selected(sel) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let both = sel.moves.iter().find(|&p| zone.contains(p)).unwrap();
    let scene = s.scene(&c);
    assert_eq!(
        scene.tile(both).unwrap().tints,
        [RangeKind::Danger, RangeKind::Move]
    );
    assert_eq!(scene.tinted(RangeKind::Danger), rows(zone.iter()));
}

#[test]
fn an_enemys_threat_area_is_an_attack_range() {
    let mut c = ctx();
    let mut s = quick();
    s.cursor.jump(p(8, 2));
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::Idle {
        threat: Some(threat),
    } = s.mode()
    else {
        panic!("{:?}", s.mode());
    };
    let scene = s.scene(&c);
    let area = rows(threat.area.iter());
    assert!(area.contains(&p(8, 2)));
    assert_eq!(scene.tinted(RangeKind::Attack), area);
    assert_eq!(ranges(&scene), [(RangeKind::Attack, area.len())]);
    assert_eq!(scene.cursor.map(|c| c.pos), Some(p(8, 2)));
    // Hidden again: no range.
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(ranges(&s.scene(&c)).is_empty());
}

#[test]
fn where_a_unit_may_move_after_its_attack_is_a_move_range() {
    let c = ctx();
    let state = vaulted(&c);
    let tiles = state.move_after_tiles();
    let s = BattleScreen::new(state);
    assert!(matches!(s.mode(), Mode::MoveAfter { .. }), "{:?}", s.mode());
    let scene = s.scene(&c);
    assert!(!tiles.is_empty());
    assert_eq!(scene.tinted(RangeKind::Move), rows(tiles.iter().copied()));
    assert_eq!(ranges(&scene), [(RangeKind::Move, tiles.len())]);
    assert_eq!(scene.cursor.map(|c| c.pos), Some(s.cursor().pos));
}

/// The skirmish with the lord (unit 1) walked one step right to (7, 2) and
/// its action menu open.
fn lord_menu(c: &mut Ctx) -> BattleScreen {
    let mut s = BattleScreen::new(skirmish(c, 20));
    step(
        &mut s,
        c,
        &[Action::Confirm, Action::CursorRight, Action::Confirm],
    );
    assert!(matches!(s.mode(), Mode::Moving { .. }), "{:?}", s.mode());
    wait(&mut s, c, 1.0);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    s
}

#[test]
fn the_targets_of_an_attack_are_an_attack_range() {
    let mut c = ctx();
    let mut s = lord_menu(&mut c);
    // Attack, then the weapon.
    step(&mut s, &mut c, &[Action::Confirm]);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::Targeting(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let at = |id| s.state().unit(id).unwrap().pos;
    let targets: Vec<Pos> = t.targets.iter().map(|&id| at(id)).collect();
    // The raider above and the brigand to the right.
    assert_eq!(rows(targets.clone()), [p(7, 1), p(8, 2)]);
    let scene = s.scene(&c);
    assert_eq!(scene.tinted(RangeKind::Attack), rows(targets));
    assert_eq!(ranges(&scene), [(RangeKind::Attack, 2)]);
    // The attacker where it will stand, the cursor on the target.
    assert_eq!(scene.unit(UnitId(1)).map(|u| u.pos), Some(p(7, 2)));
    assert_eq!(s.state().unit(UnitId(1)).unwrap().pos, p(6, 2));
    assert_eq!(scene.cursor.map(|c| c.pos), Some(at(t.target())));
    assert!(scene.path.is_empty());
}

#[test]
fn walks_menus_and_boxes_hide_the_cursor_and_show_no_range() {
    let mut c = ctx();
    // A walk: the unit along its path.
    let mut s = quick();
    step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
    step(&mut s, &mut c, &[Action::CursorRight, Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Moving { .. }), "{:?}", s.mode());
    wait(&mut s, &mut c, 0.01);
    let scene = s.scene(&c);
    assert_eq!(scene.cursor, None);
    assert!(ranges(&scene).is_empty() && scene.path.is_empty());
    let walker = scene.unit(UnitId(1)).map(|u| u.pos);
    assert_eq!(walker, s.mode().drawn_pos(UnitId(1)));
    // Its action menu: the unit at its path's end.
    wait(&mut s, &mut c, 1.0);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    let scene = s.scene(&c);
    assert_eq!(scene.cursor, None);
    assert!(ranges(&scene).is_empty() && scene.path.is_empty());
    assert_eq!(scene.unit(UnitId(1)).map(|u| u.pos), Some(p(5, 5)));
    // The info screen and the end-turn question.
    for action in [Action::Info, Action::EndTurn] {
        let mut s = quick();
        step(&mut s, &mut c, &[action]);
        let hidden = matches!(s.mode(), Mode::Info { .. } | Mode::EndTurnPrompt { .. });
        assert!(hidden, "{:?}", s.mode());
        assert_eq!(s.scene(&c).cursor, None, "{action:?}");
    }
    // The map menu opens beside the cursor, which stays.
    let mut s = quick();
    step(&mut s, &mut c, &[Action::Cancel]);
    assert!(matches!(s.mode(), Mode::MapMenu { .. }), "{:?}", s.mode());
    assert_eq!(s.scene(&c).cursor.map(|c| c.pos), Some(p(3, 5)));
}

#[test]
fn a_combat_shows_its_units_as_the_playback_has_them() {
    let mut c = ctx();
    // The brigand on 1 HP: the lord's hit fells it.
    let mut s = BattleScreen::new(skirmish(&c, 1));
    step(
        &mut s,
        &mut c,
        &[Action::Confirm, Action::CursorRight, Action::Confirm],
    );
    wait(&mut s, &mut c, 1.0);
    step(&mut s, &mut c, &[Action::Confirm]);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::Targeting(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    // Onto the brigand (unit 4), then attack.
    if t.target() != UnitId(4) {
        step(&mut s, &mut c, &[Action::CursorRight]);
    }
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Combat(_)), "{:?}", s.mode());
    let mut fades = Vec::new();
    for _ in 0..600 {
        if !matches!(s.mode(), Mode::Combat(_)) {
            break;
        }
        let scene = s.scene(&c);
        assert_eq!(scene.cursor, None);
        assert!(ranges(&scene).is_empty() && scene.path.is_empty());
        let shown: Vec<_> = s
            .shown_units()
            .iter()
            .map(|(u, fade)| (u.id, u.pos, u.hp, u.acted, *fade))
            .collect();
        let drawn: Vec<_> = scene
            .units
            .iter()
            .map(|u| (u.id, u.pos, u.hp.0, u.acted, u.fade))
            .collect();
        assert_eq!(drawn, shown);
        fades.extend(scene.unit(UnitId(4)).map(|u| u.fade));
        wait(&mut s, &mut c, 0.05);
    }
    assert!(
        !matches!(s.mode(), Mode::Combat(_)),
        "the combat never ended"
    );
    // The brigand stood, then faded out.
    assert!(fades.first().is_some_and(|f| f.abs() < f32::EPSILON));
    assert!(fades.iter().any(|&f| f > 0.0 && f < 1.0), "{fades:?}");
    assert_eq!(s.scene(&c).unit(UnitId(4)), None);
}

/// A skin that shows 10 × 8 tiles whatever the area (as bigger tiles would
/// show fewer), drawn as the glyph skin draws them.
#[derive(Debug)]
struct Narrow;

impl MapSkin for Narrow {
    fn name(&self) -> &'static str {
        "narrow"
    }

    fn view_tiles(&self, _: Rect) -> (i32, i32) {
        (10, 8)
    }

    fn paint(&self, ctx: &Ctx, scene: &MapScene, area: Rect, buf: &mut GlyphBuffer) {
        GlyphSkin.paint(ctx, scene, area, buf);
    }

    fn tile_px(&self, scene: &MapScene, area: Rect, tile: Pos) -> Option<PxRect> {
        GlyphSkin.tile_px(scene, area, tile)
    }
}

/// The Quick Battle's units on a 64 × 40 plain, the lord at (30, 20), the
/// knight at (34, 20) and the archer at (35, 20).
fn big_battle(c: &Ctx) -> BattleScreen {
    let (_, mut units) = quick_units(c);
    units[0].pos = p(30, 20);
    units[1].pos = p(34, 20);
    units[2].pos = p(35, 20);
    let map = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
    BattleScreen::new(battle(c, map, units))
}

#[test]
fn the_view_is_as_big_as_the_skin_says() {
    let mut c = ctx();
    c.map_skin = Rc::new(Narrow);
    let mut s = big_battle(&c);
    // The screen starts with the default skin's 35 × 30 tiles in mind…
    assert_eq!(s.camera().origin, p(13, 5));
    // …but its first frame is already this skin's: centred on the cursor.
    let scene = s.scene(&c);
    assert_eq!((scene.origin, scene.size), (p(25, 16), (10, 8)));
    assert_eq!(scene.tiles.len(), 80);
    let ids: Vec<u32> = scene.units.iter().map(|u| u.id.0).collect();
    assert_eq!(ids, [1, 2], "the archer is one tile off the view");
    assert_eq!(scene.cursor.map(|c| c.pos), Some(p(30, 20)));
    // Only those tiles are drawn: the top-left 20 × 8 cells of the map view.
    let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(&c, &mut buf);
    let (text, bg) = (c.palette.get(UiColor::Text), c.palette.get(UiColor::Black));
    let blank = Cell::new(' ', text, bg);
    assert_ne!(buf.get(19, 7), Some(&blank));
    assert_eq!(buf.get(20, 7), Some(&blank));
    assert_eq!(buf.get(19, 8), Some(&blank));
    assert_eq!(buf.get(MAP_VIEW.w - 1, MAP_VIEW.h - 1), Some(&blank));
    // The lord at (30, 20): tile (5, 4) of the view, cells 10 and 11.
    assert_eq!(buf.get(10, 4).map(|c| c.glyph), Some('L'));
    // An update keeps the camera for that view…
    step(&mut s, &mut c, &[]);
    assert_eq!(s.camera().origin, p(25, 16));
    assert_eq!(s.scene(&c).origin, p(25, 16));
    // …and it follows the cursor 3 tiles from the edges of 10 × 8.
    step(&mut s, &mut c, &[Action::CursorRight; 4]);
    assert_eq!(s.cursor().pos, p(34, 20));
    assert_eq!(s.camera().origin, p(28, 16));
    let scene = s.scene(&c);
    assert_eq!(scene.origin, p(28, 16));
    assert_eq!(scene.units.len(), 3);
    // Back to the default skin: fitted again, around the cursor.
    c.map_skin = Rc::new(GlyphSkin);
    let scene = s.scene(&c);
    assert_eq!((scene.origin, scene.size), (p(17, 5), (35, 30)));
    assert_eq!(s.camera().origin, p(28, 16));
    step(&mut s, &mut c, &[]);
    assert_eq!(s.camera().origin, p(17, 5));
}

#[test]
fn a_skin_switch_refits_the_kept_camera_and_an_ai_pan() {
    let mut c = ctx();
    let mut s = big_battle(&c);
    step(&mut s, &mut c, &[]);
    assert_eq!(s.camera().origin, p(13, 5));
    // As if the player phase had ended with the cursor on the lord, and
    // an enemy's action were panning over to the archer's tile.
    s.player_view = Some((p(30, 20), s.camera));
    let before = s.state.units().to_vec();
    let archer = before[2].id;
    s.cursor.jump(p(60, 35));
    let pan = (s.camera.origin, p(29, 10));
    let action = AiAction::new(archer, before, pan, vec![], Mode::default(), PACING);
    let walk_start = action.walk_start();
    s.mode = Mode::AiAction(Box::new(action));
    // The debug menu switches to a skin showing 10 × 8 tiles.
    c.map_skin = Rc::new(Narrow);
    s.begin_frame(&c, 0.0);
    // The camera centres on the cursor; the kept one on the kept cursor.
    assert_eq!(s.camera().origin, p(54, 31));
    assert_eq!(s.player_view.map(|(_, c)| c.origin), Some(p(25, 16)));
    // The pan goes from there to show the archer at (35, 20) 3 tiles from
    // the edges, in the same time.
    let Mode::AiAction(a) = &s.mode else {
        panic!("{:?}", s.mode);
    };
    let mut a = a.as_ref().clone();
    assert_eq!(a.camera(), p(54, 31));
    assert!((a.walk_start() - walk_start).abs() < 1e-6);
    a.tick(PACING.pan, false);
    assert_eq!(a.camera(), p(32, 17));
    // The same size again changes nothing.
    let kept = (s.camera, s.player_view);
    s.begin_frame(&c, 0.0);
    assert_eq!((s.camera, s.player_view), kept);
}

#[test]
fn menus_open_beside_the_tile_where_the_skin_has_it() {
    let mut c = ctx();
    c.map_skin = Rc::new(Narrow);
    let mut s = big_battle(&c);
    step(&mut s, &mut c, &[Action::Cancel]);
    assert!(matches!(s.mode(), Mode::MapMenu { .. }), "{:?}", s.mode());
    let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(&c, &mut buf);
    // The cursor's tile is at cells (10, 4) and (11, 4) in this view (it
    // would be at (34, 15) in the default one): the menu's box starts one
    // cell right of it, one row up.
    assert_eq!(buf.get(13, 3).map(|c| c.glyph), Some('┌'));
    assert_eq!(buf.get(37, 14).map(|c| c.glyph), Some(' '));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Wherever the units stand, whatever the player presses and however
    /// many tiles the skin shows, the scene has a tile for every tile of
    /// its view, and every unit in it (and the cursor) is on one of them;
    /// while no combat or AI action plays, it has every unit drawn on a
    /// visible tile.
    #[test]
    fn scene_units_are_visible(
        spots in prop::collection::vec((0..64i32, 0..6i32), 6),
        keys in prop::collection::vec(0usize..7, 0..40),
        narrow in any::<bool>(),
    ) {
        const KEYS: [Action; 7] = [
            Action::CursorLeft,
            Action::CursorRight,
            Action::CursorUp,
            Action::CursorDown,
            Action::Confirm,
            Action::Cancel,
            Action::NextUnit,
        ];
        let mut c = ctx();
        if narrow {
            c.map_skin = Rc::new(Narrow);
        }
        let (_, mut units) = quick_units(&c);
        // Each unit in its own band of six rows: no two on one tile.
        for ((unit, (x, dy)), band) in units.iter_mut().zip(spots).zip(0..) {
            unit.pos = p(x, band * 6 + dy);
        }
        let map = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        let mut s = BattleScreen::new(battle(&c, map, units));
        for key in std::iter::once(None).chain(keys.into_iter().map(Some)) {
            if let Some(key) = key {
                s.update(&mut c, &FrameInput::new(vec![KEYS[key]], 0.1, vec![]));
            }
            let scene = s.scene(&c);
            let (w, h) = scene.size;
            prop_assert_eq!((w, h), if narrow { (10, 8) } else { (35, 30) });
            prop_assert_eq!(scene.tiles.len(), usize::try_from(w * h).unwrap());
            for u in &scene.units {
                prop_assert!(scene.contains(u.pos), "{:?} in {:?}", u, s.mode());
            }
            if let Some(cursor) = scene.cursor {
                prop_assert!(scene.contains(cursor.pos), "{:?}", s.mode());
            }
            if !s.playing() && s.rewind().is_none() {
                let drawn = s.state().units().iter().map(|u| s.drawn_pos(u));
                let visible = drawn.filter(|&pos| scene.contains(pos)).count();
                prop_assert_eq!(scene.units.len(), visible, "{:?}", s.mode());
            }
        }
    }
}
