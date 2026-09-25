//! `cargo xtask font-atlas <font.bdf> <out-dir>`: turns a BDF bitmap font into
//! the game's font atlas (`atlas.png` + `atlas.ron`, see
//! `trpg_content::font`). Only glyphs in [`ATLAS_RANGES`] are kept.

use std::collections::BTreeMap;
use std::fs;
use std::io::BufWriter;
use std::path::Path;

use trpg_content::FontAtlasDef;

/// Unicode ranges copied into the atlas (whatever the font has of them).
pub const ATLAS_RANGES: &[(char, char)] = &[
    (' ', '~'),               // ASCII
    ('\u{a0}', '\u{ff}'),     // Latin-1 supplement
    ('\u{2c7}', '\u{2c7}'),   // ˇ caron
    ('\u{391}', '\u{3c9}'),   // Greek (CP437 has several)
    ('\u{2010}', '\u{206f}'), // General punctuation (• † …)
    ('\u{2190}', '\u{21ff}'), // Arrows
    ('\u{2200}', '\u{22ff}'), // Mathematical operators (≈ ≤ ≥ ∞)
    ('\u{2300}', '\u{23ff}'), // Miscellaneous technical (⌂ ⌐)
    ('\u{2500}', '\u{25ff}'), // Box drawing, blocks, geometric shapes
    ('\u{2600}', '\u{26ff}'), // Miscellaneous symbols (☺ ♣ ♪)
];

/// Atlas cells per row.
pub const COLUMNS: u32 = 32;

/// A bitmap font: fixed cell size, each glyph an on/off pixel grid placed
/// inside the cell (row-major, `cell_w × cell_h`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitmapFont {
    /// Cell width in pixels.
    pub cell_w: u32,
    /// Cell height in pixels.
    pub cell_h: u32,
    /// Glyph pixels by character.
    pub glyphs: BTreeMap<char, Vec<bool>>,
}

/// Numbered, trimmed lines of a BDF file.
type Lines<'a> = dyn Iterator<Item = (usize, &'a str)> + 'a;

/// The font bounding box: width, height, x offset, y offset.
type BoundingBox = [i64; 4];

/// Parses a BDF font. Glyph bitmaps are positioned in the cell using the
/// font and glyph bounding boxes; pixels outside the cell are dropped.
pub fn parse_bdf(text: &str) -> Result<BitmapFont, String> {
    let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim()));
    let mut font_box: Option<BoundingBox> = None;
    let mut glyphs = BTreeMap::new();
    while let Some((n, line)) = lines.next() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("FONTBOUNDINGBOX") => font_box = Some(numbers(words, n)?),
            Some("STARTCHAR") => {
                let font_box =
                    font_box.ok_or(format!("line {n}: STARTCHAR before FONTBOUNDINGBOX"))?;
                if let (Some(c), pixels) = parse_char(&mut lines, font_box)? {
                    glyphs.insert(c, pixels);
                }
            }
            _ => {}
        }
    }
    let [w, h, _, _] = font_box.ok_or("no FONTBOUNDINGBOX")?;
    let size = |v: i64| u32::try_from(v).ok().filter(|&v| v > 0);
    let (Some(cell_w), Some(cell_h)) = (size(w), size(h)) else {
        return Err(format!("bad FONTBOUNDINGBOX size {w}×{h}"));
    };
    Ok(BitmapFont {
        cell_w,
        cell_h,
        glyphs,
    })
}

/// Parses one glyph, from after `STARTCHAR` through `ENDCHAR`. The char is
/// `None` for glyphs without a Unicode encoding.
fn parse_char(
    lines: &mut Lines<'_>,
    font_box: BoundingBox,
) -> Result<(Option<char>, Vec<bool>), String> {
    let [cell_w, cell_h, _, _] = font_box;
    let mut encoding = None;
    let mut glyph_box = None;
    let mut pixels = vec![false; to_usize(cell_w * cell_h)];
    while let Some((n, line)) = lines.next() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("ENCODING") => {
                let [e] = numbers(words, n)?;
                encoding = u32::try_from(e).ok().and_then(char::from_u32);
            }
            Some("BBX") => glyph_box = Some(numbers(words, n)?),
            Some("BITMAP") => {
                let glyph_box = glyph_box.ok_or(format!("line {n}: BITMAP before BBX"))?;
                parse_bitmap(lines, n, font_box, glyph_box, &mut pixels)?;
            }
            Some("ENDCHAR") => break,
            _ => {}
        }
    }
    Ok((encoding, pixels))
}

