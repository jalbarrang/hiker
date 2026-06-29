# hiker

A tiny DSL for stating **architectural intent** that *compiles* — plus a bridge
that turns that intent into property tests (Rust, TypeScript, Python) so your
real code can't quietly drift away from it.

> **Why.** AI agents preserve the *local* shape of a module but get creative with
> the *global* semantics of an architecture — collapsing a distinction to make a
> test pass, in a diff that looks reasonable. hiker puts the intent in a compiled
> artifact so that collapse becomes a **compile error** and a **failing test**
> instead of silent drift.

You *pitch a tent* — intent files use the `.tent` extension (in-**tent** →
intent) and live in `.hiker/`.

## Two safety nets

1. **Intent compiles.** Incoherent intent is a *type error*. Write `p.t0` (an
   interval field) on a point and `hiker check` rejects it — the collapse can't
   even be stated.
2. **Intent is enforced.** Generated property tests (oracle = the law) fail when
   the real code contradicts a law.

## The language (4 concepts)

| Concept | What it is | Example |
|---|---|---|
| **sort** | a kind of entity | `sort TemporalPoint { media: MediaItem, t: Int }` |
| **relation** | a named, *typed* relationship | `relation point_in_interval(p: TemporalPoint, i: TemporalInterval)` |
| **law** | the predicate(s) that must hold | `i.t0 <= p.t`, `p.t <= i.t1` |
| **well-formedness** | when a relation is valid to state | enforced via the relation's parameter sorts |

A law body is comparisons, implicitly AND-ed. A clause may be an implication
`a => b` (lowers to `!a || b` in every backend) — useful for conditional
invariants like determinism: `a.tag == b.tag => a.status == b.status`.

Full grammar + type rules: [`skills/hiker/reference/grammar.md`](skills/hiker/reference/grammar.md).
Worked spec: [`.hiker/temporal.tent`](.hiker/temporal.tent).

## Pipeline

```
.tent text ─lexer→ tokens ─parser→ AST ─checker→ "intent compiles" ✅/❌
                                     │            (language-agnostic front end)
                                     └─backend→ rust   → proptest
                                                ts     → fast-check    → catches drift
                                                python → Hypothesis
```

The front end (lexer/parser/checker) is language-agnostic; only the backend is
target-specific. Adding a language = adding a `Backend`.

## Install

```sh
# stable (default)
curl -fsSL https://raw.githubusercontent.com/jalbarrang/hiker/stable/install | sh
# latest beta
curl -fsSL https://raw.githubusercontent.com/jalbarrang/hiker/stable/install | sh -s -- --channel beta
```

Or build from source: `cargo build --release -p hiker` → `target/release/hiker`.

## Commands

```sh
hiker check .hiker/temporal.tent                    # does intent compile?
hiker gen   .hiker/temporal.tent --target rust      # emit the test bridge
hiker gen   .hiker/temporal.tent --target ts --module ../../src/temporal
hiker --version
```

- `--target` ∈ `rust | ts | python` (default `rust`).
- `--module` sets the system-under-test import (crate / import path / module).
- `gen` refuses to emit from intent that does not `check`.
- `check` **warns** on a relation with no law (declared but unenforced) and
  **errors** on an empty law body (constrains nothing).

## Generated tests are disposable

`gen` writes to **`.hiker-cache/`** (gitignored). The `.tent` file is the single
source of truth; tests are regenerated, never committed — so **run `hiker gen`
before running tests** (CI does this as a pre-test step). Each runner skips
dotted dirs, so the examples wire discovery explicitly:

| Target | Discovery | Runner |
|---|---|---|
| rust | committed `include!` shim in `tests/` | `cargo test` |
| ts | committed shim in `tests/` imports the cache file | `vitest` |
| python | pass the cache file path to pytest | `pytest` |

## See it catch the bug — in all three languages

Each `examples/temporal*` project reproduces the exact mistake: to reuse the
interval-overlap code, an agent gives a point a phantom duration `[t, t+5]`,
turning point-like media into range-like media. The generated property test
catches it every time while `temporal_overlap` keeps passing.

```sh
# Rust
hiker gen .hiker/temporal.tent --target rust -o .hiker-cache/rust/generated.rs
cargo test -p temporal                     # correct: 2 passed
cargo test -p temporal --features buggy    # buggy: law_point_in_interval FAILS

# TypeScript
cd examples/temporal-ts && npm install && npm test   # correct: 2 passed
npm run test:buggy                                   # buggy: FAILS

# Python
cd examples/temporal-py && python3 -m venv .venv && .venv/bin/pip install -r requirements.txt
make test         # correct: 2 passed
make test-buggy   # buggy: FAILS
```

## Releases

Channel = branch: push `stable` for a semver release, `beta` for a prerelease.
See [`docs/RELEASING.md`](docs/RELEASING.md).

## Layout

```
crates/hiker/        the compiler + CLI (lexer, parser, checker, backends)
.hiker/temporal.tent the worked-example intent spec
examples/temporal*   systems-under-test (Rust / TS / Python) that the tests run against
skills/hiker/        the agent skill: how to author and check intent
scripts/ + install   release-version resolver + channel-aware installer
docs/RELEASING.md    the release/channel model
```

Status: all three backends working, CI green (fmt → clippy `-D warnings` →
test), channel-based releases wired.
