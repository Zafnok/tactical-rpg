# Look and feel

Decided: 2026-09-25
Source: ticket 0011 (attack forecast: 0404; bought portraits and battle art:
0021; bought tiles and sprites on the battle map: 0038; walking sprites,
effect marks, the glyph look as an option, busts in dialogue: 0039)

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
> seems fine?" [still images as stand-ins] … [named characters without
> their own combat picture use their class's picture, recoloured:] "sure" …
> [Rue: the Witch's face and combat picture, or a generated face and a class
> picture:] "I think we can rewrite Rue's physical description as needed once
> we buy the art... I would rather have visual consistency at this time
> between portrait and battler" … [on commissions:] "this is where I veto --
> this would be expensive for just a single asset" … [Harl:] "let's just have
> a follow up for this Harl character I can shop around and find more
> contenders"
>
> **Bought sprites on the battle map** (2026-10-02, ticket 0038; Nick had
> bought the whole Mega Tiles bundle and judged spike renders of the battle
> screen: A today's glyphs, B glyph terrain with bought unit sprites, C the
> bought tileset with bought unit sprites, C2 the same at double size, C3
> the same darkened toward palette D, G1a/G1b a combat picture with a bought
> background in the box over the map and full screen): "I'm really digging
> these sprites... I think we can go ahead and move forward with replacing
> our tile rendering and battler rendering with these bought sprites without
> regret..." … "as for how the tilesets look, I'm between C, C2, and C3. But
> I think this actually has an easy answer. 1. Per-map lighting effects
> (i.e. if it takes place during what would be a low light time, it can be
> shaded like with C3. Or if it's noon, it can be like C/C2. 2. A key/button
> to toggle zoom (1x / 2x) with memory (i.e. going to next battle, keep the
> setting)" … "for G1a and G1b I think both are a little off. The main thing
> that's off though is that you put the enemy facing backwards... let's
> always mirror the image to face the player when battling. Other than
> that, I worry G1b won't be able to convey all the info, but maybe it can
> work with some additional in-battle overlays or pop up messages" … "for
> the cast, I guess you need to help me locate some more itch bundles to
> fill in with a similar style for rest of cast... or if these are
> non-battle units, I think the generator can work" … "I paid $100 for all
> these assets" … [asked how a sprite unit shows its side and that it has
> acted: A a coloured outline, B a tinted tile, C a corner mark, D the HP
> bar only:] "A, and potentially A with C" … [asked which key and button
> toggle the zoom:] "I think we can ask this as we build the zoom feature
> in another ticket? I will decide it later" … [asked whether the Chapter 1
> playtest waits for the bought-art map: A yes, B no:] "A"
>
> **Sprite units and dialogue busts** (2026-10-02, ticket 0039; Nick judged
> animated mockups J1–J3 of sprites standing still, walking only when
> moved, and stepping on the spot as well; K, four ways to mark a unit
> under an effect; H2/H3, faces and busts in the dialogue screen): [do
> sprites walk:] "1C - but make sure the hp bar is not overlapping the
> sprite... right now it is" … [the effect mark, shown as a purple square:]
> "what does under an effect mean here? like poison or something? can you
> show what a poison mark might look like then rather than a square" …
> [faces or busts:] "can you see if bust can be zoomed in more, it seems
> like there's empty space on top and sides makes me think bust 4x might be
> possible..." … [shown J4/J5 with the feet on the HP bar, K2 with an up
> arrow for a bonus and a down arrow for a penalty in the tile's corner or
> over the head, H4/H5 with busts at 4×:] "1A [the arrow in the tile's
> corner] but I want the mark raised one more pixel up... maybe 2" … "2C"
> [busts at 4×] … [may players pick the glyph look:] "if both exist I guess
> we can add the option and we already have the decoupled art/engine logic
> afaik" … "for walking with HP bar, can you raise the sprite 1-2 more
> pixels?" … [shown sheet L, the sprite raised 0, 1 or 2 more pixels and
> the arrow raised 1 or 2:] "let me see sprite up 1 mark up 2 vs sprite up
> 0 mark up 1" … [shown the two side by side, L2:] "sprite 0 mark 1 looks
> best to me" … [does the Chapter 1 playtest wait for the walking: A no, B
> yes:] "B" … [told that an acted unit standing still was Claude's
> addition:] "sure, unit can stand still if exhausted"

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

