//! Binary PCK (Planetary Constants Kernel) reader.
//!
//! Adam-core only needs time-varying planetary body orientations for the
//! Earth-associated `ITRF93` frame. Binary PCK shares the DAF container
//! with SPK, but the summary layout differs (NI=5 instead of 6, with
//! `center` absent: segments describe a single body-fixed frame wrt a
//! reference inertial frame). The payload is the standard Type 2
//! Chebyshev record, but with 3 Euler-angle channels rather than 3
//! position channels. The angles follow the NAIF 3-1-3 convention:
//! `(t1, t2, t3)` such that `R_ref→body = Rz(t3) · Rx(t2) · Rz(t1)`.
//! The `ref` frame is the segment's reference inertial frame — the
//! standard Earth PCKs use `ECLIPJ2000` (NAIF frame ID 17), so
//! consumers must compose with the J2000 ↔ ECLIPJ2000 static rotation
//! when producing `sxform("J2000","ITRF93",et)`.
//!
//! The evaluation output is `[t1, t2, t3, dt1, dt2, dt3]` at a
//! requested TDB ET. Frame consumers assemble a 3x3 rotation (and its
//! time derivative) from these angles via
//! [`crate::frame::pck_euler_rotation_and_derivative`].
//!
//! Kernel precedence: three PCKs (`predict`, `historical`, `high_prec`)
//! may cover overlapping epoch ranges. Last-loaded-wins matches
//! CSPICE `furnsh` semantics: the caller is expected to load kernels in
//! precedence order (least- to most-precise) and the file they opened
//! most recently "wins" for overlapping coverage. This crate offers a
//! per-file reader; multi-file composition is handled at the Python
//! layer with a simple list-of-readers dispatcher.

use std::path::Path;

use thiserror::Error;

use crate::daf::{DafError, DafFile};
use crate::spk::cheby3_val_and_deriv;

