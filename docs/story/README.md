# Story

How the story gets written:
[ADR-0011](../adr/0011-story-authoring-pipeline.md) and the `story-writing`
skill. Short version: Nick gives beats → Claude builds a bible and a cast with
real arcs → outline → per-chapter beat sheets → scripts in `assets/dialogue/`
→ a separate critique pass. Nick approves at two gates: cast summary and outline.

| File | What | Created by | Status |
| ---- | ---- | ---------- | ------ |
| `beats.md` | Nick's beats, verbatim. **Canon.** | 0007 | ⏳ |
| `bible.md` | World, factions, themes, tone, magic rules, glossary | 0701 | ⏳ |
| `characters/<id>.md` | One sheet per character: want/need/flaw/arc, voice, portrait brief | 0701 | ⏳ |
| `outline.md` | Acts and chapters; which arcs each chapter advances | 0701 | ⏳ |
| `chapters/chNN.md` | Scene-by-scene beat sheet per chapter | 0701 (ch01), later tickets | ⏳ |
| `ledger.md` | Continuity: what each character knows/did as of each chapter | 0701, updated by each script ticket | ⏳ |

Scripts themselves live in `assets/dialogue/*.dlg` (format: ticket 0702).
