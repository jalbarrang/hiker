# AGENTS.md

## What this is

hiker is a Rust workspace: a tiny DSL compiler + CLI that turns architectural
intent (`.tent` files) into property tests for Rust/TS/Python. The compiler is a
classic four-stage pipeline (lexer → parser → checker → backend); each stage
reads the previous stage's output and nothing earlier. The authoritative list of
backend targets lives in `crates/hiker/src/backends/mod.rs` (`TARGETS`), not here.

Dependency direction: `crates/hiker` is the tool; `examples/temporal*` are
systems-under-test that consume generated output — they never feed back into the
crate.

## Stack

| Area | Tech |
|---|---|
| Compiler + CLI | Rust (edition 2021, MSRV pinned in `clippy.toml`) |
| Rust test bridge | `proptest` |
| TS test bridge | `fast-check` + `vitest` |
| Python test bridge | `Hypothesis` + `pytest` |
| Commit linting | cocogitto (`cog`), config in `cog.toml` |
| CI / release | GitHub Actions (`.github/workflows/`) |

## Commands

| Task | Command |
|---|---|
| Check intent | `cargo run -p hiker -- check .hiker/temporal.tent` |
| Generate tests | `cargo run -p hiker -- gen .hiker/temporal.tent --target rust -o .hiker-cache/rust/generated.rs` |
| Unit tests | `cargo test -p hiker` |
| Lint (must be clean) | `cargo clippy --all-targets -- -D warnings` (gen the rust bridge first) |
| Format check | `cargo fmt --check` |
| Resolve a release version | `scripts/release-version.sh --channel stable --bump minor` |
| Cut a release | push to `stable` / `beta`, or run the `release` workflow |

## Rules

- **No `unwrap()` in non-test code.** Banned via `disallowed-methods` in
  `clippy.toml`; tests opt out with the crate-level `#![cfg_attr(test, allow(...))]`
  in `lib.rs`. Use `?` / `.expect("reason")`.
- **Conventional Commits are enforced.** `commit-msg` hook (`.githooks/`,
  activated by `core.hooksPath`) runs `cog verify`. A plain YAML scalar with
  `": "` in it breaks `cog.toml`-adjacent parsing — unrelated, but see Gotchas.
- **`.tent` is the source of truth; generated tests are disposable.** Everything
  `gen` emits lands in `.hiker-cache/` (gitignored). Never commit generated
  tests. Run `gen` **before** any test command — a fresh checkout has none.
- **The checker owns "intent compiles".** `crates/hiker/src/checker.rs` is the
  only place that decides coherence. A relation with **no law** is a warning
  (`warnings()`); an **empty law body** is an error (`check_law`). Keep new
  coherence rules here, with a collect-all-errors style (don't stop at first).
- **Backends must agree on lowering.** Each backend in `crates/hiker/src/backends/`
  re-lowers clauses; implication `a => b` MUST lower to `!a || b` (`not a or b`
  in Python). Diverging lowerings are a bug — they have per-backend tests.
- **Spec↔code correspondence is by name convention** (documented in each
  backend). The generated tests call functions/structs that must match the
  `.tent` names; a mismatch fails at the target compiler, not in `check`.
- **Adding a language = adding a `Backend`.** Implement the trait in
  `backends/`, register it in `for_target` + `TARGETS` (`backends/mod.rs`). That
  file owns the target list.
- **Release channel = git branch.** `stable` → semver, `beta` → prerelease. The
  version is computed by `scripts/release-version.sh` and **stamped into
  `crates/hiker/Cargo.toml` by CI at build time** — do not hand-bump it.
- **Renames keep `.tent`.** The extension is independent of the binary name; the
  pun (you *pitch a tent*) is deliberate. Don't rename it.

## Key paths

```
crates/hiker/src/
  lexer.rs parser.rs ast.rs   front end: text → tokens → AST
  checker.rs                  "intent compiles" — coherence + lints (warnings())
  backends/mod.rs             Backend trait, for_target(), TARGETS (owner)
  backends/{rust,typescript,python}.rs   codegen per target
  main.rs                     CLI: check / gen / --version
.hiker/temporal.tent          canonical worked-example spec
examples/temporal*/           systems-under-test (rust / ts / python)
skills/hiker/                 agent skill (SKILL.md + reference/)
scripts/release-version.sh    channel + version resolver
install                       channel-aware installer (repo slug: jalbarrang/hiker)
.github/workflows/            ci.yml (fmt→clippy→test), release.yml (channels)
```

## Gotchas

- **Rust example `include!` path resolves to the repo root, not the example
  dir.** `examples/temporal/tests/hiker_generated.rs` includes
  `../../../.hiker-cache/rust/generated.rs`. Always `gen -o .hiker-cache/rust/generated.rs`
  from the repo root before `cargo test -p temporal`.
- **`npm test` needs `gen` first.** The TS example's `pretest` runs it; a bare
  `vitest` against a fresh checkout fails with "file does not exist".
- **SKILL.md frontmatter: a `description` containing `": "` must be a block
  scalar (`>-`).** A plain scalar errors with "mapping values are not allowed
  here". Folded block scalars allow colons and backticks.
- **`git mv` stages the rename, but later content edits are unstaged.** After a
  bulk rename + `sed` pass, `git add` the content changes too — otherwise the
  commit captures renames with old content ("rename 100%" in the summary).
- **Pushing to `stable`/`beta` fires `release.yml`.** Don't push those branches
  unless you intend to cut a release.