#[derive(Debug, Error)]
pub enum PckError {
    #[error(transparent)]
    Daf(#[from] DafError),
    #[error("no segment covers body {body} in frame {ref_frame} at et {et}")]
    NoCoverage { body: i32, ref_frame: i32, et: f64 },
    #[error("unsupported PCK data type {0}")]
    UnsupportedType(i32),
    #[error("malformed PCK Type 2 segment: {0}")]
    BadType2(&'static str),
}

#[derive(Clone)]
pub struct PckSegment {
    /// Body-fixed frame ID described by this segment (e.g. 3000 for ITRF93).
    pub body_frame: i32,
    /// Inertial reference frame ID (always 1 = J2000 in practice).
    pub ref_frame: i32,
    pub data_type: i32,
    pub start_et: f64,
    pub end_et: f64,
    pub name: String,
    payload: PckPayload,
}

#[derive(Clone)]
enum PckPayload {
    Type2(PckType2),
    Unsupported,
}

/// PCK Type 2: 3-channel Chebyshev (RA, DEC, W) with analytic derivative.
///
/// Record layout mirrors SPK Type 2 but the three angle channels
/// replace position (x, y, z). Trailer `[INIT, INTLEN, RSIZE, N]` is
/// identical.
#[derive(Clone)]
struct PckType2 {
    file: DafFile,
    init: f64,
    intlen: f64,
    rsize: usize,
    n_records: usize,
    n_coef: usize,
    start_addr: u32,
}

impl PckType2 {
    fn from_segment(file: &DafFile, start_addr: u32, end_addr: u32) -> Result<Self, PckError> {
        let trailer = file.read_doubles(end_addr - 3, end_addr)?;
        let init = trailer[0];
        let intlen = trailer[1];
        let rsize = trailer[2] as usize;
        let n_records = trailer[3] as usize;
        if rsize < 2 || (rsize - 2) % 3 != 0 {
            return Err(PckError::BadType2("RSIZE not 2 + 3N"));
        }
        let n_coef = (rsize - 2) / 3;
        if n_coef == 0 || intlen <= 0.0 {
            return Err(PckError::BadType2("degree<0 or INTLEN<=0"));
        }
        Ok(PckType2 {
            file: file.clone(),
            init,
            intlen,
            rsize,
            n_records,
            n_coef,
            start_addr,
        })
    }

    fn evaluate(&self, et: f64) -> Result<[f64; 6], PckError> {
        let raw_idx = ((et - self.init) / self.intlen).floor() as isize;
        let idx = raw_idx.clamp(0, self.n_records as isize - 1) as usize;
        let rec_start = self.start_addr + (idx * self.rsize) as u32;
        let rec_end = rec_start + self.rsize as u32 - 1;
        let rec = self.file.doubles_native(rec_start, rec_end)?;

        let mid = rec[0];
        let radius = rec[1];
        if radius == 0.0 {
            return Err(PckError::BadType2("RADIUS == 0"));
        }
        let s = (et - mid) / radius;

        let n = self.n_coef;
        let ra_c = &rec[2..2 + n];
        let dec_c = &rec[2 + n..2 + 2 * n];
        let w_c = &rec[2 + 2 * n..2 + 3 * n];

        // Three-channel evaluation with shared T_k(s) / dT_k/ds basis
        // recurrence — mathematically equivalent to three independent
        // calls but ~3x cheaper on the recurrence half of the work.
        let (ang, dang) = cheby3_val_and_deriv(ra_c, dec_c, w_c, s);
        let inv_r = 1.0 / radius;
        Ok([
            ang[0],
            ang[1],
            ang[2],
            dang[0] * inv_r,
            dang[1] * inv_r,
            dang[2] * inv_r,
        ])
    }
}

pub struct PckFile {
    segments: Vec<PckSegment>,
}

impl PckFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PckError> {
        let daf = DafFile::open(path)?;
        Self::from_daf(daf)
    }

    pub fn from_daf(daf: DafFile) -> Result<Self, PckError> {
        let mut segments = Vec::new();
        for summary in daf.summaries()? {
            if summary.doubles.len() < 2 || summary.integers.len() < 5 {
                continue;
            }
            let start_et = summary.doubles[0];
            let end_et = summary.doubles[1];
            let body_frame = summary.integers[0];
            let ref_frame = summary.integers[1];
            let data_type = summary.integers[2];
            let start_addr = summary.integers[3] as u32;
            let end_addr = summary.integers[4] as u32;

            let payload = match data_type {
                2 => PckPayload::Type2(PckType2::from_segment(&daf, start_addr, end_addr)?),
                _ => PckPayload::Unsupported,
            };

            segments.push(PckSegment {
                body_frame,
                ref_frame,
                data_type,
                start_et,
                end_et,
                name: summary.name,
                payload,
            });
        }
        Ok(PckFile { segments })
    }

    pub fn segments(&self) -> &[PckSegment] {
        &self.segments
    }

    /// Raw evaluation: returns `[RA, DEC, W, dRA/dt, dDEC/dt, dW/dt]` at
    /// ET for the requested body-fixed frame wrt its reference frame.
    /// Returns `NoCoverage` if no segment covers the epoch.
    pub fn euler_state(&self, body_frame: i32, et: f64) -> Result<[f64; 6], PckError> {
        Ok(self.euler_state_with_ref(body_frame, et)?.1)
    }

    /// Raw evaluation returning `(ref_frame_id, [RA, DEC, W, dRA, dDEC, dW])`.
    /// The reference frame is usually 1 (J2000) or 17 (ECLIPJ2000); the
    /// caller composes with the inter-inertial rotation to land in a
    /// chosen inertial frame.
    pub fn euler_state_with_ref(
        &self,
        body_frame: i32,
        et: f64,
    ) -> Result<(i32, [f64; 6]), PckError> {
        // Scan newest-first so late segments (typically higher precision)
        // win when segments overlap within a single file.
        for seg in self.segments.iter().rev() {
            if seg.body_frame != body_frame {
                continue;
            }
            if et < seg.start_et || et > seg.end_et {
                continue;
            }
            let state = match &seg.payload {
                PckPayload::Type2(t) => t.evaluate(et)?,
                PckPayload::Unsupported => return Err(PckError::UnsupportedType(seg.data_type)),
            };
            return Ok((seg.ref_frame, state));
        }
        Err(PckError::NoCoverage {
            body: body_frame,
            ref_frame: 1,
            et,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daf::{DOUBLE_BYTES, RECORD_BYTES};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// ITRF93 NAIF frame ID (what Earth PCKs actually record).
    const ITRF93_FRAME: i32 = 3000;
    /// ECLIPJ2000 frame ID (the reference frame the standard Earth PCKs use).
    const ECLIPJ2000_FRAME: i32 = 17;

    /// Build a PCK file with one Type 2 segment. `coefs_ra`, `coefs_dec`,
    /// and `coefs_w` must all have the same length (= n_coef) and will
    /// appear in a single data record spanning `[mid-radius, mid+radius]`.
    // Test helper that lays out a PCK byte-for-byte; each argument is a
    // distinct file field, so a struct would add noise without clarity.
    #[allow(clippy::too_many_arguments)]
    fn build_single_segment_pck(
        body_frame: i32,
        ref_frame: i32,
        start_et: f64,
        end_et: f64,
        mid: f64,
        radius: f64,
        coefs_ra: &[f64],
        coefs_dec: &[f64],
        coefs_w: &[f64],
    ) -> NamedTempFile {
        assert_eq!(coefs_ra.len(), coefs_dec.len());
        assert_eq!(coefs_ra.len(), coefs_w.len());
        let n_coef = coefs_ra.len();
        let rsize = 2 + 3 * n_coef; // [mid, radius, RA..., DEC..., W...]
        let n_records = 1;

        // One data record + 4-double trailer [INIT, INTLEN, RSIZE, N].
        let mut data: Vec<f64> = Vec::with_capacity(rsize + 4);
        data.push(mid);
        data.push(radius);
        data.extend_from_slice(coefs_ra);
        data.extend_from_slice(coefs_dec);
        data.extend_from_slice(coefs_w);
        // trailer
        data.push(start_et); // INIT
        data.push(end_et - start_et); // INTLEN = full coverage for 1 record
        data.push(rsize as f64);
        data.push(n_records as f64);

        // Data starts at record 4 in our layout.
        let data_start_record = 4u32;
        let doubles_per_record = (RECORD_BYTES / DOUBLE_BYTES) as u32;
        let data_start_addr = 1 + (data_start_record - 1) * doubles_per_record;
        let data_end_addr = data_start_addr + data.len() as u32 - 1;

        // Build file: record 1 (file record), record 2 (summary), record 3 (names),
        // record 4 (data).
        let mut record1 = vec![0u8; RECORD_BYTES];
        record1[0..8].copy_from_slice(b"DAF/PCK ");
        record1[8..12].copy_from_slice(&2u32.to_le_bytes()); // nd
        record1[12..16].copy_from_slice(&5u32.to_le_bytes()); // ni
        for b in &mut record1[16..76] {
            *b = b' ';
        }
        record1[76..80].copy_from_slice(&2u32.to_le_bytes()); // fward
        record1[80..84].copy_from_slice(&2u32.to_le_bytes()); // bward
        record1[84..88].copy_from_slice(&0u32.to_le_bytes()); // free
        record1[88..96].copy_from_slice(b"LTL-IEEE");

        let mut record2 = vec![0u8; RECORD_BYTES];
        record2[0..8].copy_from_slice(&0.0f64.to_le_bytes()); // NEXT
        record2[8..16].copy_from_slice(&0.0f64.to_le_bytes()); // PREV
        record2[16..24].copy_from_slice(&1.0f64.to_le_bytes()); // NSUM

        // nd=2 leading doubles (start_et, end_et), ni=5 integers
        // (body_frame, ref_frame, dtype, start_addr, end_addr).
        let soff = 24;
        record2[soff..soff + 8].copy_from_slice(&start_et.to_le_bytes());
        record2[soff + 8..soff + 16].copy_from_slice(&end_et.to_le_bytes());
        let int_start = soff + 2 * DOUBLE_BYTES;
        record2[int_start..int_start + 4].copy_from_slice(&body_frame.to_le_bytes());
        record2[int_start + 4..int_start + 8].copy_from_slice(&ref_frame.to_le_bytes());
        record2[int_start + 8..int_start + 12].copy_from_slice(&2i32.to_le_bytes()); // dtype=2
        record2[int_start + 12..int_start + 16]
            .copy_from_slice(&(data_start_addr as i32).to_le_bytes());
        record2[int_start + 16..int_start + 20]
            .copy_from_slice(&(data_end_addr as i32).to_le_bytes());

        let mut record3 = vec![b' '; RECORD_BYTES];
        let name = b"SYNTHETIC TEST SEGMENT";
        record3[..name.len()].copy_from_slice(name);

        let mut data_bytes = Vec::new();
        for d in &data {
            data_bytes.extend_from_slice(&d.to_le_bytes());
        }
        while data_bytes.len() % RECORD_BYTES != 0 {
            data_bytes.push(0);
        }

        let mut all = Vec::new();
        all.extend_from_slice(&record1);
        all.extend_from_slice(&record2);
        all.extend_from_slice(&record3);
        all.extend_from_slice(&data_bytes);

        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&all).unwrap();
        tmp.flush().unwrap();
        tmp
    }

    #[test]
    fn from_daf_recovers_segment_metadata() {
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -3e8,
            3e8,
            0.0,
            3e8,
            &[0.1, 0.0],
            &[0.2, 0.0],
            &[0.3, 0.5],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        let segs = pck.segments();
        assert_eq!(segs.len(), 1);
        let seg = &segs[0];
        assert_eq!(seg.body_frame, ITRF93_FRAME);
        assert_eq!(seg.ref_frame, ECLIPJ2000_FRAME);
        assert_eq!(seg.data_type, 2);
        assert_eq!(seg.start_et, -3e8);
        assert_eq!(seg.end_et, 3e8);
        assert_eq!(seg.name, "SYNTHETIC TEST SEGMENT");
    }

    #[test]
    fn euler_state_at_record_midpoint_returns_degree_zero_coefs() {
        // At et = mid, s = 0, so Chebyshev T0(0)=1, T1(0)=0, T2(0)=-1, ...
        // With coefs [c0, c1] => value = c0 - 0*c1 = c0.
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -50.0,
            50.0,
            0.0,
            50.0,
            &[1.25, 0.0],
            &[2.5, 0.0],
            &[3.75, 0.5],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        let state = pck.euler_state(ITRF93_FRAME, 0.0).expect("eval at mid");
        assert!((state[0] - 1.25).abs() < 1e-15);
        assert!((state[1] - 2.5).abs() < 1e-15);
        assert!((state[2] - 3.75).abs() < 1e-15);
        // dW = c1 * dT1/ds * 1/radius = 0.5 * 1 * 1/50 = 0.01
        assert!((state[3] - 0.0).abs() < 1e-15);
        assert!((state[4] - 0.0).abs() < 1e-15);
        assert!((state[5] - 0.01).abs() < 1e-15);
    }

    #[test]
    fn euler_state_with_ref_returns_reference_frame_id() {
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -10.0,
            10.0,
            0.0,
            10.0,
            &[0.0, 0.0],
            &[0.0, 0.0],
            &[0.0, 0.0],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        let (ref_id, _state) = pck.euler_state_with_ref(ITRF93_FRAME, 0.0).expect("eval");
        assert_eq!(ref_id, ECLIPJ2000_FRAME);
    }

    #[test]
    fn derivative_matches_central_finite_difference() {
        // Use non-trivial coefficients; verify dW/dt against symmetric FD.
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -1000.0,
            1000.0,
            0.0,
            1000.0,
            &[0.1, -0.05, 0.02, -0.003],
            &[0.2, 0.04, -0.01, 0.005],
            &[0.3, 0.08, 0.015, -0.004],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        for &et in &[-500.0_f64, -100.0, 0.0, 250.0, 750.0] {
            let h = 1e-3_f64;
            let [_, _, _, dra, ddec, dw] = pck.euler_state(ITRF93_FRAME, et).unwrap();
            let [rap, decp, wp, ..] = pck.euler_state(ITRF93_FRAME, et + h).unwrap();
            let [ram, decm, wm, ..] = pck.euler_state(ITRF93_FRAME, et - h).unwrap();
            let dra_fd = (rap - ram) / (2.0 * h);
            let ddec_fd = (decp - decm) / (2.0 * h);
            let dw_fd = (wp - wm) / (2.0 * h);
            assert!(
                (dra - dra_fd).abs() < 1e-9,
                "dra et={et}: {dra} vs {dra_fd}"
            );
            assert!(
                (ddec - ddec_fd).abs() < 1e-9,
                "ddec et={et}: {ddec} vs {ddec_fd}"
            );
            assert!((dw - dw_fd).abs() < 1e-9, "dw et={et}: {dw} vs {dw_fd}");
        }
    }

    #[test]
    fn no_coverage_when_body_frame_not_present() {
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -10.0,
            10.0,
            0.0,
            10.0,
            &[0.0, 0.0],
            &[0.0, 0.0],
            &[0.0, 0.0],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        let err = pck.euler_state(9999, 0.0).unwrap_err();
        match err {
            PckError::NoCoverage { body, et, .. } => {
                assert_eq!(body, 9999);
                assert_eq!(et, 0.0);
            }
            other => panic!("expected NoCoverage, got {other:?}"),
        }
    }

    #[test]
    fn no_coverage_when_epoch_outside_segment_range() {
        // Segment covers [-10, 10]; query at et=50 must miss.
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -10.0,
            10.0,
            0.0,
            10.0,
            &[0.0, 0.0],
            &[0.0, 0.0],
            &[0.0, 0.0],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        let err = pck.euler_state(ITRF93_FRAME, 50.0).unwrap_err();
        assert!(matches!(err, PckError::NoCoverage { .. }));
        let err_neg = pck.euler_state(ITRF93_FRAME, -100.0).unwrap_err();
        assert!(matches!(err_neg, PckError::NoCoverage { .. }));
    }

    #[test]
    fn segment_value_matches_chebyshev_by_hand() {
        // RA coefs [1, 2, 3] → f(s) = 1 + 2s + 3*(2s² - 1) = 6s² + 2s - 2.
        // At et=25, mid=0, radius=50 → s = 0.5 → f(0.5) = 6*0.25 + 1 - 2 = 0.5.
        // df/ds = 12s + 2 → 8 at s=0.5; df/dt = df/ds / radius = 0.16.
        let tmp = build_single_segment_pck(
            ITRF93_FRAME,
            ECLIPJ2000_FRAME,
            -50.0,
            50.0,
            0.0,
            50.0,
            &[1.0, 2.0, 3.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
        );
        let pck = PckFile::open(tmp.path()).expect("open");
        let state = pck.euler_state(ITRF93_FRAME, 25.0).expect("eval");
        assert!((state[0] - 0.5).abs() < 1e-14, "RA={}", state[0]);
        assert!((state[3] - 0.16).abs() < 1e-14, "dRA={}", state[3]);
    }

    /// Descriptor for one PCK segment in a multi-segment file.
    struct SegSpec {
        body_frame: i32,
        ref_frame: i32,
        start_et: f64,
        end_et: f64,
        mid: f64,
        radius: f64,
        ra: Vec<f64>,
        dec: Vec<f64>,
        w: Vec<f64>,
    }

    /// Build a DAF/PCK file containing multiple Type 2 segments. Each
    /// segment owns one data record packed back-to-back after the name
    /// record; summaries appear in a single summary record in the
    /// order supplied.
    fn build_multi_segment_pck(segs: &[SegSpec]) -> NamedTempFile {
        let ss_doubles = 2 + 5_usize.div_ceil(2); // nd=2, ni=5 → 5 doubles
        let summary_bytes = ss_doubles * DOUBLE_BYTES; // 40

        let doubles_per_record = (RECORD_BYTES / DOUBLE_BYTES) as u32;
        let mut cursor = 1 + 3 * doubles_per_record;
        let mut segment_payloads: Vec<(u32, u32, Vec<f64>)> = Vec::new();
        for s in segs {
            assert_eq!(s.ra.len(), s.dec.len());
            assert_eq!(s.ra.len(), s.w.len());
            let n_coef = s.ra.len();
            let rsize = 2 + 3 * n_coef;
            let mut data = Vec::with_capacity(rsize + 4);
            data.push(s.mid);
            data.push(s.radius);
            data.extend_from_slice(&s.ra);
            data.extend_from_slice(&s.dec);
            data.extend_from_slice(&s.w);
            data.push(s.start_et);
            data.push(s.end_et - s.start_et);
            data.push(rsize as f64);
            data.push(1.0);
            let start_addr = cursor;
            let end_addr = start_addr + data.len() as u32 - 1;
            cursor = end_addr + 1;
            segment_payloads.push((start_addr, end_addr, data));
        }

        let mut record1 = vec![0u8; RECORD_BYTES];
        record1[0..8].copy_from_slice(b"DAF/PCK ");
        record1[8..12].copy_from_slice(&2u32.to_le_bytes());
        record1[12..16].copy_from_slice(&5u32.to_le_bytes());
        for b in &mut record1[16..76] {
            *b = b' ';
        }
        record1[76..80].copy_from_slice(&2u32.to_le_bytes());
        record1[80..84].copy_from_slice(&2u32.to_le_bytes());
        record1[88..96].copy_from_slice(b"LTL-IEEE");

        let mut record2 = vec![0u8; RECORD_BYTES];
        record2[16..24].copy_from_slice(&(segs.len() as f64).to_le_bytes());
        for (i, s) in segs.iter().enumerate() {
            let (start_addr, end_addr, _) = &segment_payloads[i];
            let soff = 24 + i * summary_bytes;
            record2[soff..soff + 8].copy_from_slice(&s.start_et.to_le_bytes());
            record2[soff + 8..soff + 16].copy_from_slice(&s.end_et.to_le_bytes());
            let int_start = soff + 2 * DOUBLE_BYTES;
            record2[int_start..int_start + 4].copy_from_slice(&s.body_frame.to_le_bytes());
            record2[int_start + 4..int_start + 8].copy_from_slice(&s.ref_frame.to_le_bytes());
            record2[int_start + 8..int_start + 12].copy_from_slice(&2i32.to_le_bytes());
            record2[int_start + 12..int_start + 16]
                .copy_from_slice(&(*start_addr as i32).to_le_bytes());
            record2[int_start + 16..int_start + 20]
                .copy_from_slice(&(*end_addr as i32).to_le_bytes());
        }

        let record3 = vec![b' '; RECORD_BYTES];

        let mut data_bytes = Vec::new();
        for (_, _, data) in &segment_payloads {
            for d in data {
                data_bytes.extend_from_slice(&d.to_le_bytes());
            }
        }
        while data_bytes.len() % RECORD_BYTES != 0 {
            data_bytes.push(0);
        }

        let mut all = Vec::new();
        all.extend_from_slice(&record1);
        all.extend_from_slice(&record2);
        all.extend_from_slice(&record3);
        all.extend_from_slice(&data_bytes);

        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&all).unwrap();
        tmp.flush().unwrap();
        tmp
    }

    #[test]
    fn later_segment_wins_when_coverage_overlaps() {
        // Two segments for the same body frame with overlapping
        // coverage. The one added last (stored later in the file)
        // reports a different constant RA; the newest-first scan
        // means lookups in the overlap region return the later value.
        let earlier = SegSpec {
            body_frame: ITRF93_FRAME,
            ref_frame: ECLIPJ2000_FRAME,
            start_et: -100.0,
            end_et: 100.0,
            mid: 0.0,
            radius: 100.0,
            ra: vec![1.0, 0.0],
            dec: vec![0.0, 0.0],
            w: vec![0.0, 0.0],
        };
        let later = SegSpec {
            body_frame: ITRF93_FRAME,
            ref_frame: ECLIPJ2000_FRAME,
            start_et: -50.0,
            end_et: 50.0,
            mid: 0.0,
            radius: 50.0,
            ra: vec![2.0, 0.0],
            dec: vec![0.0, 0.0],
            w: vec![0.0, 0.0],
        };
        let tmp = build_multi_segment_pck(&[earlier, later]);
        let pck = PckFile::open(tmp.path()).expect("open");
        // In the overlap: later wins (RA=2.0).
        let overlap = pck.euler_state(ITRF93_FRAME, 0.0).expect("overlap");
        assert!(
            (overlap[0] - 2.0).abs() < 1e-14,
            "RA in overlap={}",
            overlap[0]
        );
        // Outside the later segment but inside the earlier: earlier wins (RA=1.0).
        let outside = pck.euler_state(ITRF93_FRAME, 75.0).expect("outside");
        assert!(
            (outside[0] - 1.0).abs() < 1e-14,
            "RA outside={}",
            outside[0]
        );
    }
}
