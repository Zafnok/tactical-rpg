# Roadmap

Goal of the first phase: **a playable, story-driven Chapter 1** on Windows and
in the browser, then publish to itch.io, then Steam.

All work is in [`tickets/`](../tickets/README.md). Status is read from the
folders: `tickets/open/` vs `tickets/done/`.

## Milestones

| Block | Milestone | Outcome | Tickets |
| ----- | --------- | ------- | ------- |
| `00xx` | Design decisions | Nick's answers recorded in `docs/design/` | 0001–0029 |
| `01xx` | M0 Foundation | Workspace, CI on 3 OSes, security scanners, SonarCloud, mutation gate, release + Pages pipelines | 0101–0108 |
| `02xx` | M1 Engine | Coloured glyph console in a window and browser, keyboard input with right/left-handed layouts, screens + test harness | 0201–0208 |
| `03xx` | M2 Core rules | Maps, units, pathfinding, combat, turns, items, rewind, gold/shops, spells, terrain magic, class skills, Combat Arts — pure and heavily tested | 0301–0312 |
| `04xx` | M3 Battle UI | Cursor, move/attack loop, forecast, combat playback, info screens, tips, item menus, preparations, shops, spell menus, battle notes, skill menus, combat scene art, arts menu | 0401–0414 |
| `05xx` | M4 Enemy AI | Enemies that fight back, animated enemy phase, boss arts; automated playtest bots, autobalancing | 0501–0509 |
| `06xx` | M5 Progression | EXP and class points, level-up screen, promotion and reclass, level-up sounds | 0601–0605 |
| `07xx` | M6 Story & dialogue | Story bible, dialogue engine, two-portrait scenes, Chapter 1 art + script, lead reply choices | 0701–0708 |
| `08xx` | M7 Chapter 1 | Full flow, saves, Chapter 1 content, **Nick's playtest**, options, colour themes, music, credits, end-of-battle music, results, title art, chapter card, transitions | 0801–0813 |
| `09xx` | M8 Release | itch.io, Windows polish, Steam readiness | 0901–0903 |

**Audio** (decided in 0020, [`docs/design/audio.md`](design/audio.md)) runs
across the blocks: **0212** playback plumbing → **0213** our own sounds and
**0214** the imported music and sounds → **0424** battle sounds, **0425** menu and
cursor sounds, **0710** `@music` in scenes, **0807** title and battle music,
**0808** credits screen, **0809** victory/defeat stings and Game Over music
(picked in **0022–0024**); later places get their music picked in **0025**
world map, **0026** capital, **0027** camp, **0028** shops (played by 1007,
1003, 0409). Level-up and EXP sounds are Nick's own work (**0605** plays
them once he supplies them). The Content ID check of the music (**0904**) was
closed without doing it: Nick chose to swap a track or add a streamer mode
only if a claim ever happens (`audio.md`).

**Title intro cinematic** (Nick, 2026-10-01: a cinematic on the title as
long as the title song, looping with it, ending on the logo): **0036**
decided 2026-10-03 (`design/title-screen.md`, *Intro cinematic*: one
press to the menu, the cinematic after 15 idle seconds, a press skips to
the logo; "Trailer cuts" storyboard; whole-step zoom; it did not wait for
**0811** title art, the logo shot shows whatever 0811 makes) →
**0228** pan and zoom over a glyph scene → **0817**
cinematic file format and player (also needs **0227**, the music clock,
which has no open dependencies and can start any time) → **0818** character and conversation shots and
**0819** the title plays it in time with the music, and **0825** shots
where units march and fight → **0820** the real shots (after Chapter 1's
map, faces and script) → **1010** the overworld shot (after the world map,
1007; Nick places it). **0041** picks the skip sound (any time). **0717**
rewrites the Chapter 1 script with fewer quips; the cinematic shows that
script's opening lines, whatever they are. Not on the Chapter 1 critical path.

