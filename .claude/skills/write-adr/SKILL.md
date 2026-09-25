---
name: write-adr
description: Record a technical/architecture decision as an ADR in docs/adr/. Use when a ticket introduces a new pattern, file format, widely-used dependency, crate, or changes/supersedes an existing ADR.
---

# Write an ADR

1. Next number: highest `docs/adr/NNNN-*.md` + 1.
2. Copy `docs/adr/0000-template.md` to `docs/adr/NNNN-kebab-title.md`.
3. Fill in Context (facts and forces), Decision (concrete enough to follow
   without guessing), Consequences (good and bad), Alternatives considered
   (each with why it lost).
4. Add a row to the table in `docs/adr/README.md`.
5. **Superseding:** never rewrite an accepted ADR's decision. Write the new ADR,
   then change only the old one's `Status:` line to
   `Superseded by ADR-NNNN` and update both rows in the index.
6. Include the ADR in the same PR as the ticket that needed it.

Game-design decisions (anything a player would notice as "how the game works")
are **not** ADRs. They belong in `docs/design/` and must come from Nick via the
`ask-nick` skill.
