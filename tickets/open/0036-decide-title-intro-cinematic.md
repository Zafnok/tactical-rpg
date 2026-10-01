---
id: "0036"
title: "Decide: the title screen's intro cinematic (how it fits the menu, storyboard, zoom)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0811"]
nick_input: decision
completed:
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
  0811 decides the logo and title art; this ticket comes after it so the
  mockups can end on the real logo.
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
   `world-structure.md`, `chapter-1.md` and 0811's result. Listen to the
   title song with the timing table in hand.
2. Build the mockup page (scratchpad, published as an Artifact): the font
   atlas, a real map, the logo from 0811, and store previews for character
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

- [ ] `docs/design/title-screen.md` has an *Intro cinematic* section with
      Nick's words, the rules for Q1–Q5 and the storyboard table.
- [ ] Every shot in the storyboard names its start time, content, movement
      and size, and the times fit the song (logo at the pause).
- [ ] The snippet lines and characters Nick approved are listed.
- [ ] Tickets 0228, 0817, 0818, 0819, 0820 and 1010 match the answers; new
      tickets exist for anything the answers added.
- [ ] `cargo xtask ticket-lint` passes.

## Tests required

- None (documentation and tickets only).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
