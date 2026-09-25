# Death, rewind, difficulty and saving

Decided: 2026-09-25
Source: ticket 0006

## Nick's words

> **Q1. What happens when a unit falls?** "1D" (the player picks Classic or
> Casual at New Game).
>
> **Q2. Undo / rewind** "2A but charges can vary by our perceived difficulty of
> battle, and if you restart a battle it refunds the charges"
>
> **Q3. Difficulty modes** "3B to start, we should just tune for one difficulty
> to start and can add a hard or merciless difficulty later"
>
> **Q4. Saving** "4A" (FE: save between chapters + a suspend save mid-battle
> that's deleted on load).

Claude then stated the recorded rules back (lord falling is game over in both
modes; per-map charges starting at 2 / 3 / 5; restart refunds all charges;
3 save slots) and Nick did not veto them.

## Falling units

A unit falls when its HP reaches 0.

**Game mode**, picked on a screen right after `New Game`:

| Mode | What happens to a fallen player unit |
| ---- | ------------------------------------ |
| **Classic** | Gone for good. It plays its death quote (0705), leaves the map, and is removed from the roster when the battle's result is applied. |
| **Casual** | Retreats. It plays its retreat line (same trigger slot as a death quote), leaves the map, and is back in the roster for the next chapter. |

- Within a battle both modes behave the same: the unit leaves the map, can't
  act, can't be targeted, and doesn't block tiles. The difference only matters
  when the campaign applies the battle result.
- The mode is stored in the campaign (and therefore in every save) and is shown
  on the save slot picker.
- Enemy, ally and neutral units that fall are always removed; the mode only
  affects the player's own units.

**Loss conditions** (checked after every action and at every phase boundary,
per `turn-structure.md`), in **both** modes:

1. Any player unit marked as a **lord** falls → game over.
2. Every player unit on the map has fallen → game over.
3. A map's turn limit runs out (`turn-structure.md`).

Game over offers `Retry` (restart the battle) or `Title`.

## Rewind

Limited rewind, like Three Houses' Divine Pulse and Echoes' Turnwheel, plus the
standard FE "cancel your move before you choose an action".

- **Cancel before commit:** a moved unit can be put back freely until an action
  is chosen (`turn-structure.md`). Free and unlimited; it isn't a rewind.
- **Rewind charges:** each map sets its own number of charges, based on how hard
  the battle is meant to be. Starting values (*tunable*, chosen by Claude):

  | Map difficulty | Charges |
  | -------------- | ------- |
  | Easy | 2 |
  | Normal | 3 |
  | Hard | 5 |

  Chapter files store the number directly (`rewind_charges`), not the
  difficulty label, so any map can be given any count.
- A rewind jumps back to **any earlier action** in the current battle (either
  side's) and costs **1 charge**, however far back it goes.
- Luck is part of the saved state: repeating the same actions after a rewind
  gives the same results. Doing something different changes the outcome.
- Charges are per battle. Unused charges don't carry over to the next map.
- **Restarting a battle refunds every charge.** A restart (Game Over → `Retry`,
  or `Restart battle` in the map menu, with a confirm) puts the battle back at
  its first turn with the map's full charge count.
- Rewind works the same in Classic and Casual.

## Difficulty

**One difficulty, tuned well.** There is no difficulty picker. Hard or Merciless
modes may be added later (the roadmap lists difficulty modes as post-Chapter 1).
When that happens, a new decision ticket is needed.

## Saving

FE style: chapter saves plus a one-time suspend.

- **3 save slots** (*tunable*, chosen by Claude). After every chapter victory:
  "Save your progress?", then a slot picker showing chapter title, mode, roster
  size and playtime, with an overwrite confirm.
- `Load Game` on the title screen starts the saved campaign at the beginning of
  its next chapter.
- **Suspend:** `Suspend` in the map menu saves the whole battle (including rewind
  history and charges left) to a single suspend save and returns to the title.
  The title then shows `Continue`. Continuing **deletes** the suspend save, so it
  can't be reloaded to undo a turn.
- No saving anywhere else mid-battle.

## Open sub-questions (deferred)

- Can the mode be changed after New Game (e.g. Classic → Casual, like modern FE)?
  Until decided: no, the mode is fixed for the playthrough.
- What happens to a Classic-dead unit's equipped items (lost, or sent to the
  convoy)? Until decided: they go to the stock.
- Whether a Casual retreat has any cost (e.g. no EXP for that battle). Until
  decided: none.
- Harder difficulty modes: after Chapter 1 (see Difficulty).
