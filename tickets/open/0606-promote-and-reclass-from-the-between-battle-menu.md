---
id: "0606"
title: Promote and reclass from the between-battle menu
type: feature
milestone: M5 Progression
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0603", "0801", "0408"]
nick_input: answer-first
completed:
---

# 0606 — Promote and reclass from the between-battle menu

## Context

Ticket 0603 built the promotion and reclass rules
(`trpg_core::progression::class_change`: `promote`, `reclass`,
`promotion_targets`, `reclass_targets`) and the class-choice screen
(`crates/ui/src/screens/class_change.rs`, `ClassChangeScreen`). Nick decided
there that **no seal works in a battle**: a unit promotes or changes class
**between battles** only (`docs/design/progression.md`, *Promotion* and
*Reclass*; seals live in the party's stock).

No between-battle menu existed then, so the screen is only reachable from
the debug menu (`ClassChangeScreen::demo`, a test knight with one of every
seal), and it works on its own copy of a unit and a stock: nothing it
changes is kept. This ticket puts it in the real game: the campaign's
roster and stock (`core::campaign::Campaign`, ticket 0801), opened from a
between-battle menu.

## Nick input

**Answer first** (ask with the `ask-nick` skill, in this ticket, before
writing code): **where does the player promote and reclass?** Offer real
examples, e.g. a `Promote` / `Class change` entry on each unit in the
Preparations screen's unit list (FE GBA's preparations "Items → use"),
a camp activity (Three Houses' class change in the menu between battles),
or both. Chapter 1 has no Preparations screen (`chapter-1.md`), so also ask
whether Chapter 1's "between battles" (after the victory scenes, before the
next chapter) needs the entry, or whether nobody can promote that early
anyway (no unit masters a class in Chapter 1, `progression.md`).

**Sign-off:** Nick promotes and reclasses a unit from the menu and checks
the change is still there in the next battle.

## Scope

**In:**
- The menu entry Nick chooses, shown per unit, enabled when the change is
  possible.
- `ClassChangeScreen` reads and writes the campaign's unit and stock.

**Out (do not do):**
- Any way to use a seal during a battle (Nick: "cannot be used in battle").
- Where seals come from (shop lists, chests, story rewards): chapter and
  shop data.
- Changing the rules in `class_change.rs` or the screen's layout.

## Implementation steps

1. Run the `ask-nick` question above and record the answer in
   `docs/design/progression.md` (replace the *Which between-battle menu…*
   line under *Open sub-questions*).
2. Give `ClassChangeScreen` its result a way back to the caller. Today
   `ClassChangeScreen::new(ctx, kind, unit, stock)` takes copies and
   `unit()` / `stock()` read them, but a popped screen is dropped. Follow
   whatever 0801 uses to hold the `Campaign` (e.g. in `Ctx`): build the
   screen for a roster index, and when a change is made (`Stage::Result`
   reached in `apply`), write the unit and the stock back to the campaign.
   Keep `new` for tests and the debug tool.
3. Add the entry in the chosen menu, per unit:
   - `Promote`: enabled when `promotion_targets` is not empty, the unit has
     mastered its class (`has_mastered`) and the stock holds the seal of
     that tier (`Stock::seal(SealKind::Tier(n), items)`). Otherwise dimmed.
   - `Class change`: enabled when the stock holds a Reclass Seal and
     `reclass_targets` is not empty.
   Choosing one pushes the screen; after it pops, the menu shows the unit's
   new class.
4. Keys and help text through `Action`s and the keymap (`keyboard-input`
   skill).
5. Leave the debug menu's two class change tools as they are.

## Acceptance criteria

- [ ] Nick's answer is recorded in `docs/design/progression.md`.
- [ ] Harness: from the menu, promote a unit that has mastered its class with a tier seal in stock → the roster unit's class and stats changed, the seal is gone from the campaign's stock, and the next battle starts with the promoted unit.
- [ ] Harness: reclass a unit → class changed, stats the same, one Reclass Seal used.
- [ ] `Promote` is dimmed without a seal or without mastery; `Class change` without a Reclass Seal.
- [ ] Cancelling the screen leaves the campaign unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the entries' enabled rules.
- Snapshot / integration: Harness tests above; a snapshot of the menu with the entries.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
