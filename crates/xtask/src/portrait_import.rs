//! `cargo xtask portrait-import`: cuts a character's bought busts into
//! portrait images in `assets-private/game/portraits/` (ticket 0711,
//! ADR-0043).
//!
//! A bought bust is 80×80. Dialogue draws its **middle 64 columns and
//! bottom 64 rows** at 4×, which fills the 256×256 px portrait frame
//! (`docs/design/look-and-feel.md`, *Dialogue portraits*). A 48×48 face is
//! written as it is (drawn at 5×).
//!
//! The output is bought art (ADR-0040): it goes to the private assets
//! checkout, never into this repository.

use std::fs;
use std::path::{Path, PathBuf};

use crate::font_atlas::{decode_png, encode_png};

/// Shown for `--help` and after a bad argument.
pub const USAGE: &str = "usage: cargo xtask portrait-import <bust-folder-or-sheet> <character-id> [--shift-x <pixels>]\n\n\
<bust-folder-or-sheet>  a folder of single bust files (bust_<expression>.png, 80×80 each),\n                        \
or one bust sheet (320×160: 4×2 busts). A folder without bust_ files\n                        \
gives its face_ files, or else all its PNGs; 48×48 faces and a\n                        \
192×96 face sheet are written as they are\n\
<character-id>          lower-case letters, digits and _ (the portrait's file name)\n\
--shift-x <pixels>      move the cut sideways, -8 to 8 (default 0: the middle 64 columns)\n\n\
Writes assets-private/game/portraits/<character-id>/<expression>.png (64×64) and, if it\n\
isn't there yet, the sidecar assets-private/game/portraits/<character-id>.ron to fill in.\n\
Run `cargo xtask private-assets --library` first.";

/// Side of a bought bust in pixels.
pub const BUST: u32 = 80;
/// Side of a cut bust, and of the cut, in pixels.
pub const CUT: u32 = 64;
/// Left edge of the cut with no shift: the middle [`CUT`] columns.
pub const CUT_X: u32 = (BUST - CUT) / 2;
/// Top edge of the cut: the bottom [`CUT`] rows.
pub const CUT_Y: u32 = BUST - CUT;
/// The furthest `--shift-x` may move the cut either way.
pub const MAX_SHIFT: i32 = 8;
/// Side of a bought face in pixels.
pub const FACE: u32 = 48;
/// Columns and rows of a sheet.
pub const SHEET: (u32, u32) = (4, 2);
/// The expressions of a sheet, row by row (checked on all 16 bought heroes'
/// sheets against their single files, 2026-10-02).
pub const SHEET_EXPRESSIONS: [&str; 8] = [
    "neutral", "smile", "sad", "sly", "thinking", "stern", "surprise", "unique",
];
/// The private assets' portraits directory, relative to the repo root.
pub const OUT_DIR: &str = "assets-private/game/portraits";
/// The expressions a sidecar must map (`trpg_content::portrait`), each with
/// the bought expressions the stub tries for it, best first.
const STUB_GUESSES: [(&str, &[&str]); 5] = [
    ("neutral", &["neutral"]),
    ("happy", &["smile"]),
    ("angry", &["stern"]),
    ("sad", &["sad"]),
    ("surprised", &["surprise", "surprised"]),
];

/// What the command was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The folder of bust files, or the sheet.
    pub source: PathBuf,
    /// The character id the portrait is for.
    pub character: String,
    /// How far the cut is moved to the right, in pixels.
    pub shift_x: i32,
}

/// Parses the command's arguments.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let (source, character, shift) = match args {
        [source, character] => (source, character, None),
        [source, character, flag, shift] if flag == "--shift-x" => (source, character, Some(shift)),
        _ => return Err("expected <bust-folder-or-sheet> <character-id>".to_owned()),
    };
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_';
    if character.is_empty() || !character.chars().all(valid) {
        return Err(format!(
            "character id \"{character}\" must be lower-case letters, digits and _"
        ));
    }
    let shift_x = match shift {
        None => 0,
        Some(text) => text
            .parse::<i32>()
            .ok()
            .filter(|s| (-MAX_SHIFT..=MAX_SHIFT).contains(s))
            .ok_or(format!(
                "--shift-x must be a whole number from -{MAX_SHIFT} to {MAX_SHIFT}, not \"{text}\""
            ))?,
    };
    Ok(Options {
        source: PathBuf::from(source),
        character: character.clone(),
        shift_x,
    })
}

