//! Pure-Rust DAF/SPK writer.
//!
//! Builds a complete DAF/SPK file in memory and serializes it atomically
//! to disk, so there is no partial-file state visible to a concurrent
//! reader (DAF is mmap-backed, so atomic rename is required). Supports
//! SPK Type 3 (Chebyshev position + velocity) and Type 9 (Lagrange,
//! unequal time steps) — the two types adam-core writes in
//! `orbits/spice_kernel.py`.
//!
//! Layout produced (record numbers 1-indexed, 1024 bytes each):
//!   rec 1   : file record (idword, nd=2, ni=6, fward=2, bward=2, LTL-IEEE)
//!   rec 2   : summary record chain head (single record; NSUM ≤ 25)
//!   rec 3   : matching name record (one 40-byte name slot per summary)
//!   rec 4+  : data records (padded to a full 128-double record)
//!
//! The resulting file round-trips through CSPICE's `spkez` and through
//! our own `SpkFile::open`; parity tests in the Python suite assert
//! bit-exact matches against the `sp.spkw03` / `sp.spkw09` reference.

use std::io::Write;
use std::path::Path;

use thiserror::Error;

use crate::daf::{DOUBLE_BYTES, RECORD_BYTES};

pub const SPK_ND: u32 = 2;
pub const SPK_NI: u32 = 6;
/// Summary size in doubles = ND + (NI+1)/2 = 5.
pub const SPK_SUMMARY_DOUBLES: usize = 2 + 6_usize.div_ceil(2);
/// Summary slot size in bytes = 40.
pub const SPK_SUMMARY_BYTES: usize = SPK_SUMMARY_DOUBLES * DOUBLE_BYTES;
/// One summary record header = 24 bytes (NEXT, PREV, NSUM as f64 each).
const SUMMARY_HEADER_BYTES: usize = 24;
/// Per-record summary capacity = (1024 - 24) / 40 = 25.
pub const SPK_SUMMARIES_PER_RECORD: usize =
    (RECORD_BYTES - SUMMARY_HEADER_BYTES) / SPK_SUMMARY_BYTES;
/// Doubles per DAF record = 1024 / 8 = 128.
pub const DOUBLES_PER_RECORD: usize = RECORD_BYTES / DOUBLE_BYTES;

