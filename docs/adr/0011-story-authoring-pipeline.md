# ADR-0011: Story authoring pipeline with LLMs

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Nick wants an engaging overall story plus personal character stories (Fire
Emblem style), presented as dialogue with two ASCII character portraits on
screen. He can provide a few story beats but not full character arcs, and asked
us to look into story-generating LLMs.

### What we looked at

- **Dedicated fiction tools** (Sudowrite, NovelCrafter, NovelAI and similar)
  are front-ends over general-purpose LLMs, tuned for novel prose. They add
  useful structure (story bibles, "codex" entries, beat sheets) but cost money,
  don't know our dialogue file format, and add a copy-paste step between tool
  and repo.
- **Frontier general models** (Claude Fable / Opus) are as strong or stronger at
  long-form structured writing, and are already how this project is built.
- The consistent lesson from those tools and from practice: **LLM story quality
  comes from process, not from the model alone.** Unstructured "write me a story"
  gives generic results. What works is a persistent *story bible*, top-down
  outlining, explicit character arcs (want / need / flaw / change), and a
  separate critique pass.

## Decision

We borrow the *process* of the fiction tools and run it with Claude, inside the
repo, via the `story-writing` skill.

### Artefacts (all under `docs/story/`)

| File | Contents | Owner |
| ---- | -------- | ----- |
| `beats.md` | Nick's story beats, verbatim. **Canon — never contradicted.** | Nick (captured by ticket 0007) |
| `bible.md` | World, history, factions, themes, tone, rules of magic/tech, glossary | Generated, Nick signs off |
| `characters/<id>.md` | One sheet per named character: role, want, need, flaw, secret, arc across the game, voice notes + 3 sample lines, relationships | Generated |
| `outline.md` | Act → chapter list; per chapter: goal, conflict, turn, which personal arcs advance | Generated, Nick signs off |
| `chapters/chNN.md` | Beat sheet for one chapter: scenes, who's on screen, what changes | Generated |
| `ledger.md` | Continuity ledger: what each character knows / has done / relationships as of each chapter | Updated after every chapter script |

Final dialogue is written straight into `assets/dialogue/*.dlg` (ADR-0005).

### Pipeline

1. **Beats** — Nick gives beats (ticket 0007). Recorded verbatim.
2. **Bible + cast** — generate world and character sheets from the beats.
   *Gate: Nick reads a one-page summary and approves or redirects.*
3. **Outline** — act/chapter structure. Every chapter must advance the main plot
   **and** at least one personal arc. *Gate: Nick approves.*
4. **Chapter beat sheet** — scenes for one chapter.
5. **Script** — write `.dlg` scenes in character voice.
6. **Critique pass** — a *separate* session (or a clearly separated step) reviews
   the script against a checklist: voice consistency with character sheets,
   show-don't-tell, no exposition dumps longer than 4 lines, every scene changes
   something, continuity with `ledger.md`, reading level, length budget.
7. **Revise**, update `ledger.md`, commit.

Nick is only asked at gates 2 and 3 (and optionally to playtest). He never has
to write prose.

### Presentation constraints the writers must respect

- Two portraits on screen at once (left/right), speaker highlighted (ADR-0012).
- Text box is ~3 lines × ~70 characters; a "line" of dialogue should fit in one
  or two boxes.
- Expressions available per portrait are listed in the character sheet; scripts
  may only use those.

## Consequences

- Story work is ordinary tickets (07xx), routed to `fable-5.1` / `high`.
- The bible and ledger keep many independent sessions consistent.
- If Nick later wants a dedicated tool, the bible/character files export cleanly
  to any of them.

## Alternatives considered

- **Paid fiction tool** — cost + manual copying, no quality advantage we can count on.
- **Procedural / runtime-generated story** — incoherent over a campaign, and
  Nick wants authored character arcs.
- **Local open-weight models** — weaker at long-range coherence, and nothing to
  gain here.
