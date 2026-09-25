---
id: "0805"
title: "Options menu: speeds, animations, fullscreen, key rebinding"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0405", "0207", "0801"]
nick_input: none
completed:
---

# 0805 — Options menu

## Context

Players need to tune text speed, animation speed and keys. Vim-style keys are
an opinion ([ADR-0006](../../docs/adr/0006-input-actions-and-virtual-cursor.md));
rebinding makes it safe.

## Nick input

None.

## Scope

**In:** `Settings` struct persisted via `Storage` key `settings`, Options
screen reachable from title and map menu, key rebinding UI.

**Out:** audio volume (no audio yet), controller bindings.

## Implementation steps

1. `Settings { version, text_speed: Slow|Normal|Fast|Instant, anim_speed: Normal|Fast, combat_animations: On|Off, enemy_phase_speed: Normal|Fast, auto_end_turn: bool (default true, per `docs/design/turn-structure.md`; also toggled by the `ToggleAutoEnd` key in battle, and that toggle is saved too), fullscreen: bool, key_overrides: BTreeMap<Action, Vec<Chord>>, reset_tips }`.
   Defaults match current behaviour. Loaded at startup into `Ctx`; saved on change.
2. Wire each setting into its consumer (0704 typewriter, 0404 playback, 0502
   pacing, `app` fullscreen via a `FrameOutput` request flag).
3. **Options screen:** list of settings; `h/l` changes value; `f` on "Key
   bindings" opens the rebinding screen; "Reset tips"; "Restore defaults".
4. **Rebinding:** list actions with current chords; `f` → "Press a key…" →
   captures next chord (Esc cancels capture); conflicts: show which action has
   it and ask to swap; "Reset to defaults". The effective keymap = defaults +
   overrides, validated by the same code as 0204.
5. **Game mode** (only when a campaign is loaded, per
   `docs/design/death-and-difficulty.md`): shows `Classic` or `Casual`; in
   Classic, `Switch to Casual` asks for confirmation ("This can't be undone")
   and calls `Campaign::downgrade_mode()` (0801). No way back to Classic.
6. Enable `Options` in the map menu and title.

## Acceptance criteria

- [ ] Every setting changes behaviour (Harness test per setting where observable).
- [ ] Classic → Casual switch works with a confirm; Casual never offers Classic (Harness test).
- [ ] Rebinding works, persists across restart (MemoryStorage round-trip test), conflicts handled.
- [ ] Snapshots of both screens.

## Completion notes

