---
id: "0442"
title: Combat Arts in their own menu, out of the forecast
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0442 — Combat Arts in their own menu, out of the forecast

## Context

Nick, 2026-10-03, while answering ticket 0430 (which keys swap the weapon in
the forecast):

> "like three houses, but up/down should not pick an art. the arts should not
> be in the same menu. we should have an arts menu. that was something i didnt
> like about the current quick battle setup anyway"

Today (ticket 0414, `docs/design/combat-arts.md` → *Forecast display*) the
attack forecast has a list beside it: `Attack`, the weapon's Combat Arts, the
unit's combat actives. Up / down move through it while a target is chosen
([`art_list.rs`](../../crates/ui/src/screens/battle/art_list.rs),
`Targeting` in [`attack.rs`](../../crates/ui/src/screens/battle/attack.rs),
`step_targeting` in [`mode.rs`](../../crates/ui/src/screens/battle/mode.rs)).
A spell's forecast has the same list with the spell actives
([`magic.rs`](../../crates/ui/src/screens/battle/magic.rs), `CastTargeting`).

Ticket 0430 made pointing at an enemy open the forecast at once, with left /
right swapping the weapon or spell (`Targeting::options`, `Targeting::swap`;
`docs/design/turn-structure.md` → *Pointing at an enemy*). It left the arts
list where it was.

In Fire Emblem: Three Houses, `Attack`, `Magic` and `Combat Arts` are three
entries of the action menu; `Combat Arts` lists the arts, then the target is
picked with the forecast.

## Nick input

**Ask first, with the `ask-nick` skill, before building** (rendered mockups,
`feedback-nick-visual-reviews`), then record the answers in
`docs/design/combat-arts.md` (*Forecast display*) and
`docs/design/turn-structure.md`:

1. Where the arts menu is: an `Arts` entry in the action menu (Three Houses),
   or something else.
2. How an art is reached after **pointing at an enemy** (0430 opens the
   forecast directly): a key in the forecast that opens the arts menu, or only
   through Cancel → action menu → `Arts`.
3. Where the combat **actives** go (Close Shot, Overcast…): in the same arts
   menu, or their own.
4. What up / down do in the forecast once the list is gone (change target?).
5. Whether left / right swap the weapon in **every** forecast, also the one
   opened from `Attack` (which would make the weapon list before it
   unnecessary), and the same for `Magic` and its spell list.

**Sign-off:** in Quick Battle, use an art with the lord and Overcast with the
mage, by pointing at an enemy and from the action menu.

## Scope

**In:**
- The arts menu Nick chooses, for weapons' arts and for the combat actives.
- The forecast without the arts list; its keys as Nick answers.
- Help bars through the keymap (`keyboard-input` skill).
- The design docs above.

**Out (do not do):**
- New arts, costs or rules of arts (`core` is unchanged; every check stays
  `BattleState::preview_attack`, ADR-0004).
- Describing arts when choosing (ticket 0429): if 0429 is done first, keep its
  descriptions in the new menu; if this one is done first, add a line to 0429
  that the list it describes is the arts menu.
- AI changes.

## Implementation steps

1. Ask Nick (above) and record the answers.
2. `mode.rs`: a `MenuEntry` and `Mode` for the arts menu as answered, built
   from `art_choices` (`art_list.rs`); choosing a line opens `Targeting` with
   that technique fixed.
3. `attack.rs`, `Targeting`: drop the list (`choices`, `list`, `move_list`,
   `has_list`) or keep only the chosen technique; the same for
   `CastTargeting` in `magic.rs`.
4. `mod.rs`: draw the arts menu; remove the list beside the forecast;
   `help_targeting` without `art`.
5. Update the tests of 0414 (`art_tests.rs`), 0410 (`magic_tests.rs`), 0428 and
   0430 (`mode.rs`, `attack_tests.rs`) to the new flow.

## Acceptance criteria

- [ ] Nick's answers are in `docs/design/combat-arts.md` and
      `docs/design/turn-structure.md`, in his words.
- [ ] Test: an art is chosen in the arts menu, then its target, and the attack
      command carries that art.
- [ ] Test: the forecast shows no arts list, and up / down do what Nick chose.
- [ ] Test: a dimmed art (`broken`, `2 uses left`) can't be chosen and shows
      its reason in the menu.
- [ ] Test: Cancel from the forecast of an art goes back to the arts menu,
      then the action menu.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the menu's lines and the mode transitions.
- Snapshot / integration: a harness script with the default keys using an art
  and a spell active; review the changed forecast snapshots.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
