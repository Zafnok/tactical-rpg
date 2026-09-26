---
id: "1002"
title: Support points, ranks and battle bonus (core rules + data)
type: feature
milestone: Post–Chapter 1
model: opus-5.5
effort: high
status: todo
blocked_by: ["0010", "0304", "0305", "0306", "0307", "0309", "0311", "0801", "0802"]
nick_input: none
completed:
---

# 1002 — Support points, ranks and battle bonus

## Context

Nick chose FE GBA-style earned supports plus camp events (ticket 0010,
[`docs/design/supports.md`](../../docs/design/supports.md)). This ticket
builds the **rules**: support pairs as data, points gained in battle, ranks
C/B/A, the Hit/Avoid bonus, and carrying support state across battles in the
campaign. Note: a chapter is a story beat that may hold several battles and
skirmishes (`chapter-1.md`), so nothing here may assume one battle per chapter.
The screens for reading conversations and camp events are in 1003.
Crate rules: [ADR-0004](../../docs/adr/0004-crate-architecture.md) (all of
this lives in `core`, and state changes only through `Command` → `Event`).

## Nick input

None. Every rule and number comes from `supports.md`. The numbers there are
*tunable* starting values: put them in data, not in code.

## Scope

**In:**
- Content: a `supports.ron` (or a section of the character data, whichever
  fits the content layout by then) listing pairs: `(unit_a, unit_b)`, an
  optional thresholds override, optional starting points, and the conversation
  ids for C/B/A (the scripts can be placeholders).
- A tuning record with the point values, thresholds, bonus table, range (3)
  and the no-stacking rule from `supports.md`.
- `core::support`: `SupportTable`, and `SupportState` per pair (points, rank,
  and unlocked-but-unviewed rank). Pure
  functions add points, unlock ranks and view a conversation.
- Battle hooks for the point events: ending the player phase adjacent,
  fighting with an adjacent partner, and healing, buffing or using an item on
  a partner. Emitted as `Event::SupportPoints { a, b, amount }` so that rewind
  (0307) and replay just work.
- Bonus: Hit/Avoid of the single best-ranked partner within range (never
  summed), fed into the
  forecast (0304) for attacks and counters.
- Campaign: support state is saved with the campaign (0802) and applied after
  a battle. It is removed for units that died in Classic and kept for Casual
  retreats. This happens after every battle, skirmishes included.
- A campaign command to view a support conversation (e.g.
  `ViewSupport { a, b }`), which raises the rank.

**Out (do not do):**
- Any UI (1003). The forecast shows the bonus only inside the Hit/Avoid
  numbers that 0404 already shows.
- Support conversation text (story pipeline).
- Camp events (1003), hub activities (1004), pair abilities (1005), endings.

## Implementation steps

1. Read `supports.md` fully, then the code from 0304, 0305, 0306, 0307, 0309,
   0311, 0801 and 0802.
2. Add the data format and loader in `trpg-content`, with validation: both
   units exist, no duplicate pair, no unit paired with itself, thresholds
   strictly increasing, conversation ids exist (if the dialogue index exists
   by then).
3. Write `core::support` with the pure rules, unit tests first.
4. Hook point gain into `BattleState::apply` at the events listed in
   `supports.md` ("Gaining points"). A heal or buff on several allies gives
   points to every affected pair.
5. Add the bonus to the forecast input: Manhattan distance, the partner must
   be on the map, and only the highest-ranked partner in range counts.
6. Carry the state through the campaign and the save format. If 0802 has a
   save version, bump it and follow its ADR's rule for old saves.

## Acceptance criteria

- [ ] Pairs load from data; invalid data gives a clear content error.
- [ ] Every point rule in `supports.md` has a test that names it.
- [ ] Points stop at an unlocked, unviewed threshold, so a pair can't gain two ranks without a camp visit.
- [ ] Forecast Hit/Avoid include the best partner's bonus (never combined) for attacks and counters.
- [ ] Rewind undoes support points; a replay reproduces them.
- [ ] A Classic death removes the unit's supports; a Casual retreat keeps them.
- [ ] Support state survives save/load.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each point rule, thresholds, best-partner-only bonus, points held at an unviewed threshold across several battles, death/retreat.
- Property: points never pass the next unviewed threshold; the bonus never
  exceeds the A-rank bonus; pairs are symmetric ((A,B) == (B,A)).
- Integration: a scripted battle where a healer heals a partner and two units
  fight side by side, checking the events and the final `SupportState`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
