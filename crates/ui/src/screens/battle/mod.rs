//! The battle screen (ADR-0018, `docs/design/look-and-feel.md`): the map
//! viewport on the left, the side panel on the right and the help bar at the
//! bottom. The player browses with the cursor (ticket 0402); selecting and
//! overlays come with 0403.

pub mod camera;
pub mod cursor;
pub mod layout;
pub mod panel;
pub mod units;

use std::sync::Arc;

use trpg_content::{Content, character_unit, check_map_labels};
use trpg_core::{
    BattlePack, BattleSetup, BattleState, Command, Faction, ItemId, Objective, Phase, Pos, Stock,
    Unit, UnitAction, UnitId,
};

use self::camera::{Camera, tile_to_cell};
use self::cursor::{Cursor, draw_cursor};
use self::layout::{HELP_BAR, HELP_ROW, SIDE_PANEL, VIEW_TILES_H, VIEW_TILES_W};
use crate::color::{Palette, Rgb, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};

/// Map id of the debug Quick Battle.
pub const QUICK_BATTLE_MAP: &str = "test_small";

/// Seed of the debug Quick Battle's RNG.
pub const QUICK_BATTLE_SEED: u64 = 1;

/// The consumable the debug Quick Battle's pack is filled with.
pub const QUICK_BATTLE_POTION: &str = "potion";

/// How many of them (Chapter 1's default pack, `chapter-1.md`).
pub const QUICK_BATTLE_POTIONS: usize = 3;

/// The debug Quick Battle: `test_small.map` with the placeholder characters
/// against generic enemies (rout), one of them wounded and one having acted
/// so both looks show. Fails with a message if the content lacks something
/// it needs.
pub fn quick_battle(content: &Content) -> Result<BattleState, String> {
    let map = content
        .maps
        .get(QUICK_BATTLE_MAP)
        .ok_or_else(|| format!("no map \"{QUICK_BATTLE_MAP}\""))?
        .map
        .clone();
    let classes = &content.classes;
    let items = &content.items;
    let chars = &content.characters;
    let mut units = Vec::new();
    let mut next_id = 0;
    let mut id = || {
        next_id += 1;
        UnitId(next_id)
    };
    let named = [
        ("test_lord", Pos::new(3, 5)),
        ("test_knight", Pos::new(4, 6)),
        ("test_archer", Pos::new(2, 4)),
    ];
    for (name, pos) in named {
        let def = chars
            .characters
            .get(&trpg_core::CharacterId(name.into()))
            .ok_or_else(|| format!("no character \"{name}\""))?;
        let unit = character_unit(def, id(), classes, items, Faction::Player, pos)
            .map_err(|e| e.to_string())?;
        units.push(unit);
    }
    let generics = [
        ("test_brigand", Pos::new(8, 2)),
        ("test_brigand", Pos::new(12, 3)),
        ("test_raider", Pos::new(7, 1)),
    ];
    for (name, pos) in generics {
        let template = chars
            .generics
            .get(name)
            .ok_or_else(|| format!("no generic \"{name}\""))?;
        let unit = template
            .unit(id(), classes, items, Faction::Enemy, pos)
            .map_err(|e| e.to_string())?;
        units.push(unit);
    }
    // The knight is wounded (mid HP) and the second brigand badly (low HP).
    units[1].hp = units[1].stats.hp * 9 / 20;
    units[4].hp = units[4].stats.hp / 4;
    let errors = check_map_labels(QUICK_BATTLE_MAP, &units);
    if let Some(e) = errors.first() {
        return Err(e.to_string());
    }
    let archer = Command::Act {
        unit: units[2].id,
        dest: units[2].pos,
        action: UnitAction::Wait,
    };
    let (mut state, _) = BattleState::new(BattleSetup {
        map,
        terrain: Arc::new(content.terrain.rules.clone()),
        classes: Arc::new(classes.clone()),
        items: Arc::new(items.clone()),
        spells: Arc::new(content.spells.clone()),
        skills: Arc::new(content.skills.clone()),
        arts: Arc::new(content.arts.clone()),
        pack: BattlePack {
            items: vec![ItemId::new(QUICK_BATTLE_POTION); QUICK_BATTLE_POTIONS],
            cap: items.rules.default_pack_cap,
        },
        gold: 0,
        stock: Stock::default(),
        units,
        reinforcements: vec![],
        objective: Objective::Rout { turn_limit: None },
        rewind_charges: 3,
        seed: QUICK_BATTLE_SEED,
    });
    // The archer has acted: it waits where it stands.
    state.apply(&archer).map_err(|e| e.to_string())?;
    Ok(state)
}

