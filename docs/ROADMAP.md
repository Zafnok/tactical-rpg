# Roadmap

Goal of the first phase: **a playable, story-driven Chapter 1** on Windows and
in the browser, then publish to itch.io, then Steam.

All work is in [`tickets/`](../tickets/README.md). Status is read from the
folders: `tickets/open/` vs `tickets/done/`.

## Milestones

| Block | Milestone | Outcome | Tickets |
| ----- | --------- | ------- | ------- |
| `00xx` | Design decisions | Nick's answers recorded in `docs/design/` | 0001–0014 |
| `01xx` | M0 Foundation | Workspace, CI on 3 OSes, security scanners, SonarCloud, mutation gate, release + Pages pipelines | 0101–0108 |
| `02xx` | M1 Engine | Coloured glyph console in a window and browser, input with vim keys, screens + test harness | 0201–0207 |
| `03xx` | M2 Core rules | Maps, units, pathfinding, combat, turns, items, rewind, gold/shops, spells, terrain magic, class skills — pure and heavily tested | 0301–0311 |
| `04xx` | M3 Battle UI | Cursor, move/attack loop, forecast, combat playback, info screens, tips, item menus, preparations, shops, spell menus, battle notes, skill menus | 0401–0412 |
| `05xx` | M4 Enemy AI | Enemies that fight back, animated enemy phase | 0501–0502 |
| `06xx` | M5 Progression | EXP and class points, level-up screen, promotion and reclass | 0601–0603 |
| `07xx` | M6 Story & dialogue | Story bible, dialogue engine, two-portrait scenes, Chapter 1 art + script, lead reply choices | 0701–0708 |
| `08xx` | M7 Chapter 1 | Full flow, saves, Chapter 1 content, **Nick's playtest**, options | 0801–0805 |
| `09xx` | M8 Release | itch.io, Windows polish, Steam readiness | 0901–0903 |

## Nick's queue (answer these first; any order within a row)

Design answers unblock most of the rules work. Suggested order:

1. **0001** stats & combat · **0002** turn structure · **0003** weapons & items · **0004** magic · **0006** death & difficulty · **0007** setting, tone & story beats
2. **0005** level ups & classes (after 0001) · **0008** world structure · **0009** Chapter 1 scope (after 0007) · **0014** Combat Arts (after 0003)
3. **0011** look & feel sign-off (after the font ticket 0203 shows real pixels)
4. Anytime, low priority: **0010** supports · **0012** title (after the story bible)

Sign-offs come later as screens land (0402 cursor feel, 0404 combat, 0408
preparations, 0409 shops, 0410 spells, 0411 battle notes, 0502 enemy
phase, 0602 level up, 0701 story gates, 0704 dialogue, 0706 portraits, 0707
script, 0804 playtest). Setup steps (accounts/secrets): 0103, 0104, 0106,
0108, 0901, later 0903.

## Chapter 1 critical path

Dependency depth from the ticket graph (tickets on the same row can run in
parallel sessions/worktrees):

```
 1  0101 ─ plus Nick's 0001 0002 0003 0004 0006 0007 0008
 2  0102  0201                      (0005, 0009, 0701 as answers arrive)
 3  0202  0204  0301  0103 0104 0105
 4  0203  0302  0106
 5  0205  0206  0303  0304  0702   (0011 look sign-off)
 6  0207  0305  0401  0703  0107  0108
 7  0306  0402  0704  0706
 8  0403  0309  0601  0308  0708
 9  0404  0407  0408  0409  0310  0311  0501  0707
10  0405  0602  0307  0410
11  0502  0705  0406  0412
12  0801
13  0802  0411
14  0803
15  0804  ◄── Nick plays Chapter 1
```

The `01xx` gates (0103–0106) aren't needed by the game itself but should land
early so every later PR is checked by them.

## After Chapter 1 (`10xx`+, tickets created when relevant)

- World map / open-world-lite, if chosen in 0008.
- Support conversations, if chosen in 0010.
- Controller support (needed for Steam Deck) — created by 0903.
- Class tiers 4 and up; tier-3 class skills (1001).
- Audio and music.
- Colour-blind palette variant; text size options.
- Difficulty modes; more chapters (story pipeline repeats per chapter).
- Fuzzing the content parsers (`cargo-fuzz`).
