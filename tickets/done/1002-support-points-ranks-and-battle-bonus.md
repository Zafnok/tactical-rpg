---
id: "1002"
title: Support points, ranks and battle bonus (core rules + data)
type: feature
milestone: Post–Chapter 1
model: opus-5.5
effort: high
status: done
blocked_by: ["0010", "0304", "0305", "0306", "0307", "0309", "0311", "0801", "0802"]
nick_input: none
completed: 2026-10-03
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

- [x] Pairs load from data; invalid data gives a clear content error.
- [x] Every point rule in `supports.md` has a test that names it.
- [x] Points stop at an unlocked, unviewed threshold, so a pair can't gain two ranks without a camp visit.
- [x] Forecast Hit/Avoid include the best partner's bonus (never combined) for attacks and counters.
- [x] Rewind undoes support points; a replay reproduces them.
- [x] A Classic death removes the unit's supports; a Casual retreat keeps them.
- [x] Support state survives save/load.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each point rule, thresholds, best-partner-only bonus, points held at an unviewed threshold across several battles, death/retreat.
- Property: points never pass the next unviewed threshold; the bonus never
  exceeds the A-rank bonus; pairs are symmetric ((A,B) == (B,A)).
- Integration: a scripted battle where a healer heals a partner and two units
  fight side by side, checking the events and the final `SupportState`.

## Completion notes

**What was done**

- **Rules** in `trpg_core::support`: ranks C/B/A, support pairs (the same
  either way round), the tuning record `SupportRules` (thresholds, points
  per event, bonus range, bonus per rank), `SupportTable` (rules + the
  listed pairs), `SupportState` per pair (points, rank, unlocked rank,
  ended) and `SupportBook` (every pair's state; saved).
- **Battle** (`crates/core/src/battle/supports.rs`): the five point events
  emit `Event::SupportPoints { a, b, amount }`, and the Hit/Avoid of the
  best-ranked partner in range goes into every combat's numbers (attacker
  and defender), so the forecast, the AI and the fight all use it. The
  support state is part of the battle state, so rewind and replay keep it.
- **Campaign**: `Campaign::supports` goes into each battle and comes back
  after every won battle (story battle or skirmish). A Classic death ends
  the dead character's supports; a Casual retreat keeps them.
  `Campaign::view_support(a, b, table)` views an unlocked conversation,
  raises the rank and returns the scene to play (for 1003).
- **Data**: `assets/data/supports.ron` holds the numbers from `supports.md`
  and the pairs. `trpg_content::support` loads and checks it (characters
  exist, no self pair, no duplicate pair, thresholds rise, a higher rank's
  bonus isn't smaller, conversations are dialogue scenes).
- **Save format**: `SAVE_VERSION` is now 2 (ADR-0039: saves of version 1
  show as incompatible; there are no player saves yet).

**Deviations**

- **No campaign `Command`.** The campaign has no command/event layer
  (`apply_result`, `downgrade_mode` are methods), so viewing a support is
  the method `Campaign::view_support`, not a `ViewSupport` command.
- **The no-stacking rule is code, not a data switch.** The data file
  documents it; a switch would mean writing and testing a stacking rule
  Nick ruled out.
- `BattleState::restore_tables` and `BattleHistory::restore_tables` now
  take `&GameTables` (a seventh table didn't fit the argument list).
- The event's `amount` is what the pair really gained. Held at an unviewed
  threshold, a pair gains nothing and no event is emitted.
- `supports.ron` lists two **placeholder pairs** of the test characters
  (test lord with test knight and with test mage, the debug Quick Battle's
  units), with placeholder scenes in `assets/dialogue/test_supports.dlg`.
  Real pairs and conversations come from the story pipeline.

**Rules decided where the design was silent** (*Claude's starting rules*;
Nick can veto any of them)

1. **Only your own units build supports and get the bonus.** A green
   (allied) unit or an enemy never does, even if its character is in a
   listed pair. Example: a character who fights as a green unit before
   joining gains nothing with your lead in that battle.
2. **A unit that falls in a fight gains nothing for that fight.** Example:
   Ann attacks next to Ben and is felled by the counter: no points (in
   Casual she also gains nothing for the rest of that battle, as designed).
3. **One attack pays a pair once.** Example: a boss's Line Pierce strikes
   Ann and then Ben behind her; both were attacked side by side, and the
   pair still gets +3, not +6.
4. **The "adjacent at the end of the player phase" point needs the phase
   to end.** If the last enemy falls during your phase, the battle ends at
   once and that turn gives no adjacency point.
5. **A pair that starts with more points than C needs starts with C
   unlocked**, its points waiting at C's threshold like any other pair.
6. **At rank A a pair gains no more points.**

**Follow-ups**

- None created. 1003 (camp screens) can read `SupportBook::unlocked`,
  `SupportState::seen` and call `Campaign::view_support`.

**For Nick when playing**

- Nothing shows yet (no screens until 1003). In the debug Quick Battle the
  test lord, knight and mage now build points, but with no rank viewed
  the Hit/Avoid numbers are unchanged.
