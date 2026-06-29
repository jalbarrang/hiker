import { defineConfig } from "vitest/config";

// The committed shim in tests/ imports the gitignored generated tests, which
// vitest's globbing would otherwise skip (dotted dir).
export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
  },
});
