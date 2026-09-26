//! Battle map files (`assets/maps/*.map`): a RON header, a `---` line, then
//! one character per tile. The format is documented in
//! `assets/maps/README.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use trpg_core::{BattleMap, Grid, TerrainId};

use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::TerrainDisplayTable;

/// Directory of map files inside the asset bundle.
pub const MAPS_DIR: &str = "maps";
/// File extension of map files.
pub const MAP_EXTENSION: &str = ".map";
/// The line separating the RON header from the tile rows.
pub const SEPARATOR: &str = "---";
/// Largest allowed width and height, in tiles.
pub const MAX_MAP_SIZE: usize = 64;

/// A map's legend: tile character → terrain string id, plus the resolved
/// [`TerrainId`]s.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapLegend {
    /// Character → terrain string id, as written in the file.
    pub names: BTreeMap<char, String>,
    /// Character → resolved terrain.
    pub terrains: BTreeMap<char, TerrainId>,
}

impl MapLegend {
    /// Builds a legend from `char → terrain id` pairs, resolving each id in
    /// `display`; `None` if any id is unknown.
    pub fn resolve(names: BTreeMap<char, String>, display: &TerrainDisplayTable) -> Option<Self> {
        let terrains = names
            .iter()
            .map(|(&c, id)| Some((c, display.id_of(id)?)))
            .collect::<Option<_>>()?;
        Some(Self { names, terrains })
    }

    /// The first (lowest) character standing for terrain `id`.
    pub fn char_for(&self, id: TerrainId) -> Option<char> {
        self.terrains
            .iter()
            .find(|&(_, &t)| t == id)
            .map(|(&c, _)| c)
    }
}

/// A parsed map together with the legend it was written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDef {
    /// The map rules see.
    pub map: BattleMap,
    /// The legend from the file (for printing it back).
    pub legend: MapLegend,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    name: String,
    legend: BTreeMap<char, String>,
}

/// Loads every `*.map` file in the bundle, keyed by file stem
/// (`maps/test_small.map` → `"test_small"`). Reports every error of every
/// file.
pub fn load_all(
    display: &TerrainDisplayTable,
) -> Result<BTreeMap<String, MapDef>, Vec<ContentError>> {
    let mut maps = BTreeMap::new();
    let mut errors = Vec::new();
    for path in bundle::files_in(MAPS_DIR) {
        let Some(stem) = path
            .strip_prefix(MAPS_DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(MAP_EXTENSION))
        else {
            continue;
        };
        let file = bundle::display_path(path);
        let result = bundle::file(path)
            .ok_or_else(|| vec![ContentError::new(&file, "file is not valid UTF-8")])
            .and_then(|source| parse_map(&file, source, display));
        match result {
            Ok(def) => {
                maps.insert(stem.to_owned(), def);
            }
            Err(e) => errors.extend(e),
        }
    }
    if errors.is_empty() {
        Ok(maps)
    } else {
        Err(errors)
    }
}

/// Parses map `source` (errors attributed to `file`), resolving legend
/// terrain ids in `display`. Reports every problem with its line and column.
pub fn parse_map(
    file: &str,
    source: &str,
    display: &TerrainDisplayTable,
) -> Result<MapDef, Vec<ContentError>> {
    let lines: Vec<&str> = source
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let Some(sep) = lines.iter().position(|&l| l == SEPARATOR) else {
        return Err(vec![ContentError::new(
            file,
            format!("missing the \"{SEPARATOR}\" line between the header and the tiles"),
        )]);
    };
    let header_text = lines[..sep].join("\n");
    let header: Header = parse_ron(file, &header_text).map_err(|e| vec![e])?;

    let mut errors = Vec::new();
    let mut terrains = BTreeMap::new();
    for (&c, id) in &header.legend {
        if let Some(t) = display.id_of(id) {
            terrains.insert(c, t);
        } else {
            let (line, col) = legend_position(&lines[..sep], c, id);
            errors.push(
                ContentError::new(file, format!("legend '{c}': unknown terrain \"{id}\""))
                    .at(line, Some(col)),
            );
        }
    }

    let tiles = parse_rows(
        file,
        &lines[sep + 1..],
        sep + 1,
        &header.legend,
        &terrains,
        &mut errors,
    );
    match tiles {
        Some(tiles) if errors.is_empty() => Ok(MapDef {
            map: BattleMap {
                name: header.name,
                tiles,
            },
            legend: MapLegend {
                names: header.legend,
                terrains,
            },
        }),
        _ => Err(errors),
    }
}

