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
| [0004](0004-crate-architecture.md) | Crate layering and deterministic core | Accepted; `app`'s one `unsafe` set out in ADR-0034 |
| [0005](0005-data-driven-content.md) | Data-driven content formats | Accepted |
| [0006](0006-input-actions-and-virtual-cursor.md) | Input actions, vim-style keymap, virtual cursor | Superseded by ADR-0015 |
| [0007](0007-testing-strategy.md) | Testing strategy | Accepted |
| [0008](0008-ci-quality-gates.md) | CI and quality gates (free tier only) | Superseded by ADR-0014 |
| [0009](0009-distribution.md) | Distribution: Windows first, web, itch, Steam | Accepted |
| [0010](0010-ticket-workflow-and-model-routing.md) | Ticket workflow and model routing | Accepted |
| [0011](0011-story-authoring-pipeline.md) | Story authoring pipeline with LLMs | Accepted |
| [0012](0012-visual-style.md) | Visual style: cells, tiles, color, portraits | Superseded by ADR-0018 |
| [0013](0013-licensing-and-third-party-policy.md) | Licensing and third-party policy | Accepted; art/audio licenses amended by ADR-0027; bought art allowed by ADR-0032 |
| [0014](0014-ci-gates-skip-docs-only-prs.md) | CI quality gates, skipping heavy jobs on docs-only PRs | Accepted |
| [0015](0015-input-actions-and-keymap-layouts.md) | Input actions, keymap layouts, virtual cursor | Accepted; per-key overrides superseded by ADR-0031; controller buttons added by ADR-0034 |
| [0016](0016-font-atlas-and-glyph-blitting.md) | Font atlas format and glyph blitting | Accepted |
| [0017](0017-screen-stack-and-frame-driver.md) | Screen stack, frame driver and test Harness | Accepted |
| [0018](0018-visual-style-v2.md) | Visual style v2 (after Nick's look sign-off) | Accepted; browsing cursor superseded by ADR-0024, acted-label rule by ADR-0029, portraits in part by ADR-0032 |
| [0019](0019-simulation-rng-and-serde-in-core.md) | In-crate PCG32 simulation RNG; serde derives in `core` | Accepted |
| [0020](0020-battle-state-serialisation.md) | Battle state saves its own data, not the content tables | Accepted |
| [0021](0021-skill-effects-as-data.md) | Skill effects are data, gathered into combat modifiers | Accepted |
| [0022](0022-combat-arts-and-attack-preview.md) | Combat Arts as weapon-input changes, and the attack preview | Accepted |
| [0023](0023-debug-tools-feature-for-pages-build.md) | A `debug-tools` cargo feature for the Pages build | Accepted |
| [0024](0024-cursor-as-pixel-overlays.md) | Draw the battle cursor as pixel overlays, with selectable styles | Accepted |
| [0025](0025-battle-event-playback-in-the-battle-screen.md) | Event playback runs inside the battle screen, as a mode | Accepted; playback keys changed in 0418 |
| [0026](0026-audio-cues-and-music-files.md) | Audio cues as data, played by `app`; music files beside the game | Accepted; licenses and credits amended by ADR-0027; native music loading amended by ADR-0028; track lengths and the music clock added by ADR-0037 |
| [0027](0027-audio-import-and-cc-by-3.md) | Importing third-party audio; CC BY 3.0 allowed | Accepted |
| [0028](0028-native-music-decoded-off-the-main-thread.md) | Native music is decoded on a worker thread, in its own quad-snd context | Accepted |
| [0029](0029-acted-units-keep-label-case.md) | Acted units are dimmed only; the label keeps its case | Accepted |
| [0030](0030-battle-dialogue-triggers.md) | Dialogue triggers are battle data; their scenes are events at their moment | Accepted |
| [0031](0031-player-key-bindings.md) | Player key bindings: slots, per-layout config, fixed Esc/Delete | Accepted |
| [0032](0032-bought-art.md) | Bought art is allowed; audio stays free | Accepted |
| [0033](0033-playtest-bots.md) | Playtest bots play through `core`'s commands, on reseeded copies, outside the game | Accepted |
| [0034](0034-controller-input.md) | Controller input: gilrs on native, a Gamepad API plugin on web | Accepted |
| [0035](0035-battle-and-chapter-files-and-the-flow-screen.md) | Battle and chapter files, a core `Campaign`, and one flow screen that hosts the others | Accepted |
| [0036](0036-help-text-follows-the-device-and-own-font-glyphs.md) | Help text follows the device pressed last; our own glyphs join the font from a second BDF | Accepted |
| [0037](0037-music-clock.md) | A music clock: `app` reports what is sounding, `ui` wraps it at the track's length | Accepted |

Template: [`0000-template.md`](0000-template.md).
