//! Character portraits (`assets/portraits/*.portrait`, ADR-0018): a RON
//! header, then one `=== <expression>` block per expression, each a grid of
//! colour keys, one per pixel. The format is documented in
//! `assets/portraits/README.md`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::bundle;
use crate::error::ContentError;
use crate::map::{column_number, legend_position, line_number};
use crate::palette::PaletteDef;
use crate::ron_loader::parse_ron;

/// Directory of portrait files inside the asset bundle.
pub const PORTRAITS_DIR: &str = "portraits";
/// File extension of portrait files.
pub const PORTRAIT_EXTENSION: &str = ".portrait";
/// Start of a line that opens an expression block (`=== happy`).
pub const EXPRESSION_MARKER: &str = "===";
/// The pixel key for a transparent pixel; no colour may use it.
pub const TRANSPARENT: char = '.';
/// Portrait size in pixels, width × height (ADR-0018). Drawn as 32×16 cells.
pub const PORTRAIT_SIZE: (u16, u16) = (32, 32);
/// Expressions every portrait must have (ADR-0018).
pub const REQUIRED_EXPRESSIONS: [&str; 5] = ["neutral", "happy", "angry", "sad", "surprised"];

/// A validated portrait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Portrait {
    /// The character it shows (also the file stem).
    pub character: String,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Pixel key → palette colour name.
    pub colors: BTreeMap<char, String>,
    /// The expressions, in file order.
    pub expressions: Vec<Expression>,
}

/// One expression: a grid of pixel keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    /// Its name, e.g. `happy`.
    pub name: String,
    /// Row-major keys, `width × height` of them; `None` is transparent.
    pub pixels: Vec<Option<char>>,
}

impl Portrait {
    /// The expression called `name`.
    pub fn expression(&self, name: &str) -> Option<&Expression> {
        self.expressions.iter().find(|e| e.name == name)
    }

    /// The palette colour name of pixel `(x, y)` of `expression`; `None`
    /// when it is transparent or outside the portrait.
    pub fn color_at(&self, expression: &Expression, x: u16, y: u16) -> Option<&str> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = usize::from(y) * usize::from(self.width) + usize::from(x);
        let key = expression.pixels.get(i).copied().flatten()?;
        self.colors.get(&key).map(String::as_str)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    character: String,
    size: (u16, u16),
    colors: BTreeMap<char, String>,
}