/// The battle screen: the player browses the map with the cursor; Cancel
/// leaves it (until the map menu, 0405).
#[derive(Debug, Clone)]
pub struct BattleScreen {
    state: BattleState,
    camera: Camera,
    cursor: Cursor,
}

impl BattleScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "battle";

    /// A screen showing `state`, with the cursor on the first player lord
    /// (else the first player unit, else the map's centre) and the camera
    /// centred on it.
    pub fn new(state: BattleState) -> Self {
        let tiles = &state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        let player = |u: &&Unit| u.faction == Faction::Player;
        let units = state.units();
        let start = units
            .iter()
            .filter(player)
            .find(|u| u.is_lord)
            .or_else(|| units.iter().find(player))
            .map_or_else(|| Pos::new(i32::from(w) / 2, i32::from(h) / 2), |u| u.pos);
        Self {
            camera: Camera::centred_on(start, w, h),
            cursor: Cursor::new(start),
            state,
        }
    }

    /// The battle shown.
    pub fn state(&self) -> &BattleState {
        &self.state
    }

    /// The camera.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// The cursor.
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Scrolls the camera to keep `target` [`Camera::MARGIN`] tiles from the
    /// viewport's edges.
    pub fn follow(&mut self, target: Pos) {
        let tiles = &self.state.map().tiles;
        self.camera
            .follow(target, tiles.width(), tiles.height(), Camera::MARGIN);
    }

    /// The unit under the cursor, if any.
    pub fn hovered(&self) -> Option<&Unit> {
        let pos = self.cursor.pos;
        self.state.units().iter().find(|u| u.pos == pos)
    }

    /// Whether `unit` can still act this phase: its faction's phase and it
    /// hasn't acted.
    fn is_ready(&self, unit: &Unit) -> bool {
        Phase::of(unit.faction) == self.state.phase() && !unit.acted
    }

    /// The acting faction's ready units, ordered by `(y, x)`.
    pub fn ready_units(&self) -> Vec<Pos> {
        let mut ready: Vec<Pos> = self
            .state
            .units()
            .iter()
            .filter(|u| self.is_ready(u))
            .map(|u| u.pos)
            .collect();
        ready.sort_by_key(|p| (p.y, p.x));
        ready
    }

    /// Moves the cursor (and camera) to the next ready unit after the
    /// cursor in `(y, x)` order, or the previous one before it; wraps
    /// around. Nothing happens with no ready units.
    fn cycle(&mut self, forward: bool) {
        let ready = self.ready_units();
        let here = (self.cursor.pos.y, self.cursor.pos.x);
        let key = |p: &&Pos| (p.y, p.x);
        let to = if forward {
            ready.iter().find(|p| key(p) > here).or(ready.first())
        } else {
            ready.iter().rev().find(|p| key(p) < here).or(ready.last())
        };
        if let Some(&to) = to {
            self.cursor.jump(to);
            self.follow(to);
        }
    }

    /// The help line for what is under the cursor, e.g. `f select · e info
    /// · s next unit · d back` over a ready unit of the acting side. Key
    /// names come from the keymap.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        let moves = (cursor_keys_name(km), "move");
        let select = (key_name(km, Action::Confirm), "select");
        let info = (key_name(km, Action::Info), "info");
        let next = (key_name(km, Action::NextUnit), "next unit");
        let back = (key_name(km, Action::Cancel), "back");
        match self.hovered() {
            Some(u) if self.is_ready(u) => help_line(&[select, info, next, back]),
            Some(_) => help_line(&[moves, info, next, back]),
            None => help_line(&[moves, next, back]),
        }
    }

    /// Draws the terrain of every viewport tile; tiles off the map are left
    /// as they are (blank).
    fn draw_terrain(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let display = &ctx.content.terrain.display;
        let o = self.camera.origin;
        for dy in 0..VIEW_TILES_H {
            for dx in 0..VIEW_TILES_W {
                let pos = Pos::new(o.x + dx, o.y + dy);
                let Some(t) = self
                    .state
                    .map()
                    .tiles
                    .get(pos)
                    .and_then(|&id| display.get(id))
                else {
                    continue;
                };
                let Some((x, y)) = tile_to_cell(pos, &self.camera) else {
                    continue;
                };
                let fg = named(&ctx.palette, &t.fg, UiColor::Text);
                let bg = named(&ctx.palette, &t.bg, UiColor::Black);
                for (i, glyph) in (0..).zip(t.glyphs) {
                    buf.set(x + i, y, Cell::new(glyph, fg, bg));
                }
            }
        }
    }

    fn draw_units(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        for unit in self.state.units() {
            if let Some((x, y)) = tile_to_cell(unit.pos, &self.camera) {
                units::draw_unit(buf, &ctx.palette, unit, x, y);
            }
        }
    }
}

