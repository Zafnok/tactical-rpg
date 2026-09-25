//! Size of the fixed logical console and how it fits the window (ADR-0012).

/// Console width in cells.
pub const CONSOLE_W: u16 = 100;
/// Console height in cells.
pub const CONSOLE_H: u16 = 32;
/// Cell width in logical pixels.
pub const CELL_W_PX: u16 = 8;
/// Cell height in logical pixels.
pub const CELL_H_PX: u16 = 16;

/// Where the console goes in a window: integer `scale` and the top-left
/// pixel offset that centres it. Everything around it is letterbox.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Layout {
    /// Pixels per logical pixel; never 0.
    pub scale: u32,
    /// Left edge of the console in window pixels (negative if it overflows).
    pub offset_x: f32,
    /// Top edge of the console in window pixels (negative if it overflows).
    pub offset_y: f32,
}

/// Fits the console into a `screen_w × screen_h` window: the largest integer
/// scale that fits (at least 1, so a too-small window crops the edges
/// evenly), centred on whole pixels so glyphs stay crisp.
pub fn layout(screen_w: f32, screen_h: f32) -> Layout {
    let console_w = f32::from(CONSOLE_W) * f32::from(CELL_W_PX);
    let console_h = f32::from(CONSOLE_H) * f32::from(CELL_H_PX);
    let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
    let (screen_w, screen_h) = (finite(screen_w), finite(screen_h));
    let fit = (screen_w / console_w).min(screen_h / console_h).floor();
    // `fit` is in 0..=f32::MAX / 800 here; `as` saturates anyway.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scale = (fit.max(1.0) as u32).max(1);
    #[allow(clippy::cast_precision_loss)] // scale ≤ 2^24 for any real window
    let s = scale as f32;
    Layout {
        scale,
        offset_x: ((screen_w - console_w * s) / 2.0).floor(),
        offset_y: ((screen_h - console_h * s) / 2.0).floor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(scale: u32, offset_x: f32, offset_y: f32) -> Layout {
        Layout {
            scale,
            offset_x,
            offset_y,
        }
    }

    #[test]
    fn exact_fit() {
        assert_eq!(layout(800.0, 512.0), l(1, 0.0, 0.0));
        assert_eq!(layout(1600.0, 1024.0), l(2, 0.0, 0.0));
        assert_eq!(layout(2400.0, 1536.0), l(3, 0.0, 0.0));
    }

    #[test]
    fn oversize_window_letterboxes() {
        // 1080p: 2x (1600x1024), bars of 160 px left/right and 28 top/bottom.
        assert_eq!(layout(1920.0, 1080.0), l(2, 160.0, 28.0));
        // Height limits: 3000 wide would allow 3x, 1100 tall only 2x.
        assert_eq!(layout(3000.0, 1100.0), l(2, 700.0, 38.0));
        // Width limits.
        assert_eq!(layout(1700.0, 5000.0), l(2, 50.0, 1988.0));
        // Odd leftover pixels round down to a whole pixel.
        assert_eq!(layout(801.0, 513.0), l(1, 0.0, 0.0));
        assert_eq!(layout(803.0, 515.0), l(1, 1.0, 1.0));
    }

    #[test]
    fn undersize_window_keeps_scale_1_and_centres() {
        assert_eq!(layout(600.0, 412.0), l(1, -100.0, -50.0));
        assert_eq!(layout(799.0, 1024.0), l(1, -1.0, 256.0));
        assert_eq!(layout(0.0, 0.0), l(1, -400.0, -256.0));
        assert_eq!(layout(-5.0, -5.0).scale, 1);
    }

    #[test]
    fn non_finite_sizes_act_as_zero() {
        assert_eq!(layout(f32::NAN, f32::INFINITY), l(1, -400.0, -256.0));
        assert_eq!(layout(f32::NEG_INFINITY, 512.0), l(1, -400.0, 0.0));
    }
}