/// Reads a glyph's hex bitmap rows (after `BITMAP` on line `start`) and sets
/// the matching cell pixels.
fn parse_bitmap(
    lines: &mut Lines<'_>,
    start: usize,
    [cell_w, cell_h, font_x, font_y]: BoundingBox,
    [width, height, glyph_x, glyph_y]: BoundingBox,
    pixels: &mut [bool],
) -> Result<(), String> {
    // Rows from the cell top to the glyph top: cell ascent minus glyph top.
    let top = (cell_h + font_y) - (glyph_y + height);
    for row in 0..height {
        let (n, hex) = lines
            .next()
            .ok_or(format!("line {start}: bitmap ends early"))?;
        let bits =
            u128::from_str_radix(hex, 16).map_err(|e| format!("line {n}: bad bitmap row: {e}"))?;
        let row_bits = i64::try_from(hex.len() * 4).unwrap_or(0);
        for col in 0..width.min(row_bits) {
            let (x, y) = (glyph_x - font_x + col, top + row);
            let on = (bits >> (row_bits - 1 - col)) & 1 == 1;
            if on && (0..cell_w).contains(&x) && (0..cell_h).contains(&y) {
                pixels[to_usize(y * cell_w + x)] = true;
            }
        }
    }
    Ok(())
}

/// Parses exactly `N` integers from `words`.
fn numbers<'a, const N: usize>(
    words: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[i64; N], String> {
    let v: Vec<i64> = words
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("line {line}: {e}"))?;
    v.try_into()
        .map_err(|v: Vec<i64>| format!("line {line}: expected {N} numbers, got {}", v.len()))
}

/// Non-negative `i64` → `usize` (callers only pass in-range values).
fn to_usize(v: i64) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// The atlas: the glyph map, plus RGBA pixels (white where the glyph is
/// set, transparent elsewhere) of size `width × height`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// Layout and glyph map.
    pub def: FontAtlasDef,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// RGBA8 pixels, row-major.
    pub rgba: Vec<u8>,
}

/// Lays out every glyph of `font` in [`ATLAS_RANGES`], in codepoint order,
/// [`COLUMNS`] per row.
pub fn build_atlas(font: &BitmapFont) -> Atlas {
    let chosen: Vec<(&char, &Vec<bool>)> = font
        .glyphs
        .iter()
        .filter(|&(c, _)| ATLAS_RANGES.iter().any(|(lo, hi)| (lo..=hi).contains(&c)))
        .collect();
    let count = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
    let rows = count.div_ceil(COLUMNS).max(1);
    let def = FontAtlasDef {
        cell_w: font.cell_w,
        cell_h: font.cell_h,
        columns: COLUMNS,
        glyphs: chosen.iter().map(|&(&c, _)| c).zip(0..).collect(),
    };
    let (width, height) = (COLUMNS * font.cell_w, rows * font.cell_h);
    let mut rgba = vec![0u8; to_usize(i64::from(width) * i64::from(height) * 4)];
    for (index, (_, pixels)) in (0..).zip(&chosen) {
        let cell = def.cell_rect(index);
        for (i, _) in pixels.iter().enumerate().filter(|&(_, &on)| on) {
            let i = u32::try_from(i).unwrap_or(0);
            let (x, y) = (cell.x + i % font.cell_w, cell.y + i / font.cell_w);
            let at = to_usize((i64::from(y) * i64::from(width) + i64::from(x)) * 4);
            rgba[at..at + 4].copy_from_slice(&[255; 4]);
        }
    }
    Atlas {
        def,
        width,
        height,
        rgba,
    }
}