/// The palette colour called `name`, or `fallback` (content validation
/// makes sure terrain colours exist, so this only guards against a bug).
fn named(palette: &Palette, name: &str, fallback: UiColor) -> Rgb {
    palette
        .lookup(name)
        .unwrap_or_else(|| palette.get(fallback))
}

impl Screen for BattleScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, _ctx: &mut Ctx, input: &FrameInput) -> Transition {
        self.cursor.tick(input.dt);
        let tiles = &self.state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        for &action in &input.actions {
            match action {
                Action::Cancel => return Transition::Pop,
                Action::NextUnit => self.cycle(true),
                Action::PrevUnit => self.cycle(false),
                _ => {
                    if self.cursor.step(action, w, h) {
                        self.follow(self.cursor.pos);
                    }
                }
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        self.draw_terrain(ctx, buf);
        self.draw_units(ctx, buf);
        if let Some((x, y)) = tile_to_cell(self.cursor.pos, &self.camera) {
            draw_cursor(buf, &ctx.palette, &self.cursor, x, y);
        }
        let panel_bg = c(UiColor::PanelBg);
        buf.fill_rect(SIDE_PANEL, Cell::new(' ', c(UiColor::Text), panel_bg));
        buf.draw_box(
            SIDE_PANEL,
            BoxStyle::Single,
            c(UiColor::PanelBorder),
            panel_bg,
        );
        panel::draw_hover(buf, &ctx.palette, &self.state, self.cursor.pos);
        buf.fill_rect(HELP_BAR, Cell::new(' ', c(UiColor::Text), black));
        buf.print(1, HELP_ROW, &self.help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use trpg_core::{BattleMap, Grid, Phase, TerrainId, Unit};

    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::harness::Harness;
    use crate::screen::tests::ctx;

    fn quick() -> BattleScreen {
        BattleScreen::new(quick_battle(&ctx().content).unwrap())
    }

    /// A battle on `map` with `units`, using the game's tables.
    fn battle(c: &Ctx, map: BattleMap, units: Vec<Unit>) -> BattleState {
        BattleState::new(BattleSetup {
            map,
            terrain: Arc::new(c.content.terrain.rules.clone()),
            classes: Arc::new(c.content.classes.clone()),
            items: Arc::new(c.content.items.clone()),
            spells: Arc::new(c.content.spells.clone()),
            skills: Arc::new(c.content.skills.clone()),
            arts: Arc::new(c.content.arts.clone()),
            pack: BattlePack::default(),
            gold: 0,
            stock: Stock::default(),
            units,
            reinforcements: vec![],
            objective: Objective::Rout { turn_limit: None },
            rewind_charges: 0,
            seed: 0,
        })
        .0
    }

    fn render(screen: &BattleScreen, c: &Ctx) -> GlyphBuffer {
        let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
        screen.draw(c, &mut buf);
        buf
    }

    #[test]
    fn quick_battle_state() {
        let state = quick_battle(&ctx().content).unwrap();
        assert_eq!(state.map().name, "Test Field");
        assert_eq!((state.turn(), state.phase()), (1, Phase::Player));
        assert_eq!(state.objective(), Objective::Rout { turn_limit: None });
        assert_eq!(state.outcome(), None);
        let units = state.units();
        let labels: Vec<(&str, Faction)> = units
            .iter()
            .map(|u| (u.map_label.as_str(), u.faction))
            .collect();
        assert_eq!(
            labels,
            [
                ("Lo", Faction::Player),
                ("Kn", Faction::Player),
                ("Ar", Faction::Player),
                ("Br", Faction::Enemy),
                ("Br", Faction::Enemy),
                ("Ra", Faction::Enemy),
            ]
        );
        let ids: Vec<u32> = units.iter().map(|u| u.id.0).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
        assert!(units[2].acted);
        assert_eq!(units.iter().filter(|u| u.acted).count(), 1);
        let hurt: Vec<usize> = (0..6)
            .filter(|&i| units[i].hp < units[i].stats.hp)
            .collect();
        assert_eq!(hurt, [1, 4]);
        for u in units {
            assert!(state.map().tiles.in_bounds(u.pos));
        }
    }

    #[test]
    fn quick_battle_reports_missing_content() {
        let mut content = ctx().content;
        content.characters.characters.clear();
        assert_eq!(
            quick_battle(&content),
            Err("no character \"test_lord\"".to_owned())
        );
        let mut content = ctx().content;
        content.characters.generics.clear();
        assert_eq!(
            quick_battle(&content),
            Err("no generic \"test_brigand\"".to_owned())
        );
        let mut content = ctx().content;
        content.maps.clear();
        assert_eq!(
            quick_battle(&content),
            Err("no map \"test_small\"".to_owned())
        );
        let mut content = ctx().content;
        content.classes.classes.clear();
        assert_eq!(
            quick_battle(&content),
            Err("unknown class \"exile\"".to_owned())
        );
        let mut content = ctx().content;
        let mut knight =
            content.characters.characters[&trpg_core::CharacterId("test_knight".into())].clone();
        knight.map_label = Some("Lo".into());
        content
            .characters
            .characters
            .insert(knight.id.clone(), knight);
        assert_eq!(
            quick_battle(&content),
            Err(
                "test_small: Test Lord and Test Knight are both labelled \"Lo\" on the map; \
                 give one a map_label"
                    .to_owned()
            )
        );
    }

    #[test]
    fn camera_starts_on_the_first_player_unit() {
        let c = ctx();
        let s = quick();
        // test_small is smaller than the viewport: centred.
        assert_eq!(s.camera().origin, Pos::new(-10, -11));
        assert_eq!(s.state().units().len(), 6);
        let big = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        let units = s.state().units().to_vec();
        let camera = |units: Vec<Unit>| {
            BattleScreen::new(battle(&c, big.clone(), units))
                .camera()
                .origin
        };
        // Lord at (3, 5): clamped to the top-left.
        assert_eq!(camera(units.clone()), Pos::new(0, 0));
        let mut far = units.clone();
        far[0].pos = Pos::new(60, 30);
        assert_eq!(camera(far), Pos::new(29, 10));
        let mut enemies_only = units;
        enemies_only.retain(|u| u.faction != Faction::Player);
        // Map centre (32, 20).
        assert_eq!(camera(enemies_only), Pos::new(15, 5));
    }

    #[test]
    fn cancel_pops_and_nothing_else_does() {
        let mut s = quick();
        let mut c = ctx();
        let step = |s: &mut BattleScreen, c: &mut Ctx, a: &[Action]| {
            format!(
                "{:?}",
                s.update(c, &FrameInput::new(a.to_vec(), 0.0, vec![]))
            )
        };
        assert_eq!(s.name(), "battle");
        assert!(!s.is_overlay());
        assert_eq!(step(&mut s, &mut c, &[]), "None");
        assert_eq!(step(&mut s, &mut c, &[Action::Confirm]), "None");
        assert_eq!(
            step(&mut s, &mut c, &[Action::Confirm, Action::Cancel]),
            "Pop"
        );
    }

    #[test]
    fn help_depends_on_what_is_hovered() {
        let mut c = ctx();
        let mut s = quick();
        // On the lord, ready to act.
        assert_eq!(s.help(&c), "f select · e info · s next unit · d back");
        // On the archer, who has acted, and on an enemy.
        s.cursor.jump(Pos::new(2, 4));
        assert_eq!(s.help(&c), "arrows move · e info · s next unit · d back");
        s.cursor.jump(Pos::new(8, 2));
        assert_eq!(s.help(&c), "arrows move · e info · s next unit · d back");
        // On an empty tile.
        s.cursor.jump(Pos::new(0, 0));
        assert_eq!(s.help(&c), "arrows move · s next unit · d back");
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(s.help(&c), "wasd move · l next unit · k back");
        s.cursor.jump(Pos::new(3, 5));
        assert_eq!(s.help(&c), "j select · i info · l next unit · k back");
    }

    #[test]
    fn cursor_starts_on_the_first_player_lord() {
        let c = ctx();
        let s = quick();
        assert_eq!(s.cursor(), Cursor::new(Pos::new(3, 5)));
        // The lord listed after another player unit: still the lord.
        let mut units = s.state().units().to_vec();
        units.swap(0, 1);
        let map = s.state().map().clone();
        let s2 = BattleScreen::new(battle(&c, map.clone(), units.clone()));
        assert_eq!(s2.cursor().pos, Pos::new(3, 5));
        // No lord: the first player unit.
        for u in &mut units {
            u.is_lord = false;
        }
        let s3 = BattleScreen::new(battle(&c, map.clone(), units.clone()));
        assert_eq!(s3.cursor().pos, Pos::new(4, 6));
        // No player unit: the map's centre (test_small is 14 × 8).
        units.retain(|u| u.faction != Faction::Player);
        let s4 = BattleScreen::new(battle(&c, map, units));
        assert_eq!(s4.cursor().pos, Pos::new(7, 4));
    }

    /// One frame with `actions`.
    fn step(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
        s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
    }

    #[test]
    fn cursor_moves_one_tile_per_step_and_stays_on_the_map() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::CursorRight; 3]);
        assert_eq!(s.cursor().pos, Pos::new(6, 5));
        step(&mut s, &mut c, &[Action::CursorDown, Action::CursorLeft]);
        assert_eq!(s.cursor().pos, Pos::new(5, 6));
        step(&mut s, &mut c, &[Action::CursorUp; 20]);
        assert_eq!(s.cursor().pos, Pos::new(5, 0));
        step(&mut s, &mut c, &[Action::CursorLeft; 20]);
        assert_eq!(s.cursor().pos, Pos::new(0, 0));
        step(&mut s, &mut c, &[Action::CursorDown; 20]);
        step(&mut s, &mut c, &[Action::CursorRight; 20]);
        assert_eq!(s.cursor().pos, Pos::new(13, 7));
        // Other keys don't move it; the small map stays centred.
        step(
            &mut s,
            &mut c,
            &[Action::Confirm, Action::Info, Action::DangerZone],
        );
        assert_eq!(s.cursor().pos, Pos::new(13, 7));
        assert_eq!(s.camera().origin, Pos::new(-10, -11));
    }

