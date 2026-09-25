---
id: "0107"
title: "Release workflow: tag → Windows/Linux/macOS/web builds → GitHub Release"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0102", "0206"]
nick_input: none
completed:
---

# 0107 — Release workflow

## Context

[ADR-0009](../../docs/adr/0009-distribution.md): releases are cut by pushing a
`vX.Y.Z` tag. This produces downloadable builds Nick can play and that 0901
later pushes to itch.io.

## Nick input

None.

## Scope

**In:** `.github/workflows/release.yml`, third-party licence notice generation,
packaging layout.

**Out:** itch.io upload (0901), exe icon/metadata (0902), Steam (0903).

## Implementation steps

1. Trigger: `push: tags: ['v*.*.*']` and `workflow_dispatch` (dry run, no release).
2. First job `check-version`: tag without `v` must equal
   `workspace.package.version` in `Cargo.toml` (`cargo metadata` + `jq`); fail otherwise.
3. Build matrix (`--release --locked -p trpg-app`):
   - `windows-latest`, target `x86_64-pc-windows-msvc` → `tactical-rpg.exe`
   - `ubuntu-latest`, `x86_64-unknown-linux-gnu`
   - `macos-latest`, both `aarch64-apple-darwin` and `x86_64-apple-darwin`,
     merged with `lipo` into a universal binary
   - web: reuse the packaging command from 0206 to produce the web folder
4. Licence notices: use `cargo-about` (`about.toml` + a simple Markdown/HTML
   template) to generate `THIRD_PARTY_LICENSES.html`; also copy
   `assets/fonts/*LICENSE*` (from 0203) into each package.
5. Package names: `tactical-rpg-<version>-windows-x64.zip`,
   `…-linux-x64.tar.gz`, `…-macos-universal.zip`, `…-web.zip`. Each contains
   the binary (or web files), `README.txt` (how to run; macOS right-click →
   Open note), and licence files.
6. Final job: create a GitHub Release for the tag with all packages attached,
   auto-generated notes. Permissions: `contents: write` **only** on this job.
7. Pin actions by SHA. Test with `workflow_dispatch` on the branch (dry run),
   then document in Completion notes how Nick/sessions cut a release:
   bump version in `Cargo.toml` → merge → `git tag v0.1.0 && git push origin v0.1.0`.

## Acceptance criteria

- [ ] Dry run via `workflow_dispatch` builds all four packages as artifacts.
- [ ] Version mismatch makes `check-version` fail (test on the branch with a bogus tag in dry-run input or a unit check).
- [ ] Packages contain licence notices.
- [ ] Release instructions added to `README.md` (short "Releasing" section).

## Completion notes