#[derive(Debug, Error)]
pub enum SpkWriterError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("segment id '{0}' is longer than 40 bytes after UTF-8 encoding")]
    SegmentIdTooLong(String),
    #[error("too many segments ({got}); single summary record holds at most {max}")]
    TooManySegments { got: usize, max: usize },
    #[error("invalid Type 3 segment: {0}")]
    BadType3(&'static str),
    #[error("invalid Type 9 segment: {0}")]
    BadType9(&'static str),
}

/// One Chebyshev record within a Type 3 segment.
#[derive(Debug, Clone)]
pub struct Type3Record {
    pub mid: f64,
    pub radius: f64,
    /// Position X coefficients (length = degree + 1).
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Vec<f64>,
    /// Velocity X coefficients (length = degree + 1).
    pub vx: Vec<f64>,
    pub vy: Vec<f64>,
    pub vz: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Type3Segment {
    pub target: i32,
    pub center: i32,
    pub frame_id: i32,
    pub start_et: f64,
    pub end_et: f64,
    pub segment_id: String,
    /// INTLEN (seconds per record).
    pub intlen: f64,
    /// INIT (start ET of the first record's coverage).
    pub init: f64,
    pub records: Vec<Type3Record>,
}

#[derive(Debug, Clone)]
pub struct Type9Segment {
    pub target: i32,
    pub center: i32,
    pub frame_id: i32,
    pub start_et: f64,
    pub end_et: f64,
    pub segment_id: String,
    /// Polynomial degree (must be odd per NAIF convention; window = degree+1).
    pub degree: i32,
    /// States: length 6 * N (x, y, z, vx, vy, vz interleaved).
    pub states: Vec<f64>,
    /// Epochs: length N, monotonically increasing.
    pub epochs: Vec<f64>,
}

enum Segment {
    Type3(Type3Segment),
    Type9(Type9Segment),
}

pub struct SpkWriter {
    idword: [u8; 8],
    locifn: [u8; 60],
    segments: Vec<Segment>,
}

impl SpkWriter {
    /// Construct a writer with the conventional SPK idword (`b"DAF/SPK "`).
    pub fn new_spk(locifn: &str) -> Self {
        let mut locifn_bytes = [b' '; 60];
        let src = locifn.as_bytes();
        let n = src.len().min(60);
        locifn_bytes[..n].copy_from_slice(&src[..n]);
        Self {
            idword: *b"DAF/SPK ",
            locifn: locifn_bytes,
            segments: Vec::new(),
        }
    }

    pub fn add_type3(&mut self, seg: Type3Segment) -> Result<(), SpkWriterError> {
        validate_segment_id(&seg.segment_id)?;
        if seg.records.is_empty() {
            return Err(SpkWriterError::BadType3("empty records"));
        }
        let n_coef = seg.records[0].x.len();
        if n_coef == 0 {
            return Err(SpkWriterError::BadType3("degree < 0"));
        }
        for r in &seg.records {
            if r.x.len() != n_coef
                || r.y.len() != n_coef
                || r.z.len() != n_coef
                || r.vx.len() != n_coef
                || r.vy.len() != n_coef
                || r.vz.len() != n_coef
            {
                return Err(SpkWriterError::BadType3("inconsistent coefficient count"));
            }
        }
        if seg.intlen <= 0.0 {
            return Err(SpkWriterError::BadType3("INTLEN must be > 0"));
        }
        self.segments.push(Segment::Type3(seg));
        Ok(())
    }

    pub fn add_type9(&mut self, seg: Type9Segment) -> Result<(), SpkWriterError> {
        validate_segment_id(&seg.segment_id)?;
        let n = seg.epochs.len();
        if n == 0 {
            return Err(SpkWriterError::BadType9("no epochs"));
        }
        if seg.states.len() != 6 * n {
            return Err(SpkWriterError::BadType9(
                "states length != 6 * epochs length",
            ));
        }
        if seg.degree < 1 {
            return Err(SpkWriterError::BadType9("degree < 1"));
        }
        // Window = degree + 1 must be <= N to have enough samples.
        if (seg.degree as usize) + 1 > n {
            return Err(SpkWriterError::BadType9(
                "window (degree+1) exceeds sample count",
            ));
        }
        // Epochs must be strictly increasing.
        for pair in seg.epochs.windows(2) {
            if pair[0].partial_cmp(&pair[1]) != Some(std::cmp::Ordering::Less) {
                return Err(SpkWriterError::BadType9(
                    "epochs must be strictly increasing",
                ));
            }
        }
        self.segments.push(Segment::Type9(seg));
        Ok(())
    }

    /// Serialize to a byte buffer. Exposed primarily for testing; callers
    /// writing to disk should use [`write`].
    pub fn to_bytes(&self) -> Result<Vec<u8>, SpkWriterError> {
        if self.segments.len() > SPK_SUMMARIES_PER_RECORD {
            return Err(SpkWriterError::TooManySegments {
                got: self.segments.len(),
                max: SPK_SUMMARIES_PER_RECORD,
            });
        }

        // Build each segment's payload and assign 1-indexed double
        // addresses starting at the first byte of record 4 (summary +
        // name records occupy records 2-3).
        let data_start_double = 3 * DOUBLES_PER_RECORD as u32 + 1; // 385
        let mut cursor_double = data_start_double;
        let mut segment_meta: Vec<SegmentMeta> = Vec::with_capacity(self.segments.len());
        let mut payloads: Vec<Vec<f64>> = Vec::with_capacity(self.segments.len());
        for seg in &self.segments {
            let (meta_stub, payload) = encode_segment(seg)?;
            let start = cursor_double;
            let end = start + payload.len() as u32 - 1;
            cursor_double = end + 1;
            segment_meta.push(SegmentMeta {
                start_et: meta_stub.start_et,
                end_et: meta_stub.end_et,
                target: meta_stub.target,
                center: meta_stub.center,
                frame_id: meta_stub.frame_id,
                data_type: meta_stub.data_type,
                start_addr: start as i32,
                end_addr: end as i32,
                name: meta_stub.name,
            });
            payloads.push(payload);
        }

        // Total size: 3 header/summary/name records + ceil(total_data_doubles
        // / 128) data records, each 1024 bytes.
        let total_data_doubles: usize = payloads.iter().map(|p| p.len()).sum();
        let data_records = total_data_doubles.div_ceil(DOUBLES_PER_RECORD);
        let total_records = 3 + data_records;
        let mut buf = vec![0u8; total_records * RECORD_BYTES];

        // ---- record 1: file record ----
        write_file_record(
            &mut buf[0..RECORD_BYTES],
            &self.idword,
            SPK_ND,
            SPK_NI,
            &self.locifn,
            /* fward */ 2,
            /* bward */ 2,
            /* free  */ cursor_double,
        );

        // ---- record 2: summary record ----
        write_summary_record(&mut buf[RECORD_BYTES..2 * RECORD_BYTES], &segment_meta);

        // ---- record 3: name record ----
        write_name_record(&mut buf[2 * RECORD_BYTES..3 * RECORD_BYTES], &segment_meta);

        // ---- data records ----
        let data_byte_start = 3 * RECORD_BYTES;
        let mut double_idx_in_data: usize = 0;
        for payload in &payloads {
            for &d in payload {
                let off = data_byte_start + double_idx_in_data * DOUBLE_BYTES;
                buf[off..off + DOUBLE_BYTES].copy_from_slice(&d.to_le_bytes());
                double_idx_in_data += 1;
            }
        }

        Ok(buf)
    }

    /// Serialize and write atomically (write to `${path}.tmp`, then rename).
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), SpkWriterError> {
        let bytes = self.to_bytes()?;
        let target = path.as_ref();
        let tmp = target.with_extension("tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, target)?;
        Ok(())
    }
}

struct SegmentMeta {
    start_et: f64,
    end_et: f64,
    target: i32,
    center: i32,
    frame_id: i32,
    data_type: i32,
    start_addr: i32,
    end_addr: i32,
    name: String,
}

struct SegmentMetaStub {
    start_et: f64,
    end_et: f64,
    target: i32,
    center: i32,
    frame_id: i32,
    data_type: i32,
    name: String,
}

fn validate_segment_id(id: &str) -> Result<(), SpkWriterError> {
    if id.len() > 40 {
        return Err(SpkWriterError::SegmentIdTooLong(id.to_string()));
    }
    Ok(())
}

fn encode_segment(seg: &Segment) -> Result<(SegmentMetaStub, Vec<f64>), SpkWriterError> {
    match seg {
        Segment::Type3(s) => Ok((
            SegmentMetaStub {
                start_et: s.start_et,
                end_et: s.end_et,
                target: s.target,
                center: s.center,
                frame_id: s.frame_id,
                data_type: 3,
                name: s.segment_id.clone(),
            },
            encode_type3(s),
        )),
        Segment::Type9(s) => Ok((
            SegmentMetaStub {
                start_et: s.start_et,
                end_et: s.end_et,
                target: s.target,
                center: s.center,
                frame_id: s.frame_id,
                data_type: 9,
                name: s.segment_id.clone(),
            },
            encode_type9(s),
        )),
    }
}

fn encode_type3(seg: &Type3Segment) -> Vec<f64> {
    let n_coef = seg.records[0].x.len();
    let rsize = 2 + 6 * n_coef;
    let n_records = seg.records.len();
    let mut out = Vec::with_capacity(rsize * n_records + 4);
    for r in &seg.records {
        out.push(r.mid);
        out.push(r.radius);
        out.extend_from_slice(&r.x);
        out.extend_from_slice(&r.y);
        out.extend_from_slice(&r.z);
        out.extend_from_slice(&r.vx);
        out.extend_from_slice(&r.vy);
        out.extend_from_slice(&r.vz);
    }
    // Trailer: INIT, INTLEN, RSIZE, N
    out.push(seg.init);
    out.push(seg.intlen);
    out.push(rsize as f64);
    out.push(n_records as f64);
    out
}

fn encode_type9(seg: &Type9Segment) -> Vec<f64> {
    let n = seg.epochs.len();
    let n_dir = (n - 1) / 100;
    let mut out = Vec::with_capacity(6 * n + n + n_dir + 2);
    // States (interleaved).
    out.extend_from_slice(&seg.states);
    // Epochs.
    out.extend_from_slice(&seg.epochs);
    // Directory: every 100th epoch (1-indexed: epochs[99], epochs[199], ...).
    for k in 1..=n_dir {
        out.push(seg.epochs[k * 100 - 1]);
    }
    // Trailer: degree, N.
    out.push(seg.degree as f64);
    out.push(n as f64);
    out
}

// This fn writes the DAF file-record byte layout one field at a time; the
// eight arguments are the header fields themselves. Bundling them into a
// struct would only rename the bag without hiding anything, so the lint
// is suppressed here.
#[allow(clippy::too_many_arguments)]
fn write_file_record(
    rec: &mut [u8],
    idword: &[u8; 8],
    nd: u32,
    ni: u32,
    locifn: &[u8; 60],
    fward: u32,
    bward: u32,
    free: u32,
) {
    assert_eq!(rec.len(), RECORD_BYTES);
    // Zero-fill first (the writer's buf starts zeroed, but be explicit).
    for b in rec.iter_mut() {
        *b = 0;
    }
    rec[0..8].copy_from_slice(idword);
    rec[8..12].copy_from_slice(&nd.to_le_bytes());
    rec[12..16].copy_from_slice(&ni.to_le_bytes());
    rec[16..76].copy_from_slice(locifn);
    rec[76..80].copy_from_slice(&fward.to_le_bytes());
    rec[80..84].copy_from_slice(&bward.to_le_bytes());
    rec[84..88].copy_from_slice(&free.to_le_bytes());
    rec[88..96].copy_from_slice(b"LTL-IEEE");
    // FTPSTR marker at offset 500, 28 bytes — CSPICE uses this to detect
    // file-transfer corruption (binary-unsafe FTP). Not strictly required
    // for load, but standard-compliant.
    let ftpstr: &[u8] = b"FTPSTR:\r:\n:\r\n:\r\x00:\x81:\x10\xce:ENDFTP";
    rec[500..500 + ftpstr.len()].copy_from_slice(ftpstr);
}

fn write_summary_record(rec: &mut [u8], segments: &[SegmentMeta]) {
    assert_eq!(rec.len(), RECORD_BYTES);
    // Header: NEXT=0, PREV=0, NSUM
    rec[0..8].copy_from_slice(&(0.0_f64).to_le_bytes());
    rec[8..16].copy_from_slice(&(0.0_f64).to_le_bytes());
    rec[16..24].copy_from_slice(&(segments.len() as f64).to_le_bytes());
    for (i, s) in segments.iter().enumerate() {
        let off = 24 + i * SPK_SUMMARY_BYTES;
        // Doubles: start_et, end_et.
        rec[off..off + 8].copy_from_slice(&s.start_et.to_le_bytes());
        rec[off + 8..off + 16].copy_from_slice(&s.end_et.to_le_bytes());
        // Integers: 6 × i32, packed into the trailing 3 doubles.
        let int_off = off + 16;
        rec[int_off..int_off + 4].copy_from_slice(&s.target.to_le_bytes());
        rec[int_off + 4..int_off + 8].copy_from_slice(&s.center.to_le_bytes());
        rec[int_off + 8..int_off + 12].copy_from_slice(&s.frame_id.to_le_bytes());
        rec[int_off + 12..int_off + 16].copy_from_slice(&s.data_type.to_le_bytes());
        rec[int_off + 16..int_off + 20].copy_from_slice(&s.start_addr.to_le_bytes());
        rec[int_off + 20..int_off + 24].copy_from_slice(&s.end_addr.to_le_bytes());
        // NI=6 is even, so no half-filled trailing i32 to pad.
    }
}

fn write_name_record(rec: &mut [u8], segments: &[SegmentMeta]) {
    assert_eq!(rec.len(), RECORD_BYTES);
    for b in rec.iter_mut() {
        *b = b' ';
    }
    for (i, s) in segments.iter().enumerate() {
        let off = i * SPK_SUMMARY_BYTES;
        let name_bytes = s.name.as_bytes();
        let n = name_bytes.len().min(SPK_SUMMARY_BYTES);
        rec[off..off + n].copy_from_slice(&name_bytes[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daf::DafFile;
    use crate::spk::SpkFile;
    use tempfile::NamedTempFile;

    fn tmp_path() -> NamedTempFile {
        NamedTempFile::new().expect("create tempfile")
    }

    #[test]
    fn type3_roundtrip_via_daf_reader() {
        let mut w = SpkWriter::new_spk("unit-test");
        let segment = Type3Segment {
            target: 1_000_000,
            center: 0,
            frame_id: 1, // J2000
            start_et: 0.0,
            end_et: 100.0,
            segment_id: "type3-roundtrip".to_string(),
            init: 0.0,
            intlen: 50.0,
            // Use constant-only Chebyshev coefficients so evaluation at
            // s=0 reduces to c[0]*T_0(0) = c[0]. Keeps the round-trip
            // assertion simple without faking Chebyshev math.
            records: vec![
                Type3Record {
                    mid: 25.0,
                    radius: 25.0,
                    x: vec![1.0, 0.0, 0.0],
                    y: vec![2.0, 0.0, 0.0],
                    z: vec![3.0, 0.0, 0.0],
                    vx: vec![0.001, 0.0, 0.0],
                    vy: vec![0.002, 0.0, 0.0],
                    vz: vec![0.003, 0.0, 0.0],
                },
                Type3Record {
                    mid: 75.0,
                    radius: 25.0,
                    x: vec![1.1, 0.0, 0.0],
                    y: vec![2.1, 0.0, 0.0],
                    z: vec![3.1, 0.0, 0.0],
                    vx: vec![0.0011, 0.0, 0.0],
                    vy: vec![0.0022, 0.0, 0.0],
                    vz: vec![0.0033, 0.0, 0.0],
                },
            ],
        };
        w.add_type3(segment.clone()).unwrap();
        let f = tmp_path();
        w.write(f.path()).unwrap();

        // DAF-level check: summary fields match what we wrote.
        let daf = DafFile::open(f.path()).unwrap();
        let summaries = daf.summaries().unwrap();
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        assert_eq!(s.doubles.len(), 2);
        assert_eq!(s.doubles[0], 0.0);
        assert_eq!(s.doubles[1], 100.0);
        assert_eq!(s.integers[0], 1_000_000);
        assert_eq!(s.integers[1], 0);
        assert_eq!(s.integers[2], 1);
        assert_eq!(s.integers[3], 3);
        assert!(s.name.starts_with("type3-roundtrip"));
        // Trailer at end of segment: [INIT, INTLEN, RSIZE, N].
        let end_addr = s.integers[5] as u32;
        let trailer = daf.read_doubles(end_addr - 3, end_addr).unwrap();
        assert_eq!(trailer[0], 0.0);
        assert_eq!(trailer[1], 50.0);
        assert_eq!(trailer[2], (2 + 6 * 3) as f64);
        assert_eq!(trailer[3], 2.0);

        // SPK-level check: reader parses and evaluates cleanly.
        let spk = SpkFile::open(f.path()).unwrap();
        assert_eq!(spk.segments().len(), 1);
        let seg = &spk.segments()[0];
        assert_eq!(seg.data_type, 3);
        // Evaluate at mid of first record (et=25, s=0 → value = c[0]).
        let st = spk.state(1_000_000, 0, 25.0).unwrap();
        assert!((st[0] - 1.0).abs() < 1e-14);
        assert!((st[1] - 2.0).abs() < 1e-14);
        assert!((st[2] - 3.0).abs() < 1e-14);
        assert!((st[3] - 0.001).abs() < 1e-16);
        assert!((st[4] - 0.002).abs() < 1e-16);
        assert!((st[5] - 0.003).abs() < 1e-16);
    }

    #[test]
    fn type9_roundtrip_via_daf_reader() {
        let n: usize = 20;
        let mut epochs = Vec::with_capacity(n);
        let mut states = Vec::with_capacity(6 * n);
        for i in 0..n {
            let t = i as f64 * 10.0;
            epochs.push(t);
            // Linear motion in each axis: easier to cross-check Lagrange
            // interpolation since any degree ≥ 1 reproduces exactly.
            states.extend_from_slice(&[
                1.0 + 0.5 * t,  // x
                -2.0 + 0.1 * t, // y
                0.5 - 0.2 * t,  // z
                0.5,            // vx
                0.1,            // vy
                -0.2,           // vz
            ]);
        }
        let mut w = SpkWriter::new_spk("type9-test");
        w.add_type9(Type9Segment {
            target: -1,
            center: 0,
            frame_id: 1,
            start_et: epochs[0],
            end_et: *epochs.last().unwrap(),
            segment_id: "type9-linear".to_string(),
            degree: 3,
            states,
            epochs,
        })
        .unwrap();
        let f = tmp_path();
        w.write(f.path()).unwrap();

        let spk = SpkFile::open(f.path()).unwrap();
        let seg = &spk.segments()[0];
        assert_eq!(seg.data_type, 9);
        // Sample in the interior — linear in all components, so
        // Lagrange of degree ≥ 1 is exact.
        let st = spk.state(-1, 0, 55.0).unwrap();
        assert!((st[0] - (1.0 + 0.5 * 55.0)).abs() < 1e-12);
        assert!((st[1] - (-2.0 + 0.1 * 55.0)).abs() < 1e-12);
        assert!((st[2] - (0.5 - 0.2 * 55.0)).abs() < 1e-12);
        assert!((st[3] - 0.5).abs() < 1e-14);
        assert!((st[4] - 0.1).abs() < 1e-14);
        assert!((st[5] + 0.2).abs() < 1e-14);
    }

    /// Regression test for the MRU segment cache: when two segments
    /// for the same `(target, center)` have overlapping ET coverage,
    /// queries in any order must return the same answer as a fresh
    /// (cache-cold) reader. Prior to the `cacheable` flag, querying
    /// the second segment's exclusive range would cache it, then a
    /// query in the overlap region would incorrectly return the
    /// second segment's value instead of the first-loaded one that
    /// `try_direct` deterministically picks.
    #[test]
    fn overlapping_segments_do_not_corrupt_mru_cache() {
        // Segment A covers ET [0, 9], encodes x = 100 + t.
        // Segment B covers ET [5, 14], encodes x = 200 + t.
        // Overlap region: ET [5, 9]. First-loaded (A) deterministically
        // wins in `try_direct`, so any query in the overlap must
        // return A's value (~ 100 + et) regardless of call history.
        fn linear_seg(target: i32, start: f64, end: f64, x_offset: f64) -> Type9Segment {
            let n = (end as usize) - (start as usize) + 1;
            let epochs: Vec<f64> = (0..n).map(|i| start + i as f64).collect();
            let states: Vec<f64> = epochs
                .iter()
                .flat_map(|&t| [x_offset + t, 0.0, 0.0, 1.0, 0.0, 0.0].into_iter())
                .collect();
            Type9Segment {
                target,
                center: 0,
                frame_id: 1,
                start_et: start,
                end_et: end,
                segment_id: format!("seg_{}_{}", target, x_offset as i64),
                degree: 1,
                states,
                epochs,
            }
        }

        let mut w = SpkWriter::new_spk("overlap-mru");
        w.add_type9(linear_seg(42, 0.0, 9.0, 100.0)).unwrap(); // A loaded first
        w.add_type9(linear_seg(42, 5.0, 14.0, 200.0)).unwrap(); // B loaded second
        let f = tmp_path();
        w.write(f.path()).unwrap();

        // Query order 1: cache cold, hit overlap directly.
        let spk = SpkFile::open(f.path()).unwrap();
        let cold = spk.state(42, 0, 7.0).unwrap();
        assert!(
            (cold[0] - 107.0).abs() < 1e-12,
            "cold overlap query returned {} (expected ~107 from segment A)",
            cold[0]
        );

        // Query order 2: visit B's exclusive range first (would have
        // populated MRU cache with B's index pre-fix), then overlap.
        // Both queries must remain consistent with a cold reader.
        let spk = SpkFile::open(f.path()).unwrap();
        let only_b = spk.state(42, 0, 12.0).unwrap();
        assert!(
            (only_b[0] - 212.0).abs() < 1e-12,
            "B-exclusive query returned {} (expected ~212)",
            only_b[0]
        );
        let after_b = spk.state(42, 0, 7.0).unwrap();
        assert!(
            (after_b[0] - 107.0).abs() < 1e-12,
            "overlap-after-B returned {} (expected ~107 from A; cache leaked B)",
            after_b[0]
        );

        // Query order 3: visit overlap, then B-exclusive, then
        // overlap again. All overlap queries must return A.
        let spk = SpkFile::open(f.path()).unwrap();
        let r1 = spk.state(42, 0, 7.0).unwrap();
        let r2 = spk.state(42, 0, 12.0).unwrap();
        let r3 = spk.state(42, 0, 7.0).unwrap();
        assert!((r1[0] - 107.0).abs() < 1e-12);
        assert!((r2[0] - 212.0).abs() < 1e-12);
        assert!((r3[0] - 107.0).abs() < 1e-12);
        // And the two overlap queries must be bit-equal regardless of
        // intervening B query.
        assert_eq!(r1[0].to_bits(), r3[0].to_bits());
    }

    #[test]
    fn multiple_segments_in_one_file() {
        let mut w = SpkWriter::new_spk("multi");
        let t3 = Type3Segment {
            target: 100,
            center: 0,
            frame_id: 1,
            start_et: 0.0,
            end_et: 10.0,
            segment_id: "t3a".to_string(),
            init: 0.0,
            intlen: 10.0,
            records: vec![Type3Record {
                mid: 5.0,
                radius: 5.0,
                x: vec![7.0, 0.0],
                y: vec![8.0, 0.0],
                z: vec![9.0, 0.0],
                vx: vec![0.0, 0.0],
                vy: vec![0.0, 0.0],
                vz: vec![0.0, 0.0],
            }],
        };
        w.add_type3(t3).unwrap();

        let epochs: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let states: Vec<f64> = (0..10)
            .flat_map(|i| {
                let t = i as f64;
                [10.0 + t, 20.0, 30.0, 1.0, 0.0, 0.0].into_iter()
            })
            .collect();
        w.add_type9(Type9Segment {
            target: 101,
            center: 0,
            frame_id: 1,
            start_et: 0.0,
            end_et: 9.0,
            segment_id: "t9a".to_string(),
            degree: 1,
            states,
            epochs,
        })
        .unwrap();

        let f = tmp_path();
        w.write(f.path()).unwrap();

        let spk = SpkFile::open(f.path()).unwrap();
        assert_eq!(spk.segments().len(), 2);
        let s1 = spk.state(100, 0, 5.0).unwrap();
        assert!((s1[0] - 7.0).abs() < 1e-14);
        let s2 = spk.state(101, 0, 5.5).unwrap();
        assert!((s2[0] - 15.5).abs() < 1e-12);
    }

    #[test]
    fn file_record_fields_validate() {
        let mut w = SpkWriter::new_spk("header-check");
        w.add_type3(Type3Segment {
            target: 1,
            center: 0,
            frame_id: 1,
            start_et: 0.0,
            end_et: 1.0,
            segment_id: "x".to_string(),
            init: 0.0,
            intlen: 1.0,
            records: vec![Type3Record {
                mid: 0.5,
                radius: 0.5,
                x: vec![0.0],
                y: vec![0.0],
                z: vec![0.0],
                vx: vec![0.0],
                vy: vec![0.0],
                vz: vec![0.0],
            }],
        })
        .unwrap();
        let bytes = w.to_bytes().unwrap();
        assert_eq!(&bytes[0..8], b"DAF/SPK ");
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 6);
        assert_eq!(&bytes[88..96], b"LTL-IEEE");
        // FWARD=2, BWARD=2.
        assert_eq!(u32::from_le_bytes(bytes[76..80].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 2);
    }

    #[test]
    fn rejects_too_many_segments() {
        let mut w = SpkWriter::new_spk("overflow");
        for i in 0..(SPK_SUMMARIES_PER_RECORD + 1) {
            w.add_type3(Type3Segment {
                target: i as i32,
                center: 0,
                frame_id: 1,
                start_et: 0.0,
                end_et: 1.0,
                segment_id: format!("s{i}"),
                init: 0.0,
                intlen: 1.0,
                records: vec![Type3Record {
                    mid: 0.5,
                    radius: 0.5,
                    x: vec![0.0],
                    y: vec![0.0],
                    z: vec![0.0],
                    vx: vec![0.0],
                    vy: vec![0.0],
                    vz: vec![0.0],
                }],
            })
            .unwrap();
        }
        let err = w.to_bytes().unwrap_err();
        matches!(err, SpkWriterError::TooManySegments { .. });
    }

    #[test]
    fn rejects_empty_type3() {
        let mut w = SpkWriter::new_spk("empty");
        let err = w
            .add_type3(Type3Segment {
                target: 1,
                center: 0,
                frame_id: 1,
                start_et: 0.0,
                end_et: 1.0,
                segment_id: "x".to_string(),
                init: 0.0,
                intlen: 1.0,
                records: vec![],
            })
            .unwrap_err();
        matches!(err, SpkWriterError::BadType3(_));
    }

    #[test]
    fn rejects_non_monotone_type9_epochs() {
        let mut w = SpkWriter::new_spk("non-mono");
        let err = w
            .add_type9(Type9Segment {
                target: -1,
                center: 0,
                frame_id: 1,
                start_et: 0.0,
                end_et: 1.0,
                segment_id: "x".to_string(),
                degree: 1,
                states: vec![0.0; 6 * 3],
                epochs: vec![0.0, 1.0, 0.5],
            })
            .unwrap_err();
        matches!(err, SpkWriterError::BadType9(_));
    }

    #[test]
    fn rejects_segment_id_over_40_bytes() {
        let mut w = SpkWriter::new_spk("long-id");
        let long = "x".repeat(41);
        let err = w
            .add_type3(Type3Segment {
                target: 1,
                center: 0,
                frame_id: 1,
                start_et: 0.0,
                end_et: 1.0,
                segment_id: long,
                init: 0.0,
                intlen: 1.0,
                records: vec![Type3Record {
                    mid: 0.5,
                    radius: 0.5,
                    x: vec![0.0],
                    y: vec![0.0],
                    z: vec![0.0],
                    vx: vec![0.0],
                    vy: vec![0.0],
                    vz: vec![0.0],
                }],
            })
            .unwrap_err();
        matches!(err, SpkWriterError::SegmentIdTooLong(_));
    }

    #[test]
    fn type9_directory_present_when_n_over_100() {
        // With N=150, directory should have floor(149/100) = 1 entry
        // containing epochs[99].
        let n = 150;
        let epochs: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let states: Vec<f64> = (0..n)
            .flat_map(|_| [0.0_f64, 0.0, 0.0, 0.0, 0.0, 0.0].into_iter())
            .collect();
        let mut w = SpkWriter::new_spk("n150");
        w.add_type9(Type9Segment {
            target: -1,
            center: 0,
            frame_id: 1,
            start_et: 0.0,
            end_et: (n - 1) as f64,
            segment_id: "n150".to_string(),
            degree: 3,
            states,
            epochs,
        })
        .unwrap();
        let f = tmp_path();
        w.write(f.path()).unwrap();
        let daf = DafFile::open(f.path()).unwrap();
        let s = &daf.summaries().unwrap()[0];
        let end_addr = s.integers[5] as u32;
        let start_addr = s.integers[4] as u32;
        // Layout: states (6*N) + epochs (N) + directory (1) + trailer (2).
        let expected_len = 6 * n + n + 1 + 2;
        assert_eq!((end_addr - start_addr + 1) as usize, expected_len);
        // Directory entry should be epochs[99] = 99.0.
        let dir_addr = start_addr + (6 * n + n) as u32;
        let dir = daf.read_doubles(dir_addr, dir_addr).unwrap();
        assert_eq!(dir[0], 99.0);
    }
}
