//! `cargo xtask frame-png`: renders a Harness frame to a PNG without a
//! window (ticket 0232, ADR-0038 rule 4), for looking at a skin, a picture
//! or a mockup. Never a gate: snapshots stay text (ADR-0007).
//!
//! [`Painter`] is a software copy of `app`'s `Renderer::draw`, pixel for
//! pixel at whole scales: the clear colour, cell backgrounds, `Under`
//! items in order, glyphs tinted with their fg, `Over` items in order.
//! Textures are sampled at the nearest pixel and blended over what is
//! beneath with their alpha, as the GPU does.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use trpg_content::font::{AtlasRect, FALLBACK_GLYPH};
use trpg_content::{FontAtlasDef, ImageId, ImageTable};
use trpg_ui::console::{CELL_H_PX, CELL_W_PX};
use trpg_ui::harness::Harness;
use trpg_ui::input::{Button, Chord, Layout};
use trpg_ui::{GlyphBuffer, Item, Layer, PxRect, Rgb, Sprite, UiColor};

use crate::font_atlas::{decode_png, encode_png};

pub const USAGE: &str = "\
usage: cargo xtask frame-png <out.png> [steps] [options]

Starts the game in the test Harness, runs the steps in the order given,
and writes the frame to <out.png> (800×512 times the scale; a relative
path is from the repo root).

steps (each may repeat):
  --keys \"F2 Up f\"   press and release each key chord in turn
  --pad \"South\"      press and release each controller button in turn
  --wait 0.5         let that many seconds pass

options:
  --layout right|left  the key layout picked before (default right)
  --scale N            whole-number scale, 1 to 8 (default 2)
  --web                start as the web build does (the title waits for a key)

Key and button names are the Harness's test-script names (Chord::parse,
Button::parse), as in crates/ui/tests: input to a test script, not game
code, so the never-hard-code-a-key rule doesn't apply to them.

With the private-assets feature the picture shows the bought art: never
commit it (ADR-0040).";

/// Largest `--scale`.
const MAX_SCALE: u32 = 8;

/// The colour of what is missing, as in `app` (macroquad's `MAGENTA`).
const MISSING_COLOR: Rgb = Rgb::new(255, 0, 255);

/// One step of the script, run in the order given.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `Harness::keys`.
    Keys(String),
    /// `Harness::pad`.
    Pad(String),
    /// `Harness::wait`, in seconds.
    Wait(f32),
}

/// The command's arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Where the PNG goes.
    pub out: PathBuf,
    /// What to do before taking the frame.
    pub steps: Vec<Step>,
    /// The layout picked before.
    pub layout: Layout,
    /// Pixels per console pixel.
    pub scale: u32,
    /// Start as the web build does.
    pub web: bool,
}

/// Parses `frame-png`'s arguments, checking key and button names up front
/// so a typo is an error, not a Harness panic.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut out = None;
    let mut options = Options {
        out: PathBuf::new(),
        steps: Vec::new(),
        layout: Layout::RightHanded,
        scale: 2,
        web: false,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        match arg.as_str() {
            "--keys" => {
                let script = value()?;
                for token in script.split_whitespace() {
                    Chord::parse(token).map_err(|e| format!("--keys: {e}"))?;
                }
                options.steps.push(Step::Keys(script));
            }
            "--pad" => {
                let script = value()?;
                for token in script.split_whitespace() {
                    Button::parse(token).map_err(|e| format!("--pad: {e}"))?;
                }
                options.steps.push(Step::Pad(script));
            }
            "--wait" => {
                let text = value()?;
                let seconds = text
                    .parse::<f32>()
                    .ok()
                    .filter(|s| s.is_finite() && *s >= 0.0)
                    .ok_or_else(|| format!("--wait: not a number of seconds: {text}"))?;
                options.steps.push(Step::Wait(seconds));
            }
            "--layout" => {
                options.layout = match value()?.as_str() {
                    "right" => Layout::RightHanded,
                    "left" => Layout::LeftHanded,
                    other => return Err(format!("--layout: right or left, not {other}")),
                };
            }
            "--scale" => {
                let text = value()?;
                options.scale = text
                    .parse()
                    .ok()
                    .filter(|s| (1..=MAX_SCALE).contains(s))
                    .ok_or_else(|| {
                        format!("--scale: a whole number 1 to {MAX_SCALE}, not {text}")
                    })?;
            }
            "--web" => options.web = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option: {flag}")),
            path if out.is_none() => out = Some(PathBuf::from(path)),
            extra => return Err(format!("one output path only, not also {extra}")),
        }
    }
    options.out = out.ok_or("missing <out.png>")?;
    Ok(options)
}

