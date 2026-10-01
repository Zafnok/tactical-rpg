# ADR-0034: Battle and chapter files, a core `Campaign`, and one flow screen that hosts the others

- **Status:** Accepted
- **Date:** 2026-09-30
- **Related tickets:** 0801, 0802, 0803, 0408, 0810, 1007, 1008

## Context

Ticket 0801 turns the separate pieces (battle screen, dialogue screen,
triggers, rewind) into a game: New Game, a mode and a lead, then chapters of
scenes and a battle, carrying the army from one battle to the next. Forces:

- **Adding a battle must need only data** (a map, a battle file, scenes).
  Battles outlive linear chapters: the world map's story battles, skirmishes
  and side quests (1007/1008, `world-structure.md`) are battles that aren't
  one per chapter.
- **Rules stay in `core`** (ADR-0004): what a won battle does to the army
  (Classic deaths, Casual retreats, the unused-rewind EXP bonus, recruits,
  pack items and gold) is game rules, and must be deterministic and saved
  (0802).
- **The screen stack has no return values** (ADR-0017): a screen pops, and
  the one below carries on without knowing what happened. The flow must
  read results (the mode picked, the lead, the battle's outcome and final
  state, Retry or Title).
- **Restarts must be exact**: `Restart Battle` and `Retry` put the battle
  back at its first turn with every rewind charge
  (`death-and-difficulty.md`).

## Decision

1. **Two file kinds.** `assets/battles/<id>.ron` holds everything to play
   one battle (map id, player slots by character, enemies from generic
   templates or characters, reinforcements, pack, clear gold, objective,
   triggers, difficulty tier, seed); `assets/chapters/<id>.ron` holds the
   story beat around it (title, intro scenes, battle id, victory scenes,
   next chapter). `assets/data/new_game.ron` names the first chapter and the
   starting roster, gold and stock. `trpg_content::battle` and
   `trpg_content::chapter` load and validate them with every other asset,
   reporting every error (ADR-0005); the formats are documented in
   `assets/battles/README.md` and `assets/chapters/README.md`.
2. **`core::campaign`.** `BattleDef` (a validated battle file, enemies
   already built as units), `Difficulty` (tier → rewind charges),
   `GameTables` (the shared `Arc` tables) and `Campaign` (serde: mode, lead,
   chapter, roster, stock, gold, flags, playtime). `Campaign::battle_setup`
   builds a `BattleSetup`; `Campaign::apply_result(def, state,
   unused_charges)` applies a won battle and returns `BattleRewards` for the
   results screen (0810). Unit ids: player slot `i` is `i + 1`, enemies and
   reinforcements are numbered after the slots, so objectives and triggers
   never need remapping.
3. **One flow screen owns the others.** `ui::flow::FlowScreen` sits on the
   stack once and holds the current screen itself (`Stage`: mode, lead,
   scene, battle, Game Over, "To be continued"). It updates and draws that
   screen, passes on anything it pushes (a battle's scene overlays), and
   when it pops, reads its typed result (`ModeSelectScreen::result`,
   `BattleScreen::restart_requested`, the battle's state, …) and moves on.
   Its `name()` is the hosted screen's, so the stack reads
   `["title", "battle"]` as before. The flow keeps the battle's
   `BattleSetup`; both restarts rebuild the battle from it.
4. **Tests reach into the stack through `Screen::as_any`.** A screen may
   opt in (`FlowScreen` does); `ScreenStack::find`/`Game::screen` downcast
   it, so Harness tests play a battle with scripted commands
   (`BattleScreen::send`, test and `harness` builds only).
5. **Typing text** (the lead's name; Nick, PR #127: type on a keyboard, a
   letter grid on a controller). `app` passes the platform's typed
   characters as `RawKeyEvent::Text` (macroquad `get_char_pressed`, so the
   keyboard's own layout and Shift apply); `Game` gives a screen this
   frame's printable characters (`FrameInput::text`) and pressed chords
   (`FrameInput::pressed_chords`). A text box ignores actions while open
   (every letter key is a letter then) and finishes, deletes and cancels on
   fixed Enter / Backspace / Escape (`input::text_key`, named in help text
   only by `input::text_keys_help`, like the fixed Escape for Cancel).

## Consequences

- A new chapter or battle is data only; the all-assets test validates it.
- The Quick Battle is a battle file and a one-battle chapter
  (`quick`), played through the same flow.
- 0802 saves `Campaign` (and a suspended battle's history) and adds the
  save prompt where the flow ends a chapter; 0408 pushes Preparations from
  the flow and drops the validator's `preparations` refusal; 0810 shows
  `BattleRewards`; 1007/1008 start battles from the world map with the same
  `battle_setup`.
- The flow is one more place that knows screen types. A new stage means a
  `Stage` variant and a line in `FlowScreen::advance`.
- Screens other than the flow's may not rely on popping to report a result;
  the flow reads its hosted screens' results directly.

## Alternatives considered

- **Results through a mailbox in `Ctx`** (a screen writes its result, the
  one below reads it on its next update): untyped, easy to leave stale, and
  a battle's whole final state would sit in shared context.
- **Each flow step as its own stack screen that replaces itself with the
  next** (mode → lead → scene → …): every screen would need to know what
  comes after it and carry the campaign along; retries and chapter
  sequencing would be spread over many screens.
- **One file per chapter holding its battle inline** (the ticket's first
  sketch): battles on the world map and skirmishes aren't one per chapter,
  so the battle would have to be split out later anyway.
- **Keeping `apply_result` in `ui`**: the rules would escape `core` and its
  tests, and saves (0802) would depend on the UI crate.
