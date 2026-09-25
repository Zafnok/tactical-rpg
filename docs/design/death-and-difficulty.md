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

Follow-ups (Nick asked to decide every lever himself: "don't decide these game
design levers yourself. Let's go through them."):

> **Classic: a dead unit's gear** "1B" (it goes back to the stock).
>
> **Casual: cost of retreating** "A" (nothing beyond missing the rest of that
> battle).
>
> **Switching mode after New Game** "3A, and same with Hard->Normal if we
> implement hard mode in the future." (Classic → Casual only, one way.)
>
> **Charges per map** "4C but more tiers of map difficulty like 2 on easy maps,
> 3 on normal, 5 on hard, 8 on big and hard (i.e. finale)"
>
> **How far back a rewind goes** "5A" (any earlier action in the battle,
> enemy actions included).
>
> **Restarting** "6A" (from the map menu at any time).
>
> **Save slots** "7C" then "let's say 30 save slots"
>
> **Unused charges** "8C but rather than gold maybe a small level xp bonus to
> each surviving unit (not crazy, it shouldn't need to be expected to never use
> them, but just a way to reward good play)"
>
> **Size of that bonus** "Again I don't know about exact numbers. But I think
> probably something small, not enough to be meta-gamey. A or B, probably in
> between the two. C is overstepping" (A = 5 EXP per charge, B = 10, C = 20.)
>
> Then: "The one thing I want to adjust is that the 7 is more like a
> percentage of level up than a fixed number. If we decide to go dragon ball
> scaling maybe it takes 1M xp to lv up and each charge gives 70K. But for now
> until I get a feel for numbers in the display then we can go with grounded
> numbers i.e. 7"
>
> **Who gets the bonus** "11B" (every deployed unit, including Casual retreats).
>
> **Lord falls in Casual** "9A" (game over in both modes).

## Falling units

A unit falls when its HP reaches 0.

**Game mode**, picked on a screen right after `New Game`:

| Mode | What happens to a fallen player unit |
| ---- | ------------------------------------ |
| **Classic** | Dies. It plays its death quote (0705), leaves the map, and is removed from the roster when the battle's result is applied. Its equipped weapons and items go to the stock. |
| **Casual** | Retreats. It plays a retreat line (same trigger slot as a death quote), leaves the map, and is back in the roster at full HP for the next chapter. It keeps all EXP and weapon progress earned before falling. No other cost. |

- Within a battle both modes behave the same: the fallen unit leaves the map,
  can't act, can't be targeted and doesn't block tiles. The difference only
  matters when the campaign applies the battle result.
- **Mode changes:** Classic → Casual is allowed at any time (options menu);
  Casual → Classic never. If harder difficulties are added later, the same
  one-way rule applies (e.g. Hard → Normal only).
- The mode is stored in the campaign, so it's part of every save.
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
- **Rewind charges per map**, by the map's difficulty tier (Nick):

  | Map tier | Charges |
  | -------- | ------- |
  | Easy | 2 |
  | Normal | 3 |
  | Hard | 5 |
  | Finale (big and hard) | 8 |

  Each chapter file names its tier; the game looks up the charge count.
- A rewind jumps back to **any earlier action** in the current battle, enemy
  actions included, and costs **1 charge** however far back it goes.
- Luck is part of the saved state: repeating the same actions after a rewind
  gives the same results. Doing something different changes the outcome.
- **Restart:** `Restart battle` in the map menu (with a confirm), available at
  any time, and `Retry` on Game Over both put the battle back at its first turn
  and **refund every charge**.
- Rewind works the same in Classic and Casual.

### Unused charges

Charges don't carry over to the next map. Instead, each unused charge gives a
**small EXP bonus** when the battle is won:

- Every **deployed** player unit gets it: units still standing *and* units that
  retreated in Casual. (Classic-dead units are gone; undeployed units get
  nothing.)
- Size (Nick): small, "not enough to be meta-gamey", and defined as a
  **share of one level**, not a fixed number: **7% of the EXP needed for a
  level per unused charge**. With today's 100 EXP per level that is **7 EXP**;
  if the EXP scale grows (e.g. 1,000,000 per level) the bonus scales with it
  (70,000). The percentage may be revisited once Nick sees the numbers on
  screen (0804).
- The bonus is added after the battle's normal EXP, following the award rules
  in `progression.md` (max 100 per award, level cap).

## Difficulty

**One difficulty, tuned well.** There is no difficulty picker. Hard or Merciless
may be added later, with a new decision ticket; a downgrade then works like
Classic → Casual (one way only).

## Saving

FE style: chapter saves plus a one-time suspend.

- **30 save slots** (Nick). After every chapter victory: "Save your progress?",
  then a slot picker showing chapter title, mode, roster size and playtime,
  with an overwrite confirm.
- `Load Game` on the title screen starts the saved campaign at the beginning of
  its next chapter.
- **Suspend:** `Suspend` in the map menu saves the whole battle (including rewind
  history and charges left) to a single suspend save and returns to the title.
  The title then shows `Continue`. Continuing **deletes** the suspend save, so it
  can't be reloaded to undo a turn.
- No other saving mid-battle.

## Open sub-questions (deferred)

- Whether 7% of a level per unused charge feels right: check at the Chapter 1
  playtest (0804).
- Harder difficulty modes: after Chapter 1.
