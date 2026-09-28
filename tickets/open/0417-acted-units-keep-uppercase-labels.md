---
id: "0417"
title: Acted units keep their uppercase label, dimmed only
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0404"]
nick_input: sign-off
completed:
---

# 0417 — Acted units keep their uppercase label, dimmed only

## Context

A unit that has acted is drawn with its map label **lowercased and dimmed**
(`Lo` → `lo`). That rule comes from ticket 0011's look-and-feel decision
(`docs/design/look-and-feel.md` → *Units on the map*) and ADR-0018
(*Never colour alone*: "Acted = lowercase letters"; *Units on the map*:
"Acted: label lowercased and dimmed"). It is drawn by `shown_label` and
`draw_fading_unit` in `crates/ui/src/screens/battle/units.rs`.

After playing the 0404 build, Nick asked for the label to stay uppercase and
only be dimmed (2026-09-28):

> "seems after moving the initials for my units goes from i.e. Lo to lo I
> think it should just stay Lo but be shaded different (it already has this
> aspect so keep it like that)"

The dimming (`ACTED_DIM`, lerp toward the tile background) stays as it is.
The dimming is a brightness change, not a hue change, so "has acted" still
doesn't rely on hue alone.

## Nick input

**Sign-off:** in Quick Battle, move a unit and choose Wait. Its initials stay
`Lo`, only dimmer. Does that read clearly enough as "done for this turn"?

## Scope

**In:**
- Acted units keep their label's case; only the dimming marks them.
- Record Nick's words and the new rule in `docs/design/look-and-feel.md`.
- A short ADR superseding ADR-0018's acted-label rule (the `write-adr`
  skill; ADR-0024 did the same for ADR-0018's cursor).
- Update tests and snapshots that expect lowercase labels.

**Out (do not do):**
- Changing `ACTED_DIM` or how dimming is computed.
- Any other acted-state marker (icons, greying the HP bar, ...).
- The light theme's dimming (0806).

## Implementation steps

1. `crates/ui/src/screens/battle/units.rs`: make `shown_label(unit)` return
   `unit.map_label` unchanged (or remove it and use `map_label` directly;
   update the callers). Update the module doc comment (line 3) and
   `shown_label`'s doc. Keep the `unit.acted` dimming in `draw_fading_unit`.
2. Tests in `units.rs`: rename `labels_lowercase_when_acted` and
   `acted_units_are_lowercase_and_dimmed_toward_the_background` and assert
   the label stays `Br`/`KN`/`Él` when acted, still dimmed by `ACTED_DIM`.
3. `crates/ui/tests/battle.rs`: `select_move_and_wait_dims_the_unit_and_lowercases_its_label`
   and `the_lord_fights_the_near_brigand_on_turn_one` expect `lo`. Expect
   `Lo` and check the dimmed colour (`faction.lerp(bg, ACTED_DIM)`) instead.
   Same for `after_the_playback_the_defenders_hp_is_the_battles` in
   `crates/ui/src/screens/battle/attack_tests.rs`. Rename tests whose names
   say "lowercase".
4. Run the tests, read every `.snap.new` (only acted units' letters should
   change case), then `cargo insta accept`.
5. `docs/design/look-and-feel.md`: in *Units on the map*, replace the
   lowercase rule with "Acted units are **dimmed** toward the background;
   their initials keep their case", with Nick's quote above and the date.
   Remove "(`al`)".
6. New ADR (next number) superseding only the acted-label bullet of
   ADR-0018: acted = dimmed, the label keeps its case. Set ADR-0018's status
   line to note the partial supersession, as done for ADR-0024, and update
   the `docs/adr/README.md` rows.

## Acceptance criteria

- [ ] After Wait, the unit's label reads `Lo` (not `lo`) and is drawn in the
      dimmed faction colour.
- [ ] `look-and-feel.md` and the new ADR record the rule and Nick's words.
- [ ] No test or doc still says acted labels are lowercased.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `units.rs` label and dimming tests as above.
- Snapshot / integration: the battle harness tests above; updated snapshots.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