### Battle map: bought tiles and unit sprites

Decided 2026-10-02, ticket 0038. The renders Nick judged used bought files,
so they aren't in this repository (ADR-0032).

- **The battle map is drawn with the bought art**: terrain from the Tiny
  Tales 16×16-pixel tilesets, and each unit as its Tiny Tales **map sprite**
  (16 pixels wide, 20 tall, so a head overlaps the tile above). The art is
  the same size as our map tile (16×16 pixels), so nothing is scaled.
  Tickets 0436 (units) and 0437 (terrain).
- **The glyph look below stays in the game** (ADR-0038): it is what a copy
  of the public repository shows, since the bought files aren't in it.
- **Players may pick the look** (decided 2026-10-02, ticket 0039): an
  Options entry, **"Map look: Pictures / Glyphs"**, like Dwarf Fortress on
  Steam (new art or classic ASCII). Nick: "if both exist I guess we can
  add the option". Pictures is the default. *(Claude's starting rules: the
  entry switches terrain and units together; it changes the map at once
  and is remembered; a build without the bought files has only glyphs and
  doesn't show the entry.)* Ticket 0824.
- **Lighting is set per map.** A battle at noon shows the tiles as bought
  (render C). A battle in low light (dusk, night, indoors by torchlight)
  shows them darkened toward our earthy palette, as in render C3. Unit
  sprites keep their own colours in every light, as they did in C3
  *(Claude's starting rule: that is how C3 was drawn; Nick didn't comment
  on it)*. Which lights exist and how dark each is: ticket 0438 *(values
  tunable)*.
