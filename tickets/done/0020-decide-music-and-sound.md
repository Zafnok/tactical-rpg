---
id: "0020"
title: "Decide: music and sound effects"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-28
---

# 0020 — Decide: music and sound effects

## Context

The game has no audio. The roadmap listed "Audio and music" as post–Chapter 1,
but Nick asked for it now (2026-09-28): "at a minimum title screen music,
conversation music, battle music", plus SFX "for i.e. sword attack, block,
arrow hitting, magic casting, selection (small beep -- not high pitched but
just a note of selecting...)".

Constraints already fixed elsewhere:

- ADR-0013: shipped audio must be `CC0-1.0`, `CC-BY-4.0` (needs an in-game
  credits screen) or made by us. Nothing that costs money. CC-BY-3.0,
  CC-BY-SA and non-commercial licenses are denied.
- Tone (`docs/design/setting-and-tone.md`): FE-style medieval fantasy war,
  dark with warmth and humour.

Technical side (Claude's call, recorded in an ADR by the first audio
implementation ticket, not asked of Nick): macroquad's `audio` feature
(quad-snd, MIT/Apache) plays WAV/OGG on Windows and web. On web, sound can
start only after the first keypress. `ui` screens emit sound cues as data, and
only `app` plays them (ADR-0004).

## Nick input

**Decision.** Nick listens to the audition page
(https://claude.ai/artifact/68oDV3HuxB2Vc2zHBzadRc): code-made menu and combat
sounds, three 8-bar music sketches in two instrument sets, and the vetted
free libraries. Then he answers the questions below.

## Questions

1. **Music style**: orchestral / GBA-FE-like low-fi ensemble / chiptune /
   medieval folk, or a mix.
2. **Where music comes from**: free CC0/CC-BY library tracks, code-made
   in-house, Nick composes (BeepBox / Bosca Ceoil), or a composer later
   (costs money, Nick's call against ADR-0013's "nothing that costs money").
3. **SFX style**: retro synth, recorded foley (Kenney etc.), or a mix. Also
   which menu-sound family (audition page A–D).

Sub-questions to ask *after* the main answers (only if relevant):

- Separate player-phase and enemy-phase battle music (as in Fire Emblem)?
  Boss, victory, defeat, sad-scene cues?
- If library music: pick the specific tracks per cue (a second audition round).

## Round notes

The working log (Nick's words for every round) moved to the appendix of
[`docs/design/audio.md`](../../docs/design/audio.md).

## What to record

`docs/design/audio.md` with Nick's words verbatim; the style; the source per
category; the cue list (which screens/events get music, which get SFX); and the
chosen tracks/sounds if picked. Update `docs/design/README.md`. Then use
`write-ticket` to create the implementation tickets (don't implement here),
roughly:

- `02xx` audio playback: enable macroquad `audio`, a sound-cue type that `ui`
  emits and `app` plays, music looping and switching per screen, the ADR.
- Music per screen (title, dialogue, battle) and SFX on menu and combat events.
- Import the chosen assets into `assets/audio/` with `THIRD_PARTY_ASSETS.md`
  rows and license files.
- Volume settings in the options menu (0805 currently lists audio volume as
  *Out*; edit it).
- A credits screen crediting every third-party work, CC0 included (Nick,
  round 1; stricter than ADR-0013, which only requires it for CC-BY).
- Update `docs/ROADMAP.md` (audio is no longer post–Chapter 1).

## Acceptance criteria

- [x] Nick answered the three questions (and the follow-ups).
- [x] `docs/design/audio.md` written; README table updated.
- [x] Implementation tickets created and listed in Completion notes.
- [x] Ticket archived.

## Completion notes

Six listening rounds on a Web Audio page
(https://claude.ai/artifact/68oDV3HuxB2Vc2zHBzadRc), plus two rounds of plain
design questions. Its final source is committed as
`assets-src/audio/sound-audition.html`: the recipes for our own sounds.
Result: [`docs/design/audio.md`](../../docs/design/audio.md), with 21 music
cues and 21 sound cues, each with its source and license.

Deviations from the plan: Nick chose track by track instead of picking one
"style" and one "source". The three questions became: recorded third-party
music and sounds (credit everyone), our own menu, dodge and heal sounds, and
no code-made music.

Implementation tickets created:
- **0212** audio playback plumbing (cues as data, `app` plays; ADR-0026)
- **0213** `cargo xtask sfx` renders our own sounds (Nick signs off by ear)
- **0214** import the chosen third-party files, with credits and tags
- **0424** battle sounds
- **0425** menu and map-cursor sounds
- **0710** `@music` in dialogue scripts
- **0807** title music and per-battle music, with the skirmish pool
- **0808** credits screen for every third-party work
- **0904** Content ID check before release

Existing tickets edited:
- 0805: music and sound volume moved from *Out* to *In*.
- 0803: Chapter 1 picks its music cues.
- 1007: place music on world-map nodes.
- 1008: skirmishes use the pool.
- `docs/ROADMAP.md`: audio is no longer post–Chapter 1.

**Claude's starting rules** (not decided by Nick; he can veto any):
- The map cursor tick plays at 60 % of the menu move sound's volume. Nick
  said "at a lower vol maybe".
- Switching music fades the old track out over 0.5 s. A cue that's already
  playing doesn't restart.
- Imported files are volume-matched.
- Placeholders until Nick picks: a banter scene plays `talk_calm`; a crit
  with a spell with no element plays the normal magic hit.
- In the new tickets:
  - An Absorb strike plays the normal magic hit (0424).
  - Selecting a disabled menu item is silent, and advancing dialogue text is
    silent (0425).
  - The Preparations screen already plays the battle's track, and Game Over
    and "To be continued" stop the music (0807).
  - Music and sound volume run 0–10 and default to 8 (0805).
