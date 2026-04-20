# spicekit

spicekit is an independent, pure-Rust reader for the SPICE kernel formats
(DAF containers, SPK ephemeris files, binary PCK rotation files, and the
text-kernel subset used for body-name bindings). It is **not affiliated
with or endorsed by NAIF/JPL, and it is not a port of the CSpice
toolkit** — it is a from-scratch reimplementation that reads NAIF-format
files without linking any C code.

## Scope

`spicekit` implements the subset of SPICE consumed by the
[adam-core](https://github.com/B612-Asteroid-Institute/adam_core) asteroid
dynamics library, which is the project it was originally extracted from:

- **DAF**: memory-mapped container parser (shared by SPK and PCK)
- **SPK**: reader for Types 2, 3, 9, 13; writer for Types 3 and 9
- **PCK**: binary PCK reader producing J2000↔ITRF93 rotations and their
  time derivatives at machine precision
- **Text kernels**: parser for `NAIF_BODY_NAME` / `NAIF_BODY_CODE` paired
  arrays (custom name ↔ code bindings used by mission kernels). Other
  keys (leapseconds, SCLK constants, frame definitions) parse without
  error but their contents are not exposed.
- **Built-in NAIF body table**: the 692-entry name ↔ ID map mirrored from
  CSpice's `zzidmap.c`, with case-insensitive whitespace-tolerant
  matching.

## Non-goals

- No async I/O — sync memory-mapped reads only.
- No CSpice linkage and no FFI. Correctness is verified in the parent
  project's test suite by comparing against CSpice output at
  machine-epsilon tolerance.
- No Python bindings ship in this crate. (adam-core exposes spicekit to
  Python via its own PyO3 layer.)
- No CLI tool.
- Read-only for PCK in v0.1.

See each module's rustdoc for details.

## License and attribution

Licensed under the [MIT License](./LICENSE).

The file `src/naif_builtin_table.rs` contains data extracted from the
NAIF CSpice toolkit (specifically, the NPERM table in `zzidmap.c`). The
NAIF toolkit is distributed by the Jet Propulsion Laboratory under terms
that permit redistribution with attribution — see `LICENSE-NOTICES` for
the full attribution and the relevant NAIF distribution-terms excerpt.

## Parity testing

spicekit's own test suite verifies internal correctness (DAF round-trip,
Chebyshev polynomial exactness on coefficient tables, Lagrange exactness
at knot points, text-kernel parser tolerance of LSK/SCLK content, etc.).
CSpice-parity testing — confirming that spicekit's numeric output
bit-matches CSpice at a science-grade tolerance — lives in the adam-core
repository, which keeps an FFI-based CSpice reference installation for
precisely this purpose.
