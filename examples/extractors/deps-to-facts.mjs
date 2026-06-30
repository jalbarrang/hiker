#!/usr/bin/env node
// deps-to-facts.mjs — emit hiker `facts.json` for the dependency-direction law.
//
// Usage:
//   node deps-to-facts.mjs <dir> > facts.json
//   cargo run -p hiker -- verify examples/architecture.tent --facts facts.json
//
// What it does: walks <dir> for JS/TS source files, treats each as a `Module`
// instance, assigns a `layer` by a path rule (apps=2, packages=1, core=0,
// default=1), and emits a `depends_on` tuple for every *relative* import that
// resolves to another file in the set. hiker then checks `imported.layer <=
// importer.layer` over those real edges.
//
// hiker never extracts facts itself — this lives in the consuming repo. Swap the
// walk for `madge --json <dir>` output if you already produce that; the emitted
// shape (instances/tuples) is the contract, not the source.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname, resolve } from "node:path";

const SRC_RE = /\.(mjs|cjs|js|jsx|ts|tsx)$/;
const SKIP = new Set(["node_modules", ".git", ".hiker-cache", "dist", "build"]);
const EXTS = ["", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

const root = resolve(process.argv[2] ?? ".");

/** Layer by path convention: apps=2, packages=1, core=0, else 1. */
function layerFor(relPath) {
  const p = `/${relPath}/`;
  if (/\/apps\//.test(p)) return 2;
  if (/\/core\//.test(p)) return 0;
  if (/\/packages\//.test(p)) return 1;
  return 1;
}

/** Recursively collect source files, skipping vendored/dotted dirs. */
function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (name.startsWith(".") || SKIP.has(name)) continue;
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, out);
    else if (SRC_RE.test(name)) out.push(full);
  }
  return out;
}

/** Pull relative import/require specifiers out of a file's text. */
function importsOf(text) {
  const specs = [];
  const re =
    /(?:import\s[^'"]*?from\s*|import\s*|require\(\s*|export\s[^'"]*?from\s*)['"](\.[^'"]+)['"]/g;
  let m;
  while ((m = re.exec(text)) !== null) specs.push(m[1]);
  return specs;
}

const files = walk(root);
const ids = new Set(files.map((f) => relative(root, f)));

/** Resolve a relative specifier from a file to a known module id, or null. */
function resolveId(fromFile, spec) {
  const base = resolve(dirname(fromFile), spec);
  for (const ext of EXTS) {
    const cand = relative(root, base + ext);
    if (ids.has(cand)) return cand;
  }
  for (const ext of EXTS.slice(1)) {
    const cand = relative(root, join(base, "index" + ext));
    if (ids.has(cand)) return cand;
  }
  return null;
}

const instances = files.map((f) => {
  const id = relative(root, f);
  return { id, fields: { layer: layerFor(id) } };
});

const tuples = [];
for (const f of files) {
  const importer = relative(root, f);
  let text;
  try {
    text = readFileSync(f, "utf8");
  } catch {
    continue;
  }
  for (const spec of importsOf(text)) {
    const imported = resolveId(f, spec);
    if (imported && imported !== importer) tuples.push([importer, imported]);
  }
}

const facts = { instances: { Module: instances }, tuples: { depends_on: tuples } };
process.stdout.write(JSON.stringify(facts, null, 2) + "\n");
