# Look and feel

Decided: 2026-09-25
Source: ticket 0011

Nick judged real renders, not descriptions. Every mockup was drawn with the
game's own font atlas at in-game size. The final ones are in
`docs/screenshots/0011-*.png`:

| Screenshot | Shows |
| ---------- | ----- |
| `0011-battle-browse.png` | Battle screen, palette D, browsing: initials, HP bars (its bracket cursor was replaced by corner marks in 0416) |
| `0011-battle-selected.png` | A unit selected: move/attack ranges, path line with arrowhead |
| `0011-conversation.png` | Conversation screen with 32×32 shaded portraits (the portrait style was replaced by bought art in 0021; the layout stands) |
| `0011-portrait-expressions.png` | Old portrait style sample: confident, happy, angry, sad (replaced by bought art, 0021) |
| `0011-theme-c/d/e/g.png` | The four palettes offered as colour themes |
| `0404-forecast.png` | The attack forecast (decided 2026-09-27, ticket 0404) |

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
> [2026-09-27, ticket 0416, after playtesting the Quick Battle:] "the cursor
> overlaps units' names when adjacent. Like if Ar is on tile 1x1 and cursor
> [ ] is on tile 1x2 then it just says A [ ] the r in Ar is cut off. […] Make
> the cursor skinnier or something. I think z-indexing won't help here it
> would just clutter it" [shown thin brackets, corner marks and a tile glow:]
> "corner marks do look good but I wonder if we can make them more square than
> they are now -- they are more wide than tall" [shown 2×2, 3×3 and 4×4
> corners:] "3x3 default with 4x4 and the full tile glow as accessibility
> options"
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
>
> **Bought character art** (2026-09-28 to 2026-09-30, ticket 0021): [on
> Claude's portraits and CaptainSkolot's bundle:] "I like the style better
> than your style" … "it's fantasy not modern day which fits the game" …
> "buying is OK as long as it's not super expensive" … "I'll buy it later
> when we finish more stuff" … [on the combat screen:] "I guess we need an
> itch artist who has a pack with portraits and battle sprites" … [shown
> mockups of five artist options:] "I like A2 the best but want to ensure you
> know how to route the animations correctly" … "AI assisted art is ok as
> long as it is not shipped without a human touch at all... the captain guy
> said he touched it up, that's alright" … [asked for a price ceiling:] "stop
> asking, I will decide each time if I am comfortable" … [told that the big
> Tiny Tales fighters are still images and only the small ones animate:] "1A
> seems best" [the big still images, moved by the game] … [characters no
> face fits:] "probably A or D" [the Character Generator, or Claude's small
> edits] … [classes with no animated fighter:] "since I am ok with 1A then 3A
> seems fine?" [still images as stand-ins]

## Rules

### Colour

- **The game's palette is mood D, "Earthy painterly":** browns, olives and
  brick reds on a near-black base, and **every terrain has its own background
  tint** (olive grass, dark-green forest, deep-teal water, brown rock). The
  values are in `assets/data/palette.ron` *(tunable)*.
- Terrain draws its glyphs in `<terrain>` and its background in
  `<terrain>_bg` (e.g. `forest` / `forest_bg`).
- **Fort glyph is `╦╦` (battlements)**, in `fort` gold (decided 2026-09-28,
  ticket 0422). Nick: "not a fan of the fort glyph it looks too similar to a
  cursor" … "I like C for the fort". The old `[]` read as the cursor's
  brackets; terrain glyphs should never look like the cursor.
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
- **Acted** units are **dimmed** toward the background; their initials keep
  their case (`Al` stays `Al`). The dimming is a brightness change, not a hue
  change, so "has acted" doesn't rely on hue alone. *(Changed 2026-09-28 from
  "lowercase and dimmed". Nick, after playing 0404: "seems after moving the
  initials for my units goes from i.e. Lo to lo I think it should just stay Lo
  but be shaded different (it already has this aspect so keep it like that)".
  ADR-0029.)*
- **HP bar:** a thin bar (2 px of the 16 px tile height) along the bottom of the
  unit's tile, filling left to right by current HP %. It's `hp_high` above 2/3,
  `hp_mid` above 1/3 and `hp_low` at or below 1/3 *(thresholds tunable)*. The
  unfilled part is dark. The bar's length carries the information, not only its
  colour.
