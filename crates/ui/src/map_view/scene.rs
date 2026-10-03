//! What is on the visible battle map, as plain data (ADR-0038): terrain
//! (flashing where it just changed, or marked with what a spell would turn
//! it into), ranges, units, the cursor and the path. No colours, glyphs, cells or
//! pixels: a [`MapSkin`](super::MapSkin) turns a scene into those.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use trpg_content::Content;
use trpg_core::{CharacterId, ClassId, Faction, Pos, StatValue, TerrainId, Unit, UnitId};

/// The visible part of a battle map.
#[derive(Debug, Clone, PartialEq)]
pub struct MapScene {
    /// The map tile at the view's top-left (negative, or past the map, when
    /// the map is smaller than the view).
    pub origin: Pos,
    /// Visible tiles, across × down.
    pub size: (i32, i32),
    /// The visible tiles, row by row: `size.0 × size.1` of them.
    pub tiles: Vec<TileView>,
    /// The units on visible tiles, in the battle's order.
    pub units: Vec<UnitView>,
    /// The cursor, if it is shown and on a visible tile.
    pub cursor: Option<CursorView>,
    /// The selected unit's path, its own tile first; empty = none. Its
    /// tiles may lie outside the view.
    pub path: Vec<Pos>,
}

/// One visible tile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileView {
    /// Its terrain; `None` = off the map.
    pub terrain: Option<TerrainId>,
    /// How strongly it flashes after its terrain changed (0410): `1` at the
    /// change, fading to `0`. One entry per change still flashing; usually
    /// none.
    pub flashes: Vec<f32>,
    /// The ranges it is in, in the order they are laid on.
    pub tints: Vec<RangeKind>,
    /// The terrain the spell being aimed would turn it into (0410): shown
    /// in place of its own, so the tile reads as terrain, not as a target.
    pub becomes: Option<TerrainId>,
}

/// A range shown on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RangeKind {
    /// Tiles the enemy could attack this turn (the danger zone).
    Danger,
    /// Tiles a unit can move to.
    Move,
    /// Tiles a unit can attack, or the targets of an attack or a skill.
    Attack,
    /// The targets of a heal.
    Heal,
}

impl RangeKind {
    /// Its letter in [`MapScene::to_text`].
    const fn mark(self) -> char {
        match self {
            RangeKind::Danger => 'd',
            RangeKind::Move => 'm',
            RangeKind::Attack => 'a',
            RangeKind::Heal => 'h',
        }
    }
}

/// A unit as shown on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    /// Which unit.
    pub id: UnitId,
    /// The tile it is shown on (along its path while it walks).
    pub pos: Pos,
    /// Its side.
    pub faction: Faction,
    /// Its two-letter map label.
    pub label: String,
    /// Its class.
    pub class: ClassId,
    /// The named character, or `None` for a generic unit.
    pub character: Option<CharacterId>,
    /// Whether it has acted this phase.
    pub acted: bool,
    /// Current and max HP.
    pub hp: (StatValue, StatValue),
    /// Whether it is under a timed effect (0412).
    pub has_effect: bool,
    /// How far it has fallen (0404): `0` = standing … `1` = gone.
    pub fade: f32,
    /// Whether it is picked out right now: a battle note on screen is
    /// about it (0411). It blinks, so this goes on and off.
    pub highlight: bool,
}

impl UnitView {
    /// `unit` standing on its own tile.
    pub fn of(unit: &Unit) -> Self {
        Self {
            id: unit.id,
            pos: unit.pos,
            faction: unit.faction,
            label: unit.map_label.clone(),
            class: unit.class.clone(),
            character: unit.character.clone(),
            acted: unit.acted,
            hp: (unit.hp, unit.stats.hp),
            has_effect: !unit.effects.is_empty(),
            fade: 0.0,
            highlight: false,
        }
    }

    /// This unit shown on `pos` instead.
    #[must_use]
    pub fn at(self, pos: Pos) -> Self {
        Self { pos, ..self }
    }

    /// This unit `fade` of the way through falling.
    #[must_use]
    pub fn fading(self, fade: f32) -> Self {
        Self { fade, ..self }
    }

    /// This unit picked out, or not.
    #[must_use]
    pub fn highlighted(self, highlight: bool) -> Self {
        Self { highlight, ..self }
    }
}

