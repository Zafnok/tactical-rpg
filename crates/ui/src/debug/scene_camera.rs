//! The scene camera (debug builds, ticket 0228): the test map as a
//! backdrop (ADR-0048) behind a window of the console, panned by the pixel
//! and zoomed in whole steps, with a caption and a text box over it. For
//! checking by eye that a renderer keeps the scene sharp and seamless.

use std::rc::Rc;

use trpg_content::bundle::display_path;
use trpg_core::Pos;

use crate::audio::MenuSound;
use crate::cinema::{self, MAX_ZOOM};
use crate::color::UiColor;
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::map_view::{CursorStyle, CursorView, MapScene};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};

/// The map shown: `assets/maps/test_small.map`.
const MAP: &str = "test_small";
/// Left column of the title and the lines under it.
const LEFT: i32 = 2;
/// The window the scene shows through: 384 × 224 console pixels, so the
/// test map in glyphs (224 × 128) is smaller than it at 1× and bigger from
/// 2× up.
const WINDOW: Rect = Rect::new(26, 4, 48, 14);
/// The text box drawn over the window's bottom-right corner.
const TEXT_BOX: Rect = Rect::new(62, 15, 24, 5);
/// The lines in the text box.
const BOX_TEXT: [&str; 3] = ["A text box drawn", "over the scene", "hides it."];
/// How fast a held cursor key pans, in scene pixels a second.
const PAN_SPEED: f32 = 60.0;
/// An area surely big enough for one tile of any skin, to ask its size.
const ROOMY: Rect = Rect::new(0, 0, 64, 64);

/// Shows the test map through a window: the cursor actions pan (held),
/// `Confirm` steps the zoom 1× to 4× and round again, `Cancel` closes.
#[derive(Debug, Clone)]
pub struct SceneCameraScreen {
    /// The whole map, painted once by the map skin in use; `None` if the
    /// content has no test map.
    scene: Option<Rc<GlyphBuffer>>,
    /// The scene pixel in the middle of the window.
    centre: (f32, f32),
    /// Console pixels per scene pixel.
    zoom: u8,
}

impl SceneCameraScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "scene_camera";

    /// The camera at 1× over the middle of the test map, as `ctx`'s map
    /// skin paints it.
    pub fn new(ctx: &Ctx) -> Self {
        let scene = map_picture(ctx).map(Rc::new);
        let (w, h) = scene.as_deref().map_or((0, 0), pixel_size);
        #[allow(clippy::cast_precision_loss)] // pixel sizes are small
        let centre = (w as f32 / 2.0, h as f32 / 2.0);
        Self {
            scene,
            centre,
            zoom: 1,
        }
    }

    /// Console pixels per scene pixel: 1 to [`MAX_ZOOM`].
    pub fn zoom(&self) -> u8 {
        self.zoom
    }

    /// The scene pixel at the window's top-left ([`cinema::view`]).
    pub fn origin_px(&self) -> (f32, f32) {
        let scene = self.scene.as_deref().map_or((0, 0), pixel_size);
        cinema::view(scene, window_px(), self.centre, self.zoom)
    }

    /// Moves the centre by `(dx, dy)` scene pixels, kept to the middle of
    /// what the window can show: holding a key at an edge doesn't run the
    /// centre away from it. Along an axis the whole scene fits in, that is
    /// the scene's middle.
    fn pan(&mut self, dx: f32, dy: f32) {
        let scene = self.scene.as_deref().map_or((0, 0), pixel_size);
        let clip = window_px();
        let zoom = f32::from(self.zoom);
        #[allow(clippy::cast_precision_loss)] // pixel sizes are small
        let axis = |centre: f32, scene: u32, clip: u32| {
            let (half, middle) = (clip as f32 / zoom / 2.0, scene as f32 / 2.0);
            centre.clamp(half.min(middle), (scene as f32 - half).max(middle))
        };
        self.centre = (
            axis(self.centre.0 + dx, scene.0, clip.0),
            axis(self.centre.1 + dy, scene.1, clip.1),
        );
    }
}

