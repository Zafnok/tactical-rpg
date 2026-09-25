---
id: "0015"
title: "Decide: controls, right/left-handed layouts, key per action"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
---

# 0015 — Decide: controls, right/left-handed layouts, key per action

## Context

Nick asked to choose the key for every action himself, and to offer a
right-handed (arrow keys) and a left-handed (WASD) layout. The default
bindings had come from ADR-0006, which was written before he was asked.
Asked with the `ask-nick` skill during ticket 0204.

## Nick input

**Decision.**

## Questions to ask

1. Which layouts should the game offer? (two layouts / add a vim layout /
   everything bound at once)
2. Which layout on first launch? (right / left / ask on first launch)
3. Right-handed confirm and cancel keys.
4. A faster way to move the cursor? (hold to scroll faster / jump 5 / none)
5. Right-handed key for each action.
6. Left-handed layout: mirror, or big keys on the right?

## Acceptance criteria

- [x] Answers recorded in `docs/design/controls.md` with Nick's words verbatim.
- [x] `docs/design/README.md` row added.
- [x] Downstream tickets updated.

## Completion notes

- Recorded in `docs/design/controls.md`. Two layouts (right-handed arrows +
  `ASDF`, left-handed `WASD` + mirrored `JKL;`), a picker on first launch, no
  fast-cursor key for now (revisit after a large-map playtest), double-tap
  `Space` to end turn, `Shift+Space` toggles auto-end.
- Recorded alongside 0204 (PR #20), whose default keymap changed to match:
  `assets/data/keymap.ron` is now the right-handed layout, and the
  `CursorJump*` actions are gone.
- ADR-0015 supersedes ADR-0006: default bindings now come from this design
  doc, not the ADR.
- `turn-structure.md`: the auto-end toggle key is now `Shift+Space`.
- Follow-up: ticket 0208 (layouts in data, left-handed layout, first-launch
  picker). Tickets 0205, 0307, 0401, 0402, 0404, 0405, 0407, 0408 and 0805
  were edited to match the new keys.