/// Loads every `*.portrait` file in the bundle, keyed by file stem, checking
/// colour names against `palette`. Reports every error of every file.
pub fn load_all(palette: &PaletteDef) -> Result<BTreeMap<String, Portrait>, Vec<ContentError>> {
    let mut portraits = BTreeMap::new();
    let mut errors = Vec::new();
    for path in bundle::files_in(PORTRAITS_DIR) {
        let Some(stem) = path
            .strip_prefix(PORTRAITS_DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(PORTRAIT_EXTENSION))
        else {
            continue;
        };
        let file = bundle::display_path(path);
        let result = bundle::file(path)
            .ok_or_else(|| vec![ContentError::new(&file, "file is not valid UTF-8")])
            .and_then(|source| parse_portrait(&file, source, palette))
            .and_then(|p| check_stem(&file, stem, p));
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

/// The portrait, if its `character` is the file stem.
fn check_stem(file: &str, stem: &str, portrait: Portrait) -> Result<Portrait, Vec<ContentError>> {
    if portrait.character == stem {
        Ok(portrait)
    } else {
        Err(vec![ContentError::new(
            file,
            format!(
                "character \"{}\" doesn't match the file name; name the file {}{PORTRAIT_EXTENSION}",
                portrait.character, portrait.character
            ),
        )])
    }
}

/// Parses portrait `source` (errors attributed to `file`), checking colour
/// names against `palette`. Reports every problem with its line and column.
pub fn parse_portrait(
    file: &str,
    source: &str,
    palette: &PaletteDef,
) -> Result<Portrait, Vec<ContentError>> {
    let lines: Vec<&str> = source
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let Some(first) = lines.iter().position(|l| l.starts_with(EXPRESSION_MARKER)) else {
        return Err(vec![ContentError::new(
            file,
            format!("no expressions: each starts with a \"{EXPRESSION_MARKER} <name>\" line"),
        )]);
    };
    let header: Header = parse_ron(file, &lines[..first].join("\n")).map_err(|e| vec![e])?;

    let mut errors = Vec::new();
    if header.size != PORTRAIT_SIZE {
        let (w, h) = PORTRAIT_SIZE;
        let mut e = ContentError::new(
            file,
            format!(
                "size is ({}, {}); portraits are ({w}, {h}) pixels",
                header.size.0, header.size.1
            ),
        );
        if let Some(i) = lines[..first].iter().position(|l| l.contains("size")) {
            e = e.at(line_number(i), None);
        }
        errors.push(e);
    }
    for (&key, name) in &header.colors {
        let problem = if key == TRANSPARENT {
            format!("'{TRANSPARENT}' is reserved for transparent pixels")
        } else if palette.get(name).is_none() {
            format!("unknown palette colour \"{name}\"")
        } else {
            continue;
        };
        let (line, col) = legend_position(&lines[..first], key, name);
        errors.push(
            ContentError::new(file, format!("colour '{key}': {problem}")).at(line, Some(col)),
        );
    }

    let mut expressions: Vec<(Expression, u32)> = Vec::new();
    for (start, block) in blocks(&lines, first) {
        let line = line_number(start);
        let name = lines[start][EXPRESSION_MARKER.len()..].trim().to_owned();
        if name.is_empty() {
            errors.push(ContentError::new(file, "expression has no name").at(line, Some(1)));
        } else if let Some((_, first_line)) = expressions.iter().find(|(e, _)| e.name == name) {
            errors.push(
                ContentError::new(
                    file,
                    format!("expression \"{name}\" appears twice (first on line {first_line})"),
                )
                .at(line, Some(1)),
            );
        }
        let pixels = parse_grid(file, &header, start + 1, block, &mut errors);
        expressions.push((Expression { name, pixels }, line));
    }
    for required in REQUIRED_EXPRESSIONS {
        if !expressions.iter().any(|(e, _)| e.name == required) {
            errors.push(ContentError::new(
                file,
                format!("missing required expression \"{required}\""),
            ));
        }
    }

    if errors.is_empty() {
        Ok(Portrait {
            character: header.character,
            width: header.size.0,
            height: header.size.1,
            colors: header.colors,
            expressions: expressions.into_iter().map(|(e, _)| e).collect(),
        })
    } else {
        Err(errors)
    }
}

/// The expression blocks from line index `first` on: each block's marker
/// line index and its grid rows (without trailing blank lines).
fn blocks<'a>(lines: &'a [&'a str], first: usize) -> Vec<(usize, &'a [&'a str])> {
    let starts: Vec<usize> = (first..lines.len())
        .filter(|&i| lines[i].starts_with(EXPRESSION_MARKER))
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(n, &start)| {
            let end = starts.get(n + 1).copied().unwrap_or(lines.len());
            let mut rows = &lines[start + 1..end];
            while rows.last().is_some_and(|r| r.is_empty()) {
                rows = &rows[..rows.len() - 1];
            }
            (start, rows)
        })
        .collect()
}

/// Parses one expression's `rows` (the first on line index `first`) into
/// pixel keys, pushing every problem onto `errors`.
fn parse_grid(
    file: &str,
    header: &Header,
    first: usize,
    rows: &[&str],
    errors: &mut Vec<ContentError>,
) -> Vec<Option<char>> {
    let (w, h) = (usize::from(header.size.0), usize::from(header.size.1));
    if rows.len() != h {
        errors.push(
            ContentError::new(
                file,
                format!("expression has {} rows; expected {h}", rows.len()),
            )
            .at(line_number(first - 1), Some(1)),
        );
    }
    let mut pixels = Vec::with_capacity(w * h);
    for (i, row) in rows.iter().enumerate() {
        let line = line_number(first + i);
        let keys: Vec<char> = row.chars().collect();
        if keys.len() != w {
            errors.push(
                ContentError::new(
                    file,
                    format!("row is {} pixels wide; expected {w}", keys.len()),
                )
                .at(line, Some(column_number(keys.len().min(w)))),
            );
        }
        for (col, &key) in keys.iter().enumerate() {
            if key == TRANSPARENT {
                pixels.push(None);
            } else if header.colors.contains_key(&key) {
                pixels.push(Some(key));
            } else {
                errors.push(
                    ContentError::new(file, format!("'{key}' is not in the colours"))
                        .at(line, Some(column_number(col))),
                );
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests;
