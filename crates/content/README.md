# trpg-content

Loads and validates all game content from the `assets/` directory, which is
embedded in the binary (ADR-0005). No filesystem I/O, no macroquad.

- `bundle` — the embedded `assets/` tree: `file(path)`, `files_in(dir)`.
- `error` — `ContentError` (`file:line:col: message`) and `ContentErrors`.
- `ron_loader` — `load_ron` / `parse_ron` with RON positions mapped to line/col.
- `palette` — `PaletteDef` from `assets/data/palette.ron`.
- `terrain` — `TerrainDef` from `assets/data/terrain.ron`: the rules half
  (`trpg_core::TerrainTable`) and the look half (`TerrainDisplayTable`).
- `map` — `.map` battle maps in `assets/maps/` (format in that folder's
  `README.md`): `parse_map`, `print_map`, `load_all`.
- `lib.rs` — `Content` (everything) and `load_embedded()`.

## Adding a new content type

1. **Asset**: add the file(s) under `assets/` (RON data goes in `assets/data/`).
2. **Schema**: a `serde::Deserialize` struct for the raw file shape, in its own
   module (`src/<type>.rs`). Keep balance numbers in the data, not in code.
3. **Loader**: `load_ron` / `parse_ron` for RON files (or a small hand-written
   parser for text formats), producing `ContentError`s with file + line/column.
   Split it into `load()` (reads from `bundle`) and `from_source(file, text)`
   so tests can use inline strings.
4. **Validation**: check every value and cross-reference (string ids must
   exist). Collect **all** problems into a `Vec<ContentError>`; never stop at
   the first one.
5. **Aggregate**: add a field to `Content` and call the loader from
   `load_embedded()`, extending the shared error list.
6. **Tests**: unit tests for parsing and each validation error (inline
   strings), property tests where there is a round trip, and make sure
   `tests/all_assets_load.rs` still passes on the real assets.
