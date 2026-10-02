---
id: "0508"
title: "Record human playthroughs and calibrate the persona bots against them"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0506", "0802"]
nick_input: sign-off
completed:
---

# 0508 — Record human play and calibrate the bots

## Context

Last step of the automated playtesting bots. 0506's persona bots use
settings chosen from descriptions, not from real players. Studios that make
bots play "like humans" learn from human games: Maia Chess (McIlroy-Young et
al., 2020) was trained on human games in each rating band, and King's
Candy Crush bot learned from real players' moves (Gudmundsson et al., 2018).
We don't have that volume, but every battle already keeps its full command
list (`BattleHistory`, `crates/core/src/history.rs`), and replaying it
rebuilds the battle exactly (ADR-0007 layer 5). So Nick's own play, and
later testers', can be recorded at almost no cost and measured with the same
numbers as the bots (0505's `BattleMeasures`).

This ticket records play in the **Pages build only** (debug tools on,
ADR-0023). The shipped game records nothing.

## Nick input

**Sign-off:** Nick plays Chapter 1 on the Pages build (a few tries if he
likes), exports the recordings from the debug menu, and answers one
question: which of Casual / Normal / Hardcore he played as. He then reads
the comparison table. Sign-off never blocks the PR.

## Scope

**In:**
- Recording each finished or abandoned battle (win, loss, restart, quit) in
  debug-tools builds.
- Exporting recordings from the F2 debug menu (0211).
- `cargo xtask playtest-compare <folder>`: replays recordings, measures them,
  and prints them next to the persona bots on the same battle.
- One calibration pass on the persona settings.

**Out (do not do):**
- Recording in shipped builds, telemetry, or sending anything anywhere.
- Imitation learning (training on human moves); a follow-up ticket if 0507
  says go and there is enough data.
- Changing 0033's targets. Only the bots' skill settings move here.

## Implementation steps

1. **Record** (`trpg-ui`, battle screen): when a battle ends, is restarted or
   is left, and `ctx.debug_tools` is on, write a `PlayRecord` through the
   `Storage` trait (`crates/ui/src/storage.rs`) under key
   `playrec-<battle>-<n>` (keys must match `^[a-z0-9_-]{1,64}$`): battle id,
   game mode, the battle's starting seed, the surviving command list
   (`BattleHistory` after rewinds), rewinds used, restarts before this try,
   result, and play time in seconds (from the frame driver's clock, which
   only `app` owns; pass it in through the existing context, don't read a
   clock in `ui`). RON, with a `version` field. Document the format in
   `docs/playtesting.md`. A record holds **nothing about the look**
   (ADR-0038, rule 5): no cell or pixel coordinates, no map skin, no
   theme. It must replay the same whatever graphics the game has by then.
2. **Export** (F2 debug menu): "Export play records". Native: writes every
   `playrec-*` to a `playrecs/` folder next to the save folder and shows the
   path. Web: downloads one `.ron` bundle through the web shell (0206). File
   access stays in `trpg-app` (ADR-0004). Also "Delete play records".
3. **Compare**: `cargo xtask playtest-compare <folder> [--runs 100]`: for
   each record, rebuild the battle from its file and seed, replay the
   commands, fill `BattleMeasures` (check the replay reaches the recorded
   result; if not, report the record as broken, don't guess). Then run the
   three persona bots on the same battle and print one table: rows = Nick's
   tries and each bot; columns = result, turns, fallen, items, rewinds,
   restarts, time.
4. **Calibrate once**: with Nick's answer to "which type did you play as",
   adjust only that persona's search settings in `crates/bots/personas.ron`
   (budget, lookahead, temperature, scoring weights) until its median turns,
   fallen and items are within ±20% of Nick's, or record why that isn't
   reachable. Write before/after settings and tables in
   `docs/playtesting.md`. Other personas keep their settings unless Nick
   says the gap between types looked wrong.

## Acceptance criteria

- [ ] In a debug-tools build, finishing, restarting or leaving a battle
      writes a `playrec-*` entry; in a build without debug tools it doesn't
      (ui test with `MemoryStorage` for both).
- [ ] Integration test `a_recorded_battle_replays_to_its_result`: a scripted
      harness battle (with one rewind in it) is recorded, exported to
      `MemoryStorage`, and `playtest-compare`'s replay reaches the same
      result and measures.
- [ ] Export works on native (folder) and web (download); manual check noted
      in Completion notes.
- [ ] If the sprite map skin (0433) is done: the replay test also passes
      with the battle recorded under that skin.
- [ ] Nick's recordings and the comparison table are in Completion notes,
      with his answer to the one question and the calibration before/after.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `PlayRecord` round-trips through RON; record keys are valid storage
  keys.
- Property: none new.
- Snapshot / integration: the replay test above; a debug-menu snapshot with
  the two new entries.

## Completion notes