/// An RGBA8 image, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Four bytes per pixel.
    pub rgba: Vec<u8>,
}

impl Picture {
    /// The `w × h` part whose top-left pixel is `(x, y)`. It must lie
    /// inside the picture.
    pub fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> Picture {
        let at = |v: u32| v as usize * 4;
        let rows = (y..y + h).flat_map(|row| {
            let start = at(row * self.width + x);
            self.rgba[start..start + at(w)].iter().copied()
        });
        Picture {
            width: w,
            height: h,
            rgba: rows.collect(),
        }
    }
}

/// The left edge of the cut moved by `shift_x` (within ±[`MAX_SHIFT`]).
fn cut_left(shift_x: i32) -> u32 {
    CUT_X.saturating_add_signed(shift_x).min(BUST - CUT)
}

/// The portrait image made from `picture` (named `name` in errors): a bust
/// is cut to its middle [`CUT`] columns, moved by `shift_x`, and bottom
/// [`CUT`] rows; a face is kept whole. Any other size is an error.
pub fn portrait_image(name: &str, picture: &Picture, shift_x: i32) -> Result<Picture, String> {
    match (picture.width, picture.height) {
        (BUST, BUST) => Ok(picture.crop(cut_left(shift_x), CUT_Y, CUT, CUT)),
        (FACE, FACE) => Ok(picture.clone()),
        (w, h) => Err(format!(
            "{name} is {w}×{h} px; a bust is {BUST}×{BUST} and a face {FACE}×{FACE}"
        )),
    }
}

/// The eight pictures of sheet `picture` (named `name` in errors) with
/// their expressions, or an error if it isn't a bust or a face sheet.
pub fn split_sheet(name: &str, picture: &Picture) -> Result<Vec<(String, Picture)>, String> {
    let (cols, rows) = SHEET;
    let side = [BUST, FACE]
        .into_iter()
        .find(|side| (picture.width, picture.height) == (side * cols, side * rows))
        .ok_or(format!(
            "{name} is {}×{} px; a bust sheet is {}×{} and a face sheet {}×{}",
            picture.width,
            picture.height,
            BUST * cols,
            BUST * rows,
            FACE * cols,
            FACE * rows
        ))?;
    let cells = (0..rows).flat_map(|row| (0..cols).map(move |col| (col, row)));
    Ok(SHEET_EXPRESSIONS
        .iter()
        .zip(cells)
        .map(|(expression, (col, row))| {
            let cell = picture.crop(col * side, row * side, side, side);
            ((*expression).to_owned(), cell)
        })
        .collect())
}

/// Reads the PNG at `path`.
fn read_picture(path: &Path) -> Result<Picture, String> {
    let bytes = fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let (width, height, rgba) =
        decode_png(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Picture {
        width,
        height,
        rgba,
    })
}

/// The lower-case stems of the files to import from folder listing
/// `stems`: the `bust_` ones (the sheet aside) if there are any, else the
/// `face_` ones, else all of them.
fn pick_files(stems: &[String]) -> Vec<&String> {
    for prefix in ["bust_", "face_"] {
        let picked: Vec<&String> = stems
            .iter()
            .filter(|s| s.starts_with(prefix) && !s.ends_with("_sheet"))
            .collect();
        if !picked.is_empty() {
            return picked;
        }
    }
    stems.iter().collect()
}

/// The expression a file with lower-case stem `stem` shows: what follows
/// its last `_` (`bust_neutral`, `bust_knight_neutral` → `neutral`).
fn expression_of(stem: &str) -> &str {
    stem.rsplit('_').next().unwrap_or(stem)
}

