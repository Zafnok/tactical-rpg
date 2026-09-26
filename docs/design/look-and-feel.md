# Look and feel

Decided: 2026-09-25
Source: ticket 0011

Nick judged real renders, not descriptions. Every mockup was drawn with the
game's own font atlas at in-game size. The final ones are in
`docs/screenshots/0011-*.png`:

| Screenshot | Shows |
| ---------- | ----- |
| `0011-battle-browse.png` | Battle screen, palette D, browsing: bracket cursor, initials, HP bars |
| `0011-battle-selected.png` | A unit selected: move/attack ranges, path line with arrowhead |
| `0011-conversation.png` | Conversation screen with 32×32 shaded portraits |
| `0011-portrait-expressions.png` | Portrait style sample: confident, happy, angry, sad |
| `0011-theme-c/d/e/g.png` | The four palettes offered as colour themes |

Names, stats, weapons and the sample face in the screenshots are placeholders.

## Nick's words

> **Colour mood** (first round A–C, then D–G): "A and C look better than B, but
> all three aren't ideal to me. Can we get some more options?" … "how about we
> just offer the choice of C, D, E, and G for the color settings. But for now
> if we want to nail the look and feel for one before doing scope creep D is
> the standout for me."
>
> **Units on the map:** "I like C [name initials] the best but I am wondering if
> we can get some more glyphs to represent rather than simply letters" … "for
> the unit symbol options I am still not impressed. […] if I had to pick the name
> initials themselves are still the best. Maybe if the whole UI is rendered and
> it shows more info when you hover a unit somewhere else on the screen then it
> would be more acceptable." … "I think backing is maybe where we can show the
> HP without hovering. Like a green/yellow/red bar showing % hp by fill." … "I
> think the thin bar works but I don't really care about faction backing."
>
> **Cursor:** "I like B [blinking brackets] the best" … "for the cursor I think B
> in general looks good and then maybe on selection it changes to E [arrows] to
> make it clear when you're in selected mode"
>
> **Path:** "I would expect a path trace when you start to move from the
> selected unit, but I do like the move range and attack range colors" … "the
> start of path should either be Z-indexed under the character, or just start
> on the next adjacent tile since right now it obscures the unit name.
> Additionally, while the arrows work for selecting while ON a unit, once you
> start moving away it seems it makes it unclear what tile you end up at. So
> maybe the end of the traced path should be a rendered arrow (singular)."
>
> **Portraits:** "I think the line art isn't good." [On the shaded-blocks
> sample:] "He should be much more expressive and human looking" [on the
> redraw:] "this looks a lot better all around… if there is a way to give
> subtle shading to give more volume to the face it would be good" … "this new
> shaded style seems OK it does make this particular character look a bit old
> (i.e. like he has wrinkles) but the general theme is ok. And we can probably
> iterate on a character by character basis."
>
> **Where portraits appear:** "it might be nice to have portraits appear in
> hover/selection in battles. But combat screen probably we would have more like
> a full body rendering of the 2 battling units. But if we can have them display
> in hover/selection we have to make sure they won't clutter the UI" … [after
> seeing the 16×16 panel mini-portrait:] "that one looks like a memeface so we
> can just forego the battle portrait"

## Rules

### Colour

- **The game's palette is mood D, "Earthy painterly":** browns, olives and
  brick reds on a near-black base, and **every terrain has its own background
  tint** (olive grass, dark-green forest, deep-teal water, brown rock). The
  values are in `assets/data/palette.ron` *(tunable)*.
- Terrain draws its glyphs in `<terrain>` and its background in
  `<terrain>_bg` (e.g. `forest` / `forest_bg`).
- Move/attack/heal/danger overlays blend their colour over the terrain
  background at about **75%** *(tunable)*, so the ground stays visible
  underneath. Nick liked the move and attack range colours as shown.
- **Colour themes (later):** the player may pick among four palettes, **C Rich
  & painterly, D Earthy painterly (default), E War-table parchment (light), G
  Moonlit**. That's a separate ticket (0806). Their mockup values are recorded
  below so they don't need re-deriving.

### Units on the map

- A unit is drawn as **two letters: an initial pair from its name** (`Al`,
  `Be`), in its **faction colour** (player blue, enemy red, ally green, neutral
  yellow) on the terrain background. **No faction backing.**
  - Generic enemies use the first two letters of their class (`Br` Brigand,
    `Ra` Raider, `Ar` Archer); named enemies and bosses use their name.
  - Content can override the pair (e.g. two party members named "Al…"). A
    duplicate pair within one side on a map is a content validation error.
- **Acted** units are drawn **lowercase and dimmed** (`al`), so "has acted" never
  relies on colour alone.
- **HP bar:** a thin bar (2 px of the 16 px tile height) along the bottom of the
  unit's tile, filling left to right by current HP %. It's `hp_high` above 2/3,
  `hp_mid` above 1/3 and `hp_low` at or below 1/3 *(thresholds tunable)*. The
  unfilled part is dark. The bar's length carries the information, not only its
  colour.
