# Releasing hiker

hiker uses a **channel = branch** release model (the same shape opencode uses,
shrunk for a single binary).

| Channel | Branch  | Version shape        | GitHub Release |
|---------|---------|----------------------|----------------|
| stable  | `stable`| `vX.Y.Z`             | normal         |
| beta    | `beta`  | `vX.Y.Z-beta.N`      | prerelease     |

## How a release happens

1. **Push to a channel branch** (`stable` or `beta`), or run the **release**
   workflow manually (Actions → release → Run workflow) and pick a `bump`.
2. `.github/workflows/release.yml` runs three jobs:
   - **version** — `scripts/release-version.sh` computes the channel + version
     from the branch, the latest `vX.Y.Z` tag, and the bump.
   - **build** — cross-compiles `hiker` for 6 targets (linux gnu/musl/arm64,
     macOS x64/arm64, Windows x64), archives each with a `.sha256`.
   - **release** — creates the GitHub Release at tag `vX.Y.Z`, marked
     *prerelease* for beta, and uploads every archive.

## Versioning rules (`scripts/release-version.sh`)

- **stable**: next version = latest `vX.Y.Z` tag bumped by `HIKER_BUMP`
  (`patch` default; `minor`/`major` via the workflow input).
- **beta**: leads the next stable — `v<next>-beta.N`, where `N` auto-increments
  off existing `v<next>-beta.*` tags. Sorts *below* the stable it precedes.
- Override anything with `HIKER_VERSION=1.2.3` or `--version 1.2.3`.

Try it locally (read-only, no tags created):

```sh
scripts/release-version.sh --channel stable --bump minor
scripts/release-version.sh --channel beta
```

## Installing a channel

```sh
# stable (default)
curl -fsSL https://raw.githubusercontent.com/OWNER/hiker/stable/install | sh

# latest beta
curl -fsSL https://raw.githubusercontent.com/OWNER/hiker/stable/install | sh -s -- --channel beta

# exact version
curl -fsSL https://raw.githubusercontent.com/OWNER/hiker/stable/install | sh -s -- --version 1.2.3
```

The installer detects OS/arch, pulls the matching asset, verifies the checksum,
and drops the binary in `$HIKER_INSTALL_DIR` (default `~/.hiker/bin`).

## One-time setup before the first release

- **Set the repo slug.** Replace `OWNER/hiker` in `install` (or have users export
  `HIKER_REPO`). The workflow itself uses `${{ github.repository }}` automatically.
- **Create the branches:** `git branch stable` and `git branch beta` and push them.
- The crate version in `crates/hiker/Cargo.toml` is **stamped by CI** at build
  time, so you don't bump it by hand.
