//! Drawing portraits (ADR-0043): each is one sprite item, the expression's
//! image at the largest whole scale that fits the 32×16-cell frame, centred.

use trpg_content::{ImageInfo, Portrait};

use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{GlyphBuffer, Layer, PxRect, Rect, Sprite};

/// The space a portrait is drawn in, in cells: 256×256 console pixels
/// (`trpg_content::portrait::FRAME_PX`).
pub const FRAME_CELLS: (i32, i32) = (32, 16);

/// Where an image of `size` is drawn in `frame` (console pixels): at the
/// largest whole scale that fits, centred (an odd pixel left over goes to
/// the right or the bottom). `None` if it doesn't fit even at 1×, or has no
/// pixels.
pub fn fit_whole_scale(size: ImageInfo, frame: PxRect) -> Option<PxRect> {
    let (w, h) = (i64::from(size.width), i64::from(size.height));
    if w == 0 || h == 0 {
        return None;
    }
    let scale = (i64::from(frame.w) / w).min(i64::from(frame.h) / h);
    if scale < 1 {
        return None;
    }
    // Both are at most the frame's side, so they fit in i32.
    let (w, h) = (
        i32::try_from(w * scale).ok()?,
        i32::try_from(h * scale).ok()?,
    );
    Some(Rect::new(
        frame.x + (frame.w - w) / 2,
        frame.y + (frame.h - h) / 2,
        w,
        h,
    ))
}

/// The opacity of a portrait dimmed by `dim` (0 = full colour, 1 = gone):
/// the renderer fades every pixel toward what is behind it (ADR-0018).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255
pub fn dim_opacity(dim: f32) -> u8 {
    (255.0 * (1.0 - dim)).round().clamp(0.0, 255.0) as u8
}

/// Draws expression `expr` of `portrait` in the [`FRAME_CELLS`] area whose
/// top-left cell is `(x, y)`, as one sprite under the glyphs: a transparent
/// pixel shows the cells' background, so clear the area first. `dim` (0 =
/// full colour, 1 = gone) fades it toward that background; `mirror` flips
/// it left to right. Clipped to the buffer. Returns `false`, drawing
/// nothing, when the portrait has no such expression.
pub fn draw_portrait(
    buf: &mut GlyphBuffer,
    (x, y): (i32, i32),
    portrait: &Portrait,
    expr: &str,
    dim: f32,
    mirror: bool,
) -> bool {
    let Some(expression) = portrait.expression(expr) else {
        return false;
    };
    let frame = Rect::new(
        x.saturating_mul(i32::from(CELL_W_PX)),
        y.saturating_mul(i32::from(CELL_H_PX)),
        FRAME_CELLS.0 * i32::from(CELL_W_PX),
        FRAME_CELLS.1 * i32::from(CELL_H_PX),
    );
    let size = expression.size;
    let src = (i32::try_from(size.width), i32::try_from(size.height));
    if let (Some(dest), (Ok(w), Ok(h))) = (fit_whole_scale(size, frame), src) {
        buf.add_sprite(Sprite {
            flip_x: mirror,
            opacity: dim_opacity(dim),
            ..Sprite::new(expression.image, Rect::new(0, 0, w, h), dest, Layer::Under)
        });
    }
    true
}

#[cfg(test)]
mod tests;
