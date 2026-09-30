---
id: "0905"
title: "Rename the game to Visions of Shuyi: repo, binary, window, title screen, web page, release packages"
type: infra
milestone: M8 Release
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-09-29
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
- **Rename the GitHub repo** to `Zafnok/visions-of-shuyi` (Nick asked for it on
  2026-09-29, knowing the play link moves to
  `zafnok.github.io/visions-of-shuyi` and the old one stops working). Update the
  links that name the repo: `Cargo.toml` `repository`, README play and release
  links, CLAUDE.md.

**Out (do not do):**
- Don't rename the crates (`trpg-*`) or the SonarCloud project key.
- Don't change `LICENSE` (CLAUDE.md rule 6: Nick's call).
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

- [x] The title screen, window title and browser tab say "Visions of Shuyi".
- [x] The release workflow produces `visions-of-shuyi-*` packages (checked by
      reading the workflow; run it with `workflow_dispatch` if possible).
- [x] The Pages build still loads (the `.wasm` name matches the loader).
- [x] A save made before the rename still loads after it (test, or constants
      unchanged).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot: the title screen with the new name.
- Unit: if saves are migrated, a test that old-location data is found.

## Completion notes

Done on 2026-09-29 as soon as the title was decided, because Nick asked for the
repo, README and title screen to change at once. The GitHub repo rename was
added to the scope at Nick's request.

- The title screen, window title, browser tab and third-party-licenses page
  now say **Visions of Shuyi**. The line under the title, "an ASCII tactics
  game", stays until the title-screen art ticket (0811).
- The binary and `.wasm` are now `visions-of-shuyi`: `crates/app/Cargo.toml`,
  `xtask web`, `web/index.html`, `web/README.md` and the release workflow
  (packages are named `visions-of-shuyi-<version>-<platform>`, and their
  README.txt says "Visions of Shuyi"). Test-only example paths in `audio.rs`
  were updated too.
- **Saves are untouched:** `APP_DIR_NAME` and the web storage `PREFIX` still
  say `tactical-rpg`, so existing saves and settings keep loading. This is
  invisible to players.
- Repo links now point at `Zafnok/visions-of-shuyi` (`Cargo.toml`, README,
  CLAUDE.md). The SonarCloud badges keep the `Zafnok_tactical-rpg` project key,
  because renaming the GitHub repo doesn't rename the SonarCloud project.
- **Not changed, Nick's call:** `LICENSE` still names the game `tactical-rpg`
  (its first line, and the credit line "tactical-rpg by Nick Wentz"). Only
  Nick edits the license.
- Left alone: ADRs and done tickets (history), and the dev-only sound
  audition page.
- The acceptance item "release packages are named `visions-of-shuyi-*`" was
  checked by reading the workflow. It will be run for real at the next
  release.
- No Claude's starting rules: this ticket made no gameplay decisions.