- The **side panel shows the unit under the cursor** (name, class, level, HP,
  stats, weapon, terrain). That's where class and full numbers are read.
- Class symbols (`†`, `»`, `}`, …) were tried and rejected: the font's symbols
  are too small and generic. Custom-drawn class icons may be explored later
  (ticket 1006); until then, initials are the rule.

### Cursor and selection

- **Browsing:** thin **corner marks** around the tile, in `cursor` colour,
  **pulsing between bright and about half brightness** (period about 1 s,
  *tunable*). Never fully off. Each corner is two 1-pixel arms, **3 px** along
  and 3 px down (decided 2026-09-27, ticket 0416; the bracket cursor in the
  0011 screenshots is replaced). The marks sit in the gaps the font leaves
  around letters, so **they never cover a unit's initials, including a
  neighbour's**.
- **Accessibility options** (picked in the Options menu, ticket 0805):
  - **Large corners:** the same marks with **4 px** arms.
  - **Tile glow:** no marks; the tile's background is tinted towards the
    `cursor` colour (up to about 30% at full brightness, *tunable*), pulsing
    the same way.
- **Unit selected, cursor still on it:** arrows `►Al◄` replace the corner
  marks. *(Open, see below: as whole glyphs they would cover a neighbour's
  initial, as the brackets did.)*
- **Moving the cursor away from a selected unit:** a **path line** runs through
  tile centres from the **edge of the unit's tile** (never over its letters) to
  the destination tile, and **ends in a single arrowhead** on that tile. It's
  drawn under glyphs, in `path` colour. The arrowhead marks the destination;
  there's no separate cursor frame there.

### Attack forecast

Decided 2026-09-27, ticket 0404, over three rounds of renders (the ticket's
two-column `DMG 7+8 ×2` panel, a stacked panel, a wide box over the map,
then three strike-list layouts). Screenshot: `0404-forecast.png`.

> "from what you shown me, none are perfect but here's what I like from each
> / I like the bar representing HP in the forecast. / I like not having all
> upper case for the statistics / as for multi strike, I am leaning towards
> having them on multiple lines somehow... idk how in practice but like
> Strike 1 Strike 2 etc? maybe showing a total? and number of strikes? / we
> should also forecast with a glyph showing it will kill on which strike
> (assuming all hit)" … "strikes should be in order they happen like E / the
> death glyph shouldn't be a cross it should be a skull or an X or something"
> … "ok skull is fine.. seems good"

- While a target is chosen, the forecast **replaces the side panel** (double
  border, title `Forecast`): the attacker in the left column, the target in
  the right, names in faction colour, weapon names dimmed under them.
- **Mixed-case labels:** `HP`, `Hit`, `Crit`. A side that can't counter shows
  `--` for Hit and Crit.
- **HP bars:** `HP 19` and a short bar. The part of the bar the unit would
  lose **if every strike hit** (no crits) is shaded in `hp_low`.
- **Strikes in the order they happen**, one per row, numbered down the
  middle: the attacker's on the left (`8 dmg →`), the target's counters on
  the right (`← 9 dmg`). `no counter` in the right column when it can't.
  After a separator, a **total** per side: `17 ×2` (damage of its strikes and
  how many).
- **Kill mark:** a small **skull**, pixel-drawn in `hp_low`, on the strike
  that makes a unit fall if every strike before it hit. Strikes after it are
  dimmed: they happen only if something misses.
- Effective weapons put `!` after `dmg` in the highlight colour; a broken
  weapon shows `(broken)` in `hp_low` under its name *(Claude's starting
  rule, not rendered for Nick)*.
- The Combat Arts list and art line (0414) go above this forecast; that
  ticket works out the combined box.

### Portraits and battle art (bought)

Decided 2026-09-30, ticket 0021 (it replaces the 2026-09-25 rule that
portraits are 32×32 pixel art drawn by Claude with Nick). Nick judged
mockups made from store previews in our dialogue screen and a stand-in combat
scene: one artist with both kinds of art (SolaarNoble, Tiny Tales, Time
Fantasy), and CaptainSkolot's portraits next to another artist's battle
sprites.