/// How the cursor is drawn (`docs/design/look-and-feel.md`): corner marks by
/// default; bigger corners and the tile glow are accessibility options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorStyle {
    /// 3 px corner marks.
    #[default]
    Corners,
    /// 4 px corner marks.
    LargeCorners,
    /// The tile's background tinted towards the cursor colour.
    TileGlow,
}

impl CursorStyle {
    /// Its name in [`MapScene::to_text`].
    const fn name(self) -> &'static str {
        match self {
            CursorStyle::Corners => "corners",
            CursorStyle::LargeCorners => "large-corners",
            CursorStyle::TileGlow => "glow",
        }
    }
}

/// The cursor as shown on the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorView {
    /// The tile it is on.
    pub pos: Pos,
    /// Where it is in its pulse: `1` = full brightness, down to about half.
    pub brightness: f32,
    /// The player's cursor style.
    pub style: CursorStyle,
}

impl MapScene {
    /// A view of `size` tiles from `origin` with nothing on it: every tile
    /// off the map. A negative size counts as zero.
    pub fn new(origin: Pos, size: (i32, i32)) -> Self {
        let size = (size.0.max(0), size.1.max(0));
        let count = usize::try_from(size.0).unwrap_or(0) * usize::try_from(size.1).unwrap_or(0);
        Self {
            origin,
            size,
            tiles: vec![TileView::default(); count],
            units: Vec::new(),
            cursor: None,
            path: Vec::new(),
        }
    }

    /// Where `pos` is in the view, as tiles right of and below
    /// [`origin`](Self::origin), or `None` if it isn't visible.
    pub fn offset(&self, pos: Pos) -> Option<(i32, i32)> {
        let dx = pos.x.checked_sub(self.origin.x)?;
        let dy = pos.y.checked_sub(self.origin.y)?;
        let inside = (0..self.size.0).contains(&dx) && (0..self.size.1).contains(&dy);
        inside.then_some((dx, dy))
    }

    /// Whether `pos` is a visible tile.
    pub fn contains(&self, pos: Pos) -> bool {
        self.offset(pos).is_some()
    }

    /// Index in [`tiles`](Self::tiles) of the tile `dx` right of and `dy`
    /// below the origin.
    fn index(&self, dx: i32, dy: i32) -> Option<usize> {
        if !(0..self.size.0).contains(&dx) || !(0..self.size.1).contains(&dy) {
            return None;
        }
        let (dx, dy) = (usize::try_from(dx).ok()?, usize::try_from(dy).ok()?);
        Some(dy * usize::try_from(self.size.0).ok()? + dx)
    }

    /// The tile `dx` right of and `dy` below the origin, if in the view.
    pub fn tile_at(&self, dx: i32, dy: i32) -> Option<&TileView> {
        self.tiles.get(self.index(dx, dy)?)
    }

    /// The tile at map position `pos`, if visible.
    pub fn tile(&self, pos: Pos) -> Option<&TileView> {
        let (dx, dy) = self.offset(pos)?;
        self.tile_at(dx, dy)
    }

    /// Mutable [`tile`](Self::tile).
    pub fn tile_mut(&mut self, pos: Pos) -> Option<&mut TileView> {
        let (dx, dy) = self.offset(pos)?;
        let index = self.index(dx, dy)?;
        self.tiles.get_mut(index)
    }

    /// Lays the range `kind` on each of `tiles` that is visible.
    pub fn tint(&mut self, tiles: impl IntoIterator<Item = Pos>, kind: RangeKind) {
        for pos in tiles {
            if let Some(tile) = self.tile_mut(pos) {
                tile.tints.push(kind);
            }
        }
    }

    /// The visible tiles in the range `kind`, row by row.
    pub fn tinted(&self, kind: RangeKind) -> Vec<Pos> {
        let (w, h) = self.size;
        let offsets = (0..h).flat_map(|dy| (0..w).map(move |dx| (dx, dy)));
        offsets
            .filter(|&(dx, dy)| {
                self.tile_at(dx, dy)
                    .is_some_and(|t| t.tints.contains(&kind))
            })
            .map(|(dx, dy)| Pos::new(self.origin.x + dx, self.origin.y + dy))
            .collect()
    }

