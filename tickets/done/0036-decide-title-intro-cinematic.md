---
id: "0036"
title: "Decide: the title screen's intro cinematic (how it fits the menu, storyboard, zoom)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-10-03
---

# 0036 — Decide: the title screen's intro cinematic

## Context

Nick asked for an intro cinematic on the title screen (2026-10-01):

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

**Already decided by those words (don't ask again):**

1. The title screen gets an intro cinematic.
2. It shows: a pan/zoom over a battlefield, over the overworld, past some
   characters, and brief conversation snippets with no spoilers.
3. It is as long as the title song and loops when the song loops.
4. The logo shows during the song's quiet pause at the end and holds there
   until the song (and the cinematic) start again.

**The title song, measured** (`music/new_sunrise_v1.ogg`, cue `title`,
2026-10-01, loudness per second with ffmpeg's `astats`):

| Time | What the music does |
| ---- | ------------------- |
| 0:00–0:30 | Very quiet opening, slowly rising |
| 0:31–1:24 | Middle section, moderate |
| 1:25–1:56 | Full and loudest |
| 1:57 | The last note ends; it has faded out by about 2:00 |
| 2:00–2:14 | Silence, then the file ends (2:13.7) and loops to the quiet opening |

So "the brief pause" is about **17 seconds** (1:57 to 2:14).

**What exists and what doesn't yet:**

- The title today: `crates/ui/src/screens/title.rs` (plain text, a menu).
  0811 decides the logo and title art. This ticket does **not** wait for it
  (Nick, 2026-10-02: "0811 shouldn't be a blocker for 0036"): the mockups
  end on a stand-in logo (the game's name in plain letters, as the title
  shows it today), and the storyboard's last shot is "the logo, whatever
  0811 makes it".
- `docs/design/title-screen.md`: on every build the title first shows
  `Press any key or button` (0034, 0032; built by 0226). On the web the
  music **can't** start before that first press, and the cinematic follows
  the music. So the cinematic can only start after the first press.
- Battlefield: maps exist; Chapter 1's map (0803) is about 24×16 tiles
  (`chapter-1.md`). The console is 800×512 px and a tile is 16×16 px, so
  that map is 384×256 px: at 2× it almost exactly fills the screen, and
  only at 3× is there room to pan across it.
- Overworld: the world map is ticket 1007, **after Chapter 1**. Nothing to
  show yet.
- Characters: bought faces (0706, 0711) and combat pictures (0413); none
  bought yet. Claude never draws character art (`look-and-feel.md`).
- Conversations: the dialogue screen (0704) exists; the Chapter 1 script is
  0707.
- 0813 (screen transitions) is a separate decision. Fades or cuts *between
  shots inside the cinematic* are asked here.

## Nick input

**Decision.** Use the `ask-nick` and `ascii-art` skills. Nick judges things
that move, so show **animated mockups** (an Artifact page that plays them
with the title song, drawn with the game's font atlas at real size), not
descriptions. Expect several rounds. At most three questions per message.

Name a real game for an option only if you have checked how that game
behaves; say so when unsure. The examples below are from memory and must be
checked first.

### Q1. How do the cinematic and the menu fit together?

After the first `Press any key or button`, the song and the cinematic start.

- **A. Cinematic first, then the title** (how Nick described old Pokémon:
  an intro movie, and a press takes you to the title screen). The cinematic
  fills the screen with no menu. A press jumps to the logo with the menu.
  Left alone, it reaches the logo at the pause and loops.
- **B. Menu over the cinematic.** The cinematic plays behind a small menu
  box that is always there. The player can start at any moment; the box
  covers part of the picture.
- **C. Title first, cinematic when left alone** (GBA Fire Emblem: leave
  "Press Start" alone and a demo plays, then the title comes back). The
  logo and menu show as today; the cinematic takes over if nobody presses
  anything. Players who start quickly never see it.
- **D.** "Describe your own."

Follow-ups that depend on the answer (ask after it):

- In A: does the menu stay up once the player has pressed, or does the
  cinematic take the screen again at the next loop if they do nothing?
- In A and C: is the menu shown on the logo shot during the pause even if
  nobody pressed anything?
- Does a press ever restart the song, or does the song always keep playing?
- Coming back to the title from the game (after a battle, "To be
  continued", Game Over): cinematic from the start, or straight to the
  logo and menu?

### Q2. The storyboard

Offer two or three **different** storyboards as animated mockups, each a
list of shots with start times against the table above. Nick fixed the
ingredients (battlefield, overworld, characters, conversation snippets,
logo at the pause); the order, how long each lasts and what each shows are
open. For each shot say exactly what is on screen.

Ask within the storyboards:

- **Battlefield:** a still map with units standing on it, or a few scripted
  moves and an attack playing out? (The second needs its own ticket: write
  it if he picks it.)
- **Characters:** which characters, and shown how (faces, combat pictures,
  name shown or not)? Only art we can buy or have (`look-and-feel.md`).
- **Conversation snippets:** shown in the game's real dialogue screen, or
  as lines over another shot? Propose the actual lines (see step 3).
- **Between shots:** hard cuts, a fade through black, or something else.
- **The pause:** the logo holds for all 17 seconds as he described. Mention
  the number; he may want the silent tail of the song trimmed instead
  (that would be a re-cut in `assets-src/audio/import.py`, its own ticket).

### Q3. How does "zoom" look in a glyph game?

Glyphs are pixel art, so zoom has choices. Show each as a short animated
mockup over a real map:

- **A. Whole steps:** the picture is shown at 2× or 3× and stays crisp; a
  "zoom" is a cut or a quick step from one size to the next.
- **B. Smooth zoom:** the size changes gradually; in-between sizes make
  pixels uneven and shimmer a little.
- **C. Pan only:** each shot has one fixed size; the camera only slides.
- **D.** "Describe your own."

### Q4. The overworld shot before the world map exists

- **A. Leave it out for now.** The first version has no overworld shot;
  ticket 1010 adds it when the world map (1007) is built.
- **B. A map of the land made just for the cinematic,** glyph-drawn, before
  1007. It must match the story bible's geography and Nick signs it off;
  1007's real map may later look different.
- **C.** "Describe your own."

### Q5. A setting to turn it off?

Whether the Options menu (0805) gets a setting that shows a still title
(logo and menu) instead of the cinematic, for players who dislike motion.
Yes / no / later.

## Scope

**In:**
- The questions above, with mockups, until Nick has picked.
- A new section *Intro cinematic* in `docs/design/title-screen.md`: Nick's
  words, the rules, the final storyboard as a shot table (start time, what
  is on screen, movement, size), the approved snippet lines and characters,
  Claude's starting rules marked as such, and open sub-questions.
- Update the table in `docs/design/README.md`.
- Update the tickets that build it so they match the answers: 0228, 0817,
  0818, 0819, 0820, 1010. Remove what the answers make unnecessary, and
  write new tickets for anything they add (e.g. scripted battle action, a
  cinematic-only land map, an Options setting in 0805, re-cutting the
  song's tail).

**Out (do not do):**
- Any game code.
- The logo and title art (0811). Transitions between screens (0813).
- Writing new story. Snippets come from scenes that exist or are planned in
  `docs/story/`; which lines are safe to show is checked against
  `docs/story/outline.md` and the ledger (ADR-0011).

## Implementation steps

1. Read `docs/design/title-screen.md`, `look-and-feel.md`, `audio.md`,
   `world-structure.md`, `chapter-1.md` and, if it is done, 0811's result.
   Listen to the
   title song with the timing table in hand.
2. Build the mockup page (scratchpad, published as an Artifact): the font
   atlas, a real map, the logo (0811's if it is done, else the stand-in
   above), and store previews for character
   art (they stay in the scratchpad, `ascii-art` skill). It plays each
   storyboard against the song and can show Q1's options and Q3's zooms.
3. Snippets: with the `story-writing` skill, pick candidate lines that give
   away nothing past the opening premise. List them for Nick with the scene
   they come from; he approves the list (it is a story decision). If the
   Chapter 1 script (0707) isn't written yet, say so and take lines from
   the beat sheet's planned scenes only with his agreement; 0820 then uses
   the final wording.
4. Ask Q1–Q5 over several messages; iterate on the mockups.
5. Record the answers; update the design README and the tickets listed in
   Scope.

## Acceptance criteria

- [x] `docs/design/title-screen.md` has an *Intro cinematic* section with
      Nick's words, the rules for Q1–Q5 and the storyboard table.
- [x] Every shot in the storyboard names its start time, content, movement
      and size, and the times fit the song (logo at the pause).
- [x] The snippet lines and characters Nick approved are listed. (The
      characters and the two scenes are; Nick approved no lines, see the
      completion notes.)
- [x] Tickets 0228, 0817, 0818, 0819, 0820 and 1010 match the answers; new
      tickets exist for anything the answers added.
- [x] `cargo xtask ticket-lint` passes.

## Tests required

- None (documentation and tickets only).

## Completion notes

Done 2026-10-03, over three rounds of animated mockups (an Artifact page
drawn with the game's font atlas and palette at 800×512, playing against
the title song; it stays outside the repo).

**Decoupled from 0811 first** (Nick: "0811 shouldn't be a blocker for
0036"): the mockups ended on a stand-in logo, and the storyboard's last
shot is "the logo (0811)", whatever that becomes. 0811 and the roadmap
say so.

**What Nick decided** (`docs/design/title-screen.md`, *Intro cinematic*):

- Q1: first he picked A (cinematic first), then replaced it with his own
  flow: one press reaches the logo and menu; after 15 idle seconds the
  menu fades and the cinematic takes over, joining the song where it is;
  a press during the cinematic plays a sword-clash sound and jumps to the
  song's silence with the logo and menu; left alone, the menu fades at the
  loop and the cinematic plays from the start. Back from the game: logo
  and menu, same rules.
- Q2: storyboard 2, "Trailer cuts": hard cuts, snippets in the real
  dialogue screen, six faces with names, units marching and fighting. The
  logo holds for the whole silence (no re-cut of the song). The lead
  follows the most recent save.
- Q3: whole-step zoom (2×, 3×, 4×), one size per shot, no gradual zoom.
- Q4: no overworld shot for now; 1010 adds it and now says plainly that it
  changes the cinematic and asks Nick where the shot goes.
- Q5 (a setting to turn it off): "Later". Open sub-question; 0820's
  sign-off asks again.

**Deviations from the ticket:**

- **No approved snippet lines.** Nick rejected the Chapter 1 lines shown
  ("cheesy and cringe"; too many quips for an opening) and decided the
  cinematic simply plays the opening lines of the real script file. So
  the design doc lists the two scenes and the six characters, not lines,
  and 0820 no longer has its own dialogue file. The third acceptance box
  is ticked on that basis.
- Step 3's spoiler rule became: only the openings of the two scenes before
  the Chapter 1 battle, never past a reply choice.
- The storyboard's first two shots were swapped compared with the mockup
  (the party pan first, the first conversation second), because on launch
  the menu hides the first 15 seconds.
- 0805 was not changed (no Options setting yet).
- Game comparisons: old Pokémon's intro-then-title was Nick's own
  reference; that Fire Emblem: The Blazing Blade plays its class roll
  when the title is left alone was only loosely checked and said so.

**Tickets updated:** 0228 (whole steps 1–4 only), 0817 (hard cuts; a
whole-frame brightness for the title's fade), 0818 (decided answers; the
lead from the last save), 0819 (the flow as states; the skip without an
audio seek), 0820 (the storyboard; lines from `ch01.dlg`; waits for 0827),
1010 (changes the cinematic; Nick places the shot), 0811 (how the logo is
used), 0706 (the lead's default face for the cinematic).

**New tickets:** 0041 (decide the skip sound), 0724 (Chapter 1 script:
fewer quips, to Nick's tone note now in `setting-and-tone.md`), 0827
(cinematic shots where units march and fight).

**Claude's starting rules** (listed in the design doc for Nick to veto):
the menu also shows when the cinematic reaches the logo by itself; any
press while the menu is up restarts the 15 seconds; a press in the silence
puts the menu back on the 15-second rule; the skip press does nothing
else; the timer runs only on the title itself; fade lengths (about 1 s
out, 0.5 s in); the cinematic makes no sounds of its own; the skip sound
is the sword hit until 0041; snippet length is whatever opening boxes can
be read in the shot's time.

**0724 waits until after the playtest** (Nick: "nah it can wait til
after"), so it is blocked by 0804.
