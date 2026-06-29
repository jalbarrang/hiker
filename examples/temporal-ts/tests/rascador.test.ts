// Committed shim so vitest discovers the gitignored generated tests.
//
// Vitest's file globbing skips dotted directories like `.rascador-cache/`, so
// we import the generated test module explicitly (explicit imports into dotted
// dirs work fine). The generated file calls `test(...)` at module load, which
// registers its cases here. Run `npm run gen` before `vitest`, or this import
// fails because the cache file does not exist yet.
import "../.rascador-cache/ts/generated.test.ts";
