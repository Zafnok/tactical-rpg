//! The battle screen (ADR-0018, `docs/design/look-and-feel.md`): the map
//! viewport on the left, the side panel on the right and the help bar at the
//! bottom. This first version only draws; the cursor and side-panel contents
//! come with ticket 0402, overlays with 0403.

pub mod camera;
pub mod layout;
pub mod units;

use std::sync::Arc;

use trpg_content::{Content, character_unit, check_map_labels};
use trpg_core::{
    BattlePack, BattleSetup, BattleState, Command, Faction, ItemId, Objective, Pos, UnitAction,
    UnitId,
};

use self::camera::{Camera, tile_to_cell};
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
        pack: BattlePack {
            items: vec![ItemId::new(QUICK_BATTLE_POTION); QUICK_BATTLE_POTIONS],
            cap: items.rules.default_pack_cap,
        },
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

/// The battle screen. Draws the battle; for now Cancel leaves it.
#[derive(Debug, Clone)]
pub struct BattleScreen {
    state: BattleState,
    camera: Camera,
}

impl BattleScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "battle";

    /// A screen showing `state`, the camera centred on the first player unit
    /// (or on the map's centre if there is none).
    pub fn new(state: BattleState) -> Self {
        let tiles = &state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        let target = state
            .units()
            .iter()
            .find(|u| u.faction == Faction::Player)
            .map_or_else(|| Pos::new(i32::from(w) / 2, i32::from(h) / 2), |u| u.pos);
        Self {
            camera: Camera::centred_on(target, w, h),
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

    /// Scrolls the camera to keep `target` [`Camera::MARGIN`] tiles from the
    /// viewport's edges.
    pub fn follow(&mut self, target: Pos) {
        let tiles = &self.state.map().tiles;
        self.camera
            .follow(target, tiles.width(), tiles.height(), Camera::MARGIN);
    }

    /// The help line, e.g. `arrows move · f select · d back · e info`.
    pub fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        help_line(&[
            (cursor_keys_name(km), "move"),
            (key_name(km, Action::Confirm), "select"),
            (key_name(km, Action::Cancel), "back"),
            (key_name(km, Action::Info), "info"),
        ])
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
        if input.actions.contains(&Action::Cancel) {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        self.draw_terrain(ctx, buf);
        self.draw_units(ctx, buf);
        let panel_bg = c(UiColor::PanelBg);
        buf.fill_rect(SIDE_PANEL, Cell::new(' ', c(UiColor::Text), panel_bg));
        buf.draw_box(
            SIDE_PANEL,
            BoxStyle::Single,
            c(UiColor::PanelBorder),
            panel_bg,
        );
        buf.fill_rect(HELP_BAR, Cell::new(' ', c(UiColor::Text), black));
        buf.print(1, HELP_ROW, &Self::help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use trpg_core::{BattleMap, Grid, Phase, TerrainId, Unit};

    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
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
            pack: BattlePack::default(),
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
        let big = BattleMap {
            name: "Big".into(),
            tiles: Grid::filled(64, 40, TerrainId(0)),
        };
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
    fn help_names_the_layout_keys() {
        let mut c = ctx();
        assert_eq!(
            BattleScreen::help(&c),
            "arrows move · f select · d back · e info"
        );
        c.use_layout(crate::input::Layout::LeftHanded);
        assert!(BattleScreen::help(&c).starts_with("wasd move · j select · k back"));
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
    /// camera scrolled there.
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
        let map = BattleMap {
            name: "Stripes".into(),
            tiles: Grid::from_cells(64, 40, cells).unwrap(),
        };
        let mut s = BattleScreen::new(battle(&c, map, units));
        assert_eq!(s.camera().origin, Pos::new(0, 0));
        s.follow(Pos::new(63, 39));
        assert_eq!(s.camera().origin, Pos::new(29, 10));
        let buf = render(&s, &c);
        // The corner unit sits in the viewport's last tile.
        assert_eq!(buf.get(68, 29).unwrap().glyph, 'R');
        assert_snapshot!(buf.to_snapshot(&c.palette));
    }
}
