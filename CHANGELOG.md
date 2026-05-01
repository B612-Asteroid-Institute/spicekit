# Changelog

All notable changes to `spicekit` (Rust) and `spicekit` (Python bindings)
are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **SPK/PCK Type 2/3/9/13 hot path** rewritten for ~4.8× lower per-call
  latency on the propagation pattern. Output is bit-for-bit identical
  to the previous implementation — verified by 543k-double identity
  check on real DE440 and by the CSpice parity oracle (26 tests at
  `rtol=1e-14, atol=1e-7`). Three composing changes:
  - Zero-copy `&[f64]` view of the mmap via `bytemuck::cast_slice`,
    replacing the per-call `Vec<f64>` allocation and per-byte LE
    decode loop. New `DafFile::doubles_native` accessor; the existing
    `read_doubles` is retained for the writer and external API.
    Compile-time `compile_error!` rejects big-endian hosts to prevent
    a silent byte-order regression.
  - Shared three-channel Chebyshev recurrence in `SpkType2`,
    `SpkType3`, and `PckType2` evaluators. New internal
    `cheby3_val_and_deriv` / `cheby3_val_only` helpers compute
    `T_k(s)` and `dT_k/ds` once per iteration and apply them to all
    three coordinate channels in lockstep. Per-channel arithmetic
    order is identical to the scalar variant, so output is bit-for-
    bit equivalent (pinned by three new unit tests).
  - `SpkFile` segment lookup uses `FxHashMap` instead of the std
    `HashMap`'s SipHash, plus an `AtomicUsize` MRU cache that
    memoizes the last successfully-evaluated segment index. The MRU
    cache is gated by an internal per-segment `cacheable` flag so it
    never fires on segments whose `(target, center)` group has
    overlapping ET coverage — the cache and a fresh
    `try_direct` lookup are guaranteed to return the same segment
    for any `(target, center, et)`. Regression test
    `overlapping_segments_do_not_corrupt_mru_cache` pins this.

### Added

- New deps: `bytemuck = "1.16"` (safe `&[u8]` → `&[f64]` cast),
  `rustc-hash = "2.0"` (faster hash for `(i32, i32)` segment-index
  keys). Zero new `unsafe`.

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
