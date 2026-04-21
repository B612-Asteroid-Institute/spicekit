# Changelog

All notable changes to `spicekit` (Rust) and `spicekit` (Python bindings)
are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-04-21

Initial public release.

### Added

- **DAF**: memory-mapped container parser shared by SPK and PCK readers.
- **SPK reader**: Types 2, 3, 9, and 13 with bit-for-bit Chebyshev and
  Lagrange evaluation matching CSpice at machine-epsilon tolerance.
- **SPK writer**: Types 3 and 9, including multi-segment files.
- **PCK reader**: binary PCK producing J2000 ↔ ITRF93 rotation matrices
  and their time derivatives (`pxform`, `sxform`, and batch variants).
- **Text kernel parser**: `NAIF_BODY_NAME` / `NAIF_BODY_CODE` paired
  arrays. Other keys (leapseconds, SCLK constants, frame definitions)
  parse without error but their contents are not exposed in v0.1.
- **Built-in NAIF body table**: 692-entry name ↔ ID map mirrored from
  CSpice's `zzidmap.c`, with case-insensitive whitespace-tolerant
  matching. `bodc2n` returns the CSpice-canonical alias for each code.
- **Python bindings** (`spicekit-py` crate, published to PyPI as
  `spicekit`): `NaifSpk`, `NaifPck`, `NaifSpkWriter`, `naif_bodn2c`,
  `naif_bodc2n`, `naif_parse_text_kernel_bindings`.
- **CSpice parity oracle** (`spicekit-bench`, unpublished): links
  `cspice-sys` behind a feature flag and asserts every code path matches
  CSpice numerically and for `bodc2n` across all 539 unique built-in
  codes.

[Unreleased]: https://github.com/B612-Asteroid-Institute/spicekit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/B612-Asteroid-Institute/spicekit/releases/tag/v0.1.0
