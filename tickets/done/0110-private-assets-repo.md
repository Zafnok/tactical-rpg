---
id: "0110"
title: Keep bought assets in a private repo and bundle them at build time
type: infra
milestone: M0 Foundation
model: opus-5.5
effort: high
status: done
blocked_by: ["0021"]
nick_input: setup
completed: 2026-10-02
---

# 0110 — Private assets repo, bundled at build time

## Context

Nick bought portrait and battle art (0021, 0413): Mega Tiles' Tiny Tales
packs (`docs/design/look-and-feel.md`, ADR-0032), whose licence allows them
in a sold game but forbids redistributing the files. This repo is public (ADR-0013),
so the bought files can't be committed here. They go in a **private** GitHub
repo and are pulled in only when a build is made. The shipped game (exe,
Pages web build, Steam) contains them, which the licence allows. The public
repo never has the raw files.

Assets are embedded with `include_dir!` over `assets/`
(`crates/content/src/bundle.rs`).

**Rewritten 2026-10-02, when the ticket was worked**, because it no longer
matched what had happened: the packs were already bought, unzipped and
sorted on Nick's machine, and the importer tickets (0711, 0413, 0436, 0437)
read the sorted folders, not the zips. The repository names were stale too
(`tactical-rpg` is now `visions-of-shuyi`). What was decided is ADR-0040.

## Nick input

**Setup** (Claude doesn't make purchases, create repos or change access
settings for him):

1. ~~Buy the packs.~~ **Done 2026-10-02:** Nick bought Mega Tiles' whole
   "2025 Bundle Sale" (37 products, $99.99). The purchase is recorded in
   `THIRD_PARTY_ASSETS.md` and `look-and-feel.md`.
