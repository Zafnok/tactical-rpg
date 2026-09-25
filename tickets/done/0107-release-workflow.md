---
id: "0107"
title: "Release workflow: tag → Windows/Linux/macOS/web builds → GitHub Release"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0102", "0206"]
nick_input: none
completed: 2026-09-25
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
   `assets/fonts/*LICENSE*` (from 0203) and every license file listed in
   `THIRD_PARTY_ASSETS.md` into each package, plus our own `LICENSE`
   ([ADR-0013](../../docs/adr/0013-licensing-and-third-party-policy.md)).
   `cargo-about` must use the same allowed-license list as `deny.toml`.
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
- [x] Version mismatch makes `check-version` fail (test on the branch with a bogus tag in dry-run input or a unit check).
- [x] Packages contain licence notices.
- [x] Release instructions added to `README.md` (short "Releasing" section).

## Completion notes

- Added `.github/workflows/release.yml`: `check-version` (tag vs.
  `workspace.package.version` via `cargo metadata`/`jq`) → `licenses`
  (`cargo about generate -m crates/app/Cargo.toml` with `about.toml`/`about.hbs`,
  same allow list as `deny.toml`) → four parallel build+package jobs
  (`build-windows`, `build-linux`, `build-macos` with `lipo`,
  `build-web` reusing `cargo xtask web --release`) → a final `release` job
  (only job with `contents: write`) that attaches all four packages to a
  GitHub Release with auto-generated notes, skipped entirely when
  `dry_run` is true. All actions pinned by commit SHA (verified against
  each project's real tag commit with `git ls-remote --tags`, not trusted
  from search results alone).
- **Deviation / known gap:** GitHub does not let a `workflow_dispatch`
  workflow be invoked via the UI or API until the workflow file exists on
  the repository's *default* branch — this applies even when targeting a
  different `ref`. Since `release.yml` is new, it could not actually be
  dispatched from this branch to verify criterion 1; the API calls returned
  404 ("workflow not found"). Verified as much as possible without that:
  - `actionlint` (v1.7.7) passes clean on `release.yml` and every existing
    workflow — no schema, expression or SHA-pin syntax errors.
  - The `check-version` bash logic was extracted and run standalone for all
    four cases (push+match, push+mismatch, dispatch+default tag,
    dispatch+bogus `fake_tag`); the two mismatch cases fail with the
    expected `::error::`, the two matching cases succeed — this is
    criterion 2, fully verified.
  - The `licenses` step (`cargo about generate -m crates/app/Cargo.toml
    about.hbs`) was run for real locally: produces a clean
    `THIRD_PARTY_LICENSES.html` with entries for `trpg-app`'s dependency
    tree only (our own unpublished, license-less workspace crates warn but
    don't appear), matching `deny.toml`'s allow list.
  - The `build-linux` and `build-web` jobs' exact build + package + archive
    shell was run locally end-to-end (`cargo build --release --locked -p
    trpg-app --target x86_64-unknown-linux-gnu`, `cargo xtask web
    --release`); both produced correct `tactical-rpg-0.1.0-linux-x64.tar.gz`
    / `tactical-rpg-0.1.0-web.zip` containing the binary/web files,
    `LICENSE`, `THIRD_PARTY_LICENSES.html`, `Terminus-LICENSE.txt` (and, for
    web, `mq_js_bundle-LICENSE-MIT.txt`) and `README.txt` — this is
    criterion 3, verified for two of the four packages.
  - `build-windows` (MSVC target) and `build-macos` (`lipo` universal
    binary) use the identical package/README/zip pattern but could not be
    built or packaged in this Linux sandbox (no MSVC target, no macOS
    toolchain/`lipo`); they were reviewed by hand against the verified
    Linux/web jobs and `actionlint`, not executed.
  - Left criterion 1 unchecked rather than claim a dry run happened: **the
    first real `workflow_dispatch` run of this workflow must happen once on
    `main` after this PR merges** (Actions → Release → Run workflow, no
    inputs needed) to confirm the four packages build and attach as
    artifacts, especially the two legs (Windows/macOS) that couldn't be
    exercised locally. If that run fails, the fix is a follow-up commit to
    `main`, not a reason to reopen this ticket.
- Local gates run: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`,
  `cargo build -p trpg-app --target wasm32-unknown-unknown --locked`,
  `cargo deny check`, `cargo machete`, `typos`, `cargo xtask ticket-lint` —
  all pass. No `core`/`content`/`ui` code changed, so the mutation-testing
  gate has nothing to mutate for this diff.
