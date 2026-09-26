//! Blits a [`GlyphBuffer`] to the window with the font atlas (ADR-0003):
//! integer-scaled, centred, black letterbox. Sub-cell overlays (ADR-0018)
//! are drawn as scaled rectangles: `Under` ones after the cell backgrounds,
//! `Over` ones after the glyphs.

use std::collections::HashSet;

use macroquad::prelude::*;
use trpg_content::FontAtlasDef;
use trpg_content::font::{AtlasRect, FALLBACK_GLYPH};
use trpg_ui::console::{CELL_H_PX, CELL_W_PX, layout};
use trpg_ui::{GlyphBuffer, Layer, Rgb};

/// Drawn for glyphs the atlas lacks, in [`MISSING_COLOR`].
const MISSING_COLOR: Color = MAGENTA;

/// Draws glyph buffers with a font atlas texture.
pub struct Renderer {
    texture: Texture2D,
    atlas: FontAtlasDef,
    /// Background drawn behind the console; cells with this bg skip their fill.
    clear: Rgb,
    /// Missing glyphs already logged, so each is reported once.
    warned: HashSet<char>,
}

impl Renderer {
    /// Uploads the atlas image (`png`). `clear` is the console background
    /// (the palette's `black`).
    pub fn new(atlas: FontAtlasDef, png: &[u8], clear: Rgb) -> Result<Self, String> {
        let image = Image::from_file_with_format(png, Some(ImageFormat::Png))
            .map_err(|e| format!("font atlas image: {e}"))?;
        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Nearest);
        Ok(Self {
            texture,
            atlas,
            clear,
            warned: HashSet::new(),
        })
    }

    /// Draws `buf` scaled to the current window size.
    ///
    /// The layout is computed in physical framebuffer pixels (the window is
    /// `high_dpi`), so the integer scale holds on scaled Windows displays;
    /// coordinates are divided by the DPI factor only because macroquad's
    /// default camera works in logical units.
    pub fn draw(&mut self, buf: &GlyphBuffer) {
        let dpi = screen_dpi_scale();
        // Rounded: logical size × DPI can land a hair under the real pixel
        // count (913.714 × 1.75 = 1599.99…), which would drop a whole scale step.
        let fit = layout(
            (screen_width() * dpi).round(),
            (screen_height() * dpi).round(),
        );
        #[allow(clippy::cast_precision_loss)] // scale is small
        let scale = fit.scale as f32 / dpi;
        let (offset_x, offset_y) = (fit.offset_x / dpi, fit.offset_y / dpi);
        let cell_w = f32::from(CELL_W_PX) * scale;
        let cell_h = f32::from(CELL_H_PX) * scale;
        clear_background(BLACK);
        draw_rectangle(
            offset_x,
            offset_y,
            f32::from(buf.width()) * cell_w,
            f32::from(buf.height()) * cell_h,
            color(self.clear),
        );
        let cells = || {
            (0..i32::from(buf.height())).flat_map(move |y| {
                (0..i32::from(buf.width())).filter_map(move |x| {
                    #[allow(clippy::cast_precision_loss)] // cell coords < 2^16
                    let at = (offset_x + x as f32 * cell_w, offset_y + y as f32 * cell_h);
                    buf.get(x, y).map(|cell| (at, cell))
                })
            })
        };
        for ((px, py), cell) in cells() {
            if cell.bg != self.clear {
                draw_rectangle(px, py, cell_w, cell_h, color(cell.bg));
            }
        }
        let overlays = |layer: Layer| {
            for o in buf.overlays().iter().filter(|o| o.layer == layer) {
                let r = o.rect;
                #[allow(clippy::cast_precision_loss)] // console pixels < 2^21
                draw_rectangle(
                    offset_x + r.x as f32 * scale,
                    offset_y + r.y as f32 * scale,
                    r.w as f32 * scale,
                    r.h as f32 * scale,
                    color(o.color),
                );
            }
        };
        overlays(Layer::Under);
        for ((px, py), cell) in cells() {
            if cell.glyph == ' ' {
                continue;
            }
            let Some((rect, fg)) = self.atlas_cell(cell.glyph, cell.fg) else {
                continue;
            };
            draw_texture_ex(
                &self.texture,
                px,
                py,
                fg,
                DrawTextureParams {
                    dest_size: Some(vec2(cell_w, cell_h)),
                    source: Some(source_rect(rect)),
                    ..Default::default()
                },
            );
        }
        overlays(Layer::Over);
    }

    /// Where to find `glyph` in the atlas and what colour to tint it: `fg`,
    /// or the fallback glyph in [`MISSING_COLOR`] (logged once per glyph).
    fn atlas_cell(&mut self, glyph: char, fg: Rgb) -> Option<(AtlasRect, Color)> {
        if let Some(rect) = self.atlas.glyph_rect(glyph) {
            return Some((rect, color(fg)));
        }
        if self.warned.insert(glyph) {
            warn!(
                "glyph {:?} (U+{:04X}) not in font atlas",
                glyph,
                u32::from(glyph)
            );
        }
        Some((self.atlas.glyph_rect(FALLBACK_GLYPH)?, MISSING_COLOR))
    }
}

fn color(c: Rgb) -> Color {
    Color::from_rgba(c.r, c.g, c.b, 255)
}

#[allow(clippy::cast_precision_loss)] // atlas coordinates are small
fn source_rect(r: AtlasRect) -> Rect {
    Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
}
