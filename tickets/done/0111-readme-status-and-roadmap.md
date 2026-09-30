---
id: "0111"
title: Refresh the README with current status and roadmap
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-09-30
---

# 0111 — Refresh the README with current status and roadmap

## Context

The root `README.md` still said "workspace skeleton only, no game logic yet",
written at ticket 0101. Since then about 90 tickets have landed: a playable
quick battle on GitHub Pages, combat, Combat Arts, rewind, enemy AI, level
ups, dialogue scenes with portraits, sounds and music. Nick asked for the
README to show the current status, the planned roadmap, and "anything fancy".
The source of truth for status stays `tickets/` and `docs/ROADMAP.md`; the
README summarises them for visitors.

## Nick input

None.

## Scope

**In:**
- Rewrite `README.md`: pitch, what you can play today, screenshots, progress
  per milestone, the road to Chapter 1 and release, what's decided vs open.
- Screenshots as SVGs in `docs/img/`, rendered from the existing `insta`
  snapshots (text + colour grid + `assets/data/palette.ron`), so they show
  real screens with the real palette.

**Out (do not do):**
- No game code, no new tooling in `crates/xtask` (the SVGs are rendered by a
  one-off script; regenerating them is not a gate).
- No story spoilers (the repo is public): no plot, twists or cast details
  beyond what the design docs state about setting and tone.
- No changes to `docs/ROADMAP.md`, `LICENSE` or design docs.

## Implementation steps

1. Count tickets per block in `tickets/open/` and `tickets/done/`.
2. Render three snapshots to SVG: the attack forecast with the arts menu, a
   dialogue scene, and the level-up screen.
3. Rewrite `README.md`, keeping the existing Play, Releasing and License
   sections and the SonarCloud badges.

## Acceptance criteria

- [x] README status matches `tickets/` as of 2026-09-30 and links
      `docs/ROADMAP.md` as the live source.
- [x] Screenshots render on GitHub and use palette colours.
- [x] No hard-coded key names presented as fixed controls (keys are
      rebindable; the README names only the default style).
- [x] All gates in the `run-gates` skill pass (docs-only change).

## Tests required

- None beyond `cargo xtask ticket-lint` (docs-only change).

## Completion notes

- README rewritten with a pitch, three SVG screenshots, a "playable today"
  list, per-milestone progress (as of 2026-09-30), a Mermaid road map to
  Chapter 1 → itch.io → Steam, and what comes after Chapter 1.
- SVGs in `docs/img/` were rendered from these snapshots by a one-off script
  (not committed): `attack_tests__forecast_with_a_double_and_a_counter`,
  `dialogue__fully_revealed`, `progress_tests__level_up`. They show test
  units ("Test Lord") and test portraits, not final art. They will go stale
  as screens change; a future ticket could regenerate them automatically.
- The progress numbers in the README are a snapshot; `tickets/` stays the
  source of truth.