    #[test]
    fn cursor_pulses_with_frame_time() {
        let mut c = ctx();
        let mut s = quick();
        s.update(&mut c, &FrameInput::new(vec![], 0.5, vec![]));
        assert!((s.cursor().brightness() - cursor::BLINK_MIN).abs() < 1e-6);
        let dim = render(&s, &c);
        let bright = render(&quick(), &c);
        let cursor = c.palette.get(UiColor::Cursor);
        // The lord's tile at (3, 5) starts at cell (26, 16).
        assert_eq!(bright.get(25, 16).unwrap().fg, cursor);
        assert_eq!(dim.get(25, 16).unwrap().fg, cursor.scale(0.5));
        assert_eq!(bright.get(25, 16).unwrap().glyph, '[');
        assert_eq!(bright.get(28, 16).unwrap().glyph, ']');
    }

    #[test]
    fn next_and_prev_unit_cycle_ready_units_in_reading_order() {
        let mut c = ctx();
        let mut s = quick();
        // Ready: the lord (3, 5) and the knight (4, 6); the archer has acted.
        assert_eq!(s.ready_units(), [Pos::new(3, 5), Pos::new(4, 6)]);
        let mut visit = |a: Action| {
            step(&mut s, &mut c, &[a]);
            s.cursor().pos
        };
        assert_eq!(visit(Action::NextUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::NextUnit), Pos::new(3, 5));
        assert_eq!(visit(Action::PrevUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::PrevUnit), Pos::new(3, 5));
        // From a tile between them, in reading order.
        assert_eq!(visit(Action::CursorRight), Pos::new(4, 5));
        assert_eq!(visit(Action::NextUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::CursorUp), Pos::new(4, 5));
        assert_eq!(visit(Action::PrevUnit), Pos::new(3, 5));
        // Below every ready unit: next wraps to the first.
        for _ in 0..5 {
            visit(Action::CursorDown);
        }
        assert_eq!(visit(Action::NextUnit), Pos::new(3, 5));
    }

