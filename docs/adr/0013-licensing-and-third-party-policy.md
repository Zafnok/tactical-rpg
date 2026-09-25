# ADR-0013: Licensing and third-party policy

- **Status:** Accepted
- **Date:** 2026-09-25
- **Related tickets:** 0101, 0103, 0107, 0203, 0206, 0903

## Context

Nick intends to **sell the game** (itch.io, Steam) while keeping the repository
**public on GitHub's free tier**. He wants a restrictive license, with room for
education and modding. Every third-party thing we ship must be compatible with
that: nothing may force us to open-source the game, forbid commercial use, or
cost money to use.

*This ADR is an engineering policy, not legal advice. Before the Steam launch,
a short review of `LICENSE` by a lawyer is cheap insurance.*

## Decision

### 1. Our license

- The repository is under the custom **source-available, proprietary**
  [`LICENSE`](../../LICENSE) ("all rights reserved" plus narrow permissions):
  personal study and education (non-commercial), non-commercial mods that
  require a legitimate copy of the Game, and videos/streams/screenshots
  (monetisation allowed). No redistribution, no commercial use, no reuse of
  characters/story/art. Contributions are licensed to Nick.
- Workspace `Cargo.toml`: `license-file = "LICENSE"` and `publish = false` on
  every crate (ticket 0101).
- Changing `LICENSE` is Nick's decision only.

### 2. What third-party material may ship

"Shipped" means anything compiled into or distributed with the game: Rust
crates in the dependency tree of `trpg-app`, vendored JS, fonts, images,
audio, and platform SDKs.

**Allowed** (permissive, no payment, no copyleft):

| Kind | Licenses |
| ---- | -------- |
| Code | `MIT`, `MIT-0`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `0BSD`, `ISC`, `Zlib`, `BSL-1.0`, `Unicode-3.0`, `Unicode-DFS-2016`, `CC0-1.0`, `Unlicense` |
| Fonts | the code licenses above, plus `OFL-1.1` (SIL Open Font License) |
| Art / audio | `CC0-1.0`, `CC-BY-4.0` (credit required), or anything we create ourselves |

A dependency offered under several licenses (`MIT OR Apache-2.0`) is fine if
**any** option is allowed.

**Denied:**

- Copyleft: `GPL-*`, `LGPL-*` (Rust links statically, which makes LGPL
  compliance impractical), `AGPL-*`, `MPL-2.0`, `EPL-*`, `CDDL-*`, `EUPL-*`,
  `OSL-*`, `CC-BY-SA-*`.
- Non-commercial or restricted: `CC-BY-NC*`, `CC-BY-ND*`, PolyForm, BUSL,
  SSPL, Elastic, Commons Clause, "free for personal/non-commercial use".
- Anything requiring **payment, royalties or revenue share**, including
  "GPL or buy a commercial license" dual-licensed libraries.
- Unknown, missing or custom licenses (`LicenseRef-*`) — denied until reviewed.
  An exception requires a comment in `deny.toml` (or `THIRD_PARTY_ASSETS.md`)
  explaining why it is safe, and must be mentioned in the PR.

**Platform SDKs:** the Steamworks SDK is proprietary but free under Valve's
SDK agreement; it is allowed behind the `steam` feature (ticket 0903). Store
fees (Steam's revenue share, itch.io's optional cut) are sales fees, not
license fees, and are fine. We use no engine with royalties (macroquad is
MIT/Apache).

**Build and CI tools** (cargo-mutants, llvm-cov, SonarCloud, CodeQL, …) are not
shipped, so their licenses don't affect ours, but they must be free (ADR-0008).

### 3. Enforcement

- **Crates:** `cargo-deny` `[licenses]` uses exactly the allowed list above;
  our own crates are ignored via `[licenses.private] ignore = true`. Runs on every
  PR (ticket 0103).
- **Non-crate material** (fonts, vendored JS, art, audio, SDK files): every item
  is listed in [`THIRD_PARTY_ASSETS.md`](../../THIRD_PARTY_ASSETS.md) with name,
  source URL, license, and where it's used; its license text sits next to it in
  the repo. The `work-ticket` skill makes this a checklist item.
- **Attribution:** release packages include `LICENSE`, a generated
  `THIRD_PARTY_LICENSES.html` (`cargo-about`, ticket 0107) and the asset
  licenses. If we ever ship `CC-BY` art/audio, an in-game credits screen is
  added (future ticket).
- **Original content only:** maps, portraits, story text and names are
  created for this game. Other games are cited as *design references* in docs
  and tickets, never copied into shipped content.

## Consequences

- The public repo stays on the free tier. GitHub doesn't require an open-source
  license for public repos, and free Actions minutes, CodeQL, secret scanning and
  Dependabot depend on visibility, not license. SonarCloud's free plan covers
  public projects (ticket 0104 double-checks nothing there requires an OSI license).
- **Free code signing via the SignPath Foundation is not available:** it requires
  an OSI open-source license. Code signing, if wanted later, is a paid choice
  for Nick (e.g. Azure Trusted Signing).
- GitHub's Terms of Service let any GitHub user view and fork the repo *within
  GitHub*. Anyone can therefore read the code (and cheat or clone). That's an
  accepted trade-off; the license gives Nick legal grounds (e.g. DMCA takedowns)
  against copies published elsewhere.
- Some useful crates will be off-limits (GPL/LGPL/MPL). Pick alternatives or
  write the small piece ourselves.

## Alternatives considered

- **PolyForm Noncommercial** — standard and well-written, but lets anyone
  redistribute the whole game for free, which undercuts sales.
- **PolyForm Strict** — blocks redistribution, but also forbids mods, which Nick
  wants to allow.
- **Plain "all rights reserved"** — simplest, but silently forbids the education
  and modding uses Nick wants to permit.
- **Private repository** — keeps code hidden, but loses free CodeQL, secret
  scanning and SonarCloud, and caps free Actions minutes (macOS runners cost 10×).
- **An open-source license (MIT, GPL)** — contrary to Nick's goal.
