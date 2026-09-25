---
name: ask-nick
description: Run a game-design decision with Nick (the owner) and record the answer in docs/design/. Use for every 00xx design-decision ticket and whenever a game-system question (stats, magic, progression, story, difficulty, world structure, look & feel) comes up. Never use for technical questions.
---

# Ask Nick a game-design question

Nick decides **game design**. Claude decides **everything technical** and never
asks Nick about languages, libraries, architecture or code. Nick has said he
likes being asked design questions *with examples from real games* and the
option to describe his own vision.

## Format for each question

Ask at most **3 questions per message**. For each:

```
### <Question in plain words>

**A. <Name> — like <Game>**
How it works in <Game>: <1–2 sentences, concrete>.
How it would feel here: <1 sentence>.

**B. …** (2–4 options total, genuinely different from each other)

**My recommendation:** <letter> — <one-sentence reason tied to what Nick
already said he wants (Fire Emblem feel, story, class changes, ASCII clarity)>.

**Or describe your own.** Mixing options is fine ("A but with B's …").
```

Rules:

- Use games Nick will likely know: Fire Emblem (GBA, Three Houses, Engage),
  Final Fantasy Tactics, Tactics Ogre, Advance Wars, Triangle Strategy, XCOM,
  Into the Breach, Disgaea, Battle Brothers, Wargroove, Unicorn Overlord.
- Show numbers when it helps (e.g. a sample level-up: `HP +1 Str +1 Spd +0 …`)
  and ASCII mockups for anything visual.
- Keep each option honest about downsides.
- If the question has sub-questions that only matter for some answers, ask
  them *after* the main answer, not all at once.
- If Nick answers vaguely ("whatever feels like FE"), take the recommended
  option, state exactly what you're recording, and let him veto.

## Recording the answer

1. Write `docs/design/<topic>.md` (topic named in the ticket) with:
   - `Decided: YYYY-MM-DD` and `Source: ticket NNNN`
   - **Nick's words**, quoted verbatim.
   - **The rule set** derived from it, concrete enough to implement: formulas,
     numbers, lists, edge cases. Mark numbers you chose as starting values
     ("tunable") vs things Nick fixed.
   - **Open sub-questions** deferred to later, if any.
2. Update the table in `docs/design/README.md`.
3. If the answer changes what downstream tickets must do, edit those tickets'
   steps/acceptance criteria in the same PR (they're still in `open/`).
4. Archive the decision ticket as in the `work-ticket` skill.