**Replaceable graphics** (Nick, 2026-10-01: swapping the map's glyphs for
sprites from image files must be a small job; ADR-0038): **0231** pictures
in the frame as sprite items (no open dependencies; 0711 and 0413 now draw
bought art with it, so it is on the Chapter 1 critical path) → **0232**
`cargo xtask frame-png`, a picture of any scripted frame without a window.
**0432** the battle map is painted by a map skin from a plain-data map
scene, look unchanged (no open dependencies) → **0433** a sprite skin read
from a tileset file, behind the debug menu, and **0434** battle tests read
the scene instead of cells. Bots and play records already never touch the
look (ADR-0033); 0505 and 0508 now check it.

**Bought art on the battle map** (Nick, 2026-10-02, ticket 0038: he bought
the whole Mega Tiles bundle and chose its tiles and map sprites for the
battle map, with per-map lighting and a zoom key): after **0433** and
**0110**, **0436** units as the bought map sprites (→ **0440** they step
on the spot and walk along their path) → **0437** terrain from
the bought tilesets (auto-tiling) → **0438** per-map lighting; **0439** the
1× / 2× zoom toggle (after 0433; Nick picks its key when it is built).
Nick wants the bought-art map in his Chapter 1 playtest, so **0432, 0433,
0436, 0437 and 0440 are on the critical path** (0438 and 0439 are not).
**0039** (done 2026-10-02) settled the rest: sprites step and walk (0440,
which the playtest waits for), an up or down arrow marks a unit under an
effect (0436), dialogue shows the busts at 4× (0711), and players may
pick the glyph look in Options (**0824**, after 0436 and 0805; not on the
critical path).

## Nick's queue (answer these first; any order within a row)

Design answers unblock most of the rules work. Suggested order:

1. **0001** stats & combat · **0002** turn structure · **0003** weapons & items · **0004** magic · **0006** death & difficulty · **0007** setting, tone & story beats
2. **0005** level ups & classes (after 0001) · **0008** world structure · **0009** Chapter 1 scope (after 0007) · **0014** Combat Arts (after 0003) · **0016** the lord's unique class line (after 0005, 0009)
3. **0011** look & feel sign-off (after the font ticket 0203 shows real pixels)
4. Before the Chapter 1 playtest: **0021** bought portraits and battle sprites (done: Tiny Tales) · **0035** Harl's combat picture · **0040** more packs for fighters with no fitting art (Claude searches, Nick buys) · **0436** outline alone or with a corner mark (a sign-off) · **0413** how the combat scene looks · **0022** victory sting · **0023** defeat sting · **0024** Game Over music · **0810** how battle rewards are shown
5. Anytime, low priority: **0039** sprite units: walking, the effect mark, the glyph look as an option, face or bust · **0012** title (then **0811** title art) · **0041** the sound when a press skips the title cinematic · **0025**–**0028** world map / capital / camp / shop music · **0029** dialogue backgrounds · **0812** chapter card · **0813** transitions
6. After the Chapter 1 playtest: **0013** number scale · **0018** higher-rank Combat Arts & special weapons · **0037** more building tiles (village, gate, throne), capturing, healing tiles

Sign-offs come later as screens land (0402 cursor feel, 0404 combat, 0408
preparations, 0409 shops, 0410 spells, 0411 battle notes, 0502 enemy
phase, 0602 level up, 0701 story gates, 0704 dialogue, 0706 portraits, 0707
script, 0804 playtest). Setup steps (accounts/secrets): 0103, 0104, 0106,
0108, 0116, 0901, later 0903.

## Chapter 1 critical path

