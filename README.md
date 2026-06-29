# rascador

A tiny DSL for stating **architectural intent** that *compiles* — and a bridge
that turns that intent into property tests (Rust, TypeScript, or Python) so your
real code can't quietly drift away from it.

> Inspired by the idea that AI agents preserve the *local* shape of a module but
> get creative with the *global* semantics of an architecture. The fix: put the
> intent in a compiled artifact so violations fail loudly instead of silently.

This is a **learn-by-building** project. We hand-roll a lexer, parser, checker,
and code generator — the anatomy of every compiler — one stage at a time.

## The 4 concepts of the language

| Concept | What it is | Example |
|---|---|---|
| **sort** | a kind of entity | `TemporalPoint`, `TemporalInterval` |
| **relation** | a named, *typed* relationship | `point_in_interval(TemporalPoint, TemporalInterval)` |
| **well-formedness** | when a relation is valid to state | left must be a point, right an interval |
| **law** | the predicate that must hold | point's instant lies within `[t0, t1]` |

Intent files live in **`.rascador/`** with the **`.tent`** extension
(in-**tent** → intent). See [`.rascador/temporal.tent`](.rascador/temporal.tent).

## The worked example

Distinguish **point-like** media (an image at a moment) from **range-like** media
(a video shot from `T0` to `T1`). Relating the two is *"does this point lie inside
this interval?"* — **not** *"do these two intervals overlap?"*. Collapsing that
distinction is the exact bug we want compiled intent to catch.

## How it will work (pipeline)

```
.tent text ──lexer──▶ tokens ──parser──▶ AST ──checker──▶ "intent compiles" ✅/❌
                                          │              (language-agnostic front end)
                                          └──backend──▶ rust   → proptest
                                                        ts     → fast-check   ──▶ catches drift
                                                        python → Hypothesis
```

The front end (lexer/parser/checker) is the same for every language; only the
chosen **backend** is target-specific. Adding a language = adding a backend.

## Commands

```sh
# Does intent compile? (language-agnostic)
cargo run -p rascador -- check .rascador/temporal.tent

# Emit the test bridge for a target. With no -o, output goes to
# .rascador-cache/<target>/<default-name> (gitignored).
cargo run -p rascador -- gen .rascador/temporal.tent --target rust
cargo run -p rascador -- gen .rascador/temporal.tent --target ts
cargo run -p rascador -- gen .rascador/temporal.tent --target python
```

`--module <name>` sets the system-under-test import (crate / import path /
module). `--target` defaults to `rust`.

## Generated tests are disposable artifacts

Everything `gen` writes lands in **`.rascador-cache/`** (gitignored). The single
source of truth is the `.tent` file; the tests are regenerated, never committed.
Consequence: **run `rascador gen` before running tests** (CI does this as a
pre-test step) — a fresh checkout has no generated tests until you do.

Each runner needs help finding the cache, because they all skip dotted dirs:

| Target | Discovery | Runner |
|---|---|---|
| rust | committed `include!` shim in `tests/` | `cargo test` |
| ts | committed shim in `tests/` imports the cache file | `vitest` |
| python | pass the explicit cache file path to pytest | `pytest` |

See the three `examples/temporal*` projects for the exact wiring.

## See it catch a bug — in all three languages

Each worked example reproduces the video's exact mistake: to reuse the
interval-overlap code, an agent gives a point a phantom duration `[t, t+5]` —
quietly turning point-like media into range-like media. The generated property
test (whose oracle is the real law) catches it every time, while
`temporal_overlap` keeps passing.

```sh
# Rust — buggy via a cargo feature
cargo run -p rascador -- gen .rascador/temporal.tent --target rust
cargo test -p temporal                     # correct: 2 passed
cargo test -p temporal --features buggy    # buggy: law_point_in_interval FAILS

# TypeScript (examples/temporal-ts) — buggy via env var
cd examples/temporal-ts && npm install && npm test        # correct: 2 passed
npm run test:buggy                                         # buggy: FAILS

# Python (examples/temporal-py) — buggy via env var
cd examples/temporal-py && python3 -m venv .venv \
  && .venv/bin/pip install -r requirements.txt
make test         # correct: 2 passed
make test-buggy   # buggy: FAILS
```

A typical Rust counterexample shrinks to `p_t = -8, i_t0 = -3, i_t1 = 0`: the
point sits *before* the interval, so the law says "not inside" — but the buggy
code reports "inside." The intent, compiled into a property test, caught drift
that a naive unit test would have waved through.

There are two distinct safety nets here:

1. **Intent compiles** — try writing `p.t0` (an interval field) on a point in
   the spec and run `rascador check`: it's a *type error*. The collapse can't
   even be stated.
2. **Intent is enforced** — the generated proptest fails when the real code
   contradicts a law.

## Build order

The work is split across two plans in `.plans/`:

- `rascador-intent-dsl` — the compiler, one stage per task (lexer → parser →
  checker → codegen → CLI), plus the Rust worked example.
- `rascador-multitarget-backends` — pluggable rust/ts/python backends, the
  cache-artifact layout, and the TS + Python worked examples.

Each task teaches one piece.