- The **side panel shows the unit under the cursor** (name, class, level, HP,
  stats, weapon, terrain). That's where class and full numbers are read.
- Class symbols (`†`, `»`, `}`, …) were tried and rejected: the font's symbols
  are too small and generic. Custom-drawn class icons may be explored later
  (ticket 1002); until then, initials are the rule.

### Cursor and selection

- **Browsing:** brackets in the half-cells either side of the tile, `[Al]`, in
  `cursor` colour, **pulsing between bright and about half brightness** (period
  about 1 s, *tunable*). Never fully off.
- **Unit selected, cursor still on it:** arrows `►Al◄` replace the brackets.
- **Moving the cursor away from a selected unit:** a **path line** runs through
  tile centres from the **edge of the unit's tile** (never over its letters) to
  the destination tile, and **ends in a single arrowhead** on that tile. It's
  drawn under glyphs, in `path` colour. The arrowhead marks the destination;
  there's no separate cursor frame there.

### Portraits

- **Style: shaded pixel art.** A portrait is a **32×32 pixel** image drawn with
  half-block cells (`▀`, top pixel = fg colour, bottom pixel = bg colour), so it
  takes **32×16 cells** and each pixel is a square 8×8 screen px. It's not
  line-art ASCII: Nick rejected both line-art and block-shading ASCII.
- **Lighting:** from the **upper left**. Mid-tone down the right side of the face
  and jaw, shadow under the fringe, nose, lower lip and chin, a soft highlight
  on the left cheek and nose bridge. **Keep shading light on young characters.**
  Nick's feedback on the sample was that heavy shade lines read as wrinkles and
  age the face.
- **Expressive, human faces:** expressions change brows, eyes and mouth (at
  minimum `neutral`, `happy`, `angry`, `sad`, `surprised`). The sample shows the
  range expected (a raised brow, a squint-smile, bared teeth, a tear).
- **Iterated per character with Nick.** The sample face is a style reference
  only. Each real character is drawn and revised with his feedback (0706).
- **Where portraits appear:**
  - **Conversations:** two full portraits, speaker on the left at full
    brightness with a double-line frame, listener dimmed on the right, name
    plates underneath, 3-line text box below.
  - **Not** in the battle side panel or hover. That panel shows stats only
    (Nick dropped the mini portrait).
  - **Combat screen:** Nick expects **full-body art of the two fighting units**
    there, not portraits. Its design is a separate ticket (0413).

### Screen layout

As in ADR-0018 and the screenshots: map on the left (single-line frame), side
panel on the right (single-line; **double-line** when a unit is selected), a
status line on top (chapter, objective, phase and turn, rewinds), and a 2-row
message and key-help bar at the bottom.

## Theme palettes for ticket 0806 (*mockup values, tunable*)

Base colours, as `bg / text / dim / highlight / panel_bg / border / focus /
player / enemy`:

| Theme | Base colours |
| ----- | ------------ |
| C Rich & painterly | `#0b1416 #b8ccc6 #5e7a78 #e8b848 #0f2024 #2e7c86 #e8b848 #6ec8f0 #e8582a` |
| D Earthy painterly | see `assets/data/palette.ron` |
| E War-table parchment | `#e8dcc0 #2a2218 #7a6a50 #8a2a10 #d8c8a4 #6a5438 #8a2a10 #1a4a9a #b01e10` |
| G Moonlit | `#06080e #f0e0c0 #7a8090 #ffb040 #0c1018 #3a4a60 #ffb040 #60b0ff #ff5030` |

Terrain glyph colour / background:

| Theme | grass | forest | water | mountain |
| ----- | ----- | ------ | ----- | -------- |
| C | `#8aa050/#1a2412` | `#3ea83a/#0e2410` | `#58c0d8/#0c3440` | `#c09868/#2c2016` |
| E | `#b0a078/#e8dcc0` | `#3a6a2a/#dcd8b0` | `#2a5a8a/#c8d4d0` | `#6a5030/#d8c8a8` |
| G | `#3a5a5a/#0a1216` | `#2e7a64/#081410` | `#4a78b0/#0a1a30` | `#6a7a8a/#10141c` |

Overlays and cursor, as `move / attack / cursor / hp_high / hp_low`:

| Theme | Values |
| ----- | ------ |
| C | `#1a5a78 #7a2c14 #f0c850 #8cd050 #e8582a` |
| E | `#a8c0e0 #e8a898 #2a2218 #3a7a2a #b01e10` |
| G | `#1c3a6a #5a1a1a #ffc860 #70d080 #ff5030` |

E is a light theme: "acted" dimming must fade toward the background, not toward
black.

## Open sub-questions

- Custom class icons vs initials (ticket 1002, after Chapter 1).
- Combat screen full-body art: style, size, animation (ticket 0413).
