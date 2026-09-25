# ADR-0015: Input actions, keymap layouts, virtual cursor

- **Status:** Accepted
- **Date:** 2026-09-25
- **Related tickets:** 0204, 0015, 0208
- **Supersedes:** ADR-0006

## Context

ADR-0006 set up the input model and also listed default key bindings. Nick
has since decided the bindings himself (`docs/design/controls.md`, ticket
0015): two layouts (right-handed arrows + `ASDF`, left-handed `WASD` +
`JKL;`), picked on first launch, and no fast-cursor "jump" key. Key choices
are a design decision, so they don't belong in an ADR. Ticket 0204 also found
that the input types had to live in `trpg-content`.

## Decision

Everything in ADR-0006 stands except as changed here.

- **Actions, not keys** (unchanged). Screens only ever see `Action`s.
  `app` translates raw keyboard input to `Key`/`Chord`; a data-driven keymap
  maps chords to actions.
- **Where the types live.** `Key`, `Chord`, `Action` and the validating
  `KeymapDef` loader are in `trpg-content::keymap` (the loader must parse
  chords and action names to validate the file, and `ui` depends on
  `content`). `trpg-ui::input` re-exports them and owns `Keymap` and
  `InputState`. Screens use `trpg_ui::input::Action`.
- **Default bindings come from `docs/design/controls.md`**, not from this
  ADR. `assets/data/keymap.ron` must match it; a test pins it.
- **Layouts are data.** Each layout is a complete set of bindings in
  `assets/data/keymap.ron`, validated like a single keymap (ticket 0208 sets
  the exact format). The player's chosen layout is a saved setting; per-key
  overrides from Options (0805) apply on top of it.
- **No jump actions.** The `CursorJump*` actions from ADR-0006 are removed;
  the four cursor actions are the only repeatable ones.
- **Key repeat** (unchanged): pure logic in `ui`, first repeat after 170 ms,
  then every 55 ms, values in data. The most recently pressed cursor key
  repeats; at most 5 repeats per update.
- **Virtual cursor** (unchanged from ADR-0006): snaps to tiles, never leaves
  the map, camera scrolls within 3 tiles of the edge, remembers position per
  phase.

## Consequences

- Changing a default key is a data edit plus a design-doc edit, never an ADR.
- Every help text must read key names from the active keymap, since the
  same action has different keys in each layout.
- Adding a layout later (for example a vim layout) is only data.

## Alternatives considered

- **Keep bindings in the ADR** — they are Nick's call and changed as soon as
  he was asked; the ADR would go stale on every tweak.
- **One layout with every scheme bound at once** — Nick chose two layouts;
  WASD would also clash with the `ASDF` action keys.