- **Zoom:** a key and a controller button switch the map between **1×**
  (a 16-pixel tile, about 34×28 tiles on screen, render C) and **2×** (a
  32-pixel tile, about 17×14 tiles, render C2). The game **remembers** the
  choice: the next battle opens at the same zoom. Ticket 0439. Nick picks
  the default key and button when that ticket is built ("I will decide it
  later").
- **A sprite unit's side** (decided 2026-10-02): a **1-pixel outline in
  its side's colour** (player blue, enemy red, ally green, neutral yellow)
  around the sprite, as in renders B, C, C2 and C3. **Perhaps also a
  small corner mark** in the same colour on its tile ("A, and potentially
  A with C"): ticket 0436 renders the outline alone and with the corner
  mark, and Nick picks.
- **An acted sprite unit** is drawn grey and darker, as in the renders;
  its outline dims with it. A change of brightness, not of hue, like the
  glyph look's rule.
- **The Chapter 1 playtest waits for the bought-art map** (decided
  2026-10-02): units (0436) and terrain (0437) are in before Nick plays,
  and so is the walking below (0440; Nick, ticket 0039: "B").
- **Sprites step on the spot and walk** (decided 2026-10-02, ticket 0039,
  "1C"; like Fire Emblem on the GBA). Each map sprite has three walking
  frames in four directions:
  - A unit that **can still act steps on the spot**, facing the camera
    (about four steps a second in the mockup, *tunable*).
  - A unit that **has acted stands still** (and is grey). So "still" also
    says "done" (Nick: "sure, unit can stand still if exhausted").
  - A **moving unit walks along its path**, gliding from tile to tile with
    its legs going, and **turns to face the way it walks** (about a fifth
    of a second per tile in the mockup, *tunable*; the speed settings
    still apply). When it arrives it faces the camera again.
  Built in ticket 0440, after 0436; until then sprites stand still.
- **The HP bar never overlaps the sprite's feet** (Nick: "make sure the hp
  bar is not overlapping the sprite"). The sprite is drawn higher on its
  tile, so its feet stand **directly on top of** the 2-pixel HP bar (the
  16×20 sprite ends 2 pixels above the tile's bottom edge, and its head
  reaches 6 pixels into the tile above). Nick saw it raised 1 and 2
  pixels more and chose this ("sprite 0 … looks best to me"). HP bars are
  drawn over every sprite: when one unit stands directly below another,
  the upper unit's bar shows in front of the lower unit's hair, so HP is
  never hidden.
- **A sprite unit under an effect** (decided 2026-10-02, ticket 0039,
  "1A"): a small **arrow in the top-right corner of its tile**: an **up
  arrow for a bonus** (a stance such as Sidestep's avoid +20) and a **down
  arrow for a penalty** (Pinning Shot's Mov −3, Pressure Point's Spd −3).
  The arrow is 5 pixels wide with a dark edge and sits **1 pixel higher
  than in the first mockup** (Nick: "mark 1"): with its edge it reaches 3
  pixels above the tile's top edge and 1 pixel past its right edge. The
  direction carries the meaning, not only the colour (light blue up,
  purple down in the mockup, *tunable*). On an acted unit the arrow dims
  with the sprite. *(Claude's starting rule: a unit with both a bonus and
  a penalty shows both arrows, the up arrow above the down arrow.)* The
  game has no poison or other lasting ailment today; if one is added it
  gets its own small picture in the same corner (Nick was shown a green
  drop as an example). The glyph look keeps its tinted background.
- Cursor, path line, HP bar and range tints keep their rules below.

### Units on the map (the glyph look)

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
- **Bought 2026-10-02:** Nick bought Mega Tiles' whole "2025 Bundle Sale"
  (37 products, $99.99; his words: "I paid $100 for all these assets").
  It holds both Heroes packs, the Character Generator EX, the still
  battler packs Vol.1–5, *Gods and Gallants*, *Epic Monsters*, the map
  sprite packs, five 16-pixel tilesets (World Map, Overworld, Dungeons 1
  and 2, Tower), *Battlebacks Vol.1* (24 battle backgrounds), and some
  packs we don't use (a sci-fi set, a sci-fi UI kit, three 48-pixel
  terrain and tree packs in another style). The files are on Nick's
  machine, outside this repository (ADR-0032; the private repository is
  ticket 0110). The purchase record is in `THIRD_PARTY_ASSETS.md`.
- **What the real files are** (checked 2026-10-02; earlier notes came
  from store previews): a face is 48×48 pixels and a **bust is 80×80**,
  and **both come in the same 8 expressions** (neutral, smile, stern,
  sad, surprise, thinking, sly, unique), as single files and as a sheet;
  a map sprite is a 48×80 sheet of 16×20 frames (3 walking frames, 4
  directions); every tile is 16×16; a battle background is 336×248. Each
  pack ships its art at 1×, 2× and 3×: we use the 1× files.
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

- **The busts, at 4×, filling the frame** (decided 2026-10-02, ticket
  0039, "2C"; it replaces 0021's rule of the 48×48 faces at 5×, whose
  reason, that the large portrait has only one expression, turned out
  wrong on the bought files). Each Tiny Tales hero has an **80×80 bust in
  8 expressions**. The bust is cut to its **middle 64 columns and bottom
  64 rows** and drawn with square pixels at **4×**, which is exactly the
  256×256 pixels of the existing 32×16-cell frame, so the dialogue layout
  doesn't change. Against the faces: the whole hat and the shoulders show,
  and a face is about four fifths as big. **What the cut loses:** the tips
  of very tall ears or horns (the Fox Miko's ears) and the outer edge of
  the widest shoulders. A bust that sits off-centre may be cut a few
  columns to one side instead *(Claude's starting rule)*. Nick first saw
  the bust at 3× with empty space around it and asked for it bigger ("makes
  me think bust 4x might be possible"). Ticket 0711.
- **Expressions:** the five the dialogue needs (`neutral`, `happy`, `angry`,
  `sad`, `surprised`) are mapped to the closest of the 8 per character
  (0706). A missing one may be made by a small edit.
- **Characters who fight and have no fitting bought art**: Nick wants
  more bought packs in a similar style (2026-10-02: "help me locate some
  more itch bundles to fill in with a similar style for rest of cast").
  Claude searches and checks each pack's licence and price; Nick decides
  each purchase (ticket 0040; Harl is ticket 0035). **Characters who never
  fight** may get a face from the Character Generator ("if these are
  non-battle units, I think the generator can work").
- **Characters no bought face fits** (in Chapter 1 likely Hollis, Harl,
  Piers and Crane): first Mega Tiles' **Character Generator EX** (bought
  with the bundle; version 1.2), a Windows program that makes new
  characters in the same style: a face with 8 expressions, a small animated
  battle sprite and a map sprite. Its licence (the same text as the packs)
  allows the characters it makes in a sold game. It makes **no big still
  battle image**, so in combat a generated character uses its class's
  still image. Version 1.2 has six kinds of outfit (soldier, rogue,
  brawler, commoner, traveller, simple dress), one wizard hat, beards and
  moustaches, and no hoods, helmets or glasses, so some faces aren't
  possible yet. Otherwise, or on
  top, Claude's small edits, or rewriting the character's written look to
  fit a bought face (see *Combat screen* below; Nick allows it, done in
  0706 after purchase). **No commissions**: Nick vetoed them as too
  expensive for single assets.
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
- **The two fighters always face each other** (decided 2026-10-02, ticket
  0038): a picture that faces the wrong way is mirrored. In the mockups the
  player's fighter stood on the left and the enemy on the right; the enemy
  was drawn as bought and so faced away, which Nick called the main thing
  wrong with them.
- **Where the scene is shown is open** (ticket 0413). Nick saw two mockups
  with a bought battle background behind the fighters, one in the box over
  the map (art at its own size) and one filling the screen (art at double
  size), and found both "a little off". His worry about the full-screen one
  is that it "won't be able to convey all the info", though "maybe it can
  work with some additional in-battle overlays or pop up messages".
- **Classes with no hero art use a still image as a stand-in in Chapter 1**:
  - **Cleric:** the church cleric (*Faith and Evil*).
  - **Guard:** the church knight (*Faith and Evil*).
  - **Brigand:** the orc axe fighter (*Monstrous Uprising*). Chapter 1's
    bandits stay human in the story *(Claude's starting rule: Nick accepted
    the stand-in without choosing between it and making the bandits orcs)*.
  - **Rider:** no pack has anything mounted; an on-foot lance fighter
    stands in until real art exists (no commissions).
  0413 picks the exact images. None of these have faces; they're generic
  enemies or get a face from the Character Generator.
- **Named characters use their class's picture, recoloured to their own
  colours** when they have no picture of their own (like Fire Emblem GBA,
  where most named units share their class's battle animation in their own
  palette). Characters with their own hero art use it for both face and
  combat picture.
- **Face and combat picture should match** (Nick prefers visual
  consistency). Where a character's written look doesn't fit the art we
  can buy, the written look is rewritten to fit the art after purchase
  (0706), not the other way round. First case: **Rue** uses the Tiny Tales
  **Witch** for both, hair recoloured as needed.
- **Harl** (Chapter 1 boss) has no fitting picture in the Mega Tiles
  catalogue: the axe fighters are orcs, beasts or a minotaur, and a human
  knight would change his weapon (weapon type is a rule, so the picture must
  match it). Nick is shopping around for candidates (ticket 0035). The
  orc, the Dragon Knight and the Magitek dark knight were shown and not
  chosen.

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
- Combat screen: scale, where the scene is shown (box or full screen), a
  background behind the fighters, and the exact motions of the still
  battle images (ticket 0413).
- Sprite units on the map: the outline alone or with a corner mark
  (0436). The zoom key and button (0439).
- More bought packs for fighters with no fitting art (tickets 0035, 0040).
