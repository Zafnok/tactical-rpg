---
id: "0421"
title: Longer first delay before held keys repeat
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: ["0204"]
nick_input: none
completed: 2026-09-28
---

# 0421 — Longer first delay before held keys repeat

## Context

Nick's feedback (2026-09-28): scrolling between menu options (weapon
selection, the left/right-handed picker, …) repeats too soon when a key is
held; a single tap often moves twice. He asked for a slightly longer delay
before the first repeat.

Key repeat is one shared timing for every held repeatable action (ADR-0006,
ticket 0204), read from `assets/data/keymap.ron`
(`repeat: (delay_ms, interval_ms)`). `docs/design/controls.md` marks the
values *tunable*.

## Nick input

None. (Sign-off: tap and hold in menus and on the map; say if 300 ms feels
too slow or still too fast.)

## Scope

**In:**
- `delay_ms` 170 → 300 in `keymap.ron` and `RepeatDef::default()`.
- `controls.md` updated; tests that hard-code the timing updated.

**Out (do not do):**
- Separate menu and map timings, a hold-to-scroll-faster key.
- The repeat interval (55 ms) stays.

## Implementation steps

1. Change the data and the default.
2. Update `embedded_keymap_has_exactly_the_design_bindings`,
   `hold_repeats_then_releases` (harness) and
   `harness_hold_repeats_with_the_keymap_timing_and_scrolls` (battle).
3. Update `docs/design/controls.md`.

## Acceptance criteria

- [x] Holding a key moves once, then again only after 300 ms, then every 55 ms.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Existing repeat tests updated to 300 ms: content keymap tests, harness,
  battle hold/scroll, `tests/title.rs` (hold 0.43 s for four moves).
  `input.rs` unit tests keep their explicit 170/55 fixture; only the
  assertion that compared it to `RepeatDef::default()` now names the values.

## Completion notes

*Claude's starting rule:* 300 ms, applied to every held direction key (menus
and the map cursor share one timing). It's the common "type-matic" default
range (250–500 ms) and removes accidental double steps from a normal tap.
ADR-0006 mentions 170 ms as the initial value; the data file is the source
of truth, so the ADR is not edited.
