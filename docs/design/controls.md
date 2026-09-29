# Controls: layouts and key bindings

Decided: 2026-09-25
Source: ticket 0015

## Nick's words

> "I want to be more interactive with our keybinds so pls ask me for each
> action what the key should be or for directionals etc. Also I'm envisioning
> an option for right or left handed play. Right handed might have cursor as
> arrow keys and left handed as wasd"
>
> **Q1. Which layouts should the game offer?** "A - I just used vim as an
> example meaning "no mouse" or "keyboard power user" but exact keybinds I
> don't care about. I think A should work fine."
>
> **Q2. Which layout on first launch?** "C" (ask on first launch)
>
> **Q3. Right-handed confirm / cancel.** "I think we can use the ASDF row for
> common actions. So maybe to start F is select/confirm and D is cancel."
>
> **Q4. A faster way to move the cursor?** "C for right now, with a note it
> might be revisited after playtesting a chapter with larger map fights"
>
> **Q5. Right-handed keys per action.** "confirm F, cancel D, I think cycling
> can take place on A and S, end turn can be space bar, with confirm by
> another spacebar (double tap space ends turn -- kinda satisfying), E can be
> unit info, W can be show danger zone, shift+space can be auto end"
>
> **Q6. Left-handed layout.** "A mirroring what I already said for 5"
>
> **Claude's three small calls** (Esc also cancels; literal finger mirror for
> previous/next unit; Rewind on R / U): "I think the small calls you made
> make sense"

## Rules

**Keyboard only.** The game is played entirely from the keyboard (no mouse
needed). Key choices are Nick's; the defaults below are his.

### Two layouts

- **Right-handed:** the right hand steers with the **arrow keys**; the left
  hand acts from the `A S D F` row.
- **Left-handed:** the left hand steers with **`W A S D`**; the right hand acts
  from the `J K L ;` row, an exact finger-for-finger mirror of right-handed.
- **First launch:** before anything else, a one-screen **"Pick your layout"**
  menu shows both layouts (with a small key diagram) and asks the player to
  choose. The choice is saved and can be changed later in Options.
- Individual keys stay rebindable in Options (tickets 0805, 0809), on top of
  the chosen layout: see *Rebinding keys* below.

### Bindings

| Action | Right-handed | Left-handed | Notes |
| ------ | ------------ | ----------- | ----- |
| Move cursor | arrow keys | `W A S D` | Held keys repeat |
| Confirm / select | `F` | `J` | Hold to fast-forward text and animations |
| Cancel / back | `D` | `K` | Skips a fight's playback (0418) |
| Previous ready unit | `A` | `;` | Finger mirror of `A` (*see note*) |
| Next ready unit | `S` | `L` | Finger mirror of `S` |
| Unit info (stat screen) | `E` | `I` | |
| Enemy danger zone on/off | `W` | `O` | |
| End turn | `Space` | `Space` | Double-tap: see below |
| Auto-end on/off | `Shift+Space` | `Shift+Space` | Replaces the `Shift+e` placeholder in `turn-structure.md` |
| Rewind (ticket 0307) | `R` | `U` | Proposed by Claude, approved by Nick: `R` was already planned; `U` is its mirror |
| Map menu | `Cancel` with nothing to cancel, or `Confirm` on an empty tile | same | Unchanged from before |

*Proposed by Claude, approved by Nick:*

