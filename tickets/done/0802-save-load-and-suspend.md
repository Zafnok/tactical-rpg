---
id: "0802"
title: Save/load between chapters and mid-battle suspend
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: done
blocked_by: ["0006", "0207", "0801"]
nick_input: answer-first
completed: 2026-10-01
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

- [x] Harness: suspend mid-battle → Continue → state byte-identical (compare serialised) → suspend key deleted.
- [x] Save after victory → Load → next chapter starts with the same roster.
- [x] Corrupt and wrong-version saves show a message, never panic (tests).
- [x] Works on web (manual check: reload page, Continue works).

## Tests required

- Unit: serde round-trip property test for random campaigns/battle states; version handling.
- Harness flows above with `MemoryStorage`.

## Completion notes

**Done.** Saving, loading and suspending work; ADR-0036 records the format.

- **`trpg_core::save`**: `SaveFile { version, campaign, point }`,
  `SavePoint` (`ChapterCleared` / `Battle(BattleHistory)`), `SAVE_VERSION`
  (1) and `SaveHeader` (reads just the version of any save).
- **`trpg_ui::save`**: RON text under `Storage` keys `slot_01`…`slot_30` and
  `suspend`; `SaveError` (`Incompatible`: "This save is from an incompatible
  version"; `Corrupt`: "This save can't be read"); what each slot holds for
  the picker.
- **Screens** (`screens/save.rs`): "Save your progress?" (`Yes` / `No`) and
  the slot picker (chapter title, Classic/Casual, army size, playtime;
  "Overwrite slot NN?" before replacing one). The picker takes a purpose
  (save or load) and reports a typed result, so 1007's world map menu can
  open it.
- **Flow** (`ui::flow`): the prompt after a chapter's victory scenes; the
  map menu's `Suspend` writes the suspend save and returns to the title;
  `FlowScreen::resume` (title `Continue`) and `FlowScreen::load_game`
  (title `Load Game`).
- **Title**: `Continue` (only while a battle is suspended), `New Game`,
  `Load Game`, `Quit`.

**Acceptance**

- Suspend → Continue → same battle to the byte, rewind points and charges
  too, and the suspend save is gone: `tests/save.rs`
  (`suspend_then_continue_restores_the_battle_exactly`,
  `continue_works_in_the_next_launch`) and core's property test
  (`a_suspend_save_round_trips_through_ron`).
- Save after victory → Load → next chapter, same roster:
  `save_after_victory_then_load_starts_the_next_chapter`.
- Corrupt and wrong-version saves show a message, never panic:
  `saves_that_cant_be_read_show_a_message` and unit tests in `ui::save`.
- Web, checked by hand in the browser pane on a `cargo xtask web
  --debug-tools` build: Quick Battle → map menu → `Suspend` → the title
  shows `Continue` and `localStorage` holds `tactical-rpg/suspend` (about
  8 KB) → reload the page → `Continue` → the battle is back and the key is
  gone.

**Deviations from the ticket**

- `point: SavePoint` instead of `battle: Option<BattleSave>`: the same
  information, with room for the world map's save point (1007) and no
  "slot with a battle in it" to police. See ADR-0036.
- No `saved_at_playtime`: the campaign already has `playtime_s`.
- A chapter save keeps the chapter just **cleared** and looks up the next
  one when loading, so a save made at the end of the content carries on
  when a later chapter exists.
- The suspend save doesn't store the battle's setup; `Continue` rebuilds it
  from the saved campaign and the battle file, so `Restart Battle` after a
  `Continue` still gives turn 1.

**Claude's starting rules** (the design doc was silent; Nick can veto any)

1. **Title menu order.** `Continue` is at the top and highlighted, and only
   there while a battle is suspended. `Load Game` is under `New Game`, and
   greyed out until some slot has a save.
2. **Suspend doesn't ask "are you sure?".** Choosing it saves and goes to
   the title at once; `Continue` puts you straight back.
3. **One suspended battle at a time.** Suspending again replaces the
   earlier one. Starting a `New Game` or loading a slot doesn't remove it:
   `Continue` is still there afterwards.
4. **"Save your progress?" is `Yes` / `No`.** `No` goes on without saving.
   Backing out of the slot list returns to the question. It is asked after
   the last chapter too; loading that save shows "To be continued" until a
   later chapter exists, then starts it.
5. **What a slot shows.** The title of the chapter you will play next
   (after the last chapter: "Test Chapter (cleared)"), the mode, "3 units",
   the playtime as `1:02:05`. An empty slot reads "Empty". 20 of the 30
   slots fit on screen; the list scrolls.
6. **Where the slot list opens.** Saving: on the slot you last used this
   session, else the first empty one. Loading: on the first slot with a
   save.
7. **A save that can't be read is never deleted.** Its slot says why and
   can be saved over (after the overwrite question); a suspend save that
   can't be continued stays until the next `Suspend` replaces it.
8. **After `Continue`** the cursor starts on the lead and the danger zone
   is off; only the battle itself is saved, not where you were looking.
9. **If saving fails** (a full disk), `Suspend` stays in the battle and
   shows why, and the slot list shows why and stays open.
10. The debug Quick Battle asks "Save your progress?" too (it runs through
    the same flow).

**For Nick to try** (Pages build): New Game → win the test chapter → `Yes`
→ pick a slot → back at the title, `Load Game` → the slot. And: Quick
Battle → move a unit, rewind once → map menu → `Suspend` → `Continue`: the
same battle, the rewind charge still spent. Reload the page before
`Continue` to see it survive.

**Follow-up tickets**

- **0821** Save format guard: a golden save that fails the build when a
  saved type changes without raising `SAVE_VERSION`. 0901 (itch.io) is now
  blocked by it.
