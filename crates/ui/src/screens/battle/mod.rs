//! The battle screen (ADR-0018, `docs/design/look-and-feel.md`): the map
//! viewport on the left, the side panel on the right and the help bar at the
//! bottom. This first version only draws; the cursor and side-panel contents
//! come with ticket 0402, overlays with 0403.

pub mod camera;
pub mod layout;
pub mod units;

use trpg_content::{Content, check_map_labels};
use trpg_core::{BattleMap, Faction, Pos, Unit, UnitId};

use self::camera::{Camera, tile_to_cell};
use self::layout::{HELP_BAR, HELP_ROW, SIDE_PANEL, VIEW_TILES_H, VIEW_TILES_W};
use crate::color::{Palette, Rgb, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};

/// What the battle screen shows: the map and the units on it.
///
/// A stand-in until ticket 0305 adds `core`'s battle state (turns, commands,
/// events); the screen only reads it, so swapping it in is local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleScene {
    /// The battlefield.
    pub map: BattleMap,
    /// Every unit on the map.
    pub units: Vec<Unit>,
}

/// Map id of the debug Quick Battle.
pub const QUICK_BATTLE_MAP: &str = "test_small";

/// The debug Quick Battle: `test_small.map` with the placeholder characters
/// against generic enemies, one of them wounded and one having acted so both
/// looks show. Fails with a message if the content lacks something it
/// needs.
pub fn quick_battle(content: &Content) -> Result<BattleScene, String> {
    let map = content
        .maps
        .get(QUICK_BATTLE_MAP)
        .ok_or_else(|| format!("no map \"{QUICK_BATTLE_MAP}\""))?
        .map
        .clone();
    let classes = &content.classes;
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
        let unit = Unit::from_character(id(), def, classes, Faction::Player, pos)
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
            .unit(id(), classes, Faction::Enemy, pos)
            .map_err(|e| e.to_string())?;
        units.push(unit);
    }
    // The knight is wounded (mid HP), the second brigand badly (low HP), and
    // the archer has acted.
    units[1].hp = units[1].stats.hp * 9 / 20;
    units[4].hp = units[4].stats.hp / 4;
    units[2].acted = true;
    let errors = check_map_labels(QUICK_BATTLE_MAP, &units);
    if let Some(e) = errors.first() {
        return Err(e.to_string());
    }
    Ok(BattleScene { map, units })
}

/// The battle screen. Draws the scene; for now Cancel leaves it.
#[derive(Debug, Clone)]
pub struct BattleScreen {
    scene: BattleScene,
    camera: Camera,
}

impl BattleScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "battle";

    /// A screen showing `scene`, the camera centred on the first player unit
    /// (or on the map's centre if there is none).
    pub fn new(scene: BattleScene) -> Self {
        let (w, h) = (scene.map.tiles.width(), scene.map.tiles.height());
        let target = scene
            .units
            .iter()
            .find(|u| u.faction == Faction::Player)
            .map_or_else(|| Pos::new(i32::from(w) / 2, i32::from(h) / 2), |u| u.pos);
        Self {
            camera: Camera::centred_on(target, w, h),
            scene,
        }
    }

    /// The scene shown.
    pub fn scene(&self) -> &BattleScene {
        &self.scene
    }

    /// The camera.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Scrolls the camera to keep `target` [`Camera::MARGIN`] tiles from the
    /// viewport's edges.
    pub fn follow(&mut self, target: Pos) {
        let tiles = &self.scene.map.tiles;
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
                    .scene
                    .map
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
        for unit in &self.scene.units {
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
    use trpg_core::{Grid, TerrainId};

    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
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
    fn quick_battle_scene() {
        let scene = quick_battle(&ctx().content).unwrap();
        assert_eq!(scene.map.name, "Test Field");
        let labels: Vec<(&str, Faction)> = scene
            .units
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
        let ids: Vec<u32> = scene.units.iter().map(|u| u.id.0).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
        assert!(scene.units[2].acted);
        assert_eq!(scene.units.iter().filter(|u| u.acted).count(), 1);
        let hurt: Vec<usize> = (0..6)
            .filter(|&i| scene.units[i].hp < scene.units[i].stats.hp)
            .collect();
        assert_eq!(hurt, [1, 4]);
        for u in &scene.units {
            assert!(scene.map.tiles.in_bounds(u.pos));
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
        let s = quick();
        // test_small is smaller than the viewport: centred.
        assert_eq!(s.camera().origin, Pos::new(-10, -11));
        assert_eq!(s.scene().units.len(), 6);
        let big = BattleScene {
            map: BattleMap {
                name: "Big".into(),
                tiles: Grid::filled(64, 40, TerrainId(0)),
            },
            units: s.scene().units.clone(),
        };
        // Lord at (3, 5): clamped to the top-left.
        assert_eq!(
            BattleScreen::new(big.clone()).camera().origin,
            Pos::new(0, 0)
        );
        let mut far = big.clone();
        far.units[0].pos = Pos::new(60, 30);
        assert_eq!(BattleScreen::new(far).camera().origin, Pos::new(29, 10));
        let mut enemies_only = big;
        enemies_only.units.retain(|u| u.faction != Faction::Player);
        // Map centre (32, 20).
        assert_eq!(
            BattleScreen::new(enemies_only).camera().origin,
            Pos::new(15, 5)
        );
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
        let mut units = quick_battle(&c.content).unwrap().units;
        units[0].pos = Pos::new(63, 39);
        units[3].pos = Pos::new(30, 12);
        units[4].pos = Pos::new(28, 9); // Above the viewport: not drawn.
        let mut s = BattleScreen::new(BattleScene {
            map: BattleMap {
                name: "Stripes".into(),
                tiles: Grid::from_cells(64, 40, cells).unwrap(),
            },
            units,
        });
        s.follow(Pos::new(63, 39));
        assert_eq!(s.camera().origin, Pos::new(29, 10));
        let buf = render(&s, &c);
        // The corner unit sits in the viewport's last tile.
        assert_eq!(buf.get(68, 29).unwrap().glyph, 'L');
        assert_snapshot!(buf.to_snapshot(&c.palette));
    }
}