/// The window's size in console pixels.
fn window_px() -> (u32, u32) {
    let px = |cells: i32, cell: u16| u32::try_from(cells).unwrap_or(0) * u32::from(cell);
    (px(WINDOW.w, CELL_W_PX), px(WINDOW.h, CELL_H_PX))
}

/// `buf`'s size in its own pixels.
fn pixel_size(buf: &GlyphBuffer) -> (u32, u32) {
    let px = buf.pixel_bounds();
    (
        u32::try_from(px.w).unwrap_or(0),
        u32::try_from(px.h).unwrap_or(0),
    )
}

/// The whole test map with a path and the cursor on it (pixel-placed
/// rectangles, to see them scale), painted by `ctx`'s map skin into a
/// buffer just big enough. `None` without the map.
fn map_picture(ctx: &Ctx) -> Option<GlyphBuffer> {
    let tiles = &ctx.content.maps.get(MAP)?.map.tiles;
    let size = (i32::from(tiles.width()), i32::from(tiles.height()));
    let mut scene = MapScene::new(Pos::new(0, 0), size);
    for pos in (0..size.1).flat_map(|y| (0..size.0).map(move |x| Pos::new(x, y))) {
        if let Some(tile) = scene.tile_mut(pos) {
            tile.terrain = tiles.get(pos).copied();
        }
    }
    scene.path = [(2, 3), (3, 3), (4, 3), (4, 4), (5, 4)]
        .map(|(x, y)| Pos::new(x, y))
        .to_vec();
    scene.cursor = Some(CursorView {
        pos: Pos::new(7, 3),
        brightness: 1.0,
        style: CursorStyle::Corners,
    });
    let skin = &ctx.map_skin;
    let tile = skin.tile_px(&scene, ROOMY, scene.origin)?;
    let cells = |tiles: i32, tile_px: i32, cell_px: u16| {
        let cell_px = i32::from(cell_px);
        u16::try_from((tiles * tile_px + cell_px - 1) / cell_px).unwrap_or(0)
    };
    let (w, h) = (
        cells(size.0, tile.w, CELL_W_PX),
        cells(size.1, tile.h, CELL_H_PX),
    );
    let c = |u| ctx.palette.get(u);
    let blank = Cell::new(' ', c(UiColor::Text), c(UiColor::Black));
    let mut buf = GlyphBuffer::new(w, h, blank);
    skin.paint(ctx, &scene, buf.bounds(), &mut buf);
    Some(buf)
}

