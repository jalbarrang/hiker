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
| Verify conformance | `cargo run -p hiker -- verify examples/architecture.tent --facts examples/architecture.facts.json` |
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
  in Python). The runtime interpreter (`eval.rs`) is bound by the same rule —
  diverging lowerings are a bug, and there are tests pinning it in each.
- **`verify` is conformance over EXTERNAL facts; hiker never extracts.** The
  extractors (import graph → `facts.json`, grep → forbidden tuples) live in the
  consuming repo, not the crate. Owner of the fact format + population modes:
  `crates/hiker/src/verify.rs` + `skills/hiker/reference/verify.md`.
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
  eval.rs                     runtime law interpreter (verify's truth oracle)
  facts.rs                    JSON fact model + loader + type-check vs spec
  verify.rs                   conformance: eval laws over facts; forbidden negatives
  backends/mod.rs             Backend trait, for_target(), TARGETS (owner)
  backends/{rust,typescript,python}.rs   codegen per target
  main.rs                     CLI: check / gen / verify / --version
.hiker/temporal.tent          canonical worked-example spec
examples/temporal*/           systems-under-test (rust / ts / python)
examples/architecture.{tent,facts.json}   verify worked example (dependency direction)
examples/extractors/          tiny fact extractors (deps-to-facts.mjs, grep-to-facts.sh)
skills/hiker/                 agent skill (SKILL.md + reference/)
scripts/release-version.sh    channel + version resolver
install                       channel-aware installer (repo slug: jalbarrang/hiker)
.github/workflows/            ci.yml (fmt→clippy→test), release.yml (channels)
```
