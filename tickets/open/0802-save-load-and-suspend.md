---
id: "0802"
title: Save/load between chapters and mid-battle suspend
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0006", "0207", "0801"]
nick_input: answer-first
completed:
---

# 0802 — Save, load, suspend

## Context

Saving rules from `docs/design/death-and-difficulty.md` (0006): FE-style
chapter saves in 30 slots plus a one-time suspend save. Storage from
0207. Determinism (0305) makes battle saves exact. Later, the world map
(0008, `world-structure.md`) adds `Save` anywhere on the world map using these
same slots; 1007 builds that on top of this format, so keep `Campaign`
serialisation extensible (versioned) and the slot picker reusable from other
menus.

## Nick input

**Answer first:** 0006.

## Scope

**In:** `SaveFile` format with version, slots, suspend, title-menu `Continue`
and `Load Game`, post-chapter save prompt, map-menu `Suspend`.

**Out:** cloud saves, Steam cloud (0903), migrations (first versioned format only).

## Implementation steps

1. `SaveFile { version: u32, saved_at_playtime: u64, campaign: Campaign, battle: Option<BattleSave> }`
   where `BattleSave` = `BattleHistory` (0307) if it exists, else `BattleState`.
   RON-serialised via `Storage` keys `slot_01..slot_30` (30 slots per design) and
   `suspend`.
2. `SAVE_VERSION` const; loading a different version shows "This save is from
   an incompatible version" (no crash). Corrupt data → same style of message.
3. After victory: "Save your progress?" → slot picker (shows chapter title,
   Classic/Casual mode, roster size, playtime per slot; overwrite confirm).
4. Map menu `Suspend` → writes `suspend` → returns to title. Title shows
   `Continue` when `suspend` exists; continuing deletes it (FE rule, per design).
5. Title `Load Game` → slot picker → loads campaign at the start of the next chapter.

## Acceptance criteria

- [ ] Harness: suspend mid-battle → Continue → state byte-identical (compare serialised) → suspend key deleted.
- [ ] Save after victory → Load → next chapter starts with the same roster.
- [ ] Corrupt and wrong-version saves show a message, never panic (tests).
- [ ] Works on web (manual check: reload page, Continue works).

## Tests required

- Unit: serde round-trip property test for random campaigns/battle states; version handling.
- Harness flows above with `MemoryStorage`.

## Completion notes