What's still open between now and Nick's playtest (0804), by dependency depth
(tickets on the same row can run in parallel sessions/worktrees). Updated
2026-10-01 from the tickets' `blocked_by` lists, after a dependency check:
done tickets were dropped, 0711 now waits for 0110 (it needs the bought
files), and 0803 now waits for 0710 and 0807 (it sets Chapter 1's music).
0231 (pictures as sprite items, ADR-0038) was added to row 1: 0711 draws
the bought faces with it.
Nick also put the combat scene (0413), Harl's picture (0035) and 0316
(non-attack skills cost uses per battle) in front of the playtest.
2026-10-02: 0714 (dialogue speakers who aren't units) was added to row 1
and the script (0707) moved to row 2 behind it; without it the script
couldn't pass its check until 0803, which waits for the script.
Later that day the script (0707) was done and left the table. It found
that a companion who dies in Classic still talks in the victory scene, so
**0715** (script lines that depend on who is still in the army) joined row
1 and **0716** (the Chapter 1 scenes rewritten with it) row 2, both in
front of the playtest unless Nick says they can wait.
Also 2026-10-02 (ticket 0038): Nick bought the art and wants the battle
map drawn with it before he plays, so the map skin (0432, then 0433), the
bought unit sprites (0436, which also needs 0110) and the bought terrain
(0437) joined the table. Ticket 0039 added 0440 (walking sprites) the
same day: Nick said the playtest waits for it.

```
 1  0022 0023 0024 0035 0231 0316 0410 0432 0435 0710 0714 0715 0801
 2  0433 0711 0716 0802 0807 0810
 3  0413 0436 0706 0809
 4  0437 0440 0803
 5  0804  ◄── Nick plays Chapter 1
```

One row-1 ticket is Nick's: **0035** is Harl's picture (0413 and 0706
wait on it; since 2026-10-02 Claude searches for candidate packs and Nick
decides, together with 0040's other gaps). The private assets repo is
done (0110 and 0116, 2026-10-02, ADR-0040): Nick uploaded it and added its
build key, so 0711 and 0436 no longer wait on it.

**0435** (found by 0411) adds the `PLAYER PHASE` banner missing at the
start of a battle, which `turn-structure.md` asks for.

The `01xx` gates (0103–0106) aren't needed by the game itself but should land
early so every later PR is checked by them.

## After Chapter 1 (`10xx`+, tickets created when relevant)

- World map (0008 chose FE Sacred Stones-style, after the linear opening
  chapters): 1007 nodes/travel/towns/save/camp, 1008 fixed and random skirmishes.
- Supports (FE GBA-style, earned in battle) and camp events: 1002, 1003.
  Parked far-future ideas: hub activities (1004), pair abilities (1005).
- Controller support on every build (web, Windows, Linux, macOS; needed for
  Steam Deck): 0032 decide buttons → 0219 input → 0220 button names in help
  bars → 0816 rebinding (after 0815). Not on the Chapter 1 critical path;
  0219 can land any time after 0032. Steam-specific gaps: 0903.
- Class tiers 4 and up; tier-3 class skills (1001).
- Claude-drawn 16×16 class icons (1006): parked. Nick chose the bought map
  sprites for units (0038), so this waits until he asks for icons.
- Audio still open after 0020: a banter conversation track, the crit sound of
  a spell with no element, fliers' movement (`docs/design/audio.md`).
  World-map / capital / camp / shop music: 0025–0028.
- Colour-blind palette variant; text size options.
- Difficulty modes; more chapters (story pipeline repeats per chapter).
- Fuzzing the content parsers (`cargo-fuzz`).
- Automated playtesting bots (Nick, 2026-09-30): 0033 decide the player
  types and targets → 0504 legal moves and luck reseeding in `core` → 0505
  `cargo xtask playtest` runner (after 0801) → 0506 Casual/Normal/Hardcore
  bots (after 0803) → 0507 trained AlphaZero-style bot (research) and 0508
  recording Nick's play to calibrate the bots (after 0802) → 0509
  autobalancing battles with the bots → 0510 generating skirmishes to order
  (after 1007, 1008). Targets: `docs/design/playtest-bots.md`; how to run
  them and read the report: [`docs/playtesting.md`](playtesting.md). Not on
  the Chapter 1 critical path; 0504 can start any time.