/// The `atlas.ron` text for `def`, with a generated-file header.
pub fn atlas_ron(def: &FontAtlasDef, source_name: &str) -> Result<String, String> {
    let body = ron::ser::to_string_pretty(def, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "// Generated by `cargo xtask font-atlas` from {source_name}. Do not edit;\n\
         // see assets/fonts/README.md. Maps each glyph to its cell in atlas.png.\n\
         {body}\n"
    ))
}

/// Encodes RGBA8 pixels as PNG.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(BufWriter::new(&mut out), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Best);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Decodes a PNG to (width, height, RGBA8 pixels).
#[cfg(test)]
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    Ok((info.width, info.height, buf))
}

/// Runs the command: reads `font_path`, writes `atlas.png` and `atlas.ron`
/// into `out_dir`. Fails if the result lacks a required glyph.
pub fn run(font_path: &Path, out_dir: &Path) -> Result<String, String> {
    let text = fs::read_to_string(font_path)
        .map_err(|e| format!("reading {}: {e}", font_path.display()))?;
    let font = parse_bdf(&text).map_err(|e| format!("{}: {e}", font_path.display()))?;
    let atlas = build_atlas(&font);
    let problems = atlas.def.problems();
    if !problems.is_empty() {
        return Err(format!(
            "{}: unusable atlas:\n  {}",
            font_path.display(),
            problems.join("\n  ")
        ));
    }
    let name = font_path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let ron = atlas_ron(&atlas.def, &name)?;
    let png = encode_png(atlas.width, atlas.height, &atlas.rgba)?;
    fs::create_dir_all(out_dir).map_err(|e| format!("creating {}: {e}", out_dir.display()))?;
    for (file, bytes) in [("atlas.png", png.as_slice()), ("atlas.ron", ron.as_bytes())] {
        let path = out_dir.join(file);
        fs::write(&path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(format!(
        "font-atlas: {} glyphs, {}×{} px → {}",
        atlas.def.glyphs.len(),
        atlas.width,
        atlas.height,
        out_dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo_root;

    /// A 4×4 font (descent 1): 'A' is a 2×2 box at the baseline's left,
    /// 'b' a single pixel with an x offset, one unencoded glyph.
    const TINY: &str = "STARTFONT 2.1\n\
        FONTBOUNDINGBOX 4 4 0 -1\n\
        CHARS 3\n\
        STARTCHAR A\nENCODING 65\nBBX 2 2 0 0\nBITMAP\nC0\n80\nENDCHAR\n\
        STARTCHAR b\nENCODING 98\nBBX 1 1 2 -1\nBITMAP\n80\nENDCHAR\n\
        STARTCHAR none\nENCODING -1\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\n\
        ENDFONT\n";

    fn grid(font: &BitmapFont, c: char) -> Vec<String> {
        let px = &font.glyphs[&c];
        px.chunks(to_usize(i64::from(font.cell_w)))
            .map(|row| row.iter().map(|&on| if on { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn parses_and_positions_glyphs() {
        let font = parse_bdf(TINY).unwrap();
        assert_eq!((font.cell_w, font.cell_h), (4, 4));
        assert_eq!(font.glyphs.keys().collect::<String>(), "Ab");
        assert_eq!(grid(&font, 'A'), ["....", "##..", "#...", "...."]);
        assert_eq!(grid(&font, 'b'), ["....", "....", "....", "..#."]);
    }

    #[test]
    fn parse_errors() {
        assert!(
            parse_bdf("STARTCHAR A\n")
                .unwrap_err()
                .contains("before FONTBOUNDINGBOX")
        );
        assert_eq!(parse_bdf("").unwrap_err(), "no FONTBOUNDINGBOX");
        assert!(
            parse_bdf("FONTBOUNDINGBOX 0 4 0 0\n")
                .unwrap_err()
                .contains("bad FONTBOUNDINGBOX")
        );
        assert!(
            parse_bdf("FONTBOUNDINGBOX 4 4 0\n")
                .unwrap_err()
                .contains("expected 4 numbers, got 3")
        );
        assert!(
            parse_bdf("FONTBOUNDINGBOX 4 x 0 0\n")
                .unwrap_err()
                .starts_with("line 1: ")
        );
        let no_bbx = "FONTBOUNDINGBOX 4 4 0 0\nSTARTCHAR A\nBITMAP\n";
        assert!(
            parse_bdf(no_bbx)
                .unwrap_err()
                .contains("line 3: BITMAP before BBX")
        );
        let short = "FONTBOUNDINGBOX 4 4 0 0\nSTARTCHAR A\nBBX 1 2 0 0\nBITMAP\n80\n";
        assert!(parse_bdf(short).unwrap_err().contains("bitmap ends early"));
        let bad = "FONTBOUNDINGBOX 4 4 0 0\nSTARTCHAR A\nBBX 1 1 0 0\nBITMAP\nzz\n";
        assert!(parse_bdf(bad).unwrap_err().contains("bad bitmap row"));
    }

    #[test]
    fn atlas_layout_and_pixels() {
        let atlas = build_atlas(&parse_bdf(TINY).unwrap());
        assert_eq!(atlas.def.columns, COLUMNS);
        assert_eq!(atlas.def.glyphs, [('A', 0), ('b', 1)].into());
        assert_eq!((atlas.width, atlas.height), (COLUMNS * 4, 4));
        let lit =
            |x: u32, y: u32| atlas.rgba[to_usize(i64::from((y * atlas.width + x) * 4 + 3))] == 255;
        assert!(lit(0, 1) && lit(1, 1) && lit(0, 2));
        assert!(!lit(1, 2) && !lit(0, 0));
        assert!(lit(4 + 2, 3));
        let lit_count = atlas.rgba.chunks(4).filter(|p| p == &[255; 4]).count();
        assert_eq!(lit_count, 4);
        assert!(atlas.rgba.chunks(4).all(|p| p == [255; 4] || p == [0; 4]));
    }

    #[test]
    fn png_round_trips() {
        let atlas = build_atlas(&parse_bdf(TINY).unwrap());
        let png = encode_png(atlas.width, atlas.height, &atlas.rgba).unwrap();
        assert_eq!(
            decode_png(&png).unwrap(),
            (atlas.width, atlas.height, atlas.rgba)
        );
        assert!(decode_png(b"nope").is_err());
    }

    #[test]
    fn ron_round_trips_with_header() {
        let def = build_atlas(&parse_bdf(TINY).unwrap()).def;
        let text = atlas_ron(&def, "tiny.bdf").unwrap();
        assert!(text.starts_with("// Generated by `cargo xtask font-atlas` from tiny.bdf."));
        assert_eq!(ron::from_str::<FontAtlasDef>(&text).unwrap(), def);
    }

    #[test]
    fn run_rejects_fonts_missing_required_glyphs() {
        let dir = std::env::temp_dir().join(format!("xtask-font-atlas-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bdf = dir.join("tiny.bdf");
        fs::write(&bdf, TINY).unwrap();
        let err = run(&bdf, &dir.join("out")).unwrap_err();
        assert!(err.contains("missing required glyphs"), "{err}");
        assert!(!dir.join("out").exists());
        assert!(
            run(&dir.join("missing.bdf"), &dir)
                .unwrap_err()
                .starts_with("reading ")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The committed atlas must be exactly what the tool produces from the
    /// committed font source.
    #[test]
    fn committed_atlas_is_up_to_date() {
        let root = repo_root();
        let dir = std::env::temp_dir().join(format!("xtask-atlas-check-{}", std::process::id()));
        let msg = run(&root.join("assets-src/fonts/ter-u16n.bdf"), &dir).unwrap();
        assert!(msg.starts_with("font-atlas: "));
        let read = |p: &Path| fs::read_to_string(p).unwrap().replace("\r\n", "\n");
        assert_eq!(
            read(&dir.join("atlas.ron")),
            read(&root.join("assets/fonts/atlas.ron")),
            "assets/fonts/atlas.ron is stale: rerun `cargo xtask font-atlas`"
        );
        let png = |p: &Path| decode_png(&fs::read(p).unwrap()).unwrap();
        assert!(
            png(&dir.join("atlas.png")) == png(&root.join("assets/fonts/atlas.png")),
            "assets/fonts/atlas.png is stale: rerun `cargo xtask font-atlas`"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
