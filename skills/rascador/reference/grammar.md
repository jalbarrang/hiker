# rascador grammar reference

The `.tent` language: grammar, type rules, CLI, and the codegen model. Read this
before writing or editing any spec. Per-command flows live next to this file
(`create.md`, `gen.md`).

## CLI

```sh
rascador check <file.tent>
rascador gen   <file.tent> [--target rust|ts|python] [-o <out>] [--module <name>]
```

- `--target` defaults to `rust`. Known targets: `rust`, `ts`, `python`.
- `--module` is the system-under-test import the tests call into: a Rust crate
  name, a TS import path, or a Python module name (default `temporal`).
- With no `-o`, output goes to `.rascador-cache/<target>/<default-name>`
  (`generated.rs` / `generated.test.ts` / `test_generated.py`).
- `gen` refuses to emit from intent that does not `check`.

## Grammar (v0)

```
spec      := item*
item      := sort | relation | law
sort      := "sort" Ident ( "{" field ("," field)* "}" )?
field     := Ident ":" Ident                 // type: "Int" or a sort name
relation  := "relation" Ident "(" param ("," param)* ")"
param     := Ident ":" Ident                 // name : sort
law       := "law" Ident "(" Ident ("," Ident)* ")" "{" clause* "}"
clause    := pred ( "=>" pred )?             // optional implication
pred      := expr op expr
expr      := Int | Ident ( "." Ident )?      // literal | arg | arg.field
op        := "==" | "<=" | "<" | ">=" | ">"
```

- Comments: `//` to end of line.
- A law body's clauses are implicitly AND-ed.
- `a => b` (implication) lowers to `!a || b` (`not a or b` in Python).

## Type rules ("intent compiles")

The checker rejects:

1. a relation parameter whose sort is undeclared;
2. a sort field whose type is an undeclared sort;
3. a law referencing an unknown relation, or with the wrong number of arguments;
4. `x.field` where `x` isn't a law argument or the field doesn't exist on its sort;
5. an ordering comparison (`<= < >= >`) on non-`Int` operands;
6. `==` between mismatched sorts.

Rule 4 is the anti-collapse guard: because relation parameters are typed, a law
argument has a known sort, and you cannot borrow a field it doesn't have.

## Worked shape (the canonical example)

```
sort MediaItem
sort TemporalPoint { media: MediaItem, t: Int }
sort TemporalInterval { media: MediaItem, t0: Int, t1: Int }

relation temporal_overlap(a: TemporalInterval, b: TemporalInterval)
relation point_in_interval(p: TemporalPoint, i: TemporalInterval)

law temporal_overlap(a, b) {
  a.media == b.media
  a.t0 <= b.t1
  b.t0 <= a.t1
}

law point_in_interval(p, i) {
  p.media == i.media
  i.t0 <= p.t
  p.t <= i.t1
}
```

Implication, for a conditional invariant such as determinism:

```
relation same_classification(a: CoreError, b: CoreError)
law same_classification(a, b) {
  a.tag == b.tag => a.status == b.status
}
```

## Codegen model

Per law, one property test that:
1. generates random values per field (Int → small range; no-field sort → small
   identity id), 2. builds the argument values, 3. computes the law as an
   **oracle** (AND of clauses), 4. calls the SUT function of the same name,
   5. asserts the implementation equals the oracle.

The law is the source of truth; the implementation is on trial.

## Correspondence (by convention)

| Concept | Rust | TypeScript | Python |
|---|---|---|---|
| sort with fields | `struct` w/ matching fields | object / interface | class (kwargs) |
| no-field sort | `u32` identity | `number` | `int` |
| relation | `fn name(&a, &b) -> bool` | `name(a, b): boolean` | `name(a, b) -> bool` |
| framework | proptest | fast-check (vitest) | Hypothesis (pytest) |

Names and field names must match the `.tent` spec exactly.

## Generated tests, cache, and discovery

Everything `gen` writes lands under `.rascador-cache/` (gitignore it); the `.tent`
file is the single source of truth. Each runner needs a small committed shim to
find the dotted cache dir. Full per-language wiring is in `gen.md`.

## Limits

- Laws are comparisons + implication only: no arithmetic, no enumerated literals.
- **Totality** (every case covered, fallback behavior) is not expressible — cover
  it with a normal test.
- `check` proves internal coherence, not implementation conformance.
