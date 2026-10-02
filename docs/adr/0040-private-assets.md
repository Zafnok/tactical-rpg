# ADR-0040: Bought art lives in a private repository, pinned by commit and embedded by a feature

- **Status:** Accepted
- **Date:** 2026-10-02
- **Related tickets:** 0110, 0232, 0413, 0436, 0437, 0438, 0706, 0711, 1006
- **Extends:** ADR-0005 (the asset bundle gains a second directory), ADR-0032
  (§4 and §5: where the bought files and their licence texts are kept)

## Context

ADR-0032 allows bought art whose licence forbids passing the files on, and
says such files never enter this public repository. Nick bought Mega Tiles'
whole Tiny Tales bundle on 2026-10-02 (`THIRD_PARTY_ASSETS.md`). The facts
this decision has to fit:

- The bundle is about 500 MB as unzipped and sorted on Nick's machine:
  `characters/`, `tilesets/`, `battle-backgrounds/`, `ui/`, licence texts,
  two character-generator programs (173 MB, 29,000 files), the 37 original
  zips (147 MB) and a second extracted copy of the zips. The importers of
  tickets 0711, 0413, 0436 and 0437 read the **sorted folders by name**, not
  the zips.
- The game needs a small part of it, cut and renamed by those importers.
  Everything under `assets/` is embedded in the binary with `include_dir!`
  (ADR-0005), so the game's part must be a folder of its own: embedding the
  bundle would add hundreds of megabytes to the game.
- Every ticket is worked in its own git worktree (over 60 exist). A folder
  beside the main checkout is outside every worktree.
- Tests are the review (ADR-0007). A test that passes or fails depending on
  whether bought files happen to be on the machine is not a gate.
- No gate reads the bought files, so nothing but the build itself would
  notice code and bought files that don't fit each other.
- Forks, Dependabot's runs and a fresh clone can't read a private
  repository, and must still build and pass every gate.

## Decision

### 1. One private repository, four folders

`Zafnok/visions-of-shuyi-assets` (private):

| Folder | Holds | Read by |
| ------ | ----- | ------- |
| `game/` | The files the game is built with, at the path they have under `assets/` (`game/portraits/x.png` ↔ `assets/portraits/x.png`) | Builds with the `private-assets` feature |
| `library/<seller>/` | The bought packs as sorted. `library/tiny-tales/` is the sorted bundle, with its `licences/` (ADR-0032 §5), `INDEX.md` and `index.html` | The importers, which write into `game/`; people choosing art |
| `tools/<seller>/` | Programs that came with the packs | Nick, by hand |
| `originals/<seller>/` | The downloads as bought, untouched | Nobody; an archive |

The repository holds the sorted folders **and** the original zips. The
extracted copy of the zips (`originals/*/_unzipped/`) is ignored there: it
can be made again from the zips.

**`game/` is embedded whole**, so it holds only files the game reads.

### 2. It is checked out into `assets-private/`, which this repository ignores

`/assets-private/` is in `.gitignore`. Nothing inside it is tracked here, not
even a README: the folder is another repository's working copy, and this
document and `CLAUDE.md` say what it is.

`cargo xtask private-assets` makes or updates the checkout:

- No flag: clones it if it isn't there, fetches, and puts it at the pinned
  commit (§3). A clone made this way is sparse and partial: it downloads only
  `game/` and the files in the repository's root.
- `--library`: also brings `library/` (what an importer ticket needs).
  `tools/` and `originals/` are never brought by the command.
- A full clone made by hand (Nick's main checkout, where the bundle was first
  committed) is left whole, and left on its branch when it is already at the
  pinned commit.
- Without access it fails with a message that says the game builds without
  the bought art. It never asks for a password on the terminal.

On Nick's machine the sorted bundle **is** `D:\tactical-rpg\assets-private\`,
a full clone. There is no second copy to drift.

### 3. `assets-private.rev` pins the commit

`assets-private.rev` in this repository's root holds the 40-character id of
the private repository's commit that this code is built with. The builds
that ship check out exactly that commit.

To change the bought files: in `assets-private/`, switch to `main`, pull,
change, commit and push; then run `cargo xtask private-assets --pin`, which
writes the checkout's commit into `assets-private.rev` after checking that
there are no uncommitted changes and that the commit is on the private
repository's `main`. Commit `assets-private.rev` in the same pull request as
the code that needs the new files.

If two open pull requests both change the pin, the second to merge has a
conflict in that one line. Resolve it by pulling the private repository's
`main` (which has both changes) and running `--pin` again.

