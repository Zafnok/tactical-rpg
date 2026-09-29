# ADR-0028: Acted units are dimmed only; the label keeps its case

- **Status:** Accepted
- **Date:** 2026-09-29
- **Related tickets:** 0417, 0404, 0011
- **Supersedes:** the acted-label rule of ADR-0018 ("Never colour alone":
  "Acted = lowercase letters"; "Units on the map": "Acted: label lowercased
  and dimmed"). The rest of ADR-0018 stands.

## Context

ADR-0018 marked a unit that had acted by lowercasing its map label (`Lo` →
`lo`) and dimming it. After playing the 0404 build Nick asked for the label to
stay `Lo` and only be shaded differently (quoted in
`docs/design/look-and-feel.md`).

## Decision

- An acted unit's label keeps its case. The only marker is the dimming:
  the faction colour lerped toward the tile background by `ACTED_DIM`
  (unchanged).
- `shown_label` is removed; the map label is drawn as is.
- The dimming is a brightness change, not a hue change, so "has acted" still
  doesn't rely on hue alone. Other acted markers are out of scope.

## Consequences

- Initials read the same before and after acting, at the cost of a subtler
  marker. Nick signs off on that when playing.
- A future light theme (0806) must keep the dim distinguishable.
