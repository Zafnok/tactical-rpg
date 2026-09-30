# Title screen

Decided: 2026-09-30
Source: ticket 0034

## "Press any key" on the web build

Browsers block a page's sound until the player presses a key or clicks, so on
the web build the title music could not start with the title. Nick picked a
keyboard prompt over a click.

### Nick's words

> since this is entirely keyboard driven game, using mouse makes it weird...
> can we have Press Enter or something to start instead? Or space or F or
> something

Asked: which key (A any key / B only the Confirm key), how it looks (A a
line on the title screen, like GBA Fire Emblem's "Press Start" / B a black
screen before the title), web only or Windows too (A web only / B both):

> 1A
> 2A
> 3A for now

### Rules (Nick fixed)

1. **Web build only.** The Windows game opens the title with the menu and
   music straight away, as before. "For now": Nick may revisit it.
2. **Any key starts.** Every key counts, whatever the player's key bindings
   are, so the prompt never names a key.
3. **On the title screen.** The title and subtitle show as normal. Where the
   New Game / Quit menu goes, a single line reads `Press any key`.
4. **The first key press** removes the line, shows the menu in its place and
   starts the title music.

```
                Visions of Shuyi
             an ASCII tactics game


                 Press any key
```

### Claude's starting rules (Nick can veto)

- The key that dismisses the prompt does nothing else: it doesn't also move
  the menu cursor or choose New Game.
- The line is in the dim text colour, like the subtitle, and doesn't blink.
- The bottom help line (move / select / back) is hidden while the prompt
  shows, since those keys do nothing yet. It appears with the menu.
- A mouse click does not dismiss the prompt (the game has no mouse
  controls).
- The prompt shows once per page load. Going back to the title later shows
  the menu straight away.
