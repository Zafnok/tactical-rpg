---
id: "0425"
title: "Menu and map-cursor sounds"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0212", "0213"]
nick_input: sign-off
completed:
---

# 0425 — Menu and map-cursor sounds

## Context

Nick chose our own menu sounds in ticket 0020
([`docs/design/audio.md`](../../docs/design/audio.md)). They are rendered by
0213 as the cues `menu_move`, `menu_select`, `menu_cancel`, and
`cursor_move`, which is `menu_move` at 60 % volume (*tunable*).

On the map cursor, Nick said "fire emblem has it for the cursor so I think we
can have it for the cursor as well at a lower vol maybe?".

## Nick input

**Sign-off:** click through menus and move the map cursor around, including
holding a key down. Does the tick get tiring? Is the cursor volume right?

## Scope

**In:**
- The shared menu widget (`crates/ui/src/widgets/menu.rs`): moving the
  highlight → `menu_move`; confirming an enabled item → `menu_select`;
  backing out → `menu_cancel`.
- Every screen that handles Confirm/Cancel without the menu widget (title,
  battle action flow, forecast, info screens) uses the same three cues for
  the same meanings. Grep for `Action::Confirm` / `Action::Cancel` handling.
  Advancing dialogue text is **not** a menu action: it plays nothing (Nick
  wasn't asked).
- The battle map cursor: each tile it moves → `cursor_move`. Key repeat
  (ticket 0421) ticks on every repeated step.
- Selecting a disabled menu item plays **nothing**. This is *Claude's starting
  rule*: list it in the PR for Nick.

**Out (do not do):**
- Battle event sounds (0424), music (0807, 0814).
- New sounds or cues.

## Implementation steps

1. Add the three calls in `widgets/menu.rs` and cover them with the widget's
   tests.
2. Walk every screen's input handling and add the cues where Confirm, Cancel
   or cursor movement change something. No sound when the input does nothing,
   e.g. moving past the edge of a list or the map.
3. The battle cursor: `cursor_move` per tile moved, including camera-scroll
   moves.

## Acceptance criteria

- [ ] Harness tests: menu move, select, cancel and disabled select emit the
      right requests (or none).
- [ ] Harness test: holding the cursor key for 1 s emits one `cursor_move`
      per tile moved, and none at the map edge.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: menu widget.
- Snapshot / integration: Harness tests on title, battle cursor and one
  menu-heavy screen.

## Completion notes

*(Filled in by the session that completes the ticket.)*
