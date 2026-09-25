---
id: "0002"
title: "Decide: turn structure"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
---

# 0002 — Decide: turn structure

## Context

The battle state machine (ticket 0305), the AI (0501) and the enemy-phase UI
(0502) all depend on who acts when. Run with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

### Q1. Who acts when?

**A. Phases — Fire Emblem, Advance Wars, Wargroove**
You move *all* your units in any order, then the enemy moves all of theirs.
Feel: each turn is a planning puzzle; you can set up combos (weaken, then finish);
enemy phase is a tense "watch it play out" moment.

**B. Initiative / charge time — Final Fantasy Tactics, Tactics Ogre, Triangle Strategy**
Each unit gets a turn when its speed meter fills, so player and enemy units
interleave. A turn-order bar shows who's next. Feel: Speed is king; fewer
alpha-strikes; each action is smaller and more reactive.

**C. Telegraphed enemy intent — Into the Breach**
Enemies show exactly what they'll attack next; you respond, then they execute.
Feel: perfect-information puzzle; very different from FE.

**D. Alternating single activations — tabletop skirmish games (Kill Team)**
You move one unit, the enemy moves one unit, repeat. Feel: back-and-forth duel.

**Recommendation:** A — it's the Fire Emblem feel and pairs naturally with an
"enemy danger zone" overlay.

### Q2 (only if A). Move-then-act rules

**A. FE standard** — move, then choose an action (Attack / Item / Wait); you can
cancel the move until you pick an action.
**B. FE + Canto** — mounted units can move again with leftover movement after acting.
**C. Split move** — any unit can split its movement before and after acting.

**Recommendation:** A now, B as a class ability later.

### Q3. Turn limit / reinforcements

Should maps be able to have reinforcements appearing on set turns (FE does,
often infamously "ambush spawns" that act immediately)?
**A. Yes, but they never act on the turn they appear.** **B. Yes, FE-style ambush.**
**C. No reinforcements.** Recommendation: A.

## What to record

`docs/design/turn-structure.md`: Nick's words; the phase/turn order as a
precise sequence (e.g. `Player phase → Enemy phase → Other phase → turn += 1`);
what "a unit's action" consists of; when a unit is "done"; how end-turn works
(manual, and auto-end when all units have acted — ask as a sub-question if
unclear); reinforcement rules; turn-limit/objective interactions.

Update `docs/design/README.md`; adjust tickets 0305, 0501, 0502 if needed.

## Acceptance criteria

- [x] Nick answered Q1 (and Q2/Q3 if relevant).
- [x] `docs/design/turn-structure.md` written with the exact sequence.
- [x] `docs/design/README.md` updated; downstream tickets adjusted.
- [x] Ticket archived.

## Completion notes

Nick picked **FE phases**, **FE-standard move-then-act only** (no built-in
Canto; post-action movement only via specific combat skills, like FE's bow
skill that steps 1 tile away), and **reinforcements that never act on arrival**.
Follow-ups: **auto-end ON by default** with a toggle shortcut, an **Other
phase** for green Ally/Neutral units, and **optional per-map turn limits**.
Recorded in `docs/design/turn-structure.md` with the exact sequence
`Player → Enemy → Other → turn += 1`.

- Claude's starting rules (Nick may veto): empty phases are skipped silently;
  reinforcements arrive at the start of their own side's phase already done,
  and wait a turn if their tile is occupied; Survive / turn limits resolve
  when turn N's last phase ends; toggle key `Shift+e`.
- Downstream edits: 0305 (`Phase` enum, Other phase, reinforcements,
  `turn_limit`, auto-end lives in the UI, skill-move hook, new acceptance
  criteria), 0501 (AI also runs the Other phase), 0502 (plays back Other
  phase too), 0405 (auto-end + toggle, `OTHER PHASE` banner, turn limit in
  Objective), 0204 (`ToggleAutoEnd` action, an addition to ADR-0006's
  bindings), 0805 (`auto_end_turn` setting), 0005 (Canto example replaced).
  0303 needed no change (Canto was already out of scope).
- No local gates to run yet (no Cargo workspace before 0101); docs only.
