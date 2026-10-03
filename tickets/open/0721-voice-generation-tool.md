---
id: "0721"
title: "`cargo xtask voice`: generate the missing and stale clips, and print recording scripts"
type: infra
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0043", "0238"]
nick_input: setup
completed:
---

# 0721 — `cargo xtask voice`: generate the missing and stale clips, and print recording scripts

## Context

Voice clips are made outside the game by a tool and kept in the private
assets repository ([ADR-0046](../../docs/adr/0046-voice-clips-by-line-id.md)
rules 5 and 7). Nick picked the text-to-speech tool and each character's
voice in 0043 (`docs/design/voices-languages-and-script.md`, *Voices*).
Scripts will keep changing (and a hired writer may rewrite them), so the
tool must redo only what changed, every time.

## Nick input

**Setup**, only if 0043's tool needs it: installing it on his machine, or
an account and a key. Write the exact steps in `docs/voice.md`. A key is
read from an environment variable or a git-ignored file, never typed
into chat, never committed. If the tool is a service, say what one
chapter costs before running it and let Nick say go.

## Scope

**In:** the `voice` xtask with `status`, `generate` and `script`
subcommands; the cast file; loudness matching; the manifest written.

**Out (do not do):** generating Chapter 1 for real (0722); anything in
the game crates beyond reusing `trpg_content`; running in CI; calling
the tool from the game.

## Implementation steps

1. `voice/<lang>/cast.ron` (in the private repository's `voice/`):
   character id → the tool's voice id and settings, from 0043's cast
   table. Narration has its own entry if 0043 voices it.
2. `cargo xtask voice status [--lang en]`: loads the scripts, cast and
   manifest; prints per scene the lines that are voiced by 0043's rule
   and have no clip, a stale clip, or a clip whose cast voice changed;
   and clips whose line no longer exists.
3. `cargo xtask voice generate [--scene <id>] [--dry-run]`: for each line
   `status` lists, send `spoken_text` (0238) to the tool with the
   speaker's voice, save `voice/<lang>/<scene>/<line id>[.m|.f].ogg`
   (OGG Vorbis, 44.1 kHz, mono), and add or replace the manifest entry
   with `made_by: Generated { tool, model, date }`. Delete clips whose
   line is gone. `--dry-run` prints the count of lines and characters
   (what a service bills by) and makes nothing.
4. The tool is called through one small trait (`Synth: text + voice →
   audio`), with the chosen tool's implementation behind it, so swapping
   tools later is one file. A fake implementation (a tone per line) is
   used in tests.
5. Loudness: match every clip to one level, the way 0214's import
   matched the music (reuse its code), and trim silence at both ends to
   at most 100 ms (*tunable*).
6. Pronunciation: `voice/<lang>/say.ron`, name id or word → how to spell
   it for the tool (invented names are often misread). `spoken` in the
   manifest stays the real text; the respelling is only what is sent.
   A change to `say.ron` marks the lines using that name for redoing.
7. `cargo xtask voice script --character <id>`: prints a recording
   script for a human actor: per line the id, scene, the line before it
   (who said what), the words, and the file name to deliver. This is how
   a cast replaces the generated clips later (ADR-0046 rule 10).
8. Never overwrite a `Recorded` clip; `generate` skips them and `status`
   lists them as stale for a human to re-record.
9. `docs/voice.md`: how to run all three, where the files go, and the
   commit-and-pin steps for the private repository (`cargo xtask
   private-assets --pin`).

## Acceptance criteria

- [ ] With the fake synth: `generate` on `test.dlg` makes one clip per voiced line; a second run makes none (test).
- [ ] Rewording a line, renaming a name in it, or changing its speaker's voice makes exactly that line's clip regenerate (tests).
- [ ] A `Recorded` clip is never overwritten (test).
- [ ] `--dry-run` writes nothing and prints the counts.
- [ ] `script --character` output for a test character is snapshot-tested.
- [ ] One real line generated with the chosen tool plays in the game (say which, in the notes).
- [ ] No key or bought file is committed to this repository.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the planning (what to make, redo, delete) as a pure function of scripts, cast, manifest and `say.ron`.
- Integration: a temp folder run with the fake synth.

## Completion notes