    /// Adds `unit` if it is on a visible tile.
    pub fn push_unit(&mut self, unit: UnitView) {
        if self.contains(unit.pos) {
            self.units.push(unit);
        }
    }

    /// The unit `id`, if it is shown.
    pub fn unit(&self, id: UnitId) -> Option<&UnitView> {
        self.units.iter().find(|u| u.id == id)
    }

    /// The unit shown on `pos`, if any (the last one, drawn on top, if two
    /// share it).
    pub fn unit_at(&self, pos: Pos) -> Option<&UnitView> {
        self.units.iter().rev().find(|u| u.pos == pos)
    }

    /// The ranges `pos` is in, in the order laid on; none if it isn't
    /// visible.
    pub fn tints_at(&self, pos: Pos) -> Vec<RangeKind> {
        self.tile(pos).map(|t| t.tints.clone()).unwrap_or_default()
    }

    /// The terrain shown on `pos`, if it is a visible map tile.
    pub fn terrain_at(&self, pos: Pos) -> Option<TerrainId> {
        self.tile(pos).and_then(|t| t.terrain)
    }

    /// The tile the cursor is on, if it is shown.
    pub fn cursor_tile(&self) -> Option<Pos> {
        self.cursor.map(|c| c.pos)
    }

    /// The scene as text, for tests and bug reports. The same scene always
    /// gives the same text.
    ///
    /// ```text
    /// origin (-10,-11) size 35x30
    ///  -11: -*35
    ///    0: -*10 sea*2 plain+m*3 forest+da burnt! plain>burning -*18
    /// units 1
    ///   #4 Br enemy brigand (7,3) hp 20/30 acted effect fade=0.25
    /// cursor (3,5) corners 1.00
    /// path (3,5) (4,5)
    /// ```
    ///
    /// One line per row of tiles, headed by its map row: terrain string ids
    /// (`-` = off the map), `+` and a letter per range on the tile in the
    /// order laid on (`d` danger, `m` move, `a` attack, `h` heal), and `*n`
    /// for `n` such tiles in a row. `!` marks a tile flashing after its
    /// terrain changed, and `>` is followed by the terrain a spell being
    /// aimed would turn it into. A unit of a named character has it in
    /// brackets after its class, and `highlight` if it is picked out. The
    /// cursor's number is its brightness.
    pub fn to_text(&self, content: &Content) -> String {
        let mut out = String::new();
        let Pos { x, y } = self.origin;
        let (w, h) = self.size;
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "origin ({x},{y}) size {w}x{h}");
        for dy in 0..h {
            let row = self.row_text(content, dy);
            let _ = writeln!(out, "{:>4}: {row}", i64::from(y) + i64::from(dy));
        }
        let _ = writeln!(out, "units {}", self.units.len());
        for u in &self.units {
            let _ = writeln!(out, "  {}", unit_text(u));
        }
        match &self.cursor {
            Some(c) => {
                let Pos { x, y } = c.pos;
                let (style, bright) = (c.style.name(), c.brightness);
                let _ = writeln!(out, "cursor ({x},{y}) {style} {bright:.2}");
            }
            None => out.push_str("cursor -\n"),
        }
        out.push_str("path");
        if self.path.is_empty() {
            out.push_str(" -");
        }
        for p in &self.path {
            let _ = write!(out, " ({},{})", p.x, p.y);
        }
        out.push('\n');
        out
    }

    /// Row `dy` of the view in [`to_text`](Self::to_text): its tiles' names,
    /// equal neighbours counted.
    fn row_text(&self, content: &Content, dy: i32) -> String {
        let mut runs: Vec<(String, usize)> = Vec::new();
        for dx in 0..self.size.0 {
            let name = self
                .tile_at(dx, dy)
                .map_or_else(|| "!".to_owned(), |t| tile_text(content, t));
            match runs.last_mut() {
                Some((last, n)) if *last == name => *n += 1,
                _ => runs.push((name, 1)),
            }
        }
        let words: Vec<String> = runs
            .into_iter()
            .map(|(name, n)| if n > 1 { format!("{name}*{n}") } else { name })
            .collect();
        words.join(" ")
    }
}