    #[test]
    fn cycling_does_nothing_without_ready_units_and_scrolls_the_camera() {
        let c = ctx();
        let mut ctx_ = ctx();
        let mut units = quick_battle(&c.content).unwrap().units().to_vec();
        for u in &mut units {
            u.acted = u.faction == Faction::Player;
        }
        let map = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        let mut s = BattleScreen::new(battle(&c, map.clone(), units.clone()));
        step(&mut s, &mut ctx_, &[Action::NextUnit, Action::PrevUnit]);
        assert_eq!(s.cursor().pos, Pos::new(3, 5));
        // A ready unit far away: the camera follows the jump.
        units[1].acted = false;
        units[1].pos = Pos::new(60, 35);
        let mut s = BattleScreen::new(battle(&c, map, units));
        step(&mut s, &mut ctx_, &[Action::NextUnit]);
        assert_eq!(s.cursor().pos, Pos::new(60, 35));
        // Scrolled just enough to keep it 3 tiles from the edges.
        assert_eq!(s.camera().origin, Pos::new(29, 9));
    }

    /// The Quick Battle's units on a 64 × 40 plain, in the harness.
    fn big_battle_harness() -> Harness {
        let c = ctx();
        let units = quick_battle(&c.content).unwrap().units().to_vec();
        let map = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        Harness::with_screen(Box::new(BattleScreen::new(battle(&c, map, units))))
    }

