#!/usr/bin/env bash
# release-version.sh — compute the release CHANNEL + VERSION for hiker.
#
# Mirrors opencode's channel model, shrunk for a single-binary Rust tool:
#   * CHANNEL = git branch (overridable). `main`/`master`/`stable` => "stable".
#     Any other branch (e.g. `beta`) is a *preview* channel.
#   * stable  => real semver, bumped off the latest `vX.Y.Z` git tag.
#   * preview => prerelease `vX.Y.Z-<channel>.N` that sorts *below* the stable
#     it leads, so `cargo`/semver treat it as a pre-release.
#
# Inputs (all optional, env or flags):
#   HIKER_CHANNEL   force the channel (stable|beta|...)
#   HIKER_BUMP      major|minor|patch   (default: patch) — stable only
#   HIKER_VERSION   hard-override the version string entirely
#
# Output: prints `channel=`, `version=`, `prerelease=` lines, and appends the
# same to $GITHUB_OUTPUT when running inside GitHub Actions.
set -euo pipefail

# ---- args -> env -------------------------------------------------------------
while [ $# -gt 0 ]; do
  case "$1" in
    --channel) HIKER_CHANNEL="$2"; shift 2 ;;
    --bump)    HIKER_BUMP="$2";    shift 2 ;;
    --version) HIKER_VERSION="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

bump="${HIKER_BUMP:-patch}"

# ---- channel -----------------------------------------------------------------
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo main)"
channel="${HIKER_CHANNEL:-$branch}"
case "$channel" in
  main|master|stable) channel="stable" ;;
esac

if [ "$channel" = "stable" ]; then prerelease="false"; else prerelease="true"; fi

# ---- helpers -----------------------------------------------------------------
# Latest stable tag `vX.Y.Z` (no prerelease suffix), or 0.1.0 if none yet.
latest_stable() {
  { git tag --list 'v*' 2>/dev/null || true; } \
    | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' \
    | sed 's/^v//' | sort -V | tail -1 || true
}

bump_semver() { # $1=X.Y.Z  $2=major|minor|patch
  local IFS=. ; read -r MA MI PA <<<"$1"
  case "$2" in
    major) echo "$((MA+1)).0.0" ;;
    minor) echo "${MA}.$((MI+1)).0" ;;
    *)     echo "${MA}.${MI}.$((PA+1))" ;;
  esac
}

# ---- version -----------------------------------------------------------------
if [ -n "${HIKER_VERSION:-}" ]; then
  version="${HIKER_VERSION#v}"
else
  base="$(latest_stable)"; base="${base:-0.1.0}"
  next="$(bump_semver "$base" "$bump")"
  if [ "$prerelease" = "false" ]; then
    version="$next"
  else
    # Preview leads the next stable: vNEXT-<channel>.N, N auto-incremented.
    count="$(git tag --list "v${next}-${channel}.*" 2>/dev/null | wc -l | tr -d ' ')"
    version="${next}-${channel}.$((count+1))"
  fi
fi

# ---- emit --------------------------------------------------------------------
echo "channel=$channel"
echo "version=$version"
echo "prerelease=$prerelease"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  {
    echo "channel=$channel"
    echo "version=$version"
    echo "prerelease=$prerelease"
  } >>"$GITHUB_OUTPUT"
fi