- **Artist: Mega Tiles, "Tiny Tales" packs** (<https://megatiles.itch.io/>),
  hand-drawn pixel art, medieval fantasy. The core packs are *Tiny Tales 2D
  Heroes: A New Beginning* and *Heroes 2: Rebellious Souls* ($24.99 each on
  2026-09-30). Each hero comes with a face set, a large portrait, a big still
  battle image, a small animated battle sprite and a map sprite. The still
  battler packs (e.g. *Vol.5 Faith and Evil*, *Vol.1 Monstrous Uprising*)
  add classes with a still battle image and a map sprite but **no face**.
  Nothing has been bought yet; Nick buys when he's ready.
- **No Claude-drawn character art.** Portraits and battle art are bought.
  Claude may only make **small edits** to bought art: recolours, a scar,
  spectacles, a missing expression made from an existing face. No new hair,
  clothes or bodies (that's redrawing).
- **Paid art is fine; Nick decides each purchase himself.** He doesn't want a
  price ceiling asked. Music and sound stay free (`audio.md`).
- **AI-assisted art is acceptable if a human has worked on it**, e.g.
  CaptainSkolot's "local generative tool for rough concepts, then refined by
  hand". Art that is AI output with no human touch is not. The Tiny Tales
  packs say no generative AI was used. If AI-assisted art ships, the Steam
  page discloses it (ticket 0903).
- **Every bought work is credited** on the credits screen (0808), even when
  its licence doesn't ask for credit (`audio.md` rule 3).

#### Dialogue portraits

- **The face set faces**, not the large portraits: each Tiny Tales hero has
  **8 expressions**; the large portrait has one. The faces are **48×48**
  pixel art. They're drawn with square pixels at the largest whole scale that
  fits the existing 32×16-cell frame (0711 works out the size), so the
  dialogue layout doesn't change.
- **Expressions:** the five the dialogue needs (`neutral`, `happy`, `angry`,
  `sad`, `surprised`) are mapped to the closest of the 8 per character
  (0706). A missing one may be made by a small edit.
- **Characters no bought face fits** (in Chapter 1 likely Hollis, Harl,
  Piers and Crane): first Mega Tiles' **Character Generator EX** ($49.99,
  early access), which makes new characters in the same style with 8
  expressions, a battle image and a map sprite, *if its licence allows
  generated characters in a sold game* (Claude checks before Nick buys).
  Otherwise, or on top, Claude's small edits. Commissioning, or changing a
  character's look to fit a face (a story change), needs asking Nick first.
- **Where portraits appear** (unchanged):
  - **Conversations:** two full portraits, speaker on the left at full
    brightness with a double-line frame, listener dimmed on the right, name
    plates underneath, 3-line text box below.
  - **Not** in the battle side panel or hover. That panel shows stats only
    (Nick dropped the mini portrait).

#### Combat screen

- Nick expects **full-body art of the two fighting units** there, not
  portraits.
- **The art is the big Tiny Tales still battle images, moved by the game**
  (like the enemies in Final Fantasy VI or Dragon Quest): each fighter is one
  picture; the game makes it lunge to strike, flash on a hit, shake, and fade
  when defeated. The packs' small animated battle sprites are **not** used
  there. Scale, the exact motions and the rest of the scene are ticket 0413.
- **Classes with no hero art use a still image as a stand-in in Chapter 1**:
  - **Cleric:** the church cleric (*Faith and Evil*).
  - **Guard:** the church knight (*Faith and Evil*).
  - **Brigand:** the orc axe fighter (*Monstrous Uprising*). Chapter 1's
    bandits stay human in the story *(Claude's starting rule: Nick accepted
    the stand-in without choosing between it and making the bandits orcs)*.
  - **Rider:** no pack has anything mounted; an on-foot lance fighter
    stands in until real art exists. Commissioning a Rider is Nick's call
    later.
  0413 picks the exact images. None of these have faces; they're generic
  enemies or get a face from the Character Generator.

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

- Selection arrows `►Al◄` next to another unit: whole-glyph arrows would hide
  one of its letters (the 0416 bracket problem). Ask Nick before 0403 draws them.
- Custom class icons vs initials (ticket 1006, after Chapter 1).
- Combat screen: scale and the exact motions of the still battle images
  (ticket 0413).
- Map sprites: the Tiny Tales packs include 16×20 map sprites; whether they
  replace name initials on the map is ticket 1006's question.
