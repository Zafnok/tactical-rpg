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

## Every build, keys and buttons (changed 2026-09-30, ticket 0032)

With controller support (ticket 0032, `controls.md` *Controller*), Nick
revisited the "for now":

> "pick your layout is after the screen that grabs audio focus whcih says
> press any button... so I guess we can add that screen to native build as
> well... and then use it to also infer which control style plaeyr is
> using."
>
> Wording (A `Press any key or button` / B keep `Press any key`): "A - but
> also display a keyboard and controller glyph or sprite to make it
> obvious"

Changes to the rules above:

- **Rule 1 is replaced:** the prompt shows on **every build** (web,
  Windows, Linux, later Steam), once per launch.
- **Rule 2 is extended:** any key **or any controller button** starts.
- The line reads **`Press any key or button`**, shown with a small
  **keyboard picture and controller picture** so it's obvious either works.
  How they look is mocked up for Nick first (ticket 0226).
- The first press also tells the game which the player is using: a key
  opens "Pick your layout" if no layout has been chosen yet; a button
  skips it (`controls.md`, *Pick your layout with a controller*).
- The web build still needs the first press to unlock sound; a controller
  press may not unlock it in every browser. If it doesn't, the music starts
  at the first key press instead (*Claude's starting rule*, checked in
  0226).

```
                Visions of Shuyi
             an ASCII tactics game


          [keyboard]  Press any key or button  [pad]
```