/// Runs the command: plays the steps in a Harness, writes the frame under
/// `root` (if `out` is relative) and returns what to print.
pub fn run(root: &Path, options: &Options) -> Result<String, String> {
    let mut h = if options.web {
        Harness::on_web_with_layout(options.layout)
    } else {
        Harness::with_layout(options.layout)
    };
    for step in &options.steps {
        match step {
            Step::Keys(script) => h.keys(script),
            Step::Pad(script) => h.pad(script),
            Step::Wait(seconds) => h.wait(*seconds),
        };
    }
    let ctx = h.game().ctx();
    let painter = Painter::new(
        ctx.content.font.clone(),
        trpg_content::bundle::bytes(trpg_content::font::ATLAS_PNG_PATH).unwrap_or_default(),
        &ctx.content.images,
        trpg_content::bundle::bytes,
        ctx.palette.get(UiColor::Black),
    )?;
    let frame = painter.render(h.game().buffer(), options.scale);
    let png = encode_png(frame.width, frame.height, &frame.rgba)?;
    let path = root.join(&options.out);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    fs::write(&path, png).map_err(|e| format!("writing {}: {e}", path.display()))?;
    let shown = format!("{} → {}", h.top_screen(), options.out.display());
    Ok(summary(&shown, &frame, cfg!(feature = "private-assets")))
}

/// What `run` prints for frame `frame` of `shown` (screen → path), with a
/// warning if it was built with the bought art.
fn summary(shown: &str, frame: &Image, private_assets: bool) -> String {
    let mut out = format!("frame-png: {shown} ({}×{})", frame.width, frame.height);
    if private_assets {
        out.push_str(
            "
warning: built with private-assets: the picture shows bought art;              never commit it (ADR-0040)",
        );
    }
    out
}

/// An RGBA8 picture, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Four bytes per pixel.
    pub rgba: Vec<u8>,
}

impl Image {
    /// Decodes PNG file `png`.
    fn decode(png: &[u8]) -> Result<Self, String> {
        let (width, height, rgba) = decode_png(png)?;
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// The pixel at `(x, y)`, clamped to the picture (an empty picture is
    /// transparent).
    fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        if self.width == 0 || self.height == 0 {
            return [0; 4];
        }
        let clamp =
            |v: i64, size: u32| usize::try_from(v.clamp(0, i64::from(size) - 1)).unwrap_or(0);
        let i = (clamp(y, self.height) * self.width as usize + clamp(x, self.width)) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// The pixel at `(x, y)`, or `None` outside the picture.
    #[cfg(test)]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        (x < self.width && y < self.height).then(|| self.texel(i64::from(x), i64::from(y)))
    }

    /// Blends `color` with alpha `alpha` (0 to 1) over the pixel at
    /// `(x, y)`, as the GPU does: in floats, rounded once. Off the picture
    /// it does nothing.
    fn blend(&mut self, x: i64, y: i64, color: [f32; 3], alpha: f32) {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        for (c, &src) in color.iter().enumerate() {
            let dst = f32::from(self.rgba[i + c]) / 255.0;
            let out = src * alpha + dst * (1.0 - alpha);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=255
            let byte = (out * 255.0).round().clamp(0.0, 255.0) as u8;
            self.rgba[i + c] = byte;
        }
        self.rgba[i + 3] = 255;
    }

    /// Fills `(x, y, w, h)` with `color`, solid.
    fn fill(&mut self, (x, y, w, h): (i64, i64, i64, i64), color: Rgb) {
        for py in y..y + h {
            for px in x..x + w {
                self.blend(px, py, unit(color), 1.0);
            }
        }
    }
}

/// `c` as floats from 0 to 1.
fn unit(c: Rgb) -> [f32; 3] {
    [c.r, c.g, c.b].map(|v| f32::from(v) / 255.0)
}

/// The decoded font atlas and images, ready to render any number of
/// frames (each image is decoded once).
pub struct Painter {
    atlas: FontAtlasDef,
    atlas_image: Image,
    /// Background drawn behind the console; cells with this bg skip their
    /// fill, as in `app`.
    clear: Rgb,
    /// The images that decoded; the others draw as [`MISSING_COLOR`].
    images: HashMap<ImageId, Image>,
}

