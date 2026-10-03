---
id: "0720"
title: The dialogue screen plays each line's voice
type: feature
milestone: M6 Story & dialogue
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0043", "0238"]
nick_input: answer-first
completed:
---

# 0720 — The dialogue screen plays each line's voice

## Context

0238 can play a clip for a dialogue line
([ADR-0046](../../docs/adr/0046-voice-clips-by-line-id.md)). Nick decides
in 0043 what is voiced and how voice and text go together
(`docs/design/voices-languages-and-script.md`, *Voices*). This ticket
makes the dialogue screen ask for the clips.

## Nick input

**Answer first:** ticket 0043 (questions 2, 4 and 7). Build what the
design doc says; anything a player would notice that it doesn't cover
goes back to Nick through `ask-nick`.

## Scope

**In:** `crates/ui/src/screens/dialogue.rs` asking for, stopping and
preloading voices; scenes shown during battles (ADR-0030) included, since
they use the same screen.

**Out (do not do):** voices outside dialogue scenes (battle shouts are a
new ticket if 0043 asks for them); an auto-advance mode (a new ticket if
0043 asks for it); Options (0826); changing the typewriter.

## Implementation steps

1. When a text box for a line appears, call `ctx.audio.play_voice(line
   id, lead gender)`. A line split over several pages plays its clip on
   the first page only and keeps playing across the pages.
2. Advancing to another line, skipping the scene, opening a reply choice
   or leaving the screen calls `stop_voice()` first.
3. Which lines ask at all follows 0043: narration or not, the lead's
   lines, reply text. Put the rule in one function
   (`fn is_voiced(step) -> bool`) with the design doc's sentence quoted
   above it.
4. When a scene starts, and after each line, send `PreloadVoices` for the
   lines that can come next (both branches' first lines at a choice).
5. The typewriter and text speed behave as today unless 0043's question
   7 says otherwise.
6. Rewinding a battle past a scene, or replaying one, plays voices again
   like the first time.

## Acceptance criteria

- [ ] Harness test with `test.dlg` and 0238's test manifest: each voiced line queues one `PlayVoice` when its box appears; advancing queues `StopVoice` before the next.
- [ ] Skipping a scene queues no `PlayVoice` for skipped lines and stops the current one.
- [ ] With voices off, no voice request is queued.
- [ ] Lines 0043 says aren't voiced never ask (one test per kind).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: Harness tests on `audio_requests()`; no snapshot changes.

## Completion notes

