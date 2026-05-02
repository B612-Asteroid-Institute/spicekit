//! Pure-Rust readers (and writer) for NASA/NAIF SPICE kernel formats.
//!
//! Implements the subset of SPICE this crate explicitly supports — DAF
//! containers, SPK ephemerides, binary PCK rotations, and the
//! body-name slice of text kernels — with memory-mapped I/O and no
//! FFI. The DAF parser ([`daf`]) is the shared container reader; the
//! per-kernel-type parsers ([`spk`], [`pck`], [`text_kernel`]) build
//! on top of it. [`spk_writer`] emits SPK Type 3 and Type 9 segments.
//!
//! This crate does not link CSPICE. Bit-for-bit parity against CSPICE
//! is asserted in the sibling `spicekit-bench` crate, which links
//! `cspice-sys` behind a feature flag and exercises every numeric
//! code path (`spkez`, `pxform`, `sxform`, `bodc2n`, `bodn2c`, plus
//! the text-kernel precedence semantics) at machine-epsilon
//! tolerance.

pub mod daf;
pub mod frame;
pub mod naif_ids;
pub mod pck;
pub mod spk;
pub mod spk_writer;
pub mod text_kernel;

pub use daf::{DafError, DafFile, Summary};
pub use frame::{rotate_state, NaifFrame, OBLIQUITY_J2000_RAD};
pub use naif_ids::{bodc2n, bodn2c, NaifIdError};
pub use pck::{PckError, PckFile, PckSegment};
pub use spk::{SpkError, SpkFile, SpkSegment};
pub use spk_writer::{SpkWriter, SpkWriterError, Type3Record, Type3Segment, Type9Segment};
pub use text_kernel::{
    parse_body_bindings, parse_body_bindings_from_str, BodyBinding, TextKernelError,
};
