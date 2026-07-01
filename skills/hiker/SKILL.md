---
name: hiker
description: >-
  Capture, compile, and enforce architectural intent with hiker, a tiny DSL
  whose `.tent` specs declare sorts, relations, and laws and generate
  Rust/TypeScript/Python property tests. Use when a repo has a `.hiker/`
  directory or `.tent` files, when setting up intent for a new initiative or
  plan, when stating or checking an architectural invariant (a mapping, a
  distinction two parts must agree on, a deterministic rule), or when an agent
  might quietly collapse a domain distinction to make a test pass. Sub-commands:
  `create` (set up intent for an initiative), `gen` (wire property tests),
  `check` (compile the intent), `verify` (enforce laws over extracted codebase
  facts). Not for general feature or UI work.
version: 1.2.1
---

# hiker

A small DSL that keeps **architectural intent** in a *compiled* artifact so code
can't quietly drift from it. Intent lives in `.hiker/*.tent`; the `hiker`
CLI checks the intent is coherent and generates property tests from it. Three
safety nets: **intent compiles** (incoherent specs are type errors), **intent
is enforced** (generated tests fail when code contradicts a law), and **intent
conforms** (`verify` evaluates the same laws over extracted codebase facts).

## Setup

Do these before proceeding:

1. **Locate intent.** Each intent is a folder `.hiker/tents/<slug>/` holding
   `<slug>.tent` (the spec) and `CONTEXT.md` (what it means + code anchors).
   Read any `CONTEXT.md`, then `hiker check` each `<slug>.tent` to confirm the
   intent still compiles. If `hiker` isn't on PATH:
   `cargo install --path <hiker-repo>/crates/hiker`.
2. **If invoked with a sub-command** (`create`, `gen`, `verify`), you MUST read
   `reference/<command>.md` next — it defines the flow. Don't improvise it.
3. **Before writing or editing any `.tent`**, read `reference/grammar.md`.

## Commands

- **`create`** — capture an initiative / plan / domain's intent into a new
  `.tent` and wire a check script. → `reference/create.md`
- **`gen`** — emit property tests from a spec and wire them into the project's
  test runner (rust/ts/python). → `reference/gen.md`
- **`verify`** — enforce a spec's laws over facts extracted from a real codebase
  (structural conformance: dependency direction, forbidden imports). →
  `reference/verify.md`
- **`check`** — `hiker check .hiker/tents/<slug>/<slug>.tent`. Prints
  `OK: N sorts, N relations, N laws` (exit 0) or line-numbered errors. Run it
  after every `.tent` edit. (inline; no reference needed)

## The model (4 concepts)

- **sort** — a kind of entity. `sort Tag`, `sort Point { media: Tag, t: Int }`.
  Field types are `Int` or another sort; a no-field sort is an identity.
- **relation** — a named, *typed* relationship. `relation maps_to(a: X, b: Y)`.
  A **`forbidden relation`** states a structural negative (no fact may match,
  e.g. a banned import) and carries no law — `verify` flags any matching fact.
  → `reference/grammar.md`, `reference/verify.md`.
- **well-formedness** — the checker enforces relation parameter types, so a law
  can't read a field a sort doesn't have. Violations = won't compile.
- **law** — the predicate(s) a relation must satisfy. Comparisons
  (`== <= < >= >`) AND-ed; a clause may be an implication `a => b`.

## Principles

- The `.tent` is the **source of truth** — edit it first, then change code.
- Model the distinction that must NOT collapse as *types*, so the wrong thing is
  unstatable (the anti-collapse guard).
- A green `check` proves the intent is *internally coherent*, not that the code
  matches it — pair it with tests.
- Generated tests are **gitignored artifacts**: regenerate, never commit, run
  `gen` before the test command.

## Limits

Laws are comparisons + implication only — no arithmetic, no enumerated literals,
so **totality** ("every case covered") isn't expressible yet. Cover fallbacks
with a normal test. Full detail in `reference/grammar.md`.
