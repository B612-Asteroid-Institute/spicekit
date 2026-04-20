//! Pure-Rust NAIF kernel readers.
//!
//! Reimplements the subset of CSPICE that adam-core actually uses, with
//! memory-mapped I/O and no FFI. The DAF parser ([`daf`]) is the shared
//! container reader; per-kernel-type parsers (starting with SPK in
//! [`spk`]) build on top of it.
//!
//! This crate does not link CSPICE. Parity against CSPICE is asserted
//! in the Python test suite via the existing SPICE golden fixture.

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
pub use spk_writer::{
    SpkWriter, SpkWriterError, Type3Record, Type3Segment, Type9Segment,
};
pub use text_kernel::{parse_body_bindings, parse_body_bindings_from_str, BodyBinding, TextKernelError};
