# ADR-0006: Input actions, vim-style keymap, virtual cursor

- **Status:** Superseded by ADR-0015
- **Date:** 2026-09-25

## Context

Nick wants a Fire Emblem-style **virtual cursor** driven entirely from the
keyboard, with **vim-like hotkeys** — i.e. hands on the home row, keyboard used
like a gamepad/mouse, *not* modal command-line stuff like `:wq`.

## Decision

### Actions, not keys

`ui` defines an `Action` enum. Screens only ever see `Action`s, never raw keys.
`app` translates raw keyboard input to `ui`'s key type, and a `Keymap` (data,
rebindable, defaults in `assets/data/keymap.ron`) maps keys to actions. This
makes screens testable with scripted action sequences and makes controller
support a later mapping exercise.

### Default bindings

| Action | Primary | Alternates | Notes |
| ------ | ------- | ---------- | ----- |
| Cursor left/down/up/right | `h` `j` `k` `l` | Arrow keys | Held keys repeat (see below) |
| Cursor jump ×5 | `Shift` + `h/j/k/l` | `Shift` + arrows | Fast travel across the map |
| Confirm / select | `f` | `Space`, `Enter` | Left-hand home row, index finger |
| Cancel / back | `d` | `Esc`, `Backspace` | Left-hand home row, middle finger |
| Unit info / details | `s` | `i` | Full stat screen for the unit under cursor |
| Toggle enemy danger zone | `a` | | Shows combined enemy threat range |
| Next / previous ready unit | `Tab` / `Shift+Tab` | `n` / `p` | Cursor jumps to the next unit that can act |
| Map menu | `Esc` when nothing to cancel | `f` on an empty tile | End Turn, Units, Objective, Options, Save |
| End turn | via map menu, or `e` then confirm | | Always asks for confirmation |
| Fast-forward animations / text | Hold `f` or `Space` | | Skips typewriter text, speeds up enemy phase |

The layout keeps the right hand on `hjkl` (movement) and the left on
`a s d f` (actions): one hand steers, the other acts, like a controller.

### Key repeat

Pure logic in `ui` (tested with a fake clock): first repeat after **170 ms**,
then every **55 ms** while held. Values live in data so they can be tuned after
Nick's playtest.

### Virtual cursor

- Snaps to map tiles; never leaves map bounds.
- The camera scrolls when the cursor gets within 3 tiles of the viewport edge.
- Hovering shows terrain info and a unit mini-panel; confirming on a unit selects it.
- The cursor remembers its position per phase (returns to the last unit moved).

## Consequences

- Screens and integration tests are written against `Action`s only.
- Rebinding and controller support later need no screen changes.
- Default bindings are an opinion. Nick will judge them in the first playtest
  (ticket 0804); changes are a data edit.

## Alternatives considered

- **WASD movement** — collides with the `a s d f` action keys and isn't what
  Nick asked for. Arrow keys cover players who don't want vim keys.
- **Mouse support** — out of scope for now; would be a later ticket.
