---
id: "0828"
title: "A switch to Casual reaches the battle under way (retreat lines, not death quotes)"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0805", "0705"]
nick_input: none
completed:
---

# 0828 — A switch to Casual reaches the battle under way

## Context

The Options screen (0805) lets a Classic campaign switch to Casual at any
time, also in the middle of a battle (`docs/design/death-and-difficulty.md`,
*Mode changes*). The switch changes the campaign, so a unit that falls
later in that battle **does** come back for the next one, and a restarted
battle starts in Casual. But the battle already under way keeps the mode it
started with for one thing: its triggers. `BattleState` holds the mode
(`BattleSetup::mode`, `crates/core/src/battle/triggers.rs`) and picks a
fallen unit's line by it, a Classic death quote or a Casual retreat line
(0705). So after a mid-battle switch a unit can say its death quote and
then be back in the next battle.

0805 left this alone because changing a running battle's mode means a new
`Command` in `core` (state changes only through commands, ADR-0004), which
was outside that ticket.

## Nick input

None. The rule is already decided: the mode is the campaign's, and Casual
means a retreat line.

## Scope

**In:**
- A `core` command that switches the battle to Casual (one way), recorded
  in the battle's history like any other, so a rewind to before it is
  Classic lines again only if the design says so (see step 2).
- The game flow sends it when the Options screen switches the mode during
  a battle.

**Out (do not do):**
- Any change to what Casual does (`death-and-difficulty.md`).
- Casual → Classic.

## Implementation steps

1. `core`: `Command::SwitchToCasual` (or a field on an existing command if
   that fits better): sets the battle's mode; an `Event` for it; refused in
   a battle already Casual. Unit and property tests (determinism, replay).
2. Rewind: decide technically whether the command is a rewind point. The
   campaign stays Casual whatever the player rewinds to, so the battle
   should too: the simplest way is to apply the switch to the history's
   starting setup instead of recording a command. Pick one, say why in the
   completion notes.
3. `crates/ui/src/flow.rs`, `sync_mode`: when the campaign is downgraded
   while a battle is on, tell the battle screen, which applies step 1 or 2.
   The suspend save must then hold a Casual battle too.
4. Tests below.

## Acceptance criteria

- [ ] After switching to Casual mid-battle, a player unit that falls plays
      its retreat line, not its death quote (Harness test on a battle with
      both lines).
- [ ] A rewind after the switch keeps Casual lines.
- [ ] Suspend → Continue after the switch is still Casual, in the campaign
      and in the battle.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `core` command or setup change; refused or a no-op in Casual.
- Property: replaying a history with the switch gives the same battle.
- Snapshot / integration: the Harness tests in the acceptance criteria.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
