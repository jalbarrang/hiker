# `gen` — wire property tests from a spec

Goal: turn the laws into runnable property tests against the real implementation,
so a violation fails loudly. Output is a **gitignored artifact** under
`.hiker-cache/`; the `.tent` stays the source of truth.

```sh
hiker gen .hiker/tents/<slug>/<slug>.tent --target rust|ts|python --module <import>
```

- `--module` is the system-under-test the tests call into: a Rust crate name, a
  TS import path, or a Python module name.
- No `-o` → writes `.hiker-cache/<target>/<default-name>`.
- `gen` refuses to emit from intent that doesn't `check`.

## Correspondence (by convention)

The SUT must expose a **type per sort** and a **function per relation** with names
and fields matching the spec. The generated test builds random sort values, calls
the relation function, and asserts it equals the law's oracle.

| sort/relation | Rust | TypeScript | Python |
|---|---|---|---|
| sort w/ fields | `struct` | object / interface | dataclass (kwargs) |
| no-field sort | `u32` id | `number` | `int` |
| relation | `fn r(&a,&b)->bool` | `r(a,b):boolean` | `r(a,b)->bool` |
| framework | proptest | fast-check / vitest | Hypothesis / pytest |

## Discovery (all runners skip dotted dirs — wire a bridge)

The cache dir starts with a dot, which every test runner skips during globbing.
Add a committed shim once:

- **Rust** — `tests/<name>.rs` containing
  `include!("../.hiker-cache/rust/generated.rs");`. cargo discovers `tests/`.
- **TypeScript** — `tests/<name>.test.ts` containing
  `import "../.hiker-cache/ts/generated.test.ts";`. Point vitest `include` at
  `tests/**`.
- **Python** — pass the explicit file path to pytest:
  `pytest .hiker-cache/python/test_generated.py`. Add a `conftest.py` that puts
  the SUT dir on `sys.path`.

## Gen-first

A fresh checkout has no generated tests. Run `gen` before the test command (CI
does it as a pre-test step). A typical `package.json`:

```json
{
  "scripts": {
    "intent": "for f in .hiker/tents/*/*.tent; do hiker check \"$f\" || exit 1; done",
    "pretest": "hiker gen .hiker/tents/<slug>/<slug>.tent --target ts --module <import>"
  }
}
```

## Verify the loop

Confirm the test actually catches drift: temporarily break the implementation so
it contradicts a law (e.g. collapse the distinction the spec forbids) and watch
the generated property test fail with a shrunk counterexample. Revert.
