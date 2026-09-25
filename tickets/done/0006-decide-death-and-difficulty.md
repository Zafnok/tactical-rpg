---
id: "0006"
title: "Decide: unit death, difficulty and undo"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
---

# 0006 — Decide: unit death, difficulty and undo

## Context

Affects the battle state (0305: what happens at 0 HP; game over), turn rewind (0307), saves (0802)
and the flow (0801). Run with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

### Q1. What happens when a unit falls?

**A. Permadeath (FE Classic)** — they're gone forever (with a death quote). If
the main lord dies, game over. Feel: every move matters; resets are common.
**B. Retreat (FE Casual)** — they leave the battle and return next chapter.
Feel: relaxed, story-focused.
**C. Injuries (XCOM / Battle Brothers)** — fallen units survive with a lasting
injury (stat penalty) or sit out chapters; repeated falls can kill. Feel: loss
has weight without a hard reset.
**D. Player chooses at New Game** (FE Classic vs Casual toggle).
**Recommendation:** D.

### Q2. Undo / rewind

**A. Limited rewind (FE Three Houses "Divine Pulse", FE Echoes "Mila's Turnwheel")** —
a few charges per battle to rewind to any earlier action. Feel: forgives
mis-keys and bad luck while keeping stakes. Our deterministic engine makes this
cheap to build.
**B. Undo last move only** (before confirming an action — already standard).
**C. None** — save-scum via chapter restarts only.
**Recommendation:** A, 3 charges per map (tunable), plus B.

### Q3. Difficulty modes

**A. Normal / Hard (enemy stats and AI aggression scale).** **B. One difficulty,
tuned well.** **C. Normal / Hard / Lunatic.**
**Recommendation:** B for Chapter 1; add modes once the base is fun.

### Q4. Saving

**A. FE: save between chapters + a "suspend" save mid-battle that's deleted on load.**
**B. Save anywhere, any time.** **C. Chapter-start only.**
**Recommendation:** A.

## What to record

`docs/design/death-and-difficulty.md`: Nick's words; exact fall rules
(including who triggers game over), rewind rules (charges, what can be rewound),
difficulty modes (or "one, deferred"), save rules (when, how many slots,
suspend behaviour).

Update `docs/design/README.md`; adjust 0305, 0307, 0801, 0802 if needed.

## Acceptance criteria

- [x] Nick answered Q1–Q4.
- [x] `docs/design/death-and-difficulty.md` written.
- [x] Design README updated; downstream tickets adjusted.
- [x] Ticket archived.

## Completion notes

Nick answered in the project thread on 2026-09-25: 1D, 2A (charges vary by the
map's intended difficulty; restarting a battle refunds them), 3B (one
difficulty now; Hard/Merciless later), 4A. Recorded in
`docs/design/death-and-difficulty.md`.

- Claude-chosen starting values (tunable): rewind charges easy 2 / normal 3 /
  hard 5; 3 save slots.
- Lord falling is game over in both Classic and Casual (stated back to Nick,
  not vetoed).
- Placeholder rules for deferred sub-questions (mode fixed per playthrough,
  Classic-dead unit's items go to the stock, no cost for a Casual retreat) are
  listed in the design doc's open sub-questions.
- Downstream tickets adjusted: 0305 (fall/loss rules, `rewind_charges` in
  `BattleSetup`), 0307 (no longer conditional; charge and restart rules), 0705
  (Casual retreat lines), 0801 (mode select, `rewind_charges` in chapter files,
  `Restart battle` in the map menu, fallen handling in `apply_result`), 0802
  (3 slots, mode on the slot picker, suspend deleted on Continue).
