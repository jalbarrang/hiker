#!/usr/bin/env bash
# grep-to-facts.sh — emit hiker `facts.json` for a `forbidden relation`.
#
# Usage:
#   ./grep-to-facts.sh <dir> > facts.json
#   # paired with a spec containing:
#   #   sort File
#   #   forbidden relation fs_import_under_routers(f: File)
#   cargo run -p hiker -- verify forbidden.tent --facts facts.json
#
# Finds every file under <dir>/**/routers/ that imports node:fs and emits one
# `fs_import_under_routers` tuple per offending file. A `forbidden` relation has
# no law: ANY emitted fact is a violation, so a clean tree yields zero tuples and
# `verify` exits 0; one banned import exits 1.
#
# hiker never greps source itself — extraction lives here, in the consuming repo.
set -euo pipefail

root="${1:-.}"
pattern="${BANNED_PATTERN:-from ['\"]node:fs['\"]|require\(['\"]node:fs['\"]\)}"

# Collect matching files under any routers/ directory. rg if present, else grep.
mapfile -t hits < <(
  if command -v rg >/dev/null 2>&1; then
    rg -l --glob '**/routers/**' -e "$pattern" "$root" 2>/dev/null || true
  else
    grep -rlE --include='*.ts' --include='*.js' "$pattern" "$root"/**/routers/** 2>/dev/null || true
  fi
)

# Build instances (File ids) and tuples (one per offending file).
instances=""
tuples=""
sep=""
for f in "${hits[@]:-}"; do
  [ -z "$f" ] && continue
  rel="${f#"$root"/}"
  instances+="${sep}{ \"id\": \"${rel}\" }"
  tuples+="${sep}[ \"${rel}\" ]"
  sep=", "
done

printf '{\n  "instances": { "File": [%s] },\n  "tuples": { "fs_import_under_routers": [%s] }\n}\n' \
  "$instances" "$tuples"
