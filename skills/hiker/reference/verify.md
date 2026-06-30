# `verify` — enforce laws over real codebase facts

Goal: catch **structural** violations (dependency direction, "no fs under
routers/") that a generated test can't see, because they describe the codebase
*graph*, not a callable function. `verify` evaluates the **same laws** as `gen`,
but over a **finite population of facts** the consuming repo extracted — not
random inputs.

```sh
hiker verify <file.tent> --facts <facts.json>
```

- Checks the spec first (refuses to verify intent that doesn't `check`).
- Exits `0` and prints `OK: 0 violations across N laws` when every law holds.
- Exits `1` and prints one line per violation otherwise.

This is the **third safety net**, alongside `check` (intent compiles) and `gen`
(behavioral laws over random inputs):

```
law ──gen────→ random inputs   → property test  (behavioral)
law ──verify─→ extracted facts → conformance    (structural)
```

## hiker never extracts — you do

hiker is language-agnostic; it consumes `facts.json`, it does not read TS/Rust/
Python. Extraction lives in the consuming repo: `madge --json`,
`dependency-cruiser`, `cargo-modules`, or a grep. See
[`examples/extractors/`](../../../examples/extractors/) for two tiny ones:

- `deps-to-facts.mjs` — import graph → `depends_on` edges with a `layer` per path.
- `grep-to-facts.sh` — banned import under `routers/` → tuples for a `forbidden relation`.

## Fact format (JSON)

```json
{
  "instances": {
    "Module": [
      { "id": "core",    "fields": { "layer": 0 } },
      { "id": "cms-api", "fields": { "layer": 1 } },
      { "id": "cli",     "fields": { "layer": 2 } }
    ]
  },
  "tuples": {
    "depends_on": [ ["cli", "cms-api"], ["cms-api", "core"] ]
  }
}
```

- `instances[Sort]` — one object per real entity. `fields` carry the **Int**
  values laws read; identity/no-field sorts may omit `fields`.
- `tuples[Relation]` — arrays of instance `id`s, **positional** to the relation's
  params. Optional (a relation may have no extracted edges).
- Identity is the `id` string; `==` on whole entities compares ids.

### Type-check (before evaluation, collects all errors)

1. every `instances` key is a declared sort; every `tuples` key a declared relation;
2. each instance supplies every **Int** field its sort declares;
3. each tuple's arity matches its relation, and each element is an id present in
   the population of the matching param's sort.

A failure here prints `facts do not match spec (N error(s)):` and exits 1.

## Population modes (inferred — no grammar change)

| Facts supply… | Mode | What's checked |
|---|---|---|
| tuples for the law's relation | **tuple-driven** | each tuple is one binding |
| no tuples | **cross-product** | the cartesian product of each param sort's instances |

Cross-product is the common `single_source(a, b)` pair case over one sort. It is
**capped at 10,000 combinations**; past that, `verify` refuses and tells you to
supply explicit tuples.

## `forbidden relation`

A structural negative: the relation carries **no law** (a law on it is a `check`
error), and `verify` flags **any** matching fact.

```
sort File
forbidden relation fs_import_under_routers(f: File)
```

Pair it with `grep-to-facts.sh`: a clean tree emits zero tuples → exit 0; one
banned import → a violation → exit 1.

## Worked example

```sh
hiker verify examples/architecture.tent --facts examples/architecture.facts.json
# OK: 0 violations across 1 laws

# Add an outward edge (core → cli) to the facts and re-run → one violation,
# exit 1: depends_on (line 12): core, cli — law violated by these facts
```

The committed [`examples/architecture.tent`](../../../examples/architecture.tent)
+ [`architecture.facts.json`](../../../examples/architecture.facts.json) are the
canonical conformant fixture (apps=2 → packages=1 → core=0, edges all inward).

## Verify the loop

Confirm `verify` actually catches drift: add one outward edge to the facts (an
inner module depending on an outer one) and watch the violation appear. Revert.
