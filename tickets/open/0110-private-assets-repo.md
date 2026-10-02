---
id: "0110"
title: Keep bought assets in a private repo and bundle them at build time
type: infra
milestone: M0 Foundation
model: opus-5.5
effort: high
status: todo
blocked_by: ["0021"]
nick_input: setup
completed:
---

# 0110 — Private assets repo, bundled at build time

## Context

Nick is buying portrait and battle art (0021, 0413): Mega Tiles' Tiny Tales
packs (`docs/design/look-and-feel.md`, ADR-0032), whose licence allows them
in a sold game but forbids redistributing the files. This repo is public (ADR-0013),
so the bought files can't be committed here. They go in a **private** GitHub
repo and are pulled in only when a build is made. The shipped game (exe,
Pages web build, Steam) contains them, which the licence allows. The public
repo never has the raw files.

Assets are embedded with `include_dir!` over `assets/`
(`crates/content/src/bundle.rs`).

## Nick input

**Setup** (Claude can't make purchases or create repos for him):

1. ~~Buy the packs.~~ **Done 2026-10-02:** Nick bought Mega Tiles' whole
   "2025 Bundle Sale" (37 products, $99.99). The 37 zips are untouched in
   `D:\tactical-rpg\Tiny Tales Bundle Assets\_original-zips\` on his
   machine; the same folder holds them unzipped and sorted by kind
   (`characters/`, `tilesets/`, `battle-backgrounds/`, `ui/`, `tools/`,
   `licences/`, with `README.md`, `INDEX.md` and `index.html`). That folder
   is ignored by git through `.git/info/exclude`, which only protects this
   one machine; this ticket's `.gitignore` entry is the real guard. The
   purchase is recorded in `THIRD_PARTY_ASSETS.md` and
   `look-and-feel.md`.
2. Create a **private** GitHub repo `Zafnok/tactical-rpg-assets`. Upload the
   zips as they are (the importers in 0711, 0413, 0436 and 0437 read them
   or their unzipped folders), plus the licence texts: every pack's
   `License.txt` and the generator's `EULA.txt` are already collected in
   the bought folder's `licences/`. The implementing session decides
   whether the private repo holds the zips, the sorted folders or both,
   and says how the sorted folder on Nick's machine becomes
   `assets-private/` (it must not be left as a second copy that drifts).
3. Create a deploy key (read-only) or a fine-grained token with read access
   to that repo only, and add it to `Zafnok/tactical-rpg` as the Actions
   secret `PRIVATE_ASSETS_KEY`. The implementing session gives him the exact
   clicks.

## Scope

**In:**
- `assets-private/` as a gitignored checkout location. The public repo keeps
  a README there saying what goes in it.
- Content loading that merges `assets-private/` over `assets/` when present,
  and otherwise builds and runs with the public placeholders (so forks,
  Dependabot PRs and a fresh clone still build and pass every test).
- CI: check out the private repo with the secret in the jobs that build
  shipped artefacts (release exe, Pages web build). Test, clippy and mutants
  jobs keep running without it, on placeholders.
- A local dev command that clones or updates it (`cargo xtask
  private-assets`), documented in `CLAUDE.md` § Environment.
- The `THIRD_PARTY_ASSETS.md` convention for private assets, as ADR-0032
  sets it (item, seller, URL, quoted licence, date bought, AI-assisted or
  not, marked private; the licence text kept in the private repo).

**Out (do not do):** importing or drawing portraits (0711, 0706);
encrypting assets inside the binary (the licence doesn't require it).

## Implementation steps

1. Decide how the embed picks up the optional directory. Options: a second
   `include_dir!` behind a `cfg` set by `crates/content/build.rs` when
   `assets-private/` exists, or a `build.rs` that stages
   `assets/` + `assets-private/` into `OUT_DIR`. Record the choice in an ADR
   (`write-adr`), including how tests stay deterministic (tests always use the
   public placeholders unless a test opts in).
2. Implement it, with `cargo:rerun-if-changed` on both directories.
3. `.gitignore` `assets-private/*` except its `README.md`.
4. `cargo xtask private-assets`: `git clone` or `git pull` of
   `Zafnok/tactical-rpg-assets` into `assets-private/`. Clear error when the
   user has no access.
5. Workflows: add an `actions/checkout` step for the private repo, using
   `secrets.PRIVATE_ASSETS_KEY`, to the release and Pages build jobs only.
   The step is skipped when the secret isn't available (forks), with a
   warning.
6. Document it in `CLAUDE.md`, the `run-gates` skill and `THIRD_PARTY_ASSETS.md`.

## Acceptance criteria

- [ ] A clean clone without `assets-private/` builds and passes every gate.
- [ ] With a file in `assets-private/portraits/`, a debug build shows it in
      the portrait viewer (F2 → Portraits).
- [ ] `git status` never shows files under `assets-private/` except its README.
- [ ] The Pages build on `main` includes the private assets. A PR job's log
      shows the private checkout skipped or used, as designed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the merge rule (a private file overrides a public file at the same
  path; a private-only file is added).
- Integration: the xtask command's argument and error handling.

## Completion notes

