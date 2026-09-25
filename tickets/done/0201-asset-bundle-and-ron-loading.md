---
id: "0201"
title: Embedded asset bundle, RON loading with good errors, palette data
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0101"]
nick_input: none
completed: 2026-09-25
---

# 0201 — Embedded asset bundle, RON loading, palette data

## Context

[ADR-0005](../../docs/adr/0005-data-driven-content.md) says all content is data
under `assets/`, embedded in the binary, validated with precise error messages.
This ticket builds that machinery and loads the first asset: the colour
palette ([ADR-0012](../../docs/adr/0012-visual-style.md)). Every later content
type (terrain, classes, maps, dialogue…) plugs into the pattern set here, so get
the pattern clean.

## Nick input

None.

## Scope

**In:** `trpg-content` crate: embedded bundle, error types, RON loader,
`PaletteDef`, the `Content` aggregate, `assets/data/palette.ron`, an
all-assets test.

**Out:** any other content type; UI colour types (0202); hot reload.

## Implementation steps

1. Dependencies (workspace-level versions): `serde` (derive), `ron`,
   `include_dir`, `thiserror`; dev: `proptest`.
2. `crates/content/src/bundle.rs`: `static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../assets");`
   and `pub fn file(path: &str) -> Option<&'static str>` (UTF-8 files) and a
   function listing files under a directory.
3. `crates/content/src/error.rs`:
   - `ContentError { file: String, line: Option<u32>, column: Option<u32>, message: String }`
     with `Display` like `assets/data/palette.ron:12:5: unknown colour "plyer"`.
   - `ContentErrors(Vec<ContentError>)` with `Display` listing all.
4. `crates/content/src/ron_loader.rs`: `pub fn load_ron<T: DeserializeOwned>(path: &str) -> Result<T, ContentError>`
   mapping RON's `SpannedError` position to line/column. Missing file → error
   naming the path.
5. `assets/data/palette.ron`: a map of colour name → `"#RRGGBB"`. Include at
   least: `black, white, text, text_dim, text_highlight, panel_bg, panel_border,
   panel_border_focus, player, enemy, ally, neutral, move_range, attack_range,
   heal_range, danger_zone, cursor, hp_high, hp_mid, hp_low, exp_bar`
   plus a few terrain placeholders (`grass, forest, water, mountain, stone,
   road`). Pick a pleasant starting set; Nick tunes it in 0011.
6. `crates/content/src/palette.rs`: `PaletteDef { colors: BTreeMap<String, [u8; 3]> }`
   built from the file; `parse_hex("#1a2B3c") -> Result<[u8;3], String>`
   (accepts upper/lower case, requires `#` + 6 hex digits). A constant list
   `REQUIRED_COLORS` (the UI names above); missing ones are errors.
7. `crates/content/src/lib.rs`: `pub struct Content { pub palette: PaletteDef }`
   and `pub fn load_embedded() -> Result<Content, ContentErrors>` which
   **collects all errors** from all loaders before returning.
8. `crates/content/tests/all_assets_load.rs`: `load_embedded()` must be `Ok`.
9. Write `crates/content/README.md` (short): how to add a new content type
   (schema struct → loader → validation → add to `Content` → test).

## Acceptance criteria

- [x] `trpg_content::load_embedded()` succeeds on the repo's assets.
- [x] A malformed RON file yields an error with the correct file, line and column (unit test with an inline string).
- [x] Missing required colour and bad hex values each yield a clear error; multiple errors are reported together.
- [x] `crates/content/README.md` documents the pattern.
- [x] No I/O besides the embedded bundle; no macroquad dependency.

## Tests required

- Unit: hex parsing (valid, missing `#`, wrong length, non-hex), error formatting, line/col mapping, missing required colour.
- Property: for any `[u8;3]`, `parse_hex(format_hex(c)) == Ok(c)`.
- Content validation test (step 8).

## Completion notes

- `trpg-content` now has `bundle` (embedded `assets/` via `include_dir`),
  `error` (`ContentError` / `ContentErrors`), `ron_loader` (`load_ron` +
  `parse_ron` for inline strings), `palette` (`PaletteDef`, `parse_hex`,
  `format_hex`, `REQUIRED_COLORS`) and `Content` / `load_embedded()`.
- `assets/data/palette.ron` holds a starting palette: all required UI colours
  plus terrain placeholders (`grass, forest, water, mountain, stone, road`).
  Faction colours follow ADR-0012 (player blue, enemy red, ally green,
  neutral yellow). Nick tunes these in 0011.
- Deviations:
  - Added `crates/content/build.rs` (`rerun-if-changed=../../assets`) because
    `include_dir!` doesn't make cargo rebuild when an asset changes on stable.
  - Bad-hex errors carry the line of the offending key (found by text search,
    since RON's typed parse doesn't give per-value positions); missing-colour
    errors have no line (there's nothing to point at).
  - RON positions use the end of the error span: the start can point at the
    whitespace before the bad token.
- Dependencies added (workspace versions): `serde`, `ron`, `include_dir`,
  `thiserror`; dev `proptest`. All MIT and/or Apache-2.0.
- No follow-up tickets. Nothing to play yet.
