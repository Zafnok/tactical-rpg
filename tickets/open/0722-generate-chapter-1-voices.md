---
id: "0722"
title: Generate Chapter 1's voices
type: content
milestone: M6 Story & dialogue
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0716", "0720", "0721", "0826"]
nick_input: sign-off
completed:
---

# 0722 — Generate Chapter 1's voices

## Context

The first real use of AI voices (`docs/design/voices-languages-and-script.md`,
[ADR-0046](../../docs/adr/0046-voice-clips-by-line-id.md)): every
Chapter 1 line that 0043 says is voiced, in the voices Nick picked,
made with 0721's tool. 0716 finishes Chapter 1's English scenes, so this
waits for it. Not on the Chapter 1 playtest's critical path.

## Nick input

**Sign-off.** After the merge, Nick plays Chapter 1 on Pages with voices
on and says which lines or characters sound wrong (a misread name, a flat
or odd reading, a voice that doesn't fit). His comments become a
follow-up ticket; they don't block this PR.

## Scope

**In:** the clips and manifest for `ch01.dlg` in the private assets
repository; `say.ron` entries for Chapter 1's names; the pin
(`assets-private.rev`) in this repository.

**Out (do not do):** rewording script lines so they read better aloud
(write a ticket); other chapters; Japanese voices (a ticket of its own if
0043 chose them); any clip committed to this repository.

## Implementation steps

1. `cargo xtask voice generate --dry-run`: note the line and character
   counts. If the tool bills, tell Nick the cost and wait for his go.
2. Add `say.ron` respellings for every name in Chapter 1; generate one
   line per name first and listen to each.
3. `cargo xtask voice generate`. Listen to every clip once (or, if
   listening isn't possible in the session, transcribe each clip with a
   speech-to-text tool and compare with `spoken`; list mismatches).
   Regenerate bad ones; a line that won't come out right after three
   tries is listed in the notes and left silent (remove its clip).
4. Commit to the private repository, `cargo xtask private-assets --pin`,
   commit `assets-private.rev` here.
5. Check the Pages build's size and first-scene loading on the web
   build; write both in the notes.

## Acceptance criteria

- [ ] `cargo xtask voice status` reports no missing or stale clip for `ch01` (besides lines listed as left silent).
- [ ] Every clip is OGG Vorbis, 44.1 kHz, mono, loudness-matched (the tool's own check).
- [ ] The web build plays the first scene's voices without a gap longer than the text box appearing (checked in the browser).
- [ ] `THIRD_PARTY_ASSETS.md` has the tool's row, marked AI-generated and *private*.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None new; 0238's and 0721's tests cover the code. The manifest is validated when the game loads it.

## Completion notes

