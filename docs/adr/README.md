# Architecture Decision Records

Technical decisions live here. **Game-design** decisions (stats, magic, story)
are Nick's and live in [`docs/design/`](../design/README.md) instead.

An ADR is never edited to change its meaning after it is `Accepted`. To change a
decision, write a new ADR that supersedes it and set the old one's status to
`Superseded by ADR-NNNN`. Use the `write-adr` skill.

| ADR | Title | Status |
| --- | ----- | ------ |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-language-rust.md) | Rust as the implementation language | Accepted |
| [0003](0003-rendering-glyph-grid-macroquad.md) | Glyph-grid rendering on macroquad | Accepted |
| [0004](0004-crate-architecture.md) | Crate layering and deterministic core | Accepted |
| [0005](0005-data-driven-content.md) | Data-driven content formats | Accepted |
| [0006](0006-input-actions-and-virtual-cursor.md) | Input actions, vim-style keymap, virtual cursor | Superseded by ADR-0015 |
| [0007](0007-testing-strategy.md) | Testing strategy | Accepted |
| [0008](0008-ci-quality-gates.md) | CI and quality gates (free tier only) | Superseded by ADR-0014 |
| [0009](0009-distribution.md) | Distribution: Windows first, web, itch, Steam | Accepted |
| [0010](0010-ticket-workflow-and-model-routing.md) | Ticket workflow and model routing | Accepted |
| [0011](0011-story-authoring-pipeline.md) | Story authoring pipeline with LLMs | Accepted |
| [0012](0012-visual-style.md) | Visual style: cells, tiles, color, portraits | Accepted (pending Nick's look sign-off, ticket 0011) |
| [0013](0013-licensing-and-third-party-policy.md) | Licensing and third-party policy | Accepted |
| [0014](0014-ci-gates-skip-docs-only-prs.md) | CI quality gates, skipping heavy jobs on docs-only PRs | Accepted |
| [0015](0015-input-actions-and-keymap-layouts.md) | Input actions, keymap layouts, virtual cursor | Accepted |

Template: [`0000-template.md`](0000-template.md).
