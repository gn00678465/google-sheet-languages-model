# AGENTS.md

## Project Overview

`google-sheet-languages-model` synchronizes i18n JSON Catalogs and Google
Sheets. The published Node package is `packages/gslm`; its public CLI is
`gslm`. Domain logic and all command orchestration live in Rust, with a thin
napi-rs binding for JavaScript.

## Commands

```bash
# Build the native package (debug build for local tests)
pnpm -C packages/gslm build:debug

# JavaScript binding and bin end-to-end tests
pnpm -C packages/gslm test

# Public .d.ts compiles for TypeScript consumers. Deliberately not part of
# `test`, which also runs in containers that have no node_modules.
pnpm -C packages/gslm typecheck

# Rust formatting, linting, and all crate tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Run the checked-in example after filling example/gslm.toml
pnpm example:cli:pull
pnpm example:cli:push
```

## Architecture

```
bin/gslm.js → index.js → napi-rs → gslm-cli
                                 ├─ gslm-config
                                 ├─ gslm-core
                                 └─ gslm-sheets
```

- `crates/gslm-core`: pure conversion between Catalog, Model, and Sheet Table.
- `crates/gslm-config`: discovery, TOML/JSONC parsing, environment/CLI
  precedence, and safe credential resolution.
- `crates/gslm-sheets`: Google Sheets REST client and credential token source.
- `crates/gslm-cli`: clap `pull`, `push`, `init`, and `schema`; owns local
  Catalog file I/O and all user-facing command behaviour.
- `packages/gslm`: napi adapters, JavaScript-safe credential-handle wrappers,
  and the `gslm` bin. `migrate` intentionally stays in JavaScript so it works
  without a native binding.

### Configuration

Projects use a non-executable `gslm.toml`, `gslm.jsonc`, or `gslm.json` with
`version = 1`. `path` is a `{locale}` template and the first Locale is the
source locale. CLI fields override environment values, which override the
config. Credential JSON must never be written into a config file; use
`credentials.file`, `credentials.env`, or Application Default Credentials.

`loadConfig()` returns a target with safe credential metadata. Its actual
credential source remains in Rust behind an opaque handle, so pass the original
Target to `SheetsClient.fromConfig`, `pull`, or `push` rather than serializing
and recreating it.

### CLI Rules

- `pull` refuses to replace non-empty local Catalogs with an empty Sheet unless
  `--force` is explicit.
- `push` refuses all-empty local Catalogs unless `--force`; it warns about
  Orphan keys and `--strict` turns those warnings (and format drift) into
  errors.
- `--dry-run` never writes files or Sheets. Progress uses stderr; Schema and
  dry-run summaries use stdout.
- Tests should use the public `gslm_cli::run(argv, options)` seam with a
  tempdir and mock Sheets server. JavaScript CLI coverage goes through
  `packages/gslm/bin/gslm.js`.

## Agent Skills

### Issue tracker

Issues and specs are local Markdown files under `docs/specs/<NNNN>-<slug>/`
(`spec.md` + `issues/NN-*.md`). Never create GitHub issues. See
`docs/agents/issue-tracker.md`.

### Triage labels

Recorded as a `status:` frontmatter field (`needs-triage`, `needs-info`,
`ready-for-agent`, `ready-for-human`, `wontfix`), not GitHub labels. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` at the repo root plus ADRs in `docs/adr/`.
Research notes live in `docs/research/`. See `docs/agents/domain.md`.
