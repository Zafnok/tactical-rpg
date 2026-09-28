//! The battle screen (ADR-0018, `docs/design/look-and-feel.md`): the map
//! viewport on the left, the side panel on the right and the help bar at the
//! bottom. The player browses with the cursor (ticket 0402), selects a unit,
//! steers its path, moves it and picks an action (0403, [`mode`]), and
//! attacks: picks a weapon and a target, reads the [`forecast`] and watches
//! the combat's [`playback`] (0404). Every command sent is kept in a
//! [`BattleHistory`], and Rewind opens the [`rewind`] screen (0307).

pub mod attack;
pub mod camera;
pub mod cursor;
pub mod forecast;
pub mod layout;
pub mod mode;
pub mod panel;
pub mod path;
pub mod playback;
pub mod rewind;
pub mod units;

use std::sync::Arc;

use trpg_content::{Content, character_unit, check_map_labels};
use trpg_core::{
    BattleHistory, BattlePack, BattleSetup, BattleState, Command, Faction, ItemId, Objective,
    Phase, Pos, Stock, TileSet, Unit, UnitId,
};

use self::camera::{Camera, tile_to_cell};
use self::cursor::{Cursor, draw_cursor};
use self::layout::{
    HELP_BAR, HELP_ROW, MAP_VIEW, SIDE_PANEL, TILE_W_CELLS, VIEW_TILES_H, VIEW_TILES_W,
};
use self::mode::{Effect, Mode};
use self::path::path_overlays;
use self::playback::{Playback, TIMINGS};
use self::rewind::{RewindEffect, RewindScreen};
use crate::color::{Palette, Rgb, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
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
/// against generic enemies (rout), at the start of the battle: everyone at
/// full HP and every player unit ready, one brigand close enough to fight
/// on turn 1. Fails with a message if the content
/// lacks something it needs.
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
        // In reach of every player unit on turn 1, for a first fight.
        ("test_brigand", Pos::new(7, 4)),
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
    let errors = check_map_labels(QUICK_BATTLE_MAP, &units);
    if let Some(e) = errors.first() {
        return Err(e.to_string());
    }
    let (state, _) = BattleState::new(BattleSetup {
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
    Ok(state)
}

/// How far range overlays tint a tile's background toward their colour
/// (`look-and-feel.md`: about 75%). *Tunable.*
pub const OVERLAY_BLEND: f32 = 0.75;

/// The battle screen: the player browses the map with the cursor, selects
/// and moves units ([`Mode`]); Cancel while browsing leaves it (until the
/// map menu, 0405). Rewind while browsing in the player phase opens the
/// [`RewindScreen`].
#[derive(Debug, Clone)]
pub struct BattleScreen {
    state: BattleState,
    history: BattleHistory,
    camera: Camera,
    cursor: Cursor,
    mode: Mode,
    rewind: Option<RewindScreen>,
}

impl BattleScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "battle";

    /// A screen showing `state` (a battle just started: its history, and
    /// every rewind charge, start here), with the cursor on the first player lord
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
            mode: Mode::after_command(&state),
            history: BattleHistory::new(state.clone()),
            rewind: None,
            state,
        }
    }

    /// The commands sent so far and the rewind charges left.
    pub fn history(&self) -> &BattleHistory {
        &self.history
    }

    /// The rewind screen, if open.
    pub fn rewind(&self) -> Option<&RewindScreen> {
        self.rewind.as_ref()
    }

    /// What the player is doing.
    pub fn mode(&self) -> &Mode {
        &self.mode
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

    /// The unit drawn under the cursor, if any.
    pub fn hovered(&self) -> Option<&Unit> {
        let pos = self.cursor.pos;
        self.state.units().iter().find(|u| self.drawn_pos(u) == pos)
    }

    /// Where `unit` is drawn: its tile, or where it walks or stands before
    /// its move is sent ([`Mode::drawn_pos`]).
    fn drawn_pos(&self, unit: &Unit) -> Pos {
        self.mode.drawn_pos(unit.id).unwrap_or(unit.pos)
    }

    /// Applies `cmd` (built by [`mode::step`] from legal choices) and
    /// records it in the history, then plays its combat if it had one, and
    /// continues browsing (or with the unit's move after its attack). A
    /// refused command changes nothing.
    fn apply(&mut self, cmd: &Command) {
        let before = self.state.units().to_vec();
        // Refused: the battle is unchanged and the player browses again.
        let events = self.state.apply(cmd).ok();
        if events.is_some() {
            self.history.push(cmd.clone());
        }
        let playback =
            events.and_then(|events| Playback::new(&events, &before, self.state.fallen(), TIMINGS));
        self.mode = match playback {
            Some(p) => Mode::Combat(Box::new(p)),
            None => Mode::after_command(&self.state),
        };
    }

    /// The units as drawn, each with how far it has faded out: the
    /// battle's units, except during a combat's playback, where its
    /// fighters show the HP it has reached (the attacker not dimmed yet)
    /// and the units that fell stay until they have faded.
    pub fn shown_units(&self) -> Vec<(Unit, f32)> {
        let Mode::Combat(pb) = &self.mode else {
            return self
                .state
                .units()
                .iter()
                .map(|u| (u.clone(), 0.0))
                .collect();
        };
        let attacker = pb.bouts().first().map(|b| b.attacker.unit);
        self.state
            .units()
            .iter()
            .chain(pb.falls())
            .map(|u| {
                let mut u = u.clone();
                if let Some(hp) = pb.hp(u.id) {
                    u.hp = hp;
                }
                if Some(u.id) == attacker {
                    u.acted = false;
                }
                let fade = pb.fade(u.id).unwrap_or(0.0);
                (u, fade)
            })
            .collect()
    }

    /// Whether Rewind opens the rewind screen: browsing in the player phase
    /// of a battle still running.
    fn can_open_rewind(&self) -> bool {
        matches!(self.mode, Mode::Idle { .. })
            && self.state.phase() == Phase::Player
            && self.state.outcome().is_none()
    }

    /// Handles one action on the open rewind screen.
    fn rewind_step(&mut self, action: Action) {
        let Some(screen) = self.rewind.as_mut() else {
            return;
        };
        match screen.step(action) {
            RewindEffect::None => {}
            RewindEffect::Close => self.rewind = None,
            RewindEffect::Rewind(point) => {
                // The screen only confirms a listed point with a charge left.
                if let Ok(state) = self.history.rewind_to(point) {
                    self.state = state;
                    self.mode = Mode::after_command(&self.state);
                }
                self.rewind = None;
            }
        }
    }

    /// Moves the cursor one tile for a cursor key; the camera and a
    /// selected unit's path follow.
    fn move_cursor(&mut self, action: Action) {
        let tiles = &self.state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        if self.cursor.step(action, w, h) {
            let to = self.cursor.pos;
            self.follow(to);
            self.mode.cursor_moved(to, &self.state);
        }
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

    /// The help line for the mode and what is under the cursor, e.g. `f
    /// select · e info · s next unit · d back` over a ready unit while
    /// browsing. Key names come from the keymap.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        let keys = cursor_keys_name(km);
        let confirm = |label| (key_name(km, Action::Confirm), label);
        let cancel = |label| (key_name(km, Action::Cancel), label);
        if let Some(r) = &self.rewind {
            return if r.is_confirming() {
                help_line(&[confirm("rewind"), cancel("back")])
            } else if r.can_rewind() {
                help_line(&[(keys, "choose"), confirm("rewind here"), cancel("close")])
            } else if r.entries().is_empty() {
                help_line(&[cancel("close")])
            } else {
                help_line(&[(keys, "choose"), cancel("close")])
            };
        }
        let info = (key_name(km, Action::Info), "info");
        let next = (key_name(km, Action::NextUnit), "next unit");
        let moves = (keys.clone(), "move");
        match &self.mode {
            Mode::Idle { threat } => {
                let back = cancel(if threat.is_some() {
                    "hide range"
                } else {
                    "back"
                });
                match self.hovered() {
                    Some(u) if self.is_ready(u) && u.faction == Faction::Player => {
                        help_line(&[confirm("select"), info, next, back])
                    }
                    Some(u) if u.faction != Faction::Player => {
                        help_line(&[moves, confirm("range"), info, next, back])
                    }
                    Some(_) => help_line(&[moves, info, next, back]),
                    None => help_line(&[moves, next, back]),
                }
            }
            Mode::Selected(sel) => {
                if self.cursor.pos == sel.dest() && sel.reach.is_stoppable(sel.dest()) {
                    help_line(&[moves, confirm("move here"), cancel("cancel")])
                } else {
                    help_line(&[moves, cancel("cancel")])
                }
            }
            Mode::Moving { .. } => help_line(&[confirm("skip")]),
            Mode::ActionMenu { .. } | Mode::WeaponMenu { .. } => {
                help_line(&[(keys, "choose"), confirm("confirm"), cancel("back")])
            }
            Mode::Targeting(_) => {
                help_line(&[(keys, "next target"), confirm("attack"), cancel("back")])
            }
            Mode::Combat(_) => {
                let hold = key_name(km, Action::Confirm).map(|k| format!("hold {k}"));
                help_line(&[cancel("skip"), (hold, "fast")])
            }
            Mode::MoveAfter { unit, tiles } => {
                let here = self.state.unit(*unit).map(|u| u.pos);
                if here == Some(self.cursor.pos) {
                    help_line(&[moves, confirm("stay")])
                } else if tiles.contains(&self.cursor.pos) {
                    help_line(&[moves, confirm("move here")])
                } else {
                    help_line(&[moves])
                }
            }
        }
    }

    /// Draws the terrain of every viewport tile; tiles off the map are left
    /// as they are (blank).
    fn draw_terrain(&self, ctx: &Ctx, buf: &mut GlyphBuffer, state: &BattleState) {
        let display = &ctx.content.terrain.display;
        let o = self.camera.origin;
        for dy in 0..VIEW_TILES_H {
            for dx in 0..VIEW_TILES_W {
                let pos = Pos::new(o.x + dx, o.y + dy);
                let Some(t) = state.map().tiles.get(pos).and_then(|&id| display.get(id)) else {
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

    /// Tints the tiles of the mode's ranges: a selected unit's move
    /// (`move_range`) and attack (`attack_range`) ranges, a shown threat
    /// area, or where a unit may move after its attack.
    fn draw_ranges(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let tint = |buf: &mut GlyphBuffer, tiles: Vec<Pos>, color| {
            let color = ctx.palette.get(color);
            for pos in tiles {
                if let Some((x, y)) = tile_to_cell(pos, &self.camera) {
                    buf.blend_bg(Rect::new(x, y, TILE_W_CELLS, 1), color, OVERLAY_BLEND);
                }
            }
        };
        let all = |set: &TileSet| set.iter().collect();
        match &self.mode {
            Mode::Selected(sel) => {
                tint(buf, all(&sel.moves), UiColor::MoveRange);
                tint(buf, all(&sel.attack), UiColor::AttackRange);
            }
            Mode::Idle {
                threat: Some(threat),
            } => tint(buf, all(&threat.area), UiColor::AttackRange),
            Mode::MoveAfter { tiles, .. } => tint(buf, tiles.clone(), UiColor::MoveRange),
            Mode::Targeting(t) => {
                let at = t.targets.iter().filter_map(|&id| self.state.unit(id));
                tint(buf, at.map(|u| u.pos).collect(), UiColor::AttackRange);
            }
            _ => {}
        }
    }

    fn draw_units(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        if !matches!(self.mode, Mode::Combat(_)) {
            for unit in self.state.units() {
                if let Some((x, y)) = tile_to_cell(self.drawn_pos(unit), &self.camera) {
                    units::draw_unit(buf, &ctx.palette, unit, x, y);
                }
            }
            return;
        }
        for (unit, fade) in self.shown_units() {
            if let Some((x, y)) = tile_to_cell(unit.pos, &self.camera) {
                units::draw_fading_unit(buf, &ctx.palette, &unit, x, y, fade);
            }
        }
    }

    /// Draws the cursor for the mode (in the player's cursor style): on the
    /// tile while browsing or on a selected unit; with a unit selected and
    /// the cursor away from it, the path and its arrowhead, with no cursor
    /// on the arrowhead's tile; none during a walk or in the menu.
    fn draw_cursor_and_path(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let pos = self.cursor.pos;
        match &self.mode {
            Mode::Moving { .. }
            | Mode::ActionMenu { .. }
            | Mode::WeaponMenu { .. }
            | Mode::Combat(_) => return,
            Mode::Selected(sel) => {
                let color = ctx.palette.get(UiColor::Path);
                for overlay in path_overlays(&sel.path, self.camera, color) {
                    buf.add_overlay(overlay);
                }
                if pos == sel.dest() && pos != sel.origin() {
                    return;
                }
            }
            Mode::Idle { .. } | Mode::MoveAfter { .. } | Mode::Targeting(_) => {}
        }
        if let Some((x, y)) = tile_to_cell(pos, &self.camera) {
            draw_cursor(buf, &ctx.palette, &self.cursor, ctx.cursor_style, x, y);
        }
    }

    /// Draws the action menu or the weapon list beside its unit, if open.
    fn draw_menu(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let (Mode::ActionMenu { sel, menu, .. } | Mode::WeaponMenu { sel, menu, .. }) = &self.mode
        else {
            return;
        };
        if let Some(cell) = tile_to_cell(sel.dest(), &self.camera) {
            let (x, y) = menu_origin(cell, menu.size());
            menu.draw(&ctx.palette, buf, x, y);
        }
    }
}

impl BattleScreen {
    /// Draws the rewind screen: the map as it was just before the
    /// highlighted action (as it is now with nothing listed), the list in
    /// the side panel and the help line.
    fn draw_rewind(&self, ctx: &Ctx, buf: &mut GlyphBuffer, r: &RewindScreen) {
        let shown = r.focused().map_or(&self.state, |e| &e.before);
        self.draw_terrain(ctx, buf, shown);
        for unit in shown.units() {
            if let Some((x, y)) = tile_to_cell(unit.pos, &self.camera) {
                units::draw_unit(buf, &ctx.palette, unit, x, y);
            }
        }
        r.draw(&ctx.palette, buf);
        let black = ctx.palette.get(UiColor::Black);
        let dim = ctx.palette.get(UiColor::TextDim);
        buf.print(1, HELP_ROW, &self.help(ctx), dim, black);
    }

    /// Draws the side panel: the forecast while targeting, else the
    /// terrain and unit under the cursor (as drawn, during a playback).
    fn draw_panel(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        if let Mode::Targeting(t) = &self.mode {
            forecast::draw_forecast(buf, &ctx.palette, &self.state, t);
            return;
        }
        let panel_bg = c(UiColor::PanelBg);
        buf.fill_rect(SIDE_PANEL, Cell::new(' ', c(UiColor::Text), panel_bg));
        // Double-line while a unit is selected (`look-and-feel.md`).
        let style = if self.mode.selection().is_some() {
            BoxStyle::Double
        } else {
            BoxStyle::Single
        };
        buf.draw_box(SIDE_PANEL, style, c(UiColor::PanelBorder), panel_bg);
        let pos = self.cursor.pos;
        if matches!(self.mode, Mode::Combat(_)) {
            let shown = self.shown_units();
            let hovered = shown.iter().find(|(u, f)| u.pos == pos && *f < 1.0);
            panel::draw_hover(buf, &ctx.palette, &self.state, pos, hovered.map(|(u, _)| u));
        } else {
            panel::draw_hover(buf, &ctx.palette, &self.state, pos, self.hovered());
        }
    }
}

/// Where a menu of `(w, h)` cells goes beside the tile whose left cell is
/// `(x, y)`: one cell right of the tile (clear of the cursor's marks), or
/// left of it if it would run past the map view; its first item level with
/// the tile, moved to fit the view.
fn menu_origin((x, y): (i32, i32), (w, h): (i32, i32)) -> (i32, i32) {
    let right = x + TILE_W_CELLS + 1;
    let mx = if right + w <= MAP_VIEW.x + MAP_VIEW.w {
        right
    } else {
        (x - 1 - w).max(MAP_VIEW.x)
    };
    let my = (y - 1).min(MAP_VIEW.y + MAP_VIEW.h - h).max(MAP_VIEW.y);
    (mx, my)
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
        for &action in &input.actions {
            match action {
                _ if self.rewind.is_some() => self.rewind_step(action),
                Action::Rewind if self.can_open_rewind() => {
                    let charges = self.state.rewind_charges();
                    self.rewind = Some(RewindScreen::new(&self.history, charges));
                }
                Action::NextUnit | Action::PrevUnit if matches!(self.mode, Mode::Idle { .. }) => {
                    self.cycle(action == Action::NextUnit);
                }
                Action::CursorLeft
                | Action::CursorRight
                | Action::CursorUp
                | Action::CursorDown
                    if self.mode.cursor_free() =>
                {
                    self.move_cursor(action);
                }
                _ => {
                    let mode = std::mem::take(&mut self.mode);
                    let (mode, effect) = mode::step(mode, action, self.cursor.pos, &self.state);
                    self.mode = mode;
                    match effect {
                        Effect::None => {}
                        Effect::Apply(cmd) => self.apply(&cmd),
                        Effect::Cursor(to) => {
                            self.cursor.jump(to);
                            self.follow(to);
                        }
                        Effect::Leave => return Transition::Pop,
                    }
                }
            }
        }
        let mode = std::mem::take(&mut self.mode);
        self.mode = mode.tick(input.dt, input.is_held(Action::Confirm), &self.state);
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        if let Some(r) = &self.rewind {
            self.draw_rewind(ctx, buf, r);
            return;
        }
        self.draw_terrain(ctx, buf, &self.state);
        self.draw_ranges(ctx, buf);
        self.draw_units(ctx, buf);
        self.draw_cursor_and_path(ctx, buf);
        self.draw_menu(ctx, buf);
        if let Mode::Combat(pb) = &self.mode {
            playback::draw_box(buf, &ctx.palette, pb);
        }
        self.draw_panel(ctx, buf);
        buf.fill_rect(HELP_BAR, Cell::new(' ', c(UiColor::Text), black));
        if let Mode::Combat(pb) = &self.mode
            && let Some(message) = pb.message()
        {
            buf.print(1, HELP_BAR.y, &message, c(UiColor::Text), black);
        }
        buf.print(1, HELP_ROW, &self.help(ctx), c(UiColor::TextDim), black);
    }
}

/// Helpers for this module's tests and its submodules'.
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::Arc;

    use trpg_core::{
        BattleMap, BattlePack, BattleSetup, BattleState, Command, Objective, Pos, SkillId,
        StatValue, Stock, Unit, UnitAction, UnitId,
    };

    use crate::screen::Ctx;

    /// A battle on `map` with `units` and `objective`, using the game's
    /// tables, without rewind charges.
    pub fn battle_with(
        c: &Ctx,
        map: BattleMap,
        units: Vec<Unit>,
        objective: Objective,
    ) -> BattleState {
        battle_charged(c, map, units, objective, 0)
    }

    /// [`battle_with`], with `charges` rewind charges.
    pub fn battle_charged(
        c: &Ctx,
        map: BattleMap,
        units: Vec<Unit>,
        objective: Objective,
        charges: u8,
    ) -> BattleState {
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
            objective,
            rewind_charges: charges,
            seed: 0,
        })
        .0
    }

    /// A rout battle on `map` with `units`.
    pub fn battle(c: &Ctx, map: BattleMap, units: Vec<Unit>) -> BattleState {
        battle_with(c, map, units, Objective::Rout { turn_limit: None })
    }

    /// The Quick Battle with the archer (unit 3) at (8, 4), two tiles below
    /// the first brigand (unit 4), knowing Vault, after a Vault attack on
    /// it: the archer waits to move after its attack.
    pub fn vaulted(c: &Ctx) -> BattleState {
        let quick = super::quick_battle(&c.content).unwrap_or_else(|e| panic!("{e}"));
        let mut units = quick.units().to_vec();
        units[2].pos = Pos::new(8, 4);
        assert!(units[2].learn_skill(&SkillId::new("vault"), &c.content.skills));
        let rout = Objective::Rout { turn_limit: None };
        let mut s = battle_with(c, quick.map().clone(), units, rout);
        let attack = Command::Act {
            unit: UnitId(3),
            dest: Pos::new(8, 4),
            action: UnitAction::Attack {
                target: UnitId(4),
                slot: 0,
                active: Some(SkillId::new("vault")),
                art: None,
            },
        };
        if let Err(e) = s.apply(&attack) {
            panic!("{e}");
        }
        assert!(s.pending_move().is_some());
        s
    }

    /// The Quick Battle set for a fight: the lord (unit 1) at (6, 2), one
    /// step left of (7, 2), from where it can hit the raider (unit 6, at
    /// (7, 1)) above and the first brigand (unit 4, at (8, 2), with
    /// `brigand_hp` HP) to its right; the archer (unit 3) at (8, 4), two
    /// tiles below the brigand, which can't counter at that range.
    pub fn skirmish(c: &Ctx, brigand_hp: StatValue) -> BattleState {
        skirmish_charged(c, brigand_hp, 0)
    }

    /// [`skirmish`], with `charges` rewind charges.
    pub fn skirmish_charged(c: &Ctx, brigand_hp: StatValue, charges: u8) -> BattleState {
        let quick = super::quick_battle(&c.content).unwrap_or_else(|e| panic!("{e}"));
        let mut units = quick.units().to_vec();
        units[0].pos = Pos::new(6, 2);
        units[2].pos = Pos::new(8, 4);
        units[3].hp = brigand_hp;
        let rout = Objective::Rout { turn_limit: None };
        battle_charged(c, quick.map().clone(), units, rout, charges)
    }

    /// `unit` of `state` waiting where it stands.
    pub fn wait(state: &mut BattleState, unit: usize) {
        let u = &state.units()[unit];
        let cmd = Command::Act {
            unit: u.id,
            dest: u.pos,
            action: UnitAction::Wait,
        };
        if let Err(e) = state.apply(&cmd) {
            panic!("{e}");
        }
    }
}

