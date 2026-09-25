---
id: "0405"
title: Unit info screen, danger zone, map menu, end turn, phase banners, victory/defeat
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0404"]
nick_input: none
completed:
---

# 0405 — Info screen, danger zone, map menu, phases

## Context

The remaining battle-screen essentials so a full player turn can be played and
ended. Bindings from `docs/design/controls.md` ([ADR-0015](../../docs/adr/0015-input-actions-and-keymap-layouts.md)).

## Nick input

None.

## Scope

**In:** unit info screen (`s`), danger zone (`a`), map menu, end turn (menu and
`e`), phase/turn banner, victory/defeat banners.

**Out:** enemy phase AI playback (0502), options (0805), saving (0802), tips (0406).

## Implementation steps

1. **Unit info screen** (`Info` on any unit, overlay): name, class, level, EXP,
   HP, every stat from the design with its cap shown dim (`Str 7/20`), move,
   movement type, loadout (3 weapons with stats and durability, armour,
   accessory), weapon ranks, class tags (Mounted/Flying/Armored), skills if designed.
   Up/Down or `NextUnit`/`PrevUnit` cycles units of the same faction; `d` closes. Leave a 24×12
   portrait area on the left (placeholder box until 0703 exists).
2. **Danger zone** (`DangerZone` toggle): union of hostile threat areas (0303
   `danger_zone`) blended with `danger_zone` bg under other overlays;
   recomputed after every applied command; state persists across turns until
   toggled; help bar shows `a danger zone: ON`.
3. **Map menu** (`Cancel` in Idle, or `Confirm` on an empty tile): `Units`
   (list of player units with HP and ready/acted; choosing one jumps the
   cursor), `Objective` (objective text, turn number), `Options` (disabled
   placeholder), `Suspend` (disabled placeholder), `End Turn`.
4. **End turn** (per `docs/design/turn-structure.md`): from menu or `EndTurn`
   key (`Space`; per `docs/design/controls.md` a second `Space` on the prompt
   confirms, so double-tap `Space` ends the turn) → if units still ready, confirm dialog
   `End turn with N units ready? f yes / d no` → `Command::EndPhase`; no
   confirmation when no units are ready. **Auto-end:** when ON (default) and
   the last ready player unit acts, issue `EndPhase` immediately. The
   `ToggleAutoEnd` key (0204, default `Shift+Space`) flips it, shows a brief
   `Auto-end: ON/OFF` toast, and the help bar shows its state. Keep the flag
   in the battle screen's context until 0805 persists it in `Settings`.
5. **Phase banner:** on `PhaseStarted`, a centred double-box banner
   `PLAYER PHASE` / `ENEMY PHASE` / `OTHER PHASE` (faction colour) with
   `Turn N` beneath, for 1.0 s or until Confirm. `Objective` in the map menu
   shows the turn limit if the map has one (`Turn 3/8`).
6. **Victory / defeat:** on `BattleEnded`, banner `VICTORY` / `DEFEAT` → on
   confirm, `Transition::Pop` (0801 replaces this with real flow). Until 0502
   exists, ending the player phase makes the enemy phase simply `EndPhase`
   immediately (document this stub; 0502 replaces it).

## Acceptance criteria

- [ ] All five features reachable with default keys; help bar updated.
- [ ] Danger zone equals `danger_zone()` output (test).
- [ ] Harness: play a full player turn with Wait on all units → confirm end turn → banner → next player phase `Turn 2`.
- [ ] Snapshots for info screen, map menu, banner, danger zone.

## Tests required

- Harness + snapshot as above; unit tests for menu enable/disable logic.

## Completion notes

