---
id: "0905"
title: "Rename the game to Visions of Shuyi: binary, window, title screen, web page, release packages"
type: infra
milestone: M8 Release
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0905 — Rename the game to Visions of Shuyi

## Context

Ticket 0012 decided the title: **Visions of Shuyi** (short form VoS), no
subtitle ([`docs/design/title.md`](../../docs/design/title.md)). The code and
build still say `tactical-rpg` everywhere. The store pages (0901, 0903) need the
real name.

## Nick input

**Sign-off:** after merge, open the Pages build and check that the browser tab,
the title screen and the downloaded release names say "Visions of Shuyi".

## Scope

**In:**
- Player-visible name: `crates/ui/src/screens/title.rs` (`TITLE`), the window
  title in `crates/app/src/main.rs`, `<title>` in `web/index.html`.
- Binary name: `[[bin]] name` in `crates/app/Cargo.toml` (suggested
  `visions-of-shuyi`), `BIN_NAME` and package/zip names in
  `.github/workflows/release.yml`, the `.wasm` file loaded by `web/index.html`,
  and `web/README.md`, plus any other build or deploy workflow that names the
  binary or the `.wasm` file (`grep -rn "tactical-rpg" .github web crates`).
- Store/package metadata that already exists in the repo (e.g. Windows exe
  metadata if 0902 has landed).
- Update snapshot tests that show the title, and review them.

**Out (do not do):**
- Don't rename the GitHub repo, the crates (`trpg-*`), or the Pages URL.
- **Don't break existing saves or settings.** `APP_DIR_NAME` in
  `crates/app/src/storage/native.rs` and `PREFIX` in
  `crates/app/src/storage/web.rs` decide where saves and settings live. Either
  leave them as they are, or move old data to the new location on first run
  and test that. Don't just change the constants.
- No title-screen logo art: that is a separate ascii-art ticket if Nick wants
  one.

## Implementation steps

1. `grep -rn -i "tactical-rpg\|tactical rpg" --exclude-dir=target .` and list
   every hit. Sort each into: player-visible, build/package, or save location
   (leave it), or docs/history (leave it: ADRs and done tickets).
2. Change the player-visible strings to `Visions of Shuyi`.
3. Rename the binary and the `.wasm`, and update the release workflow, the web
   loader and the Pages deploy together so nothing points at a missing file.
4. Update and review the snapshots.

## Acceptance criteria

- [ ] The title screen, window title and browser tab say "Visions of Shuyi".
- [ ] The release workflow produces `visions-of-shuyi-*` packages (checked by
      reading the workflow; run it with `workflow_dispatch` if possible).
- [ ] The Pages build still loads (the `.wasm` name matches the loader).
- [ ] A save made before the rename still loads after it (test, or constants
      unchanged).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot: the title screen with the new name.
- Unit: if saves are migrated, a test that old-location data is found.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