impl Painter {
    /// Decodes atlas image `atlas_png` and every image in `images`, whose
    /// files `read` returns (an image `read` lacks, or that doesn't decode,
    /// draws as a [`MISSING_COLOR`] rectangle). `clear` is the console
    /// background (the palette's `black`).
    pub fn new<'a>(
        atlas: FontAtlasDef,
        atlas_png: &[u8],
        images: &ImageTable,
        read: impl Fn(&str) -> Option<&'a [u8]>,
        clear: Rgb,
    ) -> Result<Self, String> {
        let atlas_image = Image::decode(atlas_png).map_err(|e| format!("font atlas image: {e}"))?;
        let images = images
            .ids()
            .filter_map(|id| Some((id, Image::decode(read(id.path())?).ok()?)))
            .collect();
        Ok(Self {
            atlas,
            atlas_image,
            clear,
            images,
        })
    }

    /// Draws `buf` with each console pixel `scale` × `scale` pixels big.
    pub fn render(&self, buf: &GlyphBuffer, scale: u32) -> Image {
        let s = i64::from(scale);
        let (cell_w, cell_h) = (i64::from(CELL_W_PX) * s, i64::from(CELL_H_PX) * s);
        let width = u32::try_from(i64::from(buf.width()) * cell_w).unwrap_or(0);
        let height = u32::try_from(i64::from(buf.height()) * cell_h).unwrap_or(0);
        let mut out = Image {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        };
        out.fill((0, 0, i64::from(width), i64::from(height)), self.clear);
        let cells = || {
            (0..i32::from(buf.height())).flat_map(move |y| {
                (0..i32::from(buf.width())).filter_map(move |x| {
                    let at = (i64::from(x) * cell_w, i64::from(y) * cell_h);
                    buf.get(x, y).map(|cell| (at, cell))
                })
            })
        };
        for ((x, y), cell) in cells() {
            if cell.bg != self.clear {
                out.fill((x, y, cell_w, cell_h), cell.bg);
            }
        }
        self.draw_items(&mut out, buf, Layer::Under, s);
        for ((x, y), cell) in cells() {
            if cell.glyph == ' ' {
                continue;
            }
            let Some((rect, fg)) = self.atlas_cell(cell.glyph, cell.fg) else {
                continue;
            };
            let fg = unit(fg);
            #[allow(clippy::cast_precision_loss)] // atlas coordinates are small
            let src = [rect.x, rect.y, rect.w, rect.h].map(|v| v as f32);
            blit(
                &mut out,
                &self.atlas_image,
                src,
                (x, y, cell_w, cell_h),
                false,
                [fg[0], fg[1], fg[2], 1.0],
            );
        }
        self.draw_items(&mut out, buf, Layer::Over, s);
        out
    }

    /// Draws `buf`'s items of `layer`, in order.
    fn draw_items(&self, out: &mut Image, buf: &GlyphBuffer, layer: Layer, scale: i64) {
        for item in buf.items().iter().filter(|item| item.layer() == layer) {
            let to_px = |r: PxRect| {
                (
                    i64::from(r.x) * scale,
                    i64::from(r.y) * scale,
                    i64::from(r.w) * scale,
                    i64::from(r.h) * scale,
                )
            };
            match item {
                Item::Rect(rect) => out.fill(to_px(rect.rect), rect.color),
                Item::Sprite(sprite) => self.draw_sprite(out, sprite, to_px(sprite.clip)),
            }
        }
    }

    /// Draws the visible part of `sprite` into `dest`: the matching part
    /// of its image, or a [`MISSING_COLOR`] rectangle.
    fn draw_sprite(&self, out: &mut Image, sprite: &Sprite, dest: (i64, i64, i64, i64)) {
        let Some(image) = self.images.get(&sprite.image) else {
            out.fill(dest, MISSING_COLOR);
            return;
        };
        let opacity = f32::from(sprite.opacity) / 255.0;
        let src = sprite.clipped_src();
        blit(
            out,
            image,
            src,
            dest,
            sprite.flip_x,
            [1.0, 1.0, 1.0, opacity],
        );
    }

    /// Where to find `glyph` in the atlas and what colour to tint it:
    /// `fg`, or the fallback glyph in [`MISSING_COLOR`].
    fn atlas_cell(&self, glyph: char, fg: Rgb) -> Option<(AtlasRect, Rgb)> {
        if let Some(rect) = self.atlas.glyph_rect(glyph) {
            return Some((rect, fg));
        }
        Some((self.atlas.glyph_rect(FALLBACK_GLYPH)?, MISSING_COLOR))
    }
}

/// Stretches the `src` part of `image` (`[x, y, w, h]` in image pixels)
/// over `dest` in `out`, mirrored if `flip_x`, each texel multiplied by
/// `tint` (RGBA, 0 to 1) and blended with the result's alpha. Each pixel
/// takes the texel under its centre, as nearest sampling does.
#[allow(clippy::cast_precision_loss)] // pixel counts are small
fn blit(
    out: &mut Image,
    image: &Image,
    src: [f32; 4],
    (left, top, width, height): (i64, i64, i64, i64),
    flip_x: bool,
    tint: [f32; 4],
) {
    let [src_x, src_y, src_w, src_h] = src;
    for dy in 0..height {
        let tex_y = src_y + (dy as f32 + 0.5) / height as f32 * src_h;
        for dx in 0..width {
            let across = (dx as f32 + 0.5) / width as f32;
            let tex_x = if flip_x {
                src_x + src_w * (1.0 - across)
            } else {
                src_x + src_w * across
            };
            #[allow(clippy::cast_possible_truncation)] // floored image coordinates
            let texel = image.texel(tex_x.floor() as i64, tex_y.floor() as i64);
            let [red, green, blue, alpha] = texel.map(|c| f32::from(c) / 255.0);
            let color = [red * tint[0], green * tint[1], blue * tint[2]];
            out.blend(left + dx, top + dy, color, alpha * tint[3]);
        }
    }
}

#[cfg(test)]
mod tests;
