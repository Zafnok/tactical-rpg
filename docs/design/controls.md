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
- Individual keys stay rebindable in Options (ticket 0805), on top of the
  chosen layout.

### Bindings

| Action | Right-handed | Left-handed | Notes |
| ------ | ------------ | ----------- | ----- |
| Move cursor | arrow keys | `W A S D` | Held keys repeat |
| Confirm / select | `F` | `J` | Hold to fast-forward text and animations |
| Cancel / back | `D` | `K` | |
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

### End turn: double-tap Space

- `Space` opens the end-turn prompt (`End turn with N units ready?`).
  `Space` again ends the turn; Confirm also accepts and Cancel backs out.
- As `turn-structure.md` already says, when no units are ready there's
  nothing to warn about, so `Space` ends the turn at once.

### Cursor speed

- **No fast-cursor key** for now: no "jump ×5" and no hold-to-scroll-faster.
  Held keys repeat (first repeat after 170 ms, then every 55 ms, *tunable*),
  and Next/Previous unit jump straight to your units.
- **Revisit after playtesting** a chapter with large-map fights (0804 or
  later). If crossing big maps feels slow, the options were: hold a key to
  scroll faster (FE's hold-B), or jump several tiles per press.

## Open sub-questions

- Fast cursor movement: revisit after a large-map playtest (above).
