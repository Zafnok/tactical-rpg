//! Character portraits (ADR-0043): one sidecar file per character,
//! `assets/portraits/<id>.ron`, that maps expression names to PNG files.
//! `content` reads only each image's size, from the image table; `app`
//! decodes and draws it. The format is documented in
//! `assets/portraits/README.md`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::bundle;
use crate::error::ContentError;
use crate::image::{ImageId, ImageInfo, ImageTable};
use crate::map::line_number;
use crate::ron_loader::parse_ron;

/// Directory of portrait files inside the asset bundle. The image paths in
/// a sidecar are relative to it.
pub const PORTRAITS_DIR: &str = "portraits";
/// File extension of a portrait's sidecar file.
pub const SIDECAR_EXTENSION: &str = ".ron";
/// The space a portrait is drawn in, in console pixels, width × height:
/// 32×16 cells (ADR-0018). An image must fit it at a scale of at least 1.
pub const FRAME_PX: (u32, u32) = (256, 256);
/// Expressions every portrait must have (ADR-0018).
pub const REQUIRED_EXPRESSIONS: [&str; 5] = ["neutral", "happy", "angry", "sad", "surprised"];

/// Portraits by character id.
pub type PortraitTable = BTreeMap<String, Portrait>;

/// A validated portrait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Portrait {
    /// The character it shows (also the file stem).
    pub character: String,
    /// The expressions: the required ones in [`REQUIRED_EXPRESSIONS`]'
    /// order, then any others by name.
    pub expressions: Vec<Expression>,
}

/// One expression: an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    /// Its name, e.g. `happy`.
    pub name: String,
    /// The image that shows it.
    pub image: ImageId,
    /// That image's size in pixels; it fits [`FRAME_PX`].
    pub size: ImageInfo,
}

impl Portrait {
    /// The expression called `name`.
    pub fn expression(&self, name: &str) -> Option<&Expression> {
        self.expressions.iter().find(|e| e.name == name)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sidecar {
    character: String,
    expressions: BTreeMap<String, String>,
}

/// Loads every sidecar in the bundle's portraits directory, keyed by file
/// stem, checking each image it names against `images`. Reports every error
/// of every file.
pub fn load_all(images: &ImageTable) -> Result<PortraitTable, Vec<ContentError>> {
    let mut portraits = BTreeMap::new();
    let mut errors = Vec::new();
    for path in bundle::files_in(PORTRAITS_DIR) {
        let Some(stem) = path
            .strip_prefix(PORTRAITS_DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(SIDECAR_EXTENSION))
        else {
            continue;
        };
        let file = bundle::display_path(path);
        let result = bundle::file(path)
            .ok_or_else(|| vec![ContentError::new(&file, "file is not valid UTF-8")])
            .and_then(|source| parse_portrait(&file, stem, source, images));
        match result {
            Ok(p) => {
                portraits.insert(stem.to_owned(), p);
            }
            Err(e) => errors.extend(e),
        }
    }
    if errors.is_empty() {
        Ok(portraits)
    } else {
        Err(errors)
    }
}

/// Parses sidecar `source` of the file with stem `stem` (errors attributed
/// to `file`), checking each image against `images`. Reports every problem.
pub fn parse_portrait(
    file: &str,
    stem: &str,
    source: &str,
    images: &ImageTable,
) -> Result<Portrait, Vec<ContentError>> {
    let sidecar: Sidecar = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    if sidecar.character != stem {
        errors.push(ContentError::new(
            file,
            format!(
                "character \"{}\" doesn't match the file name; name the file {}{SIDECAR_EXTENSION}",
                sidecar.character, sidecar.character
            ),
        ));
    }

    let mut expressions = Vec::new();
    for (name, relative) in &sidecar.expressions {
        match check_image(relative, images) {
            Ok((image, size)) => expressions.push(Expression {
                name: name.clone(),
                image,
                size,
            }),
            Err(problem) => {
                let mut e = ContentError::new(file, format!("expression \"{name}\": {problem}"));
                let key = format!("\"{name}\"");
                if let Some(i) = source.lines().position(|l| l.contains(&key)) {
                    e = e.at(line_number(i), None);
                }
                errors.push(e);
            }
        }
    }
    for required in REQUIRED_EXPRESSIONS {
        if !sidecar.expressions.contains_key(required) {
            errors.push(ContentError::new(
                file,
                format!("missing required expression \"{required}\""),
            ));
        }
    }

    if errors.is_empty() {
        // Stable: the others stay in name order.
        expressions.sort_by_key(|e| {
            let required = REQUIRED_EXPRESSIONS.iter().position(|r| *r == e.name);
            required.unwrap_or(REQUIRED_EXPRESSIONS.len())
        });
        Ok(Portrait {
            character: sidecar.character,
            expressions,
        })
    } else {
        Err(errors)
    }
}

/// The image at `relative` (to the portraits directory) and its size, or
/// what is wrong with it in plain words: it must be in `images` and fit
/// [`FRAME_PX`].
fn check_image(relative: &str, images: &ImageTable) -> Result<(ImageId, ImageInfo), String> {
    let path = format!("{PORTRAITS_DIR}/{relative}");
    let shown = bundle::display_path(&path);
    let found = images.id(&path).and_then(|id| Some((id, images.info(id)?)));
    let (id, size) = found.ok_or_else(|| format!("{shown} is not a PNG image in the assets"))?;
    let (w, h) = FRAME_PX;
    if size.width > w || size.height > h {
        return Err(format!(
            "{shown} is {}×{} px; a portrait must fit {w}×{h} px",
            size.width, size.height
        ));
    }
    Ok((id, size))
}

#[cfg(test)]
mod tests;