- `Esc` also works as Cancel in both layouts (and so opens the map menu when
  there's nothing to cancel), since players reach for it by habit.
- "Mirror" is taken literally, finger for finger, so in the left-handed
  layout *previous* unit is the right-most key (`;`) and *next* is `L`. If
  that feels backwards, swap them.

### Fight playback: Cancel skips, hold Confirm speeds up

Decided 2026-09-28, ticket 0418, after playing the 0404 build (where a
Confirm tap skipped and holding it sped up):

> "I don't think skipping and fast playback of a fight should be bound to
> same hotkey, probably use the cancel hotkey (i.e. D) to skip and the
> confirm hold key (F) to playback faster"

- **Cancel** (`D` / `K`, or `Esc`) skips the rest of a fight's playback.
- **Holding Confirm** plays it ×4 *(tunable)*. A Confirm tap does nothing.

### End turn: double-tap Space

- `Space` opens the end-turn prompt (`End turn with N units ready?`).
  `Space` again ends the turn; Confirm also accepts and Cancel backs out.
- As `turn-structure.md` already says, when no units are ready there's
  nothing to warn about, so `Space` ends the turn at once.

### Cursor speed

- **No fast-cursor key** for now: no "jump ×5" and no hold-to-scroll-faster.
  Held keys repeat (first repeat after 300 ms, then every 55 ms, *tunable*; the first delay
  was 170 ms until ticket 0421: Nick found a single tap in menus often moved twice),
  and Next/Previous unit jump straight to your units.
- **Revisit after playtesting** a chapter with large-map fights (0804 or
  later). If crossing big maps feels slow, the options were: hold a key to
  scroll faster (FE's hold-B), or jump several tiles per press.

### Rebinding keys

Decided 2026-09-29, ticket 0022. Nick's words:

> "we should allow remapping keys in the options menu. […] 1) never
> hardcode keyboard input, draw from the config which can be set by user
> 2) keyboard mapping screen, take any keyboard input to remap, check it's
> not already taken, if it is, then overwrite but make the one that was
> taken before have a flag like ! not mapped"
>
> "only some keys are necessary to map like cursor, select/confirm, cancel,
> end turn"
>
> "we can have some that are optional like if user wants separate select
> (cursor) and confirm (action) keys, or separate end turn / confirm end
> turn keys (right now both can be space)"
>
> Follow-up answers:
> - Leaving the screen while a required action has no key: **"Block leaving"**.
> - Keys per action: **"3 each, only one needs to be filled out."**
> - Custom keys when switching right/left-handed: **"Each layout keeps its own"**.
> - Backing out of "Press a key…": **"Esc backs out, not bindable"**.
> - Emptying a slot: **"Delete only"**.

**Where:** a **Key bindings** screen opened from Options (ticket 0805).

**Slots.** Every action has **3 key slots**. An action works with any of the
keys in its slots. Only one slot needs a key.

**Required and optional actions.**

| Required (must keep at least one key) | Optional (may have no key) |
| ------------------------------------- | -------------------------- |
| Cursor up, down, left, right | Select *(new, see below)* |
| Confirm | Confirm end turn *(new, see below)* |
| Cancel | Previous / next ready unit |
| End turn | Unit info, danger zone, auto-end on/off, rewind, map menu |

The developer Debug key is not on the screen.

**Binding a key.**

1. Pick an action's slot and press Confirm: the slot shows `Press a key…`.
2. The next key pressed goes in that slot. Any key the game can read counts,
   with or without `Shift`, except `Esc` and `Delete` (below).
3. **If that key is already in another slot, it moves:** the old slot is
   emptied. An action left with no keys at all shows **`! not mapped`**.
   (Same layout only; the other layout's keys are separate.)

**`Esc` is fixed.** `Esc` always backs out of `Press a key…` without
changing anything, and always works as Cancel everywhere, in both layouts.
It can't be put in a slot; the screen shows it as a fixed extra key on
Cancel.

**`Delete` empties** the highlighted slot. `Delete` can't be put in a slot.

**Leaving is blocked** while any *required* action shows `! not mapped`:
backing out of the screen shows a message naming the action and stays put.
Optional actions may be left `! not mapped`; that action then has no key
until the player binds one.

**Each layout keeps its own keys.** Custom keys are saved per layout.
Switching right/left-handed in Options loads that layout's keys (its own
custom keys if it has any, else its defaults); switching back brings the
first layout's custom keys back.

**Optional split keys.** Both start with no key, so the defaults behave
exactly as before:

- **Select (cursor)**: picking things *on the map with the cursor*
  (choosing a unit, its destination tile, a target; Confirm on an empty tile
  opening the map menu). With no key, Confirm does this. Once Select has a
  key, Confirm stops doing it on the map and only accepts menus, prompts
  and the forecast.
- **Confirm end turn**: accepting the end-turn prompt. With no key, pressing
  End turn again accepts it (today's double-tap Space). Once it has a key,
  End turn pressed again no longer accepts; the new key does. Confirm still
  accepts and Cancel still backs out, as before.

**Help bar and tips** show the player's current keys. An action with no key
shows as `! not mapped` there too.

*Claude's starting rules (Nick to veto at sign-off of 0809):*

- `Esc` doesn't count as Cancel's required key: Cancel still needs one of
  its own slots filled, so it stays on the acting hand.
- Emptying a required action's last key with `Delete` is allowed (it then
  shows `! not mapped` and leaving is blocked), the same as losing it to
  another action.
- The blocked-leave message reads `Give <action> a key first`.
- A **Restore defaults** row puts the current layout's default keys back
  (the other layout is untouched).
- Which Select/Confirm presses count as "on the map" is Claude's reading
  of "select (cursor) and confirm (action)" above.
- The Key bindings screen is steered with the keys the player had when
  they opened it; changes take effect when they leave. This way moving
  every cursor key elsewhere can't trap them on the screen.

## Open sub-questions

- Fast cursor movement: revisit after a large-map playtest (above).