/// Parses the tile `rows` (the first is on 0-based line index `first`) into
/// a grid, pushing every problem onto `errors`. `None` if the tiles can't
/// form a grid; a returned grid is only valid if no error was pushed.
fn parse_rows(
    file: &str,
    rows: &[&str],
    first: usize,
    legend: &BTreeMap<char, String>,
    terrains: &BTreeMap<char, TerrainId>,
    errors: &mut Vec<ContentError>,
) -> Option<Grid<TerrainId>> {
    let mut rows: Vec<(u32, Vec<char>)> = rows
        .iter()
        .enumerate()
        .map(|(i, l)| (line_number(first + i), l.chars().collect()))
        .collect();
    // Blank lines at the end are ignored.
    while rows.last().is_some_and(|(_, r)| r.is_empty()) {
        rows.pop();
    }
    let first_line = line_number(first);
    let Some((_, first_row)) = rows.first() else {
        errors.push(ContentError::new(file, "map has no tiles").at(first_line, Some(1)));
        return None;
    };
    let width = first_row.len();
    if width == 0 {
        errors.push(ContentError::new(file, "first tile row is empty").at(first_line, Some(1)));
        return None;
    }
    if width > MAX_MAP_SIZE {
        errors.push(
            ContentError::new(
                file,
                format!("map is {width} tiles wide; the maximum is {MAX_MAP_SIZE}"),
            )
            .at(first_line, Some(column_number(MAX_MAP_SIZE))),
        );
    }
    if rows.len() > MAX_MAP_SIZE {
        errors.push(
            ContentError::new(
                file,
                format!(
                    "map is {} tiles tall; the maximum is {MAX_MAP_SIZE}",
                    rows.len()
                ),
            )
            .at(rows[MAX_MAP_SIZE].0, Some(1)),
        );
    }
    let mut cells = Vec::with_capacity(width * rows.len());
    for (line, row) in &rows {
        if row.len() != width {
            errors.push(
                ContentError::new(
                    file,
                    format!(
                        "row is {} tiles wide; expected {width} like the first row",
                        row.len()
                    ),
                )
                .at(*line, Some(column_number(row.len().min(width)))),
            );
        }
        for (i, &c) in row.iter().enumerate() {
            match terrains.get(&c) {
                Some(&t) => cells.push(t),
                // Its unknown terrain id was already reported.
                None if legend.contains_key(&c) => cells.push(TerrainId::default()),
                None => errors.push(
                    ContentError::new(file, format!("'{c}' is not in the legend"))
                        .at(*line, Some(column_number(i))),
                ),
            }
        }
    }
    let w = u16::try_from(width).ok()?;
    let h = u16::try_from(rows.len()).ok()?;
    Grid::from_cells(w, h, cells)
}

/// Prints `map` in `.map` format using `legend`. `None` if a tile's terrain
/// has no character in the legend.
pub fn print_map(map: &BattleMap, legend: &MapLegend) -> Option<String> {
    let header = Header {
        name: map.name.clone(),
        legend: legend.names.clone(),
    };
    let mut out = ron::to_string(&header).ok()?;
    out.push('\n');
    out.push_str(SEPARATOR);
    out.push('\n');
    let width = usize::from(map.tiles.width());
    for row in map.tiles.cells().chunks(width.max(1)) {
        for &t in row {
            out.push(legend.char_for(t)?);
        }
        out.push('\n');
    }
    Some(out)
}

/// 1-based line and column of legend entry `c` in the header lines: the
/// quoted character if found, else the terrain id string, else the start.
fn legend_position(header: &[&str], c: char, id: &str) -> (u32, u32) {
    let needles = [format!("'{c}'"), format!("\"{id}\"")];
    for needle in &needles {
        for (i, line) in header.iter().enumerate() {
            if let Some(byte) = line.find(needle.as_str()) {
                return (line_number(i), column_number(line[..byte].chars().count()));
            }
        }
    }
    (1, 1)
}

/// 1-based line number of 0-based line index `i`.
fn line_number(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(u32::MAX)
}

