# Changelog

All notable changes to **ax** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.11] - 2026-02-19

### Fixed
- **Resolver**: Use deterministic alphabetical sorting when falling back to linear build order (circular dependency handling) to ensure reproducible builds

## [1.0.10] - 2026-02-19

### Fixed
- **Resolver**: Fixed race condition in dependency graph building where split packages could miss dependencies if processed in the wrong order. Now uses a two-pass approach (map -> connect) to ensure correct graph connectivity.

## [1.0.9] - 2026-02-19

### Fixed
- **Git Diff**: Explicitly fetches upstream before showing diff to prevent empty output on fresh clones
- **Refactor**: Centralized cache directory resolution logic (Config > XDG > HOME) across all modules to eliminate duplication

## [1.0.8] - 2026-02-19

### Fixed
- **Critical Interaction Fix**: Auto-detects non-interactive environments (scripts, chroots) and enables `--noconfirm` automatically to prevent hangs
- **VCS Package Resolution**: Skips version comparison for development packages (`-git`, `-hg`, etc.) to prevent incorrect rebuild loops
- **CI/CD**: Added dependency caching to speed up builds and fixed `cargo fmt` invocation

## [1.0.7] - 2026-02-18

### Fixed
- **Critical Build Order Bug**: Reversed the dependency graph topological sort to ensure correct build order
  - Previously, packages were ordered `Dependent -> Dependency` (reverse of what's needed)
  - Now correctly builds `Dependency -> Dependent`
  - Fixes `cargo test` failure in `graph::tests::test_simple_dag` which was breaking CI
- **CI/CD**: Fixed unit tests to ensure reliable CI pipelines

## [1.0.6] - 2026-02-18

### Changed
- **Smart Build Skipping**: Skips building AUR packages if they are already installed and up-to-date
  - Compares installed version against AUR metadata using `vercmp`
  - Avoids redundant rebuilds for packages like `zen-browser-bin`, `wlogout`, etc.
  - Significantly speeds up `ax -S` when dependencies are already satisfied

## [1.0.5] - 2026-02-18

### Fixed
- **GPG key fetch robustness**: Handles keyboxd lock contention gracefully
  - Uses `--batch --yes` mode and suppresses stderr noise
  - Kills ALL gpg daemons (`gpgconf --kill all`) on first failure, then retries
  - Falls back to `makepkg --skippgpcheck` if keys still can't be fetched
  - Fixes builds for packages like `wlogout` that require PGP verification

## [1.0.4] - 2026-02-18

### Fixed
- **UX**: `--noconfirm` now skips all interactive prompts (diff review, PKGBUILD review, build confirmation)
  - Previously, `--noconfirm` was only forwarded to pacman but ax's own prompts still appeared
  - Error handlers now abort immediately instead of prompting when `--noconfirm` is set
- Removed stray `log.txt` from repository and added to `.gitignore`

## [1.0.3] - 2026-02-17

### Added
- **GPG key verification**: Automatically fetch required PGP keys from keyserver before building
  - Reads `validpgpkeys` from `.SRCINFO`
  - Fetches missing keys from `keyserver.ubuntu.com`
  - Prevents `makepkg` failures due to missing signature keys
  - Fixes packages like `wlogout` that require GPG verification

### Fixed
- PGP signature verification errors during package builds

## [1.0.2] - 2026-02-17

### Fixed
- **Critical**: Fixed DAG resolution dropping packages with no AUR dependencies
  - Packages like `clipse`, `zen-browser-bin`, and `wlogout` were silently excluded from build queue
  - Issue: Graph only added nodes implicitly via edges, so packages with only repo deps had 0 nodes
  - Solution: Explicitly call `graph.add_node()` for all pkgbases before processing edges

## [1.0.1] - 2026-02-17

### Fixed
- CLI help text now correctly shows `ax` instead of `raur`

## [1.0.0] - 2026-02-17

### Added
- **Argument Forwarding**: Transparent pacman wrapper with unknown flag passthrough
- **Deterministic Artifacts**: Use `makepkg --packagelist` for exact package paths
- **Parser Maturity**: Switched to `srcinfo` crate for robust `.SRCINFO` parsing
- **Graceful Degradation**: Interactive error recovery (Retry/Skip/Abort prompts)
- **DAG Resolution**: Two-phase dependency resolution with topological sorting
- **CI/CD**: GitHub Actions workflow with Arch Linux container for releases

### Fixed
- Broken pipe panic when pager closes early
- Shell injection vulnerability in `$EDITOR` execution
- Inefficient double `.SRCINFO` parsing (50% IO reduction via metadata caching)

### Changed
- Added `makepkg -r` flag to clean up build dependencies after successful builds
- Removed dead code: `is_installed()`, `ensure_keys()`, old recursive `resolve_tree()`

[Unreleased]: https://github.com/Manpreet113/ax/compare/v1.0.2...HEAD
[1.0.2]: https://github.com/Manpreet113/ax/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/Manpreet113/ax/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/Manpreet113/ax/releases/tag/v1.0.0
