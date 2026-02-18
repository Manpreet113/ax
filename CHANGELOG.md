# Changelog

All notable changes to **ax** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