/// 1-based column number of 0-based char index `i`.
fn column_number(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_core::Pos;

    use super::*;
    use crate::terrain::TerrainDisplay;

    fn display() -> TerrainDisplayTable {
        let t = |id: &str| TerrainDisplay {
            id: id.to_owned(),
            glyphs: ['.', '.'],
            fg: "grass".into(),
            bg: "black".into(),
        };
        TerrainDisplayTable {
            terrains: vec![t("plain"), t("forest"), t("water"), t("wall")],
        }
    }

    const HEADER: &str = "(\n  name: \"Test\",\n  legend: { '.': \"plain\", 'T': \"forest\", '~': \"water\" },\n)\n---\n";

    fn parse(src: &str) -> Result<MapDef, Vec<ContentError>> {
        parse_map("m.map", src, &display())
    }

    fn errors(src: &str) -> Vec<String> {
        parse(src)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn parses_valid_map() {
        let def = parse(&format!("{HEADER}.T~\n~T.\n\n")).ok();
        let map = def.as_ref().map(|d| &d.map);
        assert_eq!(map.map(|m| m.name.as_str()), Some("Test"));
        assert_eq!(
            map.map(|m| (m.tiles.width(), m.tiles.height())),
            Some((3, 2))
        );
        let at = |x, y| map.and_then(|m| m.tiles.get(Pos::new(x, y)).copied());
        assert_eq!(at(0, 0), Some(TerrainId(0)));
        assert_eq!(at(1, 0), Some(TerrainId(1)));
        assert_eq!(at(2, 0), Some(TerrainId(2)));
        assert_eq!(at(0, 1), Some(TerrainId(2)));
        let legend = def.map(|d| d.legend).unwrap_or_default();
        assert_eq!(legend.terrains.get(&'T'), Some(&TerrainId(1)));
        assert_eq!(legend.names.get(&'~').map(String::as_str), Some("water"));
    }

    #[test]
    fn crlf_line_endings_are_accepted() {
        let src = format!("{HEADER}.T\n~.\n").replace('\n', "\r\n");
        let map = parse(&src).map(|d| d.map);
        assert_eq!(map.map(|m| (m.tiles.width(), m.tiles.height())), Ok((2, 2)));
    }

    #[test]
    fn missing_separator() {
        assert_eq!(
            errors("(name: \"x\", legend: {})\n..\n"),
            ["m.map: missing the \"---\" line between the header and the tiles"]
        );
    }

    #[test]
    fn header_syntax_error_is_positioned() {
        let errs = parse("(\n  name: \"x\"\n  legend: {},\n)\n---\n..\n")
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(3));
    }

    #[test]
    fn unknown_legend_char() {
        assert_eq!(
            errors(&format!("{HEADER}..\n.x\nx.\n")),
            [
                "m.map:7:2: 'x' is not in the legend",
                "m.map:8:1: 'x' is not in the legend"
            ]
        );
    }

    #[test]
    fn ragged_rows() {
        assert_eq!(
            errors(&format!("{HEADER}...\n..\n....\n\n...\n")),
            [
                "m.map:7:3: row is 2 tiles wide; expected 3 like the first row",
                "m.map:8:4: row is 4 tiles wide; expected 3 like the first row",
                "m.map:9:1: row is 0 tiles wide; expected 3 like the first row",
            ]
        );
    }

    #[test]
    fn unknown_terrain_in_legend() {
        let src = "(\n  name: \"x\",\n  legend: {\n    '.': \"plain\",\n    'L': \"lava\",\n  },\n)\n---\n.L\n";
        assert_eq!(
            errors(src),
            ["m.map:5:5: legend 'L': unknown terrain \"lava\""]
        );
    }

    #[test]
    fn legend_position_fallbacks() {
        let header = ["legend: { '\\'': \"lava\" }"];
        assert_eq!(legend_position(&header, '\'', "lava"), (1, 17));
        assert_eq!(legend_position(&["a", " 'q'"], 'q', "x"), (2, 2));
        assert_eq!(legend_position(&["a"], 'q', "x"), (1, 1));
        assert_eq!(legend_position(&["é'q'"], 'q', "x"), (1, 2));
    }

    #[test]
    fn empty_map() {
        assert_eq!(
            errors(&format!("{HEADER}\n\n")),
            ["m.map:6:1: map has no tiles"]
        );
        assert_eq!(errors(HEADER), ["m.map:6:1: map has no tiles"]);
        assert_eq!(
            errors(&format!("{HEADER}\n..\n")),
            ["m.map:6:1: first tile row is empty",]
        );
    }

    #[test]
    fn size_limits() {
        let wide = ".".repeat(MAX_MAP_SIZE + 1);
        assert_eq!(
            errors(&format!("{HEADER}{wide}\n")),
            ["m.map:6:65: map is 65 tiles wide; the maximum is 64"]
        );
        let tall = ".\n".repeat(MAX_MAP_SIZE + 1);
        assert_eq!(
            errors(&format!("{HEADER}{tall}")),
            ["m.map:70:1: map is 65 tiles tall; the maximum is 64"]
        );
        let max_row = ".".repeat(MAX_MAP_SIZE);
        let max = format!("{max_row}\n").repeat(MAX_MAP_SIZE);
        let map = parse(&format!("{HEADER}{max}")).map(|d| d.map);
        assert_eq!(
            map.map(|m| (m.tiles.width(), m.tiles.height())),
            Ok((64, 64))
        );
    }

    #[test]
    fn all_errors_reported_together() {
        let src = "(name: \"x\", legend: { '.': \"plain\", 'L': \"lava\" })\n---\n.L\n.q.\n";
        assert_eq!(errors(src).len(), 3);
    }

    #[test]
    fn print_round_trips_and_escapes_name() {
        let src = format!("{HEADER}.T~\n~T.\n").replace("\"Test\"", "\"Say \\\"hi\\\"\"");
        let def = parse(&src).ok();
        let printed = def.as_ref().and_then(|d| print_map(&d.map, &d.legend));
        let again = printed.as_deref().and_then(|p| parse(p).ok());
        assert!(def.is_some());
        assert_eq!(again, def);
        assert_eq!(def.map(|d| d.map.name), Some("Say \"hi\"".to_owned()));
        assert!(printed.is_some_and(|p| p.ends_with("\n---\n.T~\n~T.\n")));
    }

    #[test]
    fn print_needs_every_terrain_in_legend() {
        let legend = MapLegend::resolve(BTreeMap::from([('.', "plain".to_owned())]), &display())
            .unwrap_or_default();
        let tiles = Grid::from_cells(2, 1, vec![TerrainId(0), TerrainId(3)]);
        let map = tiles.map(|tiles| BattleMap {
            name: "m".into(),
            tiles,
        });
        assert_eq!(map.and_then(|m| print_map(&m, &legend)), None);
    }

    #[test]
    fn legend_resolve_and_char_for() {
        let names = BTreeMap::from([('b', "forest".to_owned()), ('a', "forest".to_owned())]);
        let legend = MapLegend::resolve(names, &display()).unwrap_or_default();
        assert_eq!(legend.char_for(TerrainId(1)), Some('a'));
        assert_eq!(legend.char_for(TerrainId(0)), None);
        let bad = BTreeMap::from([('a', "lava".to_owned())]);
        assert_eq!(MapLegend::resolve(bad, &display()), None);
    }

    #[test]
    fn embedded_maps_load() {
        let terrain = crate::terrain::TerrainDef::load(None).unwrap_or_default();
        let maps = load_all(&terrain.display);
        assert!(maps.is_ok(), "{maps:?}");
        let maps = maps.unwrap_or_default();
        let small = maps.get("test_small");
        assert!(small.is_some(), "{:?}", maps.keys());
        // test_small uses every legend entry.
        if let Some(def) = small {
            for (c, &t) in &def.legend.terrains {
                assert!(def.map.tiles.cells().contains(&t), "'{c}' unused");
            }
        }
    }

    const LEGEND_CHARS: &str = ".T~#^=F";

    fn arb_map() -> impl Strategy<Value = (BattleMap, MapLegend)> {
        (1u16..=12, 1u16..=12, 1usize..=4, "[ -~]{0,12}")
            .prop_flat_map(|(w, h, n, name)| {
                let cells = proptest::collection::vec(0..n, usize::from(w) * usize::from(h));
                (Just((w, h, n, name)), cells)
            })
            .prop_map(|((w, h, n, name), cells)| {
                let names: BTreeMap<char, String> = LEGEND_CHARS
                    .chars()
                    .zip(display().terrains)
                    .take(n)
                    .map(|(c, t)| (c, t.id))
                    .collect();
                let legend = MapLegend::resolve(names, &display()).unwrap_or_default();
                let cells = cells
                    .into_iter()
                    .map(|i| TerrainId(u16::try_from(i).unwrap_or(0)))
                    .collect();
                let tiles = Grid::from_cells(w, h, cells).unwrap_or_else(|| unreachable!());
                (BattleMap { name, tiles }, legend)
            })
    }

    proptest! {
        #[test]
        fn parse_print_round_trip((map, legend) in arb_map()) {
            let printed = print_map(&map, &legend);
            prop_assert!(printed.is_some());
            let parsed = parse(&printed.unwrap_or_default());
            prop_assert_eq!(parsed.as_ref().map(|d| &d.map), Ok(&map));
            prop_assert_eq!(parsed.map(|d| d.legend), Ok(legend));
        }
    }
}