2. ~~Sort the files.~~ **Done 2026-10-02** in `D:\tactical-rpg\Tiny Tales
   Bundle Assets\`, and moved by this ticket into the private repository's
   layout at `D:\tactical-rpg\assets-private\` (see *Completion notes*).
3. Create the **private** GitHub repo `Zafnok/visions-of-shuyi-assets` from
   that folder, and add a read-only deploy key as the Actions secret
   `PRIVATE_ASSETS_KEY` of `Zafnok/visions-of-shuyi`. **Moved to ticket
   0116**, which has the two commands: Nick chose to do the upload himself.

## Scope

**In:**
- `assets-private/` as a gitignored checkout location.
- Content loading that lays `assets-private/game/` over `assets/` in builds
  that ask for it, and otherwise builds and runs with the public
  placeholders (so forks, Dependabot PRs and a fresh clone still build and
  pass every test).
- CI: check out the private repo with the secret in the jobs that build
  shipped artefacts (release packages, Pages web build). Test, clippy and
  mutants jobs keep running without it, on placeholders.
- A local dev command that clones or updates it (`cargo xtask
  private-assets`), documented in `CLAUDE.md` § Environment.
- The `THIRD_PARTY_ASSETS.md` convention for private assets, as ADR-0032
  sets it (item, seller, URL, quoted licence, date bought, AI-assisted or
  not, marked private; the licence text kept in the private repo).
- What the private repository holds (the sorted folders, the zips, or
  both), and how the sorted folder on Nick's machine becomes
  `assets-private/` without leaving a second copy that drifts.

**Out (do not do):** importing or drawing portraits (0711, 0706);
encrypting assets inside the binary (the licence doesn't require it).

## Implementation steps

1. Decide how the embed picks up the optional directory and how tests stay
   deterministic (tests always use the public placeholders unless a test
   opts in). Record it in an ADR (`write-adr`).
2. Implement it, with `cargo:rerun-if-changed` on both directories.
3. `.gitignore` `assets-private/`.
4. `cargo xtask private-assets`: clone or update the private repository in
   `assets-private/`. Clear error when the user has no access.
5. Workflows: check out the private repo, using
   `secrets.PRIVATE_ASSETS_KEY`, in the release and Pages build jobs only.
   The step is skipped when the secret isn't available (forks), with a
   warning.
6. Document it in `CLAUDE.md`, the `run-gates` skill and `THIRD_PARTY_ASSETS.md`.

## Acceptance criteria

- [x] A clean clone without `assets-private/` builds and passes every gate.
- [x] With a file in `assets-private/game/portraits/`, a debug build made
      with `--features private-assets` shows it in the portrait viewer
      (F2 → Portraits). *Checked by drawing the viewer in a test build with
      the feature, not by opening the window: see Completion notes.*
- [x] `git status` never shows files under `assets-private/`.
- [ ] The Pages build on `main` includes the private assets. A PR job's log
      shows the private checkout skipped or used, as designed. **Not met
      here: it needs the upload and the key, which are Nick's (0116). 0116
      checks it.**
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the merge rule (a private file overrides a public file at the same
  path; a private-only file is added).
- Integration: the xtask command's argument and error handling.

## Completion notes

**What was built** (ADR-0040 has the reasons):

- **Private repository layout**, one repo `Zafnok/visions-of-shuyi-assets`:
  `game/` (what the game embeds), `library/tiny-tales/` (the bundle as Nick
  and Claude sorted it: `characters/`, `tilesets/`, `battle-backgrounds/`,
  `ui/`, `licences/`, `spike-renders/`, `INDEX.md`, `index.html`),
  `tools/tiny-tales/` (the two character generators) and
  `originals/tiny-tales/` (the 37 zips, untouched). So: the sorted folders
  **and** the zips. The second extracted copy (`_unzipped/`) stays on
  Nick's machine and is ignored there.
- **On Nick's machine** the folder `D:\tactical-rpg\Tiny Tales Bundle
  Assets\` was moved (nothing deleted, 50,850 files before and after) to
  `D:\tactical-rpg\assets-private\` in that layout and committed there as
  a git repository: one local commit, `7f960cb6…`, 37,301 files. It has no
  remote yet (0116). `/assets-private/` was added to the main checkout's
  `.git/info/exclude`, so the public repository can't pick it up before
  this PR's `.gitignore` reaches `main`.
- **`private-assets` cargo feature** (`trpg-content`, forwarded by
  `trpg-app`): `bundle.rs` embeds `assets-private/game/` as a second layer
  over `assets/`. A private file replaces the public one at the same path,
  a private-only file is added, listings merge. Without the feature the
  bundle is `assets/` alone whatever is on disk, so every gate reads the
  placeholders. With the feature and no folder, the build stops with a
  message naming `cargo xtask private-assets`.
- **`assets-private.rev`** pins the private repository's commit this code
  is built with.
- **`cargo xtask private-assets`**: clones (only `game/`, sparse and
  partial) or fetches, and puts the checkout at the pinned commit.
  `--library` also brings `library/`. `--pin` writes the checkout's commit
  into `assets-private.rev` once it is committed and pushed.
  `cargo xtask web --private-assets` builds the web shell with the feature.
- **CI**: `.github/actions/private-assets` checks out `game/` at the pin
  when the run has `PRIVATE_ASSETS_KEY`. Pages uses it and first runs the
  one opt-in test (`crates/content/tests/private_assets.rs`: the content
  still loads with the bought art over it). All four release packages use
  it. Dependabot now also watches `.github/actions/*`.
- Docs: ADR-0040, `CLAUDE.md`, the `run-gates` and `ascii-art` skills,
  `THIRD_PARTY_ASSETS.md` (how a bought pack's row is filled in),
  `look-and-feel.md` (where the files are), `docs/ROADMAP.md`.

**Deviations from the ticket as first written:**

- No README inside `assets-private/` in this repository: the folder is the
  private repository's working copy, and a file tracked by both would break
  the clone. ADR-0040 and `CLAUDE.md` say what the folder is.
- The embed is a cargo feature, not "merge when the folder exists": with
  the folder present, tests on Nick's machine would otherwise read other
  files than CI's.
- A pin file was added, so pushing to the private repository can't change
  or break main's Pages build until a PR moves the pin.
- A real release from `Zafnok/visions-of-shuyi` **fails** without the key
  instead of warning, so a sold build never ships placeholders by accident.
  Pages, dry runs and forks warn and use placeholders, as the ticket said.
- Open tickets that named the old folder or `assets-private/<kind>/` were
  updated to `assets-private/library/tiny-tales/…` and
  `assets-private/game/<kind>/` (0232, 0413, 0436, 0437, 0438, 0706, 0711,
  1006).

**Checked by hand:**

- `cargo xtask private-assets` against GitHub before the repository exists:
  exits 1 with "can't read … Repository not found" and the note that the
  game builds without the bought art.
- The same command in this worktree against the local repository: fetch,
  `--library` (7,943 files: no tools, no zips) and `--pin`. This found a
  bug, fixed with a test: a clone that was never checked out counted as
  "already at the pinned commit" and stayed empty.
- With a private-only portrait and a private copy of `test_knight` in
  `assets-private/game/portraits/`, a test build with the feature drew the
  portrait viewer with the private portrait in its list, and
  `display_path` named both files under `assets-private/game/`. Without
  the feature only the public portraits were there. The window itself
  wasn't opened (there is no headless render until 0232).

**Not verified:** cloning from GitHub with the deploy key, and the Pages
and release jobs fetching the art. Both need the upload and the key: 0116.

**Follow-up ticket:** **0116** — Nick uploads the repository and adds the
key (two commands), then a session checks the builds. It was added to the
`blocked_by` of 0711, 0436 and 0804, and to the critical path.

**Gameplay rules decided:** none. This ticket changes nothing a player
sees.
