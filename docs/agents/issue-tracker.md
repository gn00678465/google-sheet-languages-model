# Issue tracker: Local Markdown

Issues and specs for this repo live as markdown files under `docs/specs/`. **Never** create GitHub issues, and never use `gh issue` / `gh pr` for tracking — the maintainer tracks all work locally in the repo.

## Conventions

- One feature per directory: `docs/specs/<NNNN>-<feature-slug>/`, numbered from `0001` (scan for the highest existing number and increment)
- The spec is `docs/specs/<NNNN>-<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `docs/specs/<NNNN>-<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01` — never a single combined tickets file
- Triage state is recorded as a `status:` field in YAML frontmatter at the top of each spec / issue file (see `triage-labels.md` for the role strings)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading
- Related ADRs are listed in an `adrs:` frontmatter field (e.g. `adrs: [0001, 0005]`); ADRs themselves live in `docs/adr/`
- Write prose in Traditional Chinese (繁體中文), matching `docs/adr/` and `docs/research/`; keep code, package names, and URLs as-is

## When a skill says "publish to the issue tracker"

Create a new file under `docs/specs/<NNNN>-<feature-slug>/` (creating the directory if needed). Do not open a GitHub issue.

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the spec/issue number directly (e.g. `0001` → `docs/specs/0001-*/spec.md`; `0001/02` → that spec's `issues/02-*.md`).

## Pull requests

PRs are not a request surface. Code review (`/code-review`) may still read a branch or PR diff via `git`, but findings are reported in the conversation, not as issue files, unless the user asks.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `docs/specs/<NNNN>-<effort>/map.md` — the Notes / Decisions-so-far / Fog body.
- **Child ticket**: `docs/specs/<NNNN>-<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `type:` frontmatter field records the ticket type (`research`/`prototype`/`grilling`/`task`); `status:` records `claimed`/`resolved`.
- **Blocking**: a `blocked_by: [NN, NN]` frontmatter field. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `docs/specs/<NNNN>-<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.
