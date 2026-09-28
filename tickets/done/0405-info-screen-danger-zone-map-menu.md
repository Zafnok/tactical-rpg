---
id: "0405"
title: Unit info screen, danger zone, map menu, end turn, phase banners, victory/defeat
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: done
blocked_by: ["0404"]
nick_input: none
completed: 2026-09-28
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

- [x] All five features reachable with default keys; help bar updated.
- [x] Danger zone equals `danger_zone()` output (test).
- [x] Harness: play a full player turn with Wait on all units → confirm end turn → banner → next player phase `Turn 2`.
- [x] Snapshots for info screen, map menu, banner, danger zone.

## Tests required

- Harness + snapshot as above; unit tests for menu enable/disable logic.

## Completion notes

Done. Everything lives in the battle screen (`crates/ui/src/screens/battle/`):
new modes in `mode.rs` (map menu, unit list, objective, end-turn prompt, info),
`map_menu.rs`, `info.rs`, `banner.rs`; tests in `turn_tests.rs` and
`crates/ui/tests/battle.rs`.

- **Keys** (from the keymap, right-handed): `e` info, `w` danger zone, `Space`
  end turn (again to confirm), `Shift+Space` auto-end, `d` with nothing to
  cancel or `f` on an empty tile opens the map menu. The ticket's `s`/`a`
  letters were older placeholders; `controls.md` wins.
- **Help bar:** the key line now ends `d menu · Space end turn`; the toggles'
  state (`w danger zone: OFF · Shift+Space auto-end: ON`) is right-aligned on
  the message row, where the `Auto-end: ON/OFF` message also appears.
- **Enemy phase stub** (until 0502): when the ENEMY / OTHER PHASE banner
  closes, that phase ends at once.
- **Leaving the Quick Battle:** Cancel no longer returns to the title (it
  opens the map menu, per `controls.md`); the battle now ends only on
  VICTORY / DEFEAT. Suspend stays a disabled placeholder until 0802.
- No PLAYER PHASE banner at the very start of a battle: the screen is built
  from a state, not its start events; the real battle flow (0801) can show it.

*Claude's starting rules* (the design docs were silent; Nick may veto):

1. Info screen cycles the unit's faction in reading order (top to bottom,
   left to right); closing it leaves the cursor on the last unit shown. It
   also lists learned spells with uses left, and shows base stats (without
   gear bonuses) with the class cap dimmed.
2. Objective wording: `Rout the enemy`, `Defeat <name>`, `Seize the <terrain>`,
   `Survive N turns`; Survive maps show `Turn 3/8` like a turn limit.
3. The ENEMY PHASE banner still shows (1 s) even though enemies don't act yet.
4. `Auto-end: ON/OFF` message stays 1.5 s.
5. Turning auto-end back on after every unit has acted does not end the
   turn; the next time the last unit acts (or `Space`) does.
6. `f` on an empty tile opens the map menu even while an enemy's range is
   shown (the range is hidden).
7. The danger zone can be toggled while browsing, with a unit selected, or
   while choosing where to move after an attack.
8. Banner colours: phases in their faction colour (Other = ally green),
   VICTORY in the highlight colour, DEFEAT in the low-HP red.
9. The map menu opens beside the cursor, like the action menu; the unit list,
   objective and end-turn question are centred on the map.

