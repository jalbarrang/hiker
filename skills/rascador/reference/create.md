# `create` — set up rascador intent for an initiative

Goal: turn an initiative / plan / PRD into a compiled `.tent` spec that captures
the one or two architectural invariants the work must preserve, then wire a check
so they can't silently drift.

Read `grammar.md` first if you haven't. Keep the spec **small** — a spec is a
condensed reference, not a re-implementation. One or two relations is normal.

Each intent gets its own folder so a repo can hold several:

```
.rascador/tents/<slug>/
  <slug>.tent      # the spec
  CONTEXT.md       # what it means, code anchors, expressiveness boundary
```

## Flow

1. **Find the intent source.** Read the initiative's `INITIATIVE.md` / the plan's
   `HANDOFF.md` / the PRD. Extract the *architectural invariant(s)* — the rule the
   system must keep true everywhere, independent of any single module:
   - a **mapping** that must be preserved (e.g. error tag → HTTP status),
   - a **distinction** two parts must agree on (e.g. point-like vs range-like),
   - a **deterministic rule** (same input class → same output).

2. **Name the collapse you want to prevent.** What shortcut would make a test
   pass while quietly breaking the architecture? Design the model so that shortcut
   is *unstatable*: omit the field/sort that the wrong code would lean on (e.g. no
   `message` field if classification must be by tag).

3. **Write `.rascador/tents/<slug>/<slug>.tent`.** Declare:
   - **sorts** — the kinds of entities (give them only the fields the laws need;
     `Int` for numbers/statuses, an identity sort for opaque ids/tags);
   - **relation(s)** — the seam where the invariant lives, with typed parameters;
   - **law(s)** — the invariant as comparisons, AND-ed. Use an implication
     `a => b` for conditional rules like determinism.

4. **Write `.rascador/tents/<slug>/CONTEXT.md`** — the prose the spec can't hold:
   one paragraph on the invariant, any canonical table, the code anchors it maps
   to (files/functions), and the expressiveness boundary (what the model does
   NOT capture). This is what a reviewer reads alongside the spec.

5. **Compile it:** `rascador check .rascador/tents/<slug>/<slug>.tent` until it
   prints `OK: ...`. Errors carry line numbers. Fix the spec, not the checker.

6. **Wire a script** so the team/agents run it (check every tent):
   - npm/pnpm: `"intent": "for f in .rascador/tents/*/*.tent; do rascador check \"$f\" || exit 1; done"`.
   - Make/justfile: an `intent` target with the same loop.
   - Gitignore generated output: add `.rascador-cache/`.

7. **Optional — generate the bridge.** If a concrete function already implements
   the relation, run the `gen` flow (`gen.md`) to emit property tests against it.

## Worked example (a tag → status mapping)

```
// .rascador/tents/typed-errors/typed-errors.tent
// Classification is by TAG, never by message. There is deliberately no
// `message` field, so "classify by message" cannot be stated.

sort Tag
sort CoreError { tag: Tag, status: Int }
sort HttpError { status: Int }

relation maps_to(core: CoreError, http: HttpError)

// The mapped HTTP status equals the status the core error's tag designates.
law maps_to(core, http) {
  core.status == http.status
}

// Determinism: same tag => same status (conditional invariant via implication).
relation same_classification(a: CoreError, b: CoreError)
law same_classification(a, b) {
  a.tag == b.tag => a.status == b.status
}
```

```sh
rascador check .rascador/tents/typed-errors/typed-errors.tent
# OK: 3 sorts, 2 relations, 2 laws
```

## Done when

- `.rascador/tents/<slug>/<slug>.tent` compiles (`OK: ...`).
- `.rascador/tents/<slug>/CONTEXT.md` records the invariant + boundary.
- A `check` script exists and `.rascador-cache/` is gitignored.