    /// The console cell of the left glyph of the tile under the cursor,
    /// found from its brackets.
    fn cursor_cell(buf: &GlyphBuffer, c: &Ctx) -> (i32, i32) {
        let fort = c.palette.lookup("fort");
        for y in 0..layout::MAP_VIEW.h {
            for x in 0..layout::MAP_VIEW.w {
                let cell = buf.get(x, y).unwrap();
                if Some(cell.fg) == fort {
                    continue;
                }
                match cell.glyph {
                    '[' => return (x + 1, y),
                    ']' => return (x - 2, y),
                    _ => {}
                }
            }
        }
        panic!("no cursor on screen");
    }

    #[test]
    fn harness_keys_move_the_cursor() {
        let c = ctx();
        let mut h = big_battle_harness();
        // The lord at (3, 5), camera at the top-left.
        assert_eq!(cursor_cell(h.game().buffer(), &c), (6, 5));
        h.keys("Right Right Right");
        assert_eq!(cursor_cell(h.game().buffer(), &c), (12, 5));
        h.keys("Down Left");
        assert_eq!(cursor_cell(h.game().buffer(), &c), (10, 6));
    }

    #[test]
    fn harness_hold_repeats_with_the_keymap_timing_and_scrolls() {
        let c = ctx();
        let repeat = c.content.keymap.repeat;
        let (delay, interval) = (repeat.delay_ms, repeat.interval_ms);
        let moves = |ms: u32| i32::try_from(1 + 1 + (ms - delay) / interval).unwrap();
        let mut h = big_battle_harness();
        h.hold("Right", 1.0);
        // 1 press + repeats at 170 ms, then every 55 ms: 17 tiles, to x = 20.
        assert_eq!(moves(1000), 17);
        assert_eq!(cursor_cell(h.game().buffer(), &c), (2 * (3 + 17), 5));
        h.hold("Right", 1.0);
        // x = 37: the camera keeps it 3 tiles from the right edge.
        let x = 3 + 2 * moves(1000);
        let origin = x - (layout::VIEW_TILES_W - 1 - Camera::MARGIN);
        assert_eq!(cursor_cell(h.game().buffer(), &c), (2 * (x - origin), 5));
        // Far right, then back: the cursor stops at the edge and the
        // camera shows the map's last columns, then scrolls back.
        h.hold("Right", 3.0).hold("Down", 3.0);
        assert_eq!(cursor_cell(h.game().buffer(), &c), (68, 29));
        h.hold("Left", 4.0).hold("Up", 3.0);
        assert_eq!(cursor_cell(h.game().buffer(), &c), (0, 0));
    }