/// The pictures to import from `source` (a folder or a sheet), with their
/// expressions: a folder's in file-name order, a sheet's row by row.
fn read_source(source: &Path) -> Result<Vec<(String, Picture)>, String> {
    let shown = source.display();
    if source.is_file() {
        return split_sheet(&shown.to_string(), &read_picture(source)?);
    }
    let entries = fs::read_dir(source).map_err(|e| format!("reading {shown}: {e}"))?;
    let mut stems = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| format!("reading {shown}: {e}"))?.path();
        let png = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"));
        if let (true, Some(stem)) = (png, path.file_stem().and_then(|s| s.to_str())) {
            stems.push((stem.to_lowercase(), path.clone()));
        }
    }
    stems.sort();
    let names: Vec<String> = stems.iter().map(|(stem, _)| stem.clone()).collect();
    let picked = pick_files(&names);
    if picked.is_empty() {
        return Err(format!("{shown} has no PNG files"));
    }
    let mut pictures: Vec<(String, Picture)> = Vec::new();
    for (stem, path) in stems.iter().filter(|(stem, _)| picked.contains(&stem)) {
        let expression = expression_of(stem).to_owned();
        if pictures.iter().any(|(e, _)| *e == expression) {
            return Err(format!(
                "{} is a second file for expression \"{expression}\"",
                path.display()
            ));
        }
        pictures.push((expression, read_picture(path)?));
    }
    Ok(pictures)
}

/// A sidecar for `character` whose images are `expressions`: each of the
/// five expressions the game needs mapped to the closest bought one by
/// name (else the first), then every bought one under its own name.
pub fn sidecar_stub(character: &str, expressions: &[String]) -> String {
    let has = |name: &str| expressions.iter().any(|e| e == name);
    let mut entries: Vec<(String, String)> = Vec::new();
    for (required, guesses) in STUB_GUESSES {
        let guess = guesses.iter().copied().find(|g| has(g));
        let image = guess.or(expressions.first().map(String::as_str));
        entries.extend(image.map(|image| (required.to_owned(), image.to_owned())));
    }
    for expression in expressions {
        if !entries.iter().any(|(name, _)| name == expression) {
            entries.push((expression.clone(), expression.clone()));
        }
    }
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, image)| format!("        \"{name}\": \"{character}/{image}.png\",\n"))
        .collect();
    let lines = lines.concat();
    format!(
        "// Made by `cargo xtask portrait-import`. The first five are the expressions\n\
         // dialogue needs: point each at the closest bought one (ticket 0706).\n\
         (\n    character: \"{character}\",\n    expressions: {{\n{lines}    }},\n)\n"
    )
}

/// Runs the command under repo root `root`.
pub fn run(root: &Path, options: &Options) -> Result<String, String> {
    let Options {
        source,
        character,
        shift_x,
    } = options;
    if !root.join(OUT_DIR).parent().is_some_and(Path::is_dir) {
        return Err(
            "assets-private/game/ is missing: run `cargo xtask private-assets --library` first"
                .to_owned(),
        );
    }
    let pictures = read_source(source)?;
    // Every picture is checked before any file is written.
    let mut images = Vec::new();
    for (expression, picture) in &pictures {
        let name = format!("{} ({expression})", source.display());
        images.push((expression, portrait_image(&name, picture, *shift_x)?));
    }
    let dir = root.join(OUT_DIR).join(character);
    fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    for (expression, image) in &images {
        let path = dir.join(format!("{expression}.png"));
        let png = encode_png(image.width, image.height, &image.rgba)?;
        fs::write(&path, png).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    let names: Vec<String> = images.iter().map(|(e, _)| (*e).clone()).collect();
    let sidecar = root.join(OUT_DIR).join(format!("{character}.ron"));
    let sidecar_note = if sidecar.exists() {
        "kept the sidecar that was there"
    } else {
        fs::write(&sidecar, sidecar_stub(character, &names))
            .map_err(|e| format!("writing {}: {e}", sidecar.display()))?;
        "wrote a sidecar to fill in"
    };
    Ok(format!(
        "portrait-import: wrote {} images to {OUT_DIR}/{character}/ ({}); {sidecar_note}, {OUT_DIR}/{character}.ron",
        names.len(),
        names.join(", ")
    ))
}

#[cfg(test)]
mod tests;
