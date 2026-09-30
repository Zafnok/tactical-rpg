---
id: "0427"
title: "Render and play the \"can't do that\" tone and the close whoosh"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0031"]
nick_input: sign-off
completed:
---

# 0427 — Render and play the "can't do that" tone and the close whoosh

## Context

Ticket 0031 decides two new in-house sounds (`docs/design/audio.md`): the
"can't do that" tone and, if Nick keeps it, a whoosh when a message closes.
Ticket 0425 left placeholders:

- `MenuSound::Denied` (`crates/ui/src/audio.rs`) plays `menu_cancel`. The
  menu widget plays it on Confirm on a disabled item
  (`Menu::handle_with_sound`, `crates/ui/src/widgets/menu.rs`).
- Closing tips, banners, the EXP bar and the level-up page is silent
  (`BattleScreen::update` in `crates/ui/src/screens/battle/mod.rs`: the
  `shown_tip`, `progress_key` and `banner` branches).

## Nick input

**Sign-off:** press Confirm on something you can't do, and close a tip, a
phase banner and a level-up page. Does each sound right?

## Scope

**In:**
- Port 0031's recipes to `crates/xtask/src/sfx.rs` (as 0213 did), run
  `cargo xtask sfx`, add the cues to `assets/audio/audio.ron` with
  `credit: Own`.
- `MenuSound::Denied` → the new cue.
- Wherever 0031's Q2 says the tone plays. If that changes the menu widget
  (the highlight landing on greyed-out items) or battle input (Confirm on
  an acted unit), do that here.
- The close whoosh on the closings 0031 lists (skip if Nick dropped it).

**Out (do not do):**
- Any other sound. Dialogue stays silent.

## Implementation steps

1. Read 0031's section of `docs/design/audio.md` and 0213's completion
   notes (how `sfx.rs` ports a page recipe).
2. Add the recipes to `sfx.rs`; run `cargo xtask sfx`; commit the WAVs; add
   the manifest entries.
3. Change `MenuSound::cue()` for `Denied`; update the widget test
   `a_disabled_item_is_denied`.
4. Play the whoosh in the battle screen's close branches.
5. Make the Q2 changes, if any, with Harness tests.

## Acceptance criteria

- [ ] `cargo xtask sfx --check` passes with the new WAVs.
- [ ] Unit test: Confirm on a disabled item plays the new cue.
- [ ] Harness test: closing a tip, a banner and a level-up page each plays
      the whoosh once (if kept); reading on in dialogue plays nothing.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: menu widget; sfx determinism (the existing test covers new files).
- Snapshot / integration: Harness tests on the battle screen.

## Completion notes

*(Filled in by the session that completes the ticket.)*