    /// The text of row `y` of the side panel, inside its border, trimmed.
    fn panel_row(buf: &GlyphBuffer, y: i32) -> String {
        let (x0, x1) = (SIDE_PANEL.x + 1, SIDE_PANEL.x + SIDE_PANEL.w - 1);
        (x0..x1)
            .map(|x| buf.get(x, y).unwrap().glyph)
            .collect::<String>()
            .trim()
            .to_owned()
    }

    /// The Quick Battle with the knight moved into the forest at (1, 5).
    fn knight_in_forest() -> BattleScreen {
        let c = ctx();
        let state = quick_battle(&c.content).unwrap();
        let mut units = state.units().to_vec();
        units[1].pos = Pos::new(1, 5);
        let mut s = BattleScreen::new(battle(&c, state.map().clone(), units));
        s.cursor.jump(Pos::new(1, 5));
        s
    }

    #[test]
    fn panel_shows_the_terrain_and_unit_under_the_cursor() {
        let c = ctx();
        let s = knight_in_forest();
        let knight = s.hovered().unwrap().clone();
        let buf = render(&s, &c);
        let rows: Vec<String> = (1..10).map(|y| panel_row(&buf, y)).collect();
        let hp = format!("HP {}/{}", knight.hp, knight.stats.hp);
        assert_eq!(rows[0], "Forest");
        assert_eq!(rows[1], "DEF +1  AVO +20");
        assert_eq!(rows[2], "");
        assert_eq!(rows[4], "Test Knight");
        assert_eq!(rows[5], "Guard  Lv 1");
        assert!(rows[6].starts_with(&hp), "{}", rows[6]);
        assert_eq!(rows[7], "Player");
        // The knight is at 9/20 of max HP: 5 of 10 cells, `hp_mid`.
        let bar: String = (0..panel::HP_BAR_CELLS)
            .map(|i| buf.get(panel::HP_BAR_X + i, 7).unwrap().glyph)
            .collect();
        assert_eq!(bar, "█████░░░░░");
        let p = &c.palette;
        assert_eq!(
            buf.get(panel::HP_BAR_X, 7).unwrap().fg,
            p.get(UiColor::HpMid)
        );
        assert_eq!(
            buf.get(panel::TEXT_X, 5).unwrap().fg,
            p.get(UiColor::Player)
        );
        // An enemy: its faction, in red.
        let mut s = s;
        s.cursor.jump(Pos::new(8, 2));
        let buf = render(&s, &c);
        assert_eq!(panel_row(&buf, 8), "Enemy");
        assert_eq!(buf.get(panel::TEXT_X, 5).unwrap().fg, p.get(UiColor::Enemy));
    }

    #[test]
    fn panel_shows_healing_terrain_and_cuts_long_names() {
        let mut c = ctx();
        let s = knight_in_forest();
        let mut rules = c.content.terrain.rules.clone();
        let forest = c.content.terrain.display.id_of("forest").unwrap();
        rules.terrains[usize::from(forest.0)].heal_percent = 20;
        rules.terrains[usize::from(forest.0)].name = "A".repeat(40);
        c.content.terrain.rules = rules;
        let mut units = s.state().units().to_vec();
        units[1].name = "B".repeat(40);
        let mut s2 = BattleScreen::new(battle(&c, s.state().map().clone(), units));
        s2.cursor.jump(Pos::new(1, 5));
        let buf = render(&s2, &c);
        assert_eq!(panel_row(&buf, 1), "A".repeat(panel::TEXT_W));
        assert_eq!(panel_row(&buf, 3), "Heals 20% HP");
        assert_eq!(panel_row(&buf, 5), "B".repeat(panel::TEXT_W));
    }

