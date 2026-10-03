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

## Intro cinematic

Decided: 2026-10-03
Source: ticket 0036

Nick judged animated mockups drawn with the game's font at the game's size,
playing against the title song (three ways to fit the menu, three
storyboards, three zoom styles, then his own flow). The mockups used
stand-ins: the logo as plain letters (0811 decides the real one), the rough
Chapter 1 map sketch, empty frames for faces, a made-up fight.

### Nick's words

> on the title screen, it would be great to have an actual intro cinematic...
> something reminiscient of old pokemon games and probably other rpgs like
> dragon quest
>
> basically showing a pan/zoom over a battlefield, over the overworld, past
> some characters, brief conversaiton (non spoiler segments) snippets... all
> the little things we expect from the game. It should match the length of
> the title song, and loop when the song does (i.e. take a brief pause same
> as the song currently does) and then it can show our logo whatever it
> might be and pause there during the brief pause before looping the
> song/cinematic again...

Round 1 (cinematic and menu: A cinematic first, B menu over it, C title
first; storyboards 1 "One long march", 2 "Trailer cuts", 3 "Quiet story,
then the fight"; zoom: A whole steps, B smooth, C pan only):

> 1A
> 2 2
> 3 in between A and C right now, def not B

Round 2 (what happens to the menu after a press; coming back from the game;
the lines and the lead):

> 1. There should be 1 press to get to the menu, not 2. The beginning of
> song will play (first few seconds), and then after inaction (15s timer),
> the menu will fade opacity to 0 and cinematic will start. If the player
> ever presses again it will skip to the silence part, with some sound
> effect i.e. swords clashing. Then we will again start a short timer like
> 15s or however long silence happens, and then if player again has no
> action, then menu fade and cinematic / song loops. But again player has
> agency to skip ahead to logo screen / menu screen.
>
> 2. A, with similar behavior to 1 [A: straight to the logo and menu]
>
> 3. dawg these dialogue lines are cheesy and cringe asf we'll need to
> rewrite them. but anyway we can use C [C: the lead follows the most
> recent save]

Round 3 (the cinematic starts 15 seconds into the song: A joins the song
where it is, B the song restarts, C the cinematic opens on 15 s of logo;
what is wrong with the lines; the overworld shot: A leave it out for now,
B a map made just for the cinematic):

> 1A
> 2 idk... there's just too many quips in general as a starting tone...
> quips should be during downtime, not on ch1 opening not on a serious
> battle... I'm not against quips as support ranks rise as the game reaches
> more high notes in terms of closeness but this just ain't it chief.
> anyway that's a diff ticket than this one but you can say we'll load the
> same script file anyway and just play whatever is there for first few
> lines..
> 3A but make sure to clearly document in 1010 that it updates the
> cinematic or make a new ticket dependent on both this and 1010

A setting in Options to turn the cinematic off (yes / no / later): "Later".

### The title song

`title`, New Sunrise V1, 133.7 s. Its last note ends at about 1:57 (117 s);
the rest is silence until it loops. "The silence" below means 1:57 to the
loop, about 17 seconds.

| Time | What the music does |
| ---- | ------------------- |
| 0:00–0:30 | Very quiet opening, slowly rising |
| 0:31–1:24 | Middle section, moderate |
| 1:25–1:56 | Full and loudest |
| 1:57–2:14 | The last note has ended; silence until the loop |

### How the title behaves (Nick fixed)

1. **One press reaches the menu.** The `Press any key or button` press
   shows the logo and the menu and starts the song from its beginning, as
   the title does today.
2. **Left alone for 15 seconds, the cinematic takes over.** The menu fades
   out and the cinematic shows. Example: the player presses a key, reads
   the menu and puts the controller down; 15 seconds later the menu fades
   and the battlefield appears.
3. **The cinematic follows the song and joins it where it is.** The song
   never restarts for the cinematic. On launch the cinematic therefore
   begins about 15 seconds in, and its first shot is only seen on later
   loops.
4. **A press during the cinematic skips to the silence.** A sword-clash
   sound plays, the song jumps to its silent ending, and the logo and menu
   show. The player can always get back to the menu with one press.
5. **In the silence the logo and menu show.** If the player does nothing,
   the menu fades when the song loops (about 17 seconds), and the cinematic
   plays again from its first shot, with the song.
6. **The logo is the cinematic's last shot**, held for the whole silence.
   The silent end of the song is not trimmed.
7. **Coming back to the title from the game** (after a battle, "To be
   continued", Game Over): the logo and menu, the song from its beginning,
   then the same rules: 15 seconds without a press and the cinematic takes
   over.

### The cinematic's look (Nick fixed)

- **Storyboard 2, "Trailer cuts".** Shots change with **hard cuts** on the
  music's changes. No fades between shots (the menu's own fade in rule 2 is
  the only one).
- **Zoom is whole steps: 2×, 3× or 4×**, always crisp. **A shot keeps one
  size while it is on screen**; the size only changes at a cut. No gradual
  zoom ("def not B").
- **Conversation snippets play in the game's real dialogue screen**, by
  themselves, with the text typing out.
- **The snippets are the Chapter 1 script itself.** The cinematic loads the
  same script file the game plays (`assets/dialogue/ch01.dlg`) and shows
  the first few lines of a scene, whatever they are at the time. There is
  no separate list of cinematic lines, so when the script is rewritten the
  cinematic changes with it. The lines shown in the mockups were **not
  approved**: Nick finds the script's opening too full of quips
  (`setting-and-tone.md`, *Where the jokes go*; rewrite: ticket 0724).
- **No spoilers:** only the opening lines of scenes that play **before**
  the Chapter 1 battle (`ch01_intro`, `ch01_prebattle`), and never past a
  reply choice.
- **Six faces slide past with their names:** the lead, Hollis, Tamsin,
  Aske, Piers, Rue (the bought faces, 0706; names from the names table).
- **The lead follows the most recent save:** the name and face the player
  chose there. With no save, the default lead.
- **Units march and fight** in the loud part of the song (ticket 0827).
- **No overworld shot for now.** Ticket 1010 adds it once the world map
  (1007) exists, asks Nick where it goes, and updates this storyboard.

### Storyboard

Times are seconds into the song. 0820 may move a cut by a second or two to
land on the music. "Size" is how many times bigger than normal the map is
drawn. Which units walk where and who fights whom is set on the real
Chapter 1 map by 0820; the mockup's fight was made up.

| Start | On screen | Movement | Size |
| ----- | --------- | -------- | ---- |
| 0:00 | The Chapter 1 battlefield, the party standing at its starting places, nobody moving. On launch the menu covers this shot (rule 3); it is seen from the first loop on. | Slow pan across the party | 3× |
| 0:15 | Dialogue screen: the opening lines of `ch01_intro`, as the script has them. | None; the text types out | Dialogue screen |
| 0:31 | Six faces in frames, each with its name under it, one after another: the lead, Hollis, Tamsin, Aske, Piers, Rue. | Faces cross from right to left | Faces at dialogue size |
| 0:50 | Dialogue screen: the opening lines of `ch01_prebattle`, as the script has them. | None; the text types out | Dialogue screen |
| 1:08 | Party units march from their starting places toward the enemy; an enemy comes to meet them. | Pan that follows the march; units walk | 3× |
| 1:25 | Close-up: a party unit and an enemy trade blows; the enemy falls. | Still camera; units strike | 4× |
| 1:35 | Close-up somewhere else on the field: two party units bring down a second enemy. | Still camera; units strike | 4× |
| 1:45 | The whole battlefield at once: the rest of the party advances; the enemy fort is in view. | Still camera; units walk | 2× (whole map) |
| 1:57 | The logo (0811), alone on the screen, with the menu (rule 5). Held until the song loops. | None | Logo |

### Claude's starting rules (Nick can veto)

- **The menu also appears when the cinematic reaches the logo by itself**,
  not only after a skip: whenever the logo is up, so is the menu. (Claude's
  reading of rule 5; it was in the last mockup and said to Nick.)
- **Any press while the menu is up restarts the 15 seconds**, including
  moving the menu cursor. Example: the player moves down to Load Game at 14
  seconds; the menu stays another 15.
- **A press in the silence** (after a skip) puts the menu back on the
  15-second rule instead of fading at the loop, so it never vanishes under
  a player who is using it.
- **The press that skips the cinematic does nothing else:** it doesn't move
  the menu cursor or choose New Game.
- **The timer only runs on the title itself**, not while Load Game, Options
  or another screen is open over it.
- **Fades:** the menu fades out over about 1 second and in over about half
  a second (*tunable*). **15 seconds** is Nick's number (*tunable*).
- **Before the first press** the title shows the logo and
  `Press any key or button` as today; no cinematic and no music yet.
- **The cinematic makes no sounds of its own** (no hit or step sounds from
  its fights). Only the song, and the skip sound.
- **The skip sound** is the game's sword hit (`hit_sword`) until Nick picks
  one (ticket 0041).
- **Snippet length:** each dialogue shot shows as many opening boxes of its
  scene as can be read comfortably in its time (0818's timing; 0820 sets
  the counts).

### Open sub-questions

- **A setting to turn the cinematic off** (a still title for players who
  dislike motion). Nick: "Later". Ask again after he has seen the real
  cinematic on the title (0820's sign-off).
- **Which sword-clash sound** plays on a skip: ticket 0041.
- **Where the overworld shot goes** and what it replaces: ticket 1010.
- **The lead's default face** when there is no save (the lead has two
  looks): 0706 picks which one the cinematic shows and tells Nick.
