---
id: "0226"
title: "\"Press any key or button\" on every build, and it decides whether to show Pick your layout"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0219", "0220"]
nick_input: sign-off
completed:
---

# 0226 — "Press any key or button" on every build

## Context

Ticket 0224 added a `Press any key` prompt to the web title (it unlocks
browser sound). In ticket 0032 (controller) Nick revisited that: the prompt
now shows on **every build**, accepts a key **or a controller button**,
shows a keyboard picture and a controller picture, and the first press
decides whether "Pick your layout" is needed. Rules:
[`docs/design/title-screen.md`](../../docs/design/title-screen.md) (*Every
build, keys and buttons*) and
[`docs/design/controls.md`](../../docs/design/controls.md) (*Pick your
layout with a controller*).

Today (0224) `Game::start` pushes the layout picker **on top of** the
title at first launch, so on web the picker comes before the prompt. The
new order is prompt first, then the picker only if a key was pressed.

Builds on 0219 (pad presses reach `Game`) and 0220 (last-used device in
`InputState`). Follow the `keyboard-input` skill (the prompt names no key)
and the `ascii-art` skill (the two pictures).

## Nick input

**Sign-off, two steps:**
1. **Before building:** pick the keyboard and controller pictures from 2–3
   rendered mockups (step 1), in the title screen at real size.
2. **After merge**, on Pages and the Windows download: with a controller,
   press a button at the title (no layout screen), play, then press a key
   (layout screen opens over the game). Pick a layout; it never shows again.

## Scope

**In:**
- The prompt on every build, once per launch; `Press any key or button`
  with the chosen pictures.
- Any key or any pad button dismisses it (the press does nothing else, as
  in 0224).
- First launch order: prompt → (key) "Pick your layout" / (button) menu.
- A pad button while the picker is open closes it without choosing.
- While no layout has been chosen: a key press after pad use opens the
  picker straight away over whatever screen is showing (battle included);
  that key does nothing else.
- Once a layout is picked, the picker never opens by itself again.
- Web: if a pad press doesn't unlock the browser's sound, the music starts
  at the first key press (check Chrome and Firefox; note what happens).

**Out (do not do):**
- Title art (0811). Changing the picker screen itself.
- Button names in help bars (0220). Rebinding (0816).

## Implementation steps

1. **Mockups** (`ascii-art` skill): 2–3 options for a small keyboard
   picture and controller picture on the title line (glyph art in the
   game's font, a few cells tall, e.g. a keycap row `▐q w e▌` and a pad
   outline), rendered at real size in the full title screen. Send to Nick;
   record his pick in `title-screen.md`.
2. `crates/app/src/main.rs`: set `KeyPrompt::Waiting` on every build (not
   only wasm).
3. `crates/ui/src/game.rs`: the prompt is dismissed by a key **or** a pad
   press; remember which (`FirstPress::Key | Pad`). Stop pushing the layout
   picker in `Game::start`; instead, when the prompt is dismissed by a key
   and `ctx.layout().is_none()`, push `LayoutPickerScreen`.
4. Layout still unchosen: when 0220's `InputState::device()` changes from a
   pad to the keyboard, push `LayoutPickerScreen` on top of the current
   stack and swallow that key. While the picker is open, a pad press pops
   it without choosing. (So no key acts before a layout is chosen: the
   first one always opens the picker.)
5. `crates/ui/src/screens/title.rs`: the prompt text `Press any key or
   button` (a constant, as `PRESS_ANY_KEY` today) with the pictures from
   step 1.
6. Update `title-screen.md` if any starting rule changes while building.

## Acceptance criteria

- [ ] Nick picked the pictures (step 1) and signed off the flow.
- [ ] Snapshot: title waiting with `Press any key or button` and both
      pictures.
- [ ] Harness (first launch, no saved layout): key at the prompt → picker
      opens; pad press at the prompt → menu, no picker.
- [ ] Harness: picker open, pad press → picker closes, no layout saved.
- [ ] Harness: no layout, pad used in battle, then a key → picker opens
      over the battle and the key didn't act; after picking, switching pad
      → key never opens it again.
- [ ] Native builds show the prompt (test with the native `Ctx`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot: waiting title.
- Integration (Harness): every flow in the acceptance criteria, pad events
  only where needed (no hardware).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