    /// Hovering the wounded knight, standing in a forest.
    #[test]
    fn hover_unit_on_forest_snapshot() {
        let c = ctx();
        assert_snapshot!(render(&knight_in_forest(), &c).to_snapshot(&c.palette));
    }

    /// Hovering an empty plain: terrain only.
    #[test]
    fn hover_empty_plain_snapshot() {
        let c = ctx();
        let mut s = quick();
        s.cursor.jump(Pos::new(6, 5));
        let buf = render(&s, &c);
        assert_eq!(panel_row(&buf, 1), "Plain");
        assert_eq!(panel_row(&buf, 5), "");
        assert_snapshot!(buf.to_snapshot(&c.palette));
    }

    #[test]
    fn draw_covers_the_whole_buffer() {
        let c = ctx();
        let buf = render(&quick(), &c);
        let stale = Rgb::new(1, 2, 3);
        for y in 0..i32::from(CONSOLE_H) {
            for x in 0..i32::from(CONSOLE_W) {
                let cell = buf.get(x, y).unwrap();
                assert!(cell.fg != stale && cell.bg != stale, "({x}, {y})");
            }
        }
    }

    #[test]
    fn terrain_tiles_use_their_two_glyphs_and_colours() {
        let c = ctx();
        let s = quick();
        let buf = render(&s, &c);
        let p = &c.palette;
        // Tile (0, 0) is sea, drawn at cell (20, 11).
        let display = &c.content.terrain.display;
        let sea = display.get(display.id_of("sea").unwrap()).unwrap();
        let cell = |x| *buf.get(x, 11).unwrap();
        for x in [20, 21] {
            assert_eq!(cell(x).glyph, sea.glyphs[usize::try_from(x - 20).unwrap()]);
            assert_eq!(Some(cell(x).fg), p.lookup(&sea.fg));
            assert_eq!(Some(cell(x).bg), p.lookup(&sea.bg));
        }
        // Left of the map: blank.
        assert_eq!(
            cell(19),
            Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black))
        );
        assert_eq!(
            named(p, "no_such_colour", UiColor::Cursor),
            p.get(UiColor::Cursor)
        );
    }

    /// A 64 × 40 map of stripes, a unit in the bottom-right corner and the
    /// cursor and camera moved there.
    #[test]
    fn large_map_scrolled_to_bottom_right_snapshot() {
        let c = ctx();
        let display = &c.content.terrain.display;
        let ids =
            ["plain", "forest", "road", "water", "mountain"].map(|t| display.id_of(t).unwrap());
        let cells = (0..40)
            .flat_map(|y: usize| (0..64).map(move |x: usize| ids[(x / 3 + y / 2) % ids.len()]))
            .collect();
        let mut units = quick_battle(&c.content).unwrap().units().to_vec();
        // The camera starts on the lord at (3, 5), top-left.
        units[5].pos = Pos::new(63, 39);
        units[3].pos = Pos::new(30, 12);
        units[4].pos = Pos::new(28, 9); // Above the viewport: not drawn.
        let map = BattleMap::new("Stripes", Grid::from_cells(64, 40, cells).unwrap());
        let mut s = BattleScreen::new(battle(&c, map, units));
        assert_eq!(s.camera().origin, Pos::new(0, 0));
        s.cursor.jump(Pos::new(63, 39));
        s.follow(Pos::new(63, 39));
        assert_eq!(s.camera().origin, Pos::new(29, 10));
        let buf = render(&s, &c);
        // The corner unit sits in the viewport's last tile.
        assert_eq!(buf.get(68, 29).unwrap().glyph, 'R');
        assert_snapshot!(buf.to_snapshot(&c.palette));
    }
}
