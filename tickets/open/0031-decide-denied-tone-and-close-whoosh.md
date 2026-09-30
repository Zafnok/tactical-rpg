---
id: "0031"
title: "Decide: the \"can't do that\" tone and the message-close whoosh"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0031 — Decide: the "can't do that" tone and the message-close whoosh

## Context

Ticket 0425 wired the menu sounds (`docs/design/audio.md`). Reviewing its
starting rules on 2026-09-29, Nick asked for two sounds we don't have yet:

- **A "can't do that" tone.** "selecting greyed out menu item should
  ideally beep a new tone we haven't yet implemented, warning it's not
  possible with sound. for now, cancel tone can work, but likely should be
  a different unique tone." Today `MenuSound::Denied` plays `menu_cancel`.
- **A whoosh when a modal closes.** "maybe a woosh-like sound when a modal
  closes... something unique to dodge but not yet implemented... for now
  can be empty and I will see how it feels". Today closing a tip, a phase
  or victory/defeat banner, the EXP bar or the level-up page is silent.
  It must sound different from the dodge (`miss`, the in-house "W2" whoosh).

Both would be in-house sounds, made like the menu sounds (0020 → 0213):
recipes on the listening page `assets-src/audio/sound-audition.html`,
rendered by `cargo xtask sfx`.

## Nick input

**Decision**, with the `ask-nick` skill, by ear: a listening page with
several candidates for each sound (in the family of the chosen menu sounds:
muted chip, D4-based), plus "describe your own". Expect several rounds.

Questions:

1. **The "can't do that" tone:** which one? Candidates could be a low
   buzz, two identical notes, or a short falling step; Fire Emblem (GBA)'s
   error buzz is a reference.
2. **Where it plays.** In our menus the highlight *skips* greyed-out items,
   so today you can almost never press Confirm on one. Options:
   a) keep skipping; the tone only plays in the rare menu where nothing is
      available;
   b) let the highlight land on greyed-out items, like Fire Emblem's item
      lists, so Confirm on them plays the tone;
   c) b, and also other "can't do that" presses: Confirm on a unit that
      has already acted, Confirm on a tile the unit can't reach, rewind
      with no charges left.
3. **The close whoosh:** does Nick still want it after playing 0425's
   build? If yes, which candidate, and on which closings (tips, banners,
   the EXP bar, the level-up page; not dialogue, which stays silent)?

## Scope

**In:**
- Listening-page candidates (update `sound-audition.html`).
- Record the answers in `docs/design/audio.md`: new cue rows (e.g.
  `menu_denied`, `modal_close`), where each plays, the answer to Q2 in
  plain words with an example, and Nick's words in the appendix.

**Out (do not do):**
- Rendering the WAVs or wiring code: ticket 0427.
- Any other sound.

## Implementation steps

1. Read `docs/design/audio.md` and the `<script>` of
   `assets-src/audio/sound-audition.html` (the `MENU` and `SFX` recipes).
2. Add 5–8 candidates for each sound to the page, reusing its building
   blocks (`chip`, `pulse`, `burst`, `env`, `wet`); publish it as in 0020.
3. Run the round(s) with `ask-nick`; ask Q2 and Q3 in the same message.
4. Write the decisions into `docs/design/audio.md`. If Q2 is (b) or (c),
   add the menu and rule changes to ticket 0427's scope.

## Acceptance criteria

- [ ] `docs/design/audio.md` names both cues (or records that Nick dropped
      the whoosh), with their recipes on the listening page.
- [ ] Q2's answer is recorded in plain words with an example.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none.
- Snapshot / integration: `every_design_cue_is_in_the_manifest`
  (`crates/content/src/audio.rs`) must still pass: new in-house cues go in
  its `IN_HOUSE` list until 0427 adds them to the manifest.

## Completion notes

*(Filled in by the session that completes the ticket.)*