/// A tile in [`MapScene::to_text`]: its terrain's string id (`-` off the
/// map, `?n` for an id the content lacks), `!` if it flashes, `+` and its
/// ranges' letters, then `>` and the terrain it would become.
fn tile_text(content: &Content, tile: &TileView) -> String {
    let name = |id: TerrainId| match content.terrain.display.get(id) {
        Some(t) => t.id.clone(),
        None => format!("?{}", id.0),
    };
    let mut text = tile.terrain.map_or_else(|| "-".to_owned(), name);
    if !tile.flashes.is_empty() {
        text.push('!');
    }
    if !tile.tints.is_empty() {
        text.push('+');
        text.extend(tile.tints.iter().map(|k| k.mark()));
    }
    if let Some(id) = tile.becomes {
        text.push('>');
        text.push_str(&name(id));
    }
    text
}

/// A unit's line in [`MapScene::to_text`].
fn unit_text(u: &UnitView) -> String {
    let faction = match u.faction {
        Faction::Player => "player",
        Faction::Enemy => "enemy",
        Faction::Ally => "ally",
        Faction::Neutral => "neutral",
    };
    let mut text = format!("#{} {} {faction} {}", u.id.0, u.label, u.class.0);
    if let Some(c) = &u.character {
        let _ = write!(text, " [{}]", c.0);
    }
    let _ = write!(text, " ({},{}) hp {}/{}", u.pos.x, u.pos.y, u.hp.0, u.hp.1);
    if u.acted {
        text.push_str(" acted");
    }
    if u.has_effect {
        text.push_str(" effect");
    }
    if u.fade > 0.0 {
        let _ = write!(text, " fade={:.2}", u.fade);
    }
    if u.highlight {
        text.push_str(" highlight");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::tests::ctx;

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    fn brigand(id: u32, pos: Pos) -> UnitView {
        UnitView {
            id: UnitId(id),
            pos,
            faction: Faction::Enemy,
            label: "Br".into(),
            class: ClassId("brigand".into()),
            character: None,
            acted: false,
            hp: (20, 30),
            has_effect: false,
            fade: 0.0,
            highlight: false,
        }
    }

    #[test]
    fn a_new_scene_is_empty_tiles_off_the_map() {
        let s = MapScene::new(p(-2, 3), (4, 2));
        assert_eq!((s.origin, s.size), (p(-2, 3), (4, 2)));
        assert_eq!(s.tiles, vec![TileView::default(); 8]);
        assert_eq!(s.tiles[0].terrain, None);
        assert!(s.units.is_empty() && s.path.is_empty() && s.cursor.is_none());
        // A negative size is an empty view.
        let none = MapScene::new(p(0, 0), (-3, 5));
        assert_eq!((none.size, none.tiles.len()), ((0, 5), 0));
        let none = MapScene::new(p(0, 0), (3, -5));
        assert_eq!((none.size, none.tiles.len()), ((3, 0), 0));
        assert!(!none.contains(p(0, 0)));
    }

    #[test]
    fn tiles_are_found_by_map_position_inside_the_view_only() {
        let mut s = MapScene::new(p(-2, 3), (4, 2));
        for (i, t) in (0..).zip(&mut s.tiles) {
            t.terrain = Some(TerrainId(i));
        }
        let id = |pos| s.tile(pos).and_then(|t| t.terrain).map(|t| t.0);
        let last_row = (id(p(-2, 4)), id(p(1, 4)));
        assert_eq!(id(p(-2, 3)), Some(0));
        assert_eq!(id(p(1, 3)), Some(3));
        assert_eq!(last_row, (Some(4), Some(7)));
        for out in [p(-3, 3), p(2, 3), p(-2, 2), p(-2, 5), p(2, 4)] {
            assert_eq!(s.tile(out), None, "{out:?}");
            assert!(!s.contains(out), "{out:?}");
        }
        assert_eq!(s.offset(p(1, 4)), Some((3, 1)));
        assert_eq!(s.offset(p(i32::MIN, 3)), None);
        assert_eq!(s.offset(p(0, i32::MAX)), None);
        assert_eq!(s.tile_at(3, 1).and_then(|t| t.terrain), Some(TerrainId(7)));
        for (dx, dy) in [(4, 0), (-1, 0), (0, 2), (0, -1)] {
            assert_eq!(s.tile_at(dx, dy), None, "({dx}, {dy})");
        }
        // A scene short of tiles (not built by `new`) has none there.
        s.tiles.truncate(5);
        assert_eq!(s.tile(p(-2, 4)).and_then(|t| t.terrain), Some(TerrainId(4)));
        assert_eq!(s.tile(p(-1, 4)), None);
    }

    #[test]
    fn tints_go_on_visible_tiles_in_the_order_laid_on() {
        let mut s = MapScene::new(p(1, 1), (3, 2));
        s.tint([p(1, 1), p(3, 2), p(0, 0), p(4, 1)], RangeKind::Danger);
        s.tint([p(3, 2), p(2, 1)], RangeKind::Move);
        assert_eq!(s.tile(p(1, 1)).unwrap().tints, [RangeKind::Danger]);
        assert_eq!(
            s.tile(p(3, 2)).unwrap().tints,
            [RangeKind::Danger, RangeKind::Move]
        );
        assert_eq!(s.tinted(RangeKind::Danger), [p(1, 1), p(3, 2)]);
        assert_eq!(s.tinted(RangeKind::Move), [p(2, 1), p(3, 2)]);
        assert!(s.tinted(RangeKind::Attack).is_empty());
    }

    #[test]
    fn only_units_on_visible_tiles_are_kept() {
        let mut s = MapScene::new(p(1, 1), (3, 2));
        s.push_unit(brigand(4, p(3, 2)));
        s.push_unit(brigand(5, p(4, 2)));
        s.push_unit(brigand(6, p(1, 1)));
        let ids: Vec<u32> = s.units.iter().map(|u| u.id.0).collect();
        assert_eq!(ids, [4, 6]);
        assert_eq!(s.unit(UnitId(6)).map(|u| u.pos), Some(p(1, 1)));
        assert_eq!(s.unit(UnitId(5)), None);
    }

    #[test]
    fn tiles_answer_what_is_on_them() {
        let mut s = MapScene::new(p(1, 1), (3, 2));
        s.tile_mut(p(2, 1)).unwrap().terrain = Some(TerrainId(4));
        s.tint([p(2, 1)], RangeKind::Danger);
        s.tint([p(2, 1)], RangeKind::Move);
        s.push_unit(brigand(4, p(2, 1)));
        s.push_unit(brigand(6, p(3, 2)));
        s.push_unit(brigand(7, p(3, 2)));
        assert_eq!(s.unit_at(p(2, 1)).map(|u| u.id), Some(UnitId(4)));
        assert_eq!(s.unit_at(p(3, 2)).map(|u| u.id), Some(UnitId(7)));
        assert_eq!(s.unit_at(p(1, 1)), None);
        let both = [RangeKind::Danger, RangeKind::Move];
        assert_eq!(s.tints_at(p(2, 1)), both);
        assert!(s.tints_at(p(1, 1)).is_empty());
        assert!(s.tints_at(p(0, 0)).is_empty());
        assert_eq!(s.terrain_at(p(2, 1)), Some(TerrainId(4)));
        assert_eq!(s.terrain_at(p(1, 1)), None);
        assert_eq!(s.terrain_at(p(9, 9)), None);
        assert_eq!(s.cursor_tile(), None);
        s.cursor = Some(CursorView {
            pos: p(3, 2),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        assert_eq!(s.cursor_tile(), Some(p(3, 2)));
    }

    #[test]
    fn a_unit_view_copies_what_the_map_shows_of_a_unit() {
        let c = ctx();
        let state = crate::screens::battle::quick_battle(&c.content).unwrap();
        let lord = &state.units()[0];
        let v = UnitView::of(lord);
        assert_eq!((v.id, v.pos, v.faction), (lord.id, lord.pos, lord.faction));
        assert_eq!(v.label, "Lo");
        assert_eq!(v.class, lord.class);
        assert_eq!(v.character, lord.character);
        assert!(v.character.is_some());
        assert_eq!(v.hp, (lord.hp, lord.stats.hp));
        assert!(!v.acted && !v.has_effect && !v.highlight);
        assert!(v.fade.abs() < f32::EPSILON);
        let mut hurt = lord.clone();
        hurt.hp = 3;
        hurt.acted = true;
        let v = UnitView::of(&hurt).at(p(9, 9)).fading(0.5);
        assert!(!v.highlight && v.clone().highlighted(true).highlight);
        assert!(!v.clone().highlighted(true).highlighted(false).highlight);
        assert_eq!((v.pos, v.hp.0, v.acted), (p(9, 9), 3, true));
        assert!((v.fade - 0.5).abs() < f32::EPSILON);
        assert_eq!(v.id, lord.id);
    }

    #[test]
    fn to_text_lists_tiles_units_cursor_and_path() {
        let c = ctx();
        let display = &c.content.terrain.display;
        let id = |name| display.id_of(name);
        let mut s = MapScene::new(p(-1, 2), (5, 3));
        for pos in [p(0, 2), p(1, 2), p(2, 2), p(0, 3), p(1, 3)] {
            s.tile_mut(pos).unwrap().terrain = id("plain");
        }
        s.tile_mut(p(3, 2)).unwrap().terrain = id("forest");
        s.tile_mut(p(2, 3)).unwrap().terrain = Some(TerrainId(999));
        s.tint([p(1, 2), p(2, 2)], RangeKind::Move);
        s.tint([p(3, 2)], RangeKind::Danger);
        s.tint([p(3, 2)], RangeKind::Attack);
        s.tint([p(0, 3)], RangeKind::Heal);
        s.tile_mut(p(1, 3)).unwrap().flashes.push(0.5);
        s.tile_mut(p(0, 2)).unwrap().becomes = id("forest");
        s.tile_mut(p(2, 2)).unwrap().becomes = Some(TerrainId(998));
        s.push_unit(brigand(4, p(0, 2)));
        let mut lord = brigand(1, p(1, 3)).fading(0.25);
        lord.faction = Faction::Player;
        lord.label = "Lo".into();
        lord.class = ClassId("lord".into());
        lord.character = Some(CharacterId("test_lord".into()));
        lord.acted = true;
        lord.has_effect = true;
        lord.highlight = true;
        s.push_unit(lord);
        s.cursor = Some(CursorView {
            pos: p(1, 3),
            brightness: 0.756,
            style: CursorStyle::Corners,
        });
        s.path = vec![p(1, 3), p(1, 2)];
        assert_eq!(
            s.to_text(&c.content),
            "origin (-1,2) size 5x3\n\
             \x20  2: - plain>forest plain+m plain+m>?998 forest+da\n\
             \x20  3: - plain+h plain! ?999 -\n\
             \x20  4: -*5\n\
             units 2\n\
             \x20 #4 Br enemy brigand (0,2) hp 20/30\n\
             \x20 #1 Lo player lord [test_lord] (1,3) hp 20/30 acted effect fade=0.25 highlight\n\
             cursor (1,3) corners 0.76\n\
             path (1,3) (1,2)\n"
        );
    }

    #[test]
    fn to_text_of_an_empty_scene_and_every_name() {
        let c = ctx();
        let mut s = MapScene::new(p(0, -1), (2, 1));
        assert_eq!(
            s.to_text(&c.content),
            "origin (0,-1) size 2x1\n  -1: -*2\nunits 0\ncursor -\npath -\n"
        );
        // A scene short of tiles marks them.
        s.tiles.truncate(1);
        assert!(s.to_text(&c.content).contains("  -1: - !\n"));
        let styles = [
            (CursorStyle::LargeCorners, "large-corners"),
            (CursorStyle::TileGlow, "glow"),
        ];
        for (style, name) in styles {
            s.cursor = Some(CursorView {
                pos: p(1, -1),
                brightness: 1.0,
                style,
            });
            let line = format!("cursor (1,-1) {name} 1.00\n");
            assert!(s.to_text(&c.content).contains(&line), "{name}");
        }
        let factions = [(Faction::Ally, "ally"), (Faction::Neutral, "neutral")];
        for (faction, name) in factions {
            let mut u = brigand(7, p(0, -1));
            u.faction = faction;
            // Standing or (wrongly) negative: no fade shown.
            u.fade = -1.0;
            s.units = vec![u];
            let line = format!("  #7 Br {name} brigand (0,-1) hp 20/30\n");
            assert!(s.to_text(&c.content).contains(&line), "{name}");
        }
    }
}