#[cfg(test)]
mod attack_tests;

#[cfg(test)]
mod rewind_tests;

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::glyph_buffer::Layer;
    use trpg_core::{BattleMap, Grid, Phase, TerrainId, Unit};

    use super::testing::{battle, vaulted, wait};
    use super::*;
    use crate::console::{CELL_H_PX, CELL_W_PX, CONSOLE_H, CONSOLE_W};
    use crate::harness::Harness;
    use crate::screen::tests::ctx;

    fn quick() -> BattleScreen {
        BattleScreen::new(quick_battle(&ctx().content).unwrap())
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
        for u in units {
            assert!(!u.acted, "{}", u.name);
            assert_eq!(u.hp, u.stats.hp, "{}", u.name);
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
        // Select the lord, then cancel the selection: still here.
        assert_eq!(step(&mut s, &mut c, &[Action::Confirm]), "None");
        assert_eq!(step(&mut s, &mut c, &[Action::Cancel]), "None");
        assert_eq!(s.mode(), &Mode::default());
        // Browsing: Cancel leaves.
        assert_eq!(
            step(
                &mut s,
                &mut c,
                &[Action::Confirm, Action::Cancel, Action::Cancel]
            ),
            "Pop"
        );
    }

    #[test]
    fn help_depends_on_what_is_hovered() {
        let mut c = ctx();
        let mut state = quick_battle(&c.content).unwrap();
        // The archer waits where it stands.
        wait(&mut state, 2);
        let mut s = BattleScreen::new(state);
        // On the lord, ready to act.
        assert_eq!(s.help(&c), "f select · e info · s next unit · d back");
        // On the archer, who has acted, and on an enemy.
        s.cursor.jump(Pos::new(2, 4));
        assert_eq!(s.help(&c), "arrows move · e info · s next unit · d back");
        s.cursor.jump(Pos::new(8, 2));
        assert_eq!(
            s.help(&c),
            "arrows move · f range · e info · s next unit · d back"
        );
        // An enemy's range shown: Cancel hides it.
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(
            s.help(&c),
            "arrows move · f range · e info · s next unit · d hide range"
        );
        step(&mut s, &mut c, &[Action::Cancel]);
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
        let colours = |b: &GlyphBuffer| {
            b.overlays()
                .iter()
                .filter(|o| o.color == cursor || o.color == cursor.scale(0.5))
                .map(|o| o.color)
                .collect::<Vec<_>>()
        };
        assert_eq!(colours(&bright), [cursor; 8]);
        assert_eq!(colours(&dim), [cursor.scale(0.5); 8]);
    }

    #[test]
    fn next_and_prev_unit_cycle_ready_units_in_reading_order() {
        let mut c = ctx();
        let mut s = quick();
        // Ready, in reading order: the archer (2, 4), the lord (3, 5) and
        // the knight (4, 6).
        assert_eq!(
            s.ready_units(),
            [Pos::new(2, 4), Pos::new(3, 5), Pos::new(4, 6)]
        );
        let mut visit = |a: Action| {
            step(&mut s, &mut c, &[a]);
            s.cursor().pos
        };
        assert_eq!(visit(Action::NextUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::NextUnit), Pos::new(2, 4));
        assert_eq!(visit(Action::NextUnit), Pos::new(3, 5));
        assert_eq!(visit(Action::PrevUnit), Pos::new(2, 4));
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
        assert_eq!(visit(Action::NextUnit), Pos::new(2, 4));
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
    /// found from its top-right corner mark: the only 1 × 3 px overlays are
    /// the corner marks' vertical arms, and the right ones are in the tile's
    /// last pixel column.
    fn cursor_cell(buf: &GlyphBuffer) -> (i32, i32) {
        let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
        let arms = buf
            .overlays()
            .iter()
            .map(|o| o.rect)
            .filter(|r| (r.w, r.h) == (1, 3));
        let r = arms
            .max_by_key(|r| (r.x, -r.y))
            .expect("no cursor on screen");
        ((r.x + 1) / cw - layout::TILE_W_CELLS, r.y / ch)
    }

    #[test]
    fn harness_keys_move_the_cursor() {
        let mut h = big_battle_harness();
        // The lord at (3, 5), camera at the top-left.
        assert_eq!(cursor_cell(h.game().buffer()), (6, 5));
        h.keys("Right Right Right");
        assert_eq!(cursor_cell(h.game().buffer()), (12, 5));
        h.keys("Down Left");
        assert_eq!(cursor_cell(h.game().buffer()), (10, 6));
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
        assert_eq!(cursor_cell(h.game().buffer()), (2 * (3 + 17), 5));
        h.hold("Right", 1.0);
        // x = 37: the camera keeps it 3 tiles from the right edge.
        let x = 3 + 2 * moves(1000);
        let origin = x - (layout::VIEW_TILES_W - 1 - Camera::MARGIN);
        assert_eq!(cursor_cell(h.game().buffer()), (2 * (x - origin), 5));
        // Far right, then back: the cursor stops at the edge and the
        // camera shows the map's last columns, then scrolls back.
        h.hold("Right", 3.0).hold("Down", 3.0);
        assert_eq!(cursor_cell(h.game().buffer()), (68, 29));
        h.hold("Left", 4.0).hold("Up", 3.0);
        assert_eq!(cursor_cell(h.game().buffer()), (0, 0));
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

    /// The Quick Battle with the knight wounded (9/20 HP) and moved into
    /// the forest at (1, 5).
    fn knight_in_forest() -> BattleScreen {
        let c = ctx();
        let state = quick_battle(&c.content).unwrap();
        let mut units = state.units().to_vec();
        units[1].pos = Pos::new(1, 5);
        units[1].hp = units[1].stats.hp * 9 / 20;
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

    /// One frame of `dt` seconds with `actions`, Confirm held if `held`.
    fn frame(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], dt: f32, held: bool) {
        let held = if held { vec![Action::Confirm] } else { vec![] };
        s.update(c, &FrameInput::new(actions.to_vec(), dt, held));
    }

    /// The Quick Battle with the lord selected and walked two tiles right to
    /// (5, 5), its action menu open.
    fn lord_menu(c: &mut Ctx) -> BattleScreen {
        let mut s = quick();
        step(
            &mut s,
            c,
            &[Action::Confirm, Action::CursorRight, Action::CursorRight],
        );
        step(&mut s, c, &[Action::Confirm]);
        assert!(matches!(s.mode(), Mode::Moving { .. }), "{:?}", s.mode());
        frame(&mut s, c, &[], 1.0, false);
        assert!(
            matches!(s.mode(), Mode::ActionMenu { .. }),
            "{:?}",
            s.mode()
        );
        s
    }

    #[test]
    fn select_move_and_wait_ends_the_units_action() {
        let mut c = ctx();
        let mut s = lord_menu(&mut c);
        step(&mut s, &mut c, &[Action::Confirm]);
        let lord = &s.state().units()[0];
        assert_eq!((lord.pos, lord.acted), (Pos::new(5, 5), true));
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(s.cursor().pos, Pos::new(5, 5));
        // No longer ready: not selectable, and cycling skips it.
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(s.ready_units(), [Pos::new(2, 4), Pos::new(4, 6)]);
    }

    #[test]
    fn cancelling_from_the_menu_and_the_selection_leaves_the_battle_unchanged() {
        let mut c = ctx();
        let before = quick().state().clone();
        let mut s = lord_menu(&mut c);
        // The lord is drawn at (5, 5), but the battle hasn't changed.
        assert_eq!(s.hovered().map(|u| u.id), Some(UnitId(1)));
        assert_eq!(s.state(), &before);
        step(&mut s, &mut c, &[Action::Cancel]);
        let Mode::Selected(sel) = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(sel.path, [Pos::new(3, 5), Pos::new(4, 5), Pos::new(5, 5)]);
        assert_eq!(s.hovered(), None, "drawn back at its own tile");
        assert_eq!(s.state(), &before);
        step(&mut s, &mut c, &[Action::Cancel]);
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(s.cursor().pos, Pos::new(3, 5));
        assert_eq!(s.state(), &before);
    }

    #[test]
    fn a_held_confirm_skips_the_walk() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm]);
        step(&mut s, &mut c, &[Action::CursorRight; 2]);
        // The press starts the walk; still held after 0.2 s, it skips.
        frame(&mut s, &mut c, &[Action::Confirm], 0.0, true);
        frame(&mut s, &mut c, &[], mode::HOLD_SKIP_S / 2.0, true);
        assert!(matches!(s.mode(), Mode::Moving { .. }));
        frame(&mut s, &mut c, &[], mode::HOLD_SKIP_S / 2.0, true);
        assert!(matches!(s.mode(), Mode::ActionMenu { .. }));
    }

    #[test]
    fn keys_do_only_what_the_mode_allows() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
        // Selected: cycling does nothing.
        step(&mut s, &mut c, &[Action::NextUnit, Action::PrevUnit]);
        assert_eq!(s.cursor().pos, Pos::new(4, 5));
        // In the menu, cursor keys move its focus, not the cursor.
        let mut s = lord_menu(&mut c);
        step(
            &mut s,
            &mut c,
            &[Action::CursorUp, Action::CursorLeft, Action::NextUnit],
        );
        assert_eq!(s.cursor().pos, Pos::new(5, 5));
        // While walking, too.
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
        step(&mut s, &mut c, &[Action::Confirm, Action::CursorDown]);
        assert_eq!(s.cursor().pos, Pos::new(4, 5));
    }

    #[test]
    fn selecting_draws_ranges_and_a_double_panel_border() {
        let mut c = ctx();
        let mut s = quick();
        let plain = render(&s, &c);
        step(&mut s, &mut c, &[Action::Confirm]);
        let buf = render(&s, &c);
        let p = &c.palette;
        // The cursor stays on the lord, as corner marks: 8 arms.
        assert_eq!(cursor_marks(&buf, &c), 8);
        assert_eq!(cursor_cell(&buf), (26, 16));
        // (6, 5), reachable, is tinted `move_range`.
        let tinted = |cell: (i32, i32), color| {
            let was = plain.get(cell.0, cell.1).unwrap().bg;
            buf.get(cell.0, cell.1).unwrap().bg == was.lerp(p.get(color), OVERLAY_BLEND)
        };
        assert!(tinted((32, 16), UiColor::MoveRange));
        assert!(tinted((33, 16), UiColor::MoveRange));
        let Mode::Selected(sel) = s.mode() else {
            panic!()
        };
        let attack = sel.attack.iter().next().unwrap();
        let (ax, ay) = tile_to_cell(attack, &s.camera()).unwrap();
        assert!(tinted((ax, ay), UiColor::AttackRange));
        assert_eq!(buf.get(SIDE_PANEL.x, 0).unwrap().glyph, '╔');
        assert_eq!(plain.get(SIDE_PANEL.x, 0).unwrap().glyph, '┌');
        // No path yet.
        let path = p.get(UiColor::Path);
        assert!(!buf.overlays().iter().any(|o| o.color == path));
    }

    #[test]
    fn the_path_ends_in_an_arrowhead_with_no_cursor_frame_there() {
        let mut c = ctx();
        let mut s = quick();
        step(
            &mut s,
            &mut c,
            &[Action::Confirm, Action::CursorRight, Action::CursorRight],
        );
        let buf = render(&s, &c);
        let path = c.palette.get(UiColor::Path);
        let lines = buf.overlays().iter().filter(|o| o.color == path);
        assert_eq!(lines.clone().filter(|o| o.layer == Layer::Under).count(), 2);
        assert_eq!(lines.filter(|o| o.layer == Layer::Over).count(), 6);
        // (5, 5) is a fort, drawn `[]` from cell 30: no cursor marks
        // around it, and the lord keeps its letters.
        assert_eq!(cursor_marks(&buf, &c), 0);
        let glyphs: String = (25..34).map(|x| buf.get(x, 16).unwrap().glyph).collect();
        assert_eq!(glyphs, "·Lo..[]..");
        // On to the map's right edge, past the lord's reach (Mov 5): the
        // path, through the fort (cost 2), stops at (7, 5), and the cursor
        // shows its corner marks.
        step(&mut s, &mut c, &[Action::CursorRight; 8]);
        assert_eq!(s.cursor().pos, Pos::new(13, 5));
        let Mode::Selected(sel) = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(sel.dest(), Pos::new(7, 5));
        let buf = render(&s, &c);
        assert_eq!(cursor_marks(&buf, &c), 8);
        assert_eq!(cursor_cell(&buf), (46, 16));
    }

    /// How many overlays are in the cursor's full-brightness colour.
    fn cursor_marks(buf: &GlyphBuffer, c: &Ctx) -> usize {
        let cursor = c.palette.get(UiColor::Cursor);
        buf.overlays().iter().filter(|o| o.color == cursor).count()
    }

    #[test]
    fn the_menu_opens_beside_the_unit_drawn_at_its_new_tile() {
        let mut c = ctx();
        let s = lord_menu(&mut c);
        let buf = render(&s, &c);
        let row = |y: i32, from: i32, n: i32| -> String {
            (from..from + n)
                .map(|x| buf.get(x, y).unwrap().glyph)
                .collect()
        };
        // The lord at (5, 5), cells 30..32; its old tile shows its terrain.
        assert_eq!(row(16, 26, 6), "....Lo");
        // The menu one cell right of the tile: `Attack` (dim), `Wait`.
        assert_eq!(row(16, 33, 10), "│ Attack │");
        assert_eq!(row(17, 33, 10), "│ Wait   │");
        let p = &c.palette;
        assert_eq!(buf.get(35, 16).unwrap().fg, p.get(UiColor::TextDim));
        assert_eq!(
            buf.get(35, 17).unwrap().bg,
            p.get(UiColor::PanelBorderFocus)
        );
        // No cursor, no ranges, the panel shows the lord.
        assert_eq!(buf.get(29, 16).unwrap().glyph, '.');
        assert_eq!(panel_row(&buf, 5), "Test Lord");
    }

    #[test]
    fn menus_go_right_of_the_tile_unless_they_would_leave_the_view() {
        // The first item (under the top border) level with the tile.
        assert_eq!(menu_origin((10, 4), (10, 4)), (13, 3));
        // Too far right: left of the tile.
        assert_eq!(menu_origin((60, 4), (10, 4)), (49, 3));
        assert_eq!(menu_origin((57, 4), (10, 4)), (60, 3));
        // Too low or high: moved to fit; never off the left edge.
        assert_eq!(menu_origin((10, 29), (10, 4)), (13, 26));
        assert_eq!(menu_origin((2, 0), (70, 4)), (0, 0));
    }

    #[test]
    fn help_follows_the_mode() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm]);
        // On the unit: its own tile is a legal end.
        assert_eq!(s.help(&c), "arrows move · f move here · d cancel");
        step(&mut s, &mut c, &[Action::CursorRight, Action::CursorDown]);
        // On the knight: can't stop there.
        assert_eq!(s.help(&c), "arrows move · d cancel");
        step(&mut s, &mut c, &[Action::CursorUp, Action::CursorRight]);
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(s.help(&c), "f skip");
        frame(&mut s, &mut c, &[], 1.0, false);
        assert_eq!(s.help(&c), "arrows choose · f confirm · d back");
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(s.help(&c), "wasd choose · j confirm · k back");
    }

    #[test]
    fn an_enemys_threat_area_is_tinted_until_hidden() {
        let mut c = ctx();
        let mut s = quick();
        s.cursor.jump(Pos::new(8, 2));
        let plain = render(&s, &c);
        step(&mut s, &mut c, &[Action::Confirm]);
        let shown = render(&s, &c);
        // The brigand's tile (cells 36..38, row 13) and one it can reach.
        let red = c.palette.get(UiColor::AttackRange);
        for (x, y) in [(36, 13), (37, 13), (36, 18)] {
            let was = plain.get(x, y).unwrap().bg;
            assert_eq!(shown.get(x, y).unwrap().bg, was.lerp(red, OVERLAY_BLEND));
        }
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(
            render(&s, &c).get(36, 13).unwrap().bg,
            plain.get(36, 13).unwrap().bg
        );
    }

    #[test]
    fn a_pending_move_after_an_attack_is_chosen_on_the_map() {
        let mut c = ctx();
        let state = vaulted(&c);
        let tiles = state.move_after_tiles();
        let mut s = BattleScreen::new(state);
        let Mode::MoveAfter { unit, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(*unit, UnitId(3));
        // Its tiles are tinted `move_range`.
        let buf = render(&s, &c);
        let (x, y) = tile_to_cell(tiles[0], &s.camera()).unwrap();
        let was = render(&quick(), &c).get(x, y).unwrap().bg;
        let blue = c.palette.get(UiColor::MoveRange);
        assert_eq!(buf.get(x, y).unwrap().bg, was.lerp(blue, OVERLAY_BLEND));
        s.cursor.jump(Pos::new(8, 4));
        assert_eq!(s.help(&c), "arrows move · f stay");
        s.cursor.jump(tiles[0]);
        assert_eq!(s.help(&c), "arrows move · f move here");
        s.cursor.jump(Pos::new(0, 0));
        assert_eq!(s.help(&c), "arrows move");
        // Cancel doesn't leave; Confirm on a tile moves there.
        step(&mut s, &mut c, &[Action::Cancel]);
        assert!(matches!(s.mode(), Mode::MoveAfter { .. }));
        s.cursor.jump(tiles[0]);
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(s.mode(), &Mode::default());
        let archer = s.state().unit(UnitId(3)).unwrap();
        assert_eq!((archer.pos, archer.acted), (tiles[0], true));
    }
}