### 4. The `private-assets` feature embeds `game/` over `assets/`

`trpg-content` has a cargo feature `private-assets` (forwarded by
`trpg-app`'s feature of the same name). With it, `bundle.rs` embeds
`assets-private/game/` as a second directory and reads through both:

- A file in `game/` **replaces** the file at the same path in `assets/`.
- A file only in `game/` is **added**; directory listings merge both, sorted,
  each path once.
- `bundle::display_path` names a private file `assets-private/game/...`.

Without the feature the bundle is `assets/` alone, whatever is on disk. If
the feature is on and `assets-private/game/` is missing, the build fails
with a message naming `cargo xtask private-assets`.

**Every placeholder stays.** Each bought file the game uses must have a
public stand-in in `assets/` (or the game must do without it), so a build
without the feature runs.

### 5. Gates never turn the feature on

`cargo test`, clippy, the docs build, coverage and mutation testing run
without `private-assets`, on Nick's machine and in CI, so they read the
public placeholders everywhere. A test in `bundle.rs` asserts that a test
build has the one layer.

One test opts in: `crates/content/tests/private_assets.rs` exists only with
the feature and checks that the whole content still loads with `game/` laid
over `assets/`. Run it after changing `game/`:

```bash
cargo test -p trpg-content --features private-assets --test private_assets
```

To look at the bought art in a local build:

```bash
cargo run -p trpg-app --features private-assets
```

### 6. Builds that ship fetch it with a read-only deploy key

The Actions secret `PRIVATE_ASSETS_KEY` is the private half of a read-only
deploy key of the private repository. `.github/actions/private-assets`
checks out `game/` at the pinned commit when the run has the key, and
outputs the flags to build with:

- **Pages** (`pages.yml`): uses it, runs §5's test, then builds with
  `cargo xtask web --private-assets`. Without the key it warns and builds
  with placeholders.
- **Release** (`release.yml`): every package uses it. A real release from
  `Zafnok/visions-of-shuyi` **fails** without the key, so a sold build never
  ships placeholders by accident. A dry run or a fork warns and uses
  placeholders.
- **Everything else** (`ci.yml`, mutants, security): never has the key and
  never fetches.

Only workflows that run on `main`, on a tag or by hand use the secret; a pull
request from a fork never receives it.

## Consequences

- A clone, a fork and every gate work with no access to the private
  repository.
- What a release contained can be rebuilt: the tag's `assets-private.rev`
  names the bought files.
- Pushing to the private repository changes no build until a pull request
  moves the pin, so main's Pages build can't break from files its code
  doesn't expect.
- An importer ticket has one extra step (push, `--pin`), and two such pull
  requests open at once conflict on one line.
- The bought art is checked by one opt-in test and by looking at the build.
  A ticket that adds bought files should add what it can to that test.
- Paths in tickets become relative and work in any worktree:
  `assets-private/library/tiny-tales/characters/...` after
  `cargo xtask private-assets --library`.
- The private repository is about 370 MB. A sparse checkout of `game/` stays
  small however much is bought.
- `game/`'s files are in the shipped binary, which the licence allows. They
  are not encrypted (the licence doesn't ask for it).

## Alternatives considered

- **Upload only the zips (the ticket's first plan)**: the importers read the
  sorted folders, so every machine would have to repeat the sorting by hand.
- **Embed `assets-private/` whenever the folder exists (a `cfg` from
  `build.rs`)**: tests on a machine with the folder would read other files
  than CI's, and `cfg(test)` can't fix that for the crates that depend on
  `trpg-content`.
- **`build.rs` copies `assets/` and `assets-private/` into `OUT_DIR`, one
  `include_dir!`**: the merge rule would live in a build script, where tests
  and mutation testing don't reach, and every build would copy the assets.
- **A git submodule**: it pins a commit too, but every worktree would carry
  its own copy, tools that walk submodules (Dependabot, a plain recursive
  clone) fail on a private one, and CI would need the key for the main
  checkout.
- **No pin, always the private repository's latest commit**: a push there
  could break main's next Pages build with nothing in this repository
  changing, and no gate would see it.
- **Two private repositories (game files, bought packs)**: two keys and two
  things for Nick to set up; a sparse checkout gives the same small download.
- **Git LFS or release attachments for the zips**: the largest file is
  55 MB, under GitHub's 100 MB limit, and nobody clones the zips but Nick.
- **A personal access token instead of a deploy key**: it expires, and it
  can be scoped wider than one repository by mistake.
