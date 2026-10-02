# ADR-0039: Saves are versioned RON `SaveFile`s; a battle is saved as its history

- **Status:** Accepted
- **Date:** 2026-10-01
- **Related tickets:** 0802, 0207, 0307, 0801, 0821, 1007, 0903

## Context

Ticket 0802 adds the saves `death-and-difficulty.md` decided: chapter saves
in 30 slots, and a one-time suspend save of a battle in progress. Forces:

- **`Storage`** (0207) keeps small named texts: files on native,
  `localStorage` on web (a few MB in all).
- **`Campaign`** (ADR-0035) and **`BattleState`** / **`BattleHistory`**
  (ADR-0020, 0307) already serialise with serde, without the content
  tables.
- **A suspended battle must continue exactly**, with its rewind points and
  the charges left, and `Restart Battle` after it must still give the
  battle's first turn.
- **Formats change.** A build must never crash on a save it can't read,
  whether it is damaged or from another version. Migrations are out of
  scope for now.
- **The world map (1007)** will save from its own menu into the same slots,
  at a place that is neither "between chapters" nor "in a battle".

## Decision

1. **One type, in `core`** (`trpg_core::save`): `SaveFile { version,
   campaign, point }`. `point: SavePoint` says where in the campaign the
   save was made:
   - `ChapterCleared`: `campaign.chapter` is won and applied to the army;
     loading goes on with that chapter's `next` (looked up in the content
     when loading, so a save made at the end of the content continues once
     a later chapter exists).
   - `Battle(BattleHistory)`: the suspend save. `campaign` is the one the
     battle started with.

   A new place to save (the world map) is a new `SavePoint` variant.
2. **A battle is saved as its `BattleHistory`**: the first state, the
   commands since, the charges left. Loading replays the commands
   (`state_at`), which is what rewind already relies on, so the continued
   battle is identical to the byte and keeps its rewind points. The
   battle's `BattleSetup` (for restarts) is not saved: it is rebuilt from
   the saved campaign and the battle file, exactly as when the battle
   started. The loader reattaches the content tables
   (`BattleHistory::restore_tables`, ADR-0020).
3. **Text and keys** (`trpg_ui::save`): compact RON (`ron::to_string`),
   under `Storage` keys `slot_01` … `slot_30` and `suspend`.
4. **Versions.** `SAVE_VERSION: u32` (now 1) is written in every save and
   must be raised whenever a saved type changes shape or meaning. Reading
   parses a `SaveHeader { version }` first (serde ignores the other
   fields), so a save from another version is recognised even when the rest
   no longer parses: `SaveError::Incompatible` ("This save is from an
   incompatible version"). Text that isn't a save, a save that doesn't
   parse, a slot holding a battle, or a suspend save whose chapter or
   battle the content lacks is `SaveError::Corrupt` ("This save can't be
   read"). Neither is deleted or changed; a slot can be overwritten.
5. **Where saves are made and read**: the game flow (`ui::flow`,
   ADR-0035). It hosts "Save your progress?" and the slot picker
   (`screens::save::SlotPickerScreen`, which takes a purpose, save or
   load, and reports a typed result, so other menus can open it), writes
   the suspend save when the battle screen closes for `Suspend`, and
   builds itself from a save (`FlowScreen::load_game`,
   `FlowScreen::resume`). `resume` deletes the suspend save once it has
   been read successfully.
6. **Playtime** is `campaign.playtime_s`; the ticket's separate
   `saved_at_playtime` field would only repeat it.

## Consequences

- Saves are small: a chapter save is the roster and stock (a few KB); a
  suspend save adds one map and the commands (about 8 KB for the Quick
  Battle), well within `localStorage`.
- A content change after a suspend can make the replay differ from what the
  player saw (ADR-0020 already accepts this for a one-time save); replay
  never panics, because a refused command changes nothing.
- Every change to `Campaign`, `Unit`, `BattleState`, `Command` or anything
  inside them must raise `SAVE_VERSION`, and old saves then show as
  incompatible until a migration ticket says otherwise. Fields added with
  `#[serde(default)]` are the one change that could keep old saves
  readable; that still needs a deliberate decision, not an accident.
- `core` has no test that fails when a saved type changes without a version
  bump. Ticket 0821 adds a golden-file test for that, before saves reach
  players (0901).
- Steam Cloud (0903) can sync the same keys as files.

## Alternatives considered

- **`battle: Option<BattleSave>` next to the campaign** (the ticket's
  sketch) — two fields whose combinations must be policed (a slot with a
  battle?), and no room for the world map's save point.
- **Saving the current `BattleState` instead of the history** — smaller,
  but the rewind points are lost, and the design says the suspend save
  keeps "rewind history and charges left".
- **Saving the `BattleSetup` too** — it carries every content table by
  `Arc`; ADR-0020 keeps content out of saves.
- **A binary format (bincode, postcard)** — smaller and faster, but a new
  dependency, unreadable when a player sends a broken save, and RON is
  already what the tests round-trip.
- **Version in the storage key or a separate key** — the save and its
  version could then disagree or be copied apart.
