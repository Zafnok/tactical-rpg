---
id: "0213"
title: "Render our own sounds with cargo xtask sfx"
type: infra
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0212"]
nick_input: sign-off
completed:
---

# 0213 — Render our own sounds with `cargo xtask sfx`

## Context

In ticket 0020 Nick picked five sounds that we make ourselves
([`docs/design/audio.md`](../../docs/design/audio.md)): `menu_move` (B),
`menu_select` (L), `menu_cancel` (B), `miss` (W2) and `heal` (HE5). He picked
them by ear on a Web Audio page. That page's source is committed at
[`assets-src/audio/sound-audition.html`](../../assets-src/audio/sound-audition.html),
and its JavaScript is the exact recipe for each sound:

- `MENU.move` id `B`
- `MENU.select` id `L`
- `MENU.cancel` id `B`
- `SFX['miss:quick']`
- `HEAL.HE5`

They are built from `tone`, `burst`, `pulse`, `env`, `wet` and the master
chain in `init()`.

This ticket ports those recipes to Rust, so the WAV files are generated
reproducibly from code, like the font atlas (`cargo xtask font-atlas`).

## Nick input

**Sign-off:** Nick plays each generated WAV next to the same button in the
"Made in-house" section of the audition page
(https://claude.ai/artifact/68oDV3HuxB2Vc2zHBzadRc). They should sound the
same. Put the five WAVs somewhere he can click them (e.g. attach them in
the PR, or a small HTML page with `<audio>` tags sent with SendUserFile).

## Scope

**In:**
- `crates/xtask/src/sfx.rs`, a `cargo xtask sfx` command: a small offline
  synth that mirrors the page's building blocks.
  - Oscillators: sine, triangle, sawtooth, and the pulse wave built from the
    same Fourier series as `pulse(duty)`, 64 harmonics.
  - Linear and exponential frequency ramps.
  - The `env` gain envelope.
  - Biquad lowpass, highpass and bandpass filters matching Web Audio's
    `BiquadFilterNode` formulas, with frequency ramps.
  - A looping white-noise source.
  - The `wet` send into a generated room reverb, as in `init()`: 1.8 s of
    noise decaying as (1 − t)^3.2, at 0.32 gain.
  - The master `DynamicsCompressor` (threshold −14 dB, ratio 4). A simple
    compressor is fine; note any difference.
- Output: `assets/audio/sfx/{menu_move,menu_select,menu_cancel,miss,heal}.wav`,
  44.1 kHz, 16-bit, mono. Trim trailing silence (below −60 dB), and
  normalise so the sounds keep the page's relative loudness.
- **Deterministic:** noise and the reverb use a fixed seed, so the same
  command always writes byte-identical files. A test checks this.
- Add the five cues to `assets/audio/audio.ron` with credit `Own`.
- A row in `assets-src/README.md` already points at this command; make sure
  it's accurate.

**Out (do not do):**
- Playing them in menus or battle (0424, 0425).
- Changing the sounds. If a recipe can't be matched exactly, get as close as
  possible and describe the difference in the Completion notes.

## Implementation steps

1. Read `assets-src/audio/sound-audition.html` (the `<script>`): `init`,
   `tone`, `burst`, `pulse`, `env`, `wet`, `MENU`, `SFX`, `HEAL`, `bloom`,
   `soft`, `chip`, `nf`. Note the per-bus gains: `sfxBus` 0.9, master = the
   volume slider, default 70 → 0.7^1.6.
2. Implement the synth as plain functions over `Vec<f32>` buffers at
   44.1 kHz. Use `hound` (Apache-2.0) to write WAVs; it's only in xtask, so
   it isn't shipped.
3. Port the five recipes one to one, keeping the same numbers.
4. `cargo xtask sfx` writes the files; `cargo xtask sfx --check` verifies
   the committed files match (for CI, like the font atlas if it has a check).
5. Add the manifest entries.

## Acceptance criteria

- [ ] `cargo xtask sfx` writes five WAVs; running it twice gives
      byte-identical files (test).
- [ ] Unit tests: the pulse wave's harmonics match the page's formula; a
      lowpass biquad attenuates a tone above cutoff; `env` reaches its peak
      at `t + a`.
- [ ] Each WAV's length is within 50 ms of the page recipe's length (attack
      + hold + decay, plus the reverb tail trimmed at −60 dB). `heal` is about
      1.5 s; `menu_move` is under 0.15 s.
- [ ] Manifest validates with the five cues.
- [ ] Nick signed off that they sound like the page.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: synth building blocks, determinism.
- Snapshot / integration: none beyond the manifest check.

## Completion notes

*(Filled in by the session that completes the ticket.)*