impl Screen for SceneCameraScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            match action {
                Action::Cancel => {
                    ctx.audio.menu(MenuSound::Cancel);
                    return Transition::Pop;
                }
                Action::Confirm => {
                    ctx.audio.menu(MenuSound::Select);
                    self.zoom = self.zoom % MAX_ZOOM + 1;
                    self.pan(0.0, 0.0);
                }
                _ => {}
            }
        }
        let step = if input.dt.is_finite() {
            PAN_SPEED * input.dt.max(0.0)
        } else {
            0.0
        };
        let held = |action| f32::from(u8::from(input.is_held(action)));
        let dx = held(Action::CursorRight) - held(Action::CursorLeft);
        let dy = held(Action::CursorDown) - held(Action::CursorUp);
        self.pan(dx * step, dy * step);
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let (black, text, dim, hi) = (
            c(UiColor::Black),
            c(UiColor::Text),
            c(UiColor::TextDim),
            c(UiColor::TextHighlight),
        );
        buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
        buf.print(LEFT, 0, "Scene camera (debug)", hi, black);
        let km = ctx.help_keys();
        let help = help_line(&[
            (Some(cursor_keys_name(km)), "pan (hold)"),
            (Some(key_name(km, Action::Confirm)), "zoom"),
            (Some(key_name(km, Action::Cancel)), "back"),
        ]);
        buf.print(LEFT, i32::from(buf.height()) - 1, &help, dim, black);

        let Some(scene) = &self.scene else {
            let missing = format!("No {}", display_path(&format!("maps/{MAP}.map")));
            buf.print(LEFT, 2, &missing, text, black);
            return;
        };
        let (x, y) = self.origin_px();
        let info = format!("zoom {}x   window starts at {x:.1},{y:.1}", self.zoom);
        buf.print(LEFT, 2, &info, text, black);

        let frame = Rect::new(WINDOW.x - 1, WINDOW.y - 1, WINDOW.w + 2, WINDOW.h + 2);
        buf.draw_box(frame, BoxStyle::Single, c(UiColor::PanelBorder), black);
        buf.fill_rect(WINDOW, Cell::see_through(' ', text));
        buf.set_backdrop(Rc::clone(scene), WINDOW, (x, y), self.zoom);
        // Glyphs over the scene, with no box behind them.
        buf.print_fg(WINDOW.x + 1, WINDOW.y, "Text over the scene", hi);

        let panel_bg = c(UiColor::PanelBg);
        buf.fill_rect(TEXT_BOX, Cell::new(' ', text, panel_bg));
        buf.draw_box(
            TEXT_BOX,
            BoxStyle::Double,
            c(UiColor::PanelBorderFocus),
            panel_bg,
        );
        for (row, line) in (TEXT_BOX.y + 1..).zip(BOX_TEXT) {
            buf.print(TEXT_BOX.x + 2, row, line, text, panel_bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::map_view::skin_named;
    use crate::screen::tests::ctx;

    fn draw(ctx: &Ctx, screen: &SceneCameraScreen) -> GlyphBuffer {
        let p = &ctx.palette;
        let mut buf = GlyphBuffer::new(
            CONSOLE_W,
            CONSOLE_H,
            Cell::new('x', p.get(UiColor::Text), p.get(UiColor::PanelBg)),
        );
        screen.draw(ctx, &mut buf);
        buf
    }

    /// One frame of `dt` seconds with `pressed` pressed and `held` down.
    fn frame(pressed: &[Action], dt: f32, held: &[Action]) -> FrameInput {
        FrameInput::new(pressed.to_vec(), dt, held.to_vec())
    }

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn the_sizes_are_the_windows_and_the_buffers_pixels() {
        assert_eq!(window_px(), (384, 224));
        let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        assert_eq!(pixel_size(&GlyphBuffer::new(3, 2, blank)), (24, 32));
        // The text box overlaps the window's corner and sticks out of it.
        let overlap = WINDOW.intersect(&TEXT_BOX).unwrap();
        assert_eq!(overlap, Rect::new(62, 15, 12, 3));
    }

    #[test]
    fn the_scene_is_the_whole_test_map_as_the_skin_paints_it() {
        let mut ctx = ctx();
        let screen = SceneCameraScreen::new(&ctx);
        assert_eq!(screen.name(), "scene_camera");
        // 14 × 8 tiles of two cells by one.
        let scene = screen.scene.as_deref().unwrap();
        assert_eq!((scene.width(), scene.height()), (28, 8));
        assert_ne!(scene.get(0, 0).unwrap().glyph, ' ');
        assert!(scene.backdrop().is_none());
        // The path and the cursor are rectangles on it.
        assert!(!scene.overlays().is_empty());
        assert!(near(screen.centre, (112.0, 64.0)));
        assert_eq!(screen.zoom(), 1);
        // The test tileset's tiles are 24 px: 336 × 192, in whole cells.
        ctx.map_skin = skin_named(&ctx.content, "sprite").unwrap();
        let screen = SceneCameraScreen::new(&ctx);
        let scene = screen.scene.as_deref().unwrap();
        assert_eq!((scene.width(), scene.height()), (42, 12));
        assert!(!scene.sprites().is_empty());
        assert!(near(screen.centre, (168.0, 96.0)));
    }

    #[test]
    fn the_scene_shows_only_through_the_windows_see_through_cells() {
        let ctx = ctx();
        let screen = SceneCameraScreen::new(&ctx);
        let buf = draw(&ctx, &screen);
        let backdrop = buf.backdrop().unwrap();
        assert_eq!(backdrop.clip(), WINDOW);
        assert_eq!(backdrop.zoom(), 1);
        // A scene smaller than the window is centred in it.
        assert!(near(backdrop.origin_px(), (-80.0, -48.0)));
        assert_eq!(backdrop.scene(), screen.scene.as_deref().unwrap());
        for y in 0..i32::from(CONSOLE_H) {
            for x in 0..i32::from(CONSOLE_W) {
                let hole = WINDOW.contains(x, y) && !TEXT_BOX.contains(x, y);
                assert_eq!(buf.get(x, y).unwrap().see_through, hole, "({x}, {y})");
            }
        }
        // The caption's glyphs are over the scene; the box's cells are
        // solid, with their own background.
        let hi = ctx.palette.get(UiColor::TextHighlight);
        assert_eq!(buf.get(27, 4), Some(&Cell::see_through('T', hi)));
        let text = ctx.palette.get(UiColor::Text);
        let panel_bg = ctx.palette.get(UiColor::PanelBg);
        assert_eq!(buf.get(64, 16), Some(&Cell::new('A', text, panel_bg)));
        let row: String = (LEFT..40).map(|x| buf.get(x, 2).unwrap().glyph).collect();
        assert_eq!(row.trim(), "zoom 1x   window starts at -80.0,-48.0");
    }

    #[test]
    fn confirm_steps_the_zoom_through_the_whole_steps_and_round() {
        let mut ctx = ctx();
        let mut screen = SceneCameraScreen::new(&ctx);
        let mut zooms = Vec::new();
        for _ in 0..5 {
            let stay = screen.update(&mut ctx, &frame(&[Action::Confirm], 0.0, &[]));
            assert!(matches!(stay, Transition::None));
            zooms.push(screen.zoom());
            assert_eq!(
                draw(&ctx, &screen).backdrop().unwrap().zoom(),
                screen.zoom()
            );
        }
        assert_eq!(zooms, [2, 3, 4, 1, 2]);
        let sounds = ctx.audio.take();
        let cues: Vec<_> = sounds.iter().map(|s| s.cue()).collect();
        assert_eq!(cues, [Some("menu_select"); 5]);
        // Two presses in one frame are two steps.
        screen.update(
            &mut ctx,
            &frame(&[Action::Confirm, Action::Confirm], 0.0, &[]),
        );
        assert_eq!(screen.zoom(), 4);
    }

    #[test]
    fn held_cursor_keys_pan_by_the_pixel_and_stop_at_the_edges() {
        use Action::{CursorDown, CursorLeft, CursorRight, CursorUp};
        let mut ctx = ctx();
        let mut screen = SceneCameraScreen::new(&ctx);
        // At 1× the scene is smaller than the window: nothing to pan.
        screen.update(&mut ctx, &frame(&[], 1.0, &[CursorRight, CursorDown]));
        assert!(near(screen.origin_px(), (-80.0, -48.0)));
        // At 4× the window shows 96 × 56 of 224 × 128, from the middle.
        screen.zoom = 4;
        screen.pan(0.0, 0.0);
        assert!(near(screen.origin_px(), (64.0, 36.0)));
        // 60 px a second: a tenth of a second is 6 px, half a frame less.
        screen.update(&mut ctx, &frame(&[], 0.1, &[CursorRight]));
        assert!(near(screen.origin_px(), (70.0, 36.0)));
        screen.update(&mut ctx, &frame(&[], 0.0125, &[CursorDown]));
        assert!(near(screen.origin_px(), (70.0, 36.75)));
        screen.update(&mut ctx, &frame(&[], 0.1, &[CursorLeft, CursorUp]));
        assert!(near(screen.origin_px(), (64.0, 30.75)));
        // Opposite keys cancel; a pressed key that isn't held does nothing.
        screen.update(
            &mut ctx,
            &frame(&[CursorRight], 0.1, &[CursorLeft, CursorRight]),
        );
        assert!(near(screen.origin_px(), (64.0, 30.75)));
        // Not held: no pan, however long the frame.
        screen.update(&mut ctx, &frame(&[], 5.0, &[]));
        assert!(near(screen.origin_px(), (64.0, 30.75)));
        // Each edge stops the window, and the centre with it: one step
        // back moves at once.
        screen.update(&mut ctx, &frame(&[], 60.0, &[CursorRight, CursorDown]));
        assert!(near(screen.origin_px(), (128.0, 72.0)));
        assert!(near(screen.centre, (176.0, 100.0)));
        screen.update(&mut ctx, &frame(&[], 0.1, &[CursorLeft, CursorUp]));
        assert!(near(screen.origin_px(), (122.0, 66.0)));
        screen.update(&mut ctx, &frame(&[], 60.0, &[CursorLeft, CursorUp]));
        assert!(near(screen.origin_px(), (0.0, 0.0)));
        assert!(near(screen.centre, (48.0, 28.0)));
        // A bad frame time moves nothing.
        for dt in [f32::NAN, f32::INFINITY, -1.0] {
            screen.update(&mut ctx, &frame(&[], dt, &[CursorRight, CursorDown]));
            assert!(near(screen.origin_px(), (0.0, 0.0)), "{dt}");
        }
        // The frame shows where the window is.
        let buf = draw(&ctx, &screen);
        assert!(near(buf.backdrop().unwrap().origin_px(), (0.0, 0.0)));
    }

    #[test]
    fn zooming_keeps_the_point_where_the_window_can_show_it() {
        let mut ctx = ctx();
        let mut screen = SceneCameraScreen::new(&ctx);
        screen.zoom = 4;
        screen.pan(-20.0, 10.0);
        assert!(near(screen.centre, (92.0, 74.0)));
        // 4× → 1×: the whole scene fits, so the centre is its middle.
        screen.update(&mut ctx, &frame(&[Action::Confirm], 0.0, &[]));
        assert!(near(screen.origin_px(), (-80.0, -48.0)));
        assert!(near(screen.centre, (112.0, 64.0)));
        // → 2× → 3×: 128 × 74.67 shown, still from the middle.
        screen.update(&mut ctx, &frame(&[Action::Confirm; 2], 0.0, &[]));
        assert!(near(screen.centre, (112.0, 64.0)));
        // 3× → 4× near the top-left corner: the point is kept.
        screen.pan(-60.0, -20.0);
        assert!(near(screen.centre, (64.0, 44.0)));
        screen.update(&mut ctx, &frame(&[Action::Confirm], 0.0, &[]));
        assert!(near(screen.centre, (64.0, 44.0)));
        assert!(near(screen.origin_px(), (16.0, 16.0)));
        // At 3× the window's middle can't be nearer the edge than half of
        // what it shows: 64 and 37.33.
        screen.zoom = 3;
        screen.pan(-500.0, -500.0);
        assert!(near(screen.centre, (64.0, 112.0 / 3.0)));
        assert!(near(screen.origin_px(), (0.0, 0.0)));
    }

    #[test]
    fn cancel_closes_with_the_cancel_sound() {
        let mut ctx = ctx();
        let mut screen = SceneCameraScreen::new(&ctx);
        let pop = screen.update(
            &mut ctx,
            &frame(&[Action::Cancel, Action::Confirm], 0.0, &[]),
        );
        assert!(matches!(pop, Transition::Pop));
        assert_eq!(screen.zoom(), 1);
        let sounds = ctx.audio.take();
        let cues: Vec<_> = sounds.iter().map(|s| s.cue()).collect();
        assert_eq!(cues, [Some("menu_cancel")]);
    }

    #[test]
    fn says_so_when_the_map_is_missing() {
        let mut ctx = ctx();
        ctx.content.maps.clear();
        let mut screen = SceneCameraScreen::new(&ctx);
        assert!(screen.scene.is_none());
        screen.update(
            &mut ctx,
            &frame(&[Action::Confirm], 0.5, &[Action::CursorRight]),
        );
        let buf = draw(&ctx, &screen);
        assert!(buf.backdrop().is_none());
        let row: String = (0..40).map(|x| buf.get(x, 2).unwrap().glyph).collect();
        assert_eq!(row.trim(), "No assets/maps/test_small.map");
        assert!(!buf.get(WINDOW.x, WINDOW.y).unwrap().see_through);
    }
}
