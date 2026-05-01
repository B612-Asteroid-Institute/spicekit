//! NAIF DAF (Double-precision Array File) container reader.
//!
//! All adam-core NAIF data — SPK, PCK, and CK — live inside DAF files. This
//! module parses the file record and summary/name record chain, yielding
//! typed `Summary` descriptors. Payload interpretation (e.g. SPK Type 2
//! Chebyshev coefficients) is the caller's responsibility.
//!
//! Reference: NAIF "DAF Required Reading" (daf.req).

use std::path::Path;
use std::sync::Arc;

use memmap2::Mmap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DafError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("file too small ({0} bytes) to contain a DAF header")]
    TooSmall(usize),
    #[error("unrecognized DAF identifier: {0:?}")]
    BadIdword([u8; 8]),
    #[error("unsupported binary format: {0:?} (only LTL-IEEE is supported)")]
    UnsupportedFormat([u8; 8]),
    #[error("malformed summary at record {record}: {reason}")]
    BadSummary { record: u32, reason: &'static str },
    #[error("address range [{start},{end}] out of file (file has {file_doubles} doubles)")]
    AddressOutOfBounds {
        start: u32,
        end: u32,
        file_doubles: u64,
    },
}

pub const RECORD_BYTES: usize = 1024;
pub const DOUBLE_BYTES: usize = 8;

/// A memory-mapped DAF file. Cheap to clone (`Arc`-backed).
#[derive(Clone)]
pub struct DafFile {
    inner: Arc<DafInner>,
}

struct DafInner {
    mmap: Mmap,
    pub idword: [u8; 8],
    pub nd: u32,
    pub ni: u32,
    pub fward: u32,
    // Backward record pointer from the DAF file record. Retained for
    // spec completeness; spicekit walks summary records forward from
    // `fward` and never needs it at read time.
    #[allow(dead_code)]
    pub bward: u32,
}

/// One summary plus its 40-character name, extracted from a summary/name
/// record pair.
#[derive(Debug, Clone)]
pub struct Summary {
    pub doubles: Vec<f64>,  // length == nd
    pub integers: Vec<i32>, // length == ni
    pub name: String,
}

impl DafFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DafError> {
        let file = std::fs::File::open(path)?;
        // SAFETY: the mapped file is read-only; adam-core does not modify
        // SPK/PCK kernels after they've been delivered to the NAIF package.
        let mmap = unsafe { Mmap::map(&file)? };
        Self::from_mmap(mmap)
    }

    fn from_mmap(mmap: Mmap) -> Result<Self, DafError> {
        if mmap.len() < RECORD_BYTES {
            return Err(DafError::TooSmall(mmap.len()));
        }
        let bytes = &mmap[..];

        let mut idword = [0u8; 8];
        idword.copy_from_slice(&bytes[0..8]);
        if !idword.starts_with(b"DAF/") {
            return Err(DafError::BadIdword(idword));
        }

        let mut locfmt = [0u8; 8];
        locfmt.copy_from_slice(&bytes[88..96]);
        if &locfmt != b"LTL-IEEE" {
            return Err(DafError::UnsupportedFormat(locfmt));
        }

        let nd = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let ni = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let fward = u32::from_le_bytes(bytes[76..80].try_into().unwrap());
        let bward = u32::from_le_bytes(bytes[80..84].try_into().unwrap());

        Ok(DafFile {
            inner: Arc::new(DafInner {
                mmap,
                idword,
                nd,
                ni,
                fward,
                bward,
            }),
        })
    }

    pub fn nd(&self) -> u32 {
        self.inner.nd
    }
    pub fn ni(&self) -> u32 {
        self.inner.ni
    }
    pub fn idword(&self) -> [u8; 8] {
        self.inner.idword
    }

    /// Number of doubles per summary = ND + (NI+1)/2.
    pub fn summary_size_doubles(&self) -> usize {
        self.inner.nd as usize + (self.inner.ni as usize).div_ceil(2)
    }

    /// Enumerate every (summary, name) pair across the summary-record chain.
    pub fn summaries(&self) -> Result<Vec<Summary>, DafError> {
        let mut out = Vec::new();
        let mut rec = self.inner.fward;
        while rec != 0 {
            self.read_summary_record(rec, &mut out)?;
            rec = self.next_record(rec)?;
        }
        Ok(out)
    }

    fn next_record(&self, rec: u32) -> Result<u32, DafError> {
        let bytes = self.record_bytes(rec)?;
        // NEXT is stored as f64, but holds an integer value.
        let next = f64::from_le_bytes(bytes[0..8].try_into().unwrap());
        Ok(next as u32)
    }

    fn record_bytes(&self, rec: u32) -> Result<&[u8], DafError> {
        let start = (rec as usize - 1) * RECORD_BYTES;
        let end = start + RECORD_BYTES;
        if end > self.inner.mmap.len() {
            return Err(DafError::BadSummary {
                record: rec,
                reason: "record extends past end of file",
            });
        }
        Ok(&self.inner.mmap[start..end])
    }

    fn read_summary_record(&self, rec: u32, out: &mut Vec<Summary>) -> Result<(), DafError> {
        let sbytes = self.record_bytes(rec)?;
        let name_rec = rec + 1;
        let nbytes = self.record_bytes(name_rec)?;

        let nsum_f = f64::from_le_bytes(sbytes[16..24].try_into().unwrap());
        let nsum = nsum_f as usize;
        let ss = self.summary_size_doubles();
        let nd = self.inner.nd as usize;
        let ni = self.inner.ni as usize;
        let name_chars = ss * DOUBLE_BYTES;

        for i in 0..nsum {
            let soff = 24 + i * ss * DOUBLE_BYTES;
            if soff + ss * DOUBLE_BYTES > sbytes.len() {
                return Err(DafError::BadSummary {
                    record: rec,
                    reason: "summary past end of record",
                });
            }
            let sslice = &sbytes[soff..soff + ss * DOUBLE_BYTES];
            // ND leading doubles.
            let mut doubles = Vec::with_capacity(nd);
            for k in 0..nd {
                let off = k * DOUBLE_BYTES;
                doubles.push(f64::from_le_bytes(sslice[off..off + 8].try_into().unwrap()));
            }
            // NI integers (2 per double; trailing int padded if NI odd).
            let mut integers = Vec::with_capacity(ni);
            let int_start = nd * DOUBLE_BYTES;
            for k in 0..ni {
                let off = int_start + k * 4;
                integers.push(i32::from_le_bytes(sslice[off..off + 4].try_into().unwrap()));
            }
            // Matching name slot in the following record.
            let noff = i * name_chars;
            let name_slice = &nbytes[noff..noff + name_chars];
            let name = std::str::from_utf8(name_slice)
                .unwrap_or("")
                .trim_end_matches('\0')
                .trim_end()
                .to_string();
            out.push(Summary {
                doubles,
                integers,
                name,
            });
        }
        Ok(())
    }

    /// Read a contiguous block of doubles by DAF address range
    /// (1-indexed, inclusive on both ends).
    pub fn read_doubles(&self, start_addr: u32, end_addr: u32) -> Result<Vec<f64>, DafError> {
        if start_addr == 0 || end_addr < start_addr {
            return Err(DafError::AddressOutOfBounds {
                start: start_addr,
                end: end_addr,
                file_doubles: (self.inner.mmap.len() / DOUBLE_BYTES) as u64,
            });
        }
        let byte_start = (start_addr as usize - 1) * DOUBLE_BYTES;
        let byte_end = end_addr as usize * DOUBLE_BYTES;
        if byte_end > self.inner.mmap.len() {
            return Err(DafError::AddressOutOfBounds {
                start: start_addr,
                end: end_addr,
                file_doubles: (self.inner.mmap.len() / DOUBLE_BYTES) as u64,
            });
        }
        let n = (end_addr - start_addr + 1) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let off = byte_start + i * DOUBLE_BYTES;
            out.push(f64::from_le_bytes(
                self.inner.mmap[off..off + 8].try_into().unwrap(),
            ));
        }
        Ok(out)
    }

    /// Zero-copy view of a double range as a `&[f64]` slice into the
    /// memory-mapped file.
    ///
    /// This is the hot-path accessor used by SPK/PCK segment
    /// evaluators: it does **no allocation** and **no per-double
    /// byte-decoding loop** — the bytes are reinterpreted in place.
    ///
    /// Safety/correctness rests on three invariants enforced elsewhere:
    /// 1. We rejected non-`LTL-IEEE` files at `from_mmap`, so the on-disk
    ///    representation matches the native `f64` repr on every platform
    ///    we support (LE-IEEE-754 on aarch64-apple-darwin, x86_64-*-*).
    /// 2. DAF addresses are 1-indexed counts of 8-byte doubles; `mmap`
    ///    returns a base pointer that is page-aligned (≥ 4096 B) so every
    ///    DAF double address lands on an 8-byte boundary.
    /// 3. The byte length is `(end_addr - start_addr + 1) * 8`, exactly
    ///    a multiple of 8.
    ///
    /// `bytemuck::cast_slice` validates (2) and (3) at runtime, so this
    /// function is fully safe; on a malformed mmap it would panic before
    /// returning bad data.
    pub fn doubles_native(&self, start_addr: u32, end_addr: u32) -> Result<&[f64], DafError> {
        let bytes = self.double_slice(start_addr, end_addr)?;
        Ok(bytemuck::cast_slice(bytes))
    }

    /// Zero-copy view of a double range as a &[u8] slice.
    pub fn double_slice(&self, start_addr: u32, end_addr: u32) -> Result<&[u8], DafError> {
        if start_addr == 0 || end_addr < start_addr {
            return Err(DafError::AddressOutOfBounds {
                start: start_addr,
                end: end_addr,
                file_doubles: (self.inner.mmap.len() / DOUBLE_BYTES) as u64,
            });
        }
        let byte_start = (start_addr as usize - 1) * DOUBLE_BYTES;
        let byte_end = end_addr as usize * DOUBLE_BYTES;
        if byte_end > self.inner.mmap.len() {
            return Err(DafError::AddressOutOfBounds {
                start: start_addr,
                end: end_addr,
                file_doubles: (self.inner.mmap.len() / DOUBLE_BYTES) as u64,
            });
        }
        Ok(&self.inner.mmap[byte_start..byte_end])
    }

    /// Bulk-load N doubles starting at DAF address `start_addr`.
    pub fn read_n_doubles(&self, start_addr: u32, count: usize) -> Result<Vec<f64>, DafError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let end_addr = start_addr + count as u32 - 1;
        self.read_doubles(start_addr, end_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Build a minimal, valid DAF file with one summary record containing
    /// `summaries` entries. Records 1, 2, ..., n are laid out as:
    ///   record 1: file record (idword, nd, ni, fward=2, bward=2, LTL-IEEE marker)
    ///   record 2: summary record (NEXT=0, PREV=0, NSUM, then summaries)
    ///   record 3: name record (one 8*ss-byte name slot per summary)
    ///   record 4+: data records (filled with `data` doubles laid out sequentially)
    /// Returns (path, tempfile handle — kept alive by caller, summaries' first addresses per entry).
    fn build_daf(
        idword: &[u8; 8],
        nd: u32,
        ni: u32,
        summaries: &[(Vec<f64>, Vec<i32>, String)],
        data: &[f64],
    ) -> (NamedTempFile, Vec<u8>) {
        assert!(summaries
            .iter()
            .all(|(d, i, _)| d.len() == nd as usize && i.len() == ni as usize));
        let ss_doubles = nd as usize + (ni as usize).div_ceil(2);
        let summary_bytes = ss_doubles * DOUBLE_BYTES;

        // ---- file record (record 1) ----
        let mut record1 = vec![0u8; RECORD_BYTES];
        record1[0..8].copy_from_slice(idword);
        record1[8..12].copy_from_slice(&nd.to_le_bytes());
        record1[12..16].copy_from_slice(&ni.to_le_bytes());
        // LOCIFN (60 chars) at 16..76: blank-filled
        for b in &mut record1[16..76] {
            *b = b' ';
        }
        record1[76..80].copy_from_slice(&2u32.to_le_bytes()); // fward
        record1[80..84].copy_from_slice(&2u32.to_le_bytes()); // bward
        record1[84..88].copy_from_slice(&0u32.to_le_bytes()); // free
        record1[88..96].copy_from_slice(b"LTL-IEEE");

        // ---- summary record (record 2) ----
        let mut record2 = vec![0u8; RECORD_BYTES];
        record2[0..8].copy_from_slice(&(0.0f64).to_le_bytes()); // NEXT = 0
        record2[8..16].copy_from_slice(&(0.0f64).to_le_bytes()); // PREV = 0
        record2[16..24].copy_from_slice(&(summaries.len() as f64).to_le_bytes()); // NSUM
        for (i, (doubles, integers, _name)) in summaries.iter().enumerate() {
            let soff = 24 + i * summary_bytes;
            for (k, d) in doubles.iter().enumerate() {
                record2[soff + k * DOUBLE_BYTES..soff + (k + 1) * DOUBLE_BYTES]
                    .copy_from_slice(&d.to_le_bytes());
            }
            let int_start = soff + (nd as usize) * DOUBLE_BYTES;
            for (k, v) in integers.iter().enumerate() {
                record2[int_start + k * 4..int_start + (k + 1) * 4]
                    .copy_from_slice(&v.to_le_bytes());
            }
        }

        // ---- name record (record 3) ----
        let mut record3 = vec![b' '; RECORD_BYTES];
        for (i, (_, _, name)) in summaries.iter().enumerate() {
            let noff = i * summary_bytes;
            let nbytes = name.as_bytes();
            let n = nbytes.len().min(summary_bytes);
            record3[noff..noff + n].copy_from_slice(&nbytes[..n]);
        }

        // ---- data record(s) — pack `data` starting at record 4 ----
        let mut data_bytes = Vec::new();
        for d in data {
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
        (tmp, all)
    }

    #[test]
    fn rejects_file_smaller_than_one_record() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 512]).unwrap();
        tmp.flush().unwrap();
        match DafFile::open(tmp.path()) {
            Err(DafError::TooSmall(n)) => assert_eq!(n, 512),
            Ok(_) => panic!("expected TooSmall, got Ok"),
            Err(e) => panic!("expected TooSmall, got {e:?}"),
        }
    }

    #[test]
    fn rejects_unknown_idword() {
        let mut bytes = vec![0u8; RECORD_BYTES];
        bytes[0..8].copy_from_slice(b"BOGUSFMT");
        bytes[88..96].copy_from_slice(b"LTL-IEEE");
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&bytes).unwrap();
        tmp.flush().unwrap();
        match DafFile::open(tmp.path()) {
            Err(DafError::BadIdword(w)) => assert_eq!(&w, b"BOGUSFMT"),
            Ok(_) => panic!("expected BadIdword, got Ok"),
            Err(e) => panic!("expected BadIdword, got {e:?}"),
        }
    }

    #[test]
    fn rejects_non_ltl_ieee_format() {
        let mut bytes = vec![0u8; RECORD_BYTES];
        bytes[0..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(b"BIG-IEEE");
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&bytes).unwrap();
        tmp.flush().unwrap();
        match DafFile::open(tmp.path()) {
            Err(DafError::UnsupportedFormat(w)) => assert_eq!(&w, b"BIG-IEEE"),
            Ok(_) => panic!("expected UnsupportedFormat, got Ok"),
            Err(e) => panic!("expected UnsupportedFormat, got {e:?}"),
        }
    }

    #[test]
    fn parses_nd_ni_fward_bward_from_file_record() {
        let (tmp, _) = build_daf(
            b"DAF/SPK ",
            2,
            6,
            &[(vec![0.0, 0.0], vec![0, 0, 0, 0, 0, 0], "EMPTY".to_string())],
            &[],
        );
        let daf = DafFile::open(tmp.path()).expect("open");
        assert_eq!(&daf.idword(), b"DAF/SPK ");
        assert_eq!(daf.nd(), 2);
        assert_eq!(daf.ni(), 6);
        // nd=2 + (ni+1)/2 = 2 + 3 = 5 doubles per summary.
        assert_eq!(daf.summary_size_doubles(), 5);
    }

    #[test]
    fn summaries_round_trip_nd_ni_and_name() {
        // SPK-shape (nd=2, ni=6): summary is (start_et, end_et, target, center,
        // frame, dtype, start_addr, end_addr). Encode two summaries to verify
        // multi-summary walking within a single record.
        let s1 = (
            vec![0.0f64, 100.0],
            vec![301, 399, 1, 2, 1001, 1100],
            "EARTH FROM MOON".to_string(),
        );
        let s2 = (
            vec![100.0f64, 200.0],
            vec![10, 0, 1, 2, 1101, 1200],
            "SUN FROM SSB".to_string(),
        );
        let (tmp, _) = build_daf(b"DAF/SPK ", 2, 6, &[s1.clone(), s2.clone()], &[]);
        let daf = DafFile::open(tmp.path()).expect("open");
        let sums = daf.summaries().expect("summaries");
        assert_eq!(sums.len(), 2);
        assert_eq!(sums[0].doubles, s1.0);
        assert_eq!(sums[0].integers, s1.1);
        assert_eq!(sums[0].name, s1.2);
        assert_eq!(sums[1].doubles, s2.0);
        assert_eq!(sums[1].integers, s2.1);
        assert_eq!(sums[1].name, s2.2);
    }

    #[test]
    fn summary_strips_trailing_space_padding() {
        // Real NAIF kernels we care about (SPK/PCK from Fortran producers)
        // pad the 40/48/... byte name slot with trailing spaces; the reader
        // must strip them so downstream display/debug output isn't littered
        // with whitespace.
        //
        // `build_daf` already initializes the name record to spaces; simply
        // writing the short name at offset 0 leaves the tail as spaces.
        let (tmp, _) = build_daf(
            b"DAF/SPK ",
            2,
            6,
            &[(vec![0.0, 1.0], vec![0; 6], "SUN WRT SSB".to_string())],
            &[],
        );
        let daf = DafFile::open(tmp.path()).expect("open");
        let sums = daf.summaries().expect("summaries");
        assert_eq!(sums[0].name, "SUN WRT SSB");
    }

    #[test]
    fn summary_strips_trailing_nul_padding() {
        // C-produced kernels sometimes trail with NULs instead of spaces;
        // the reader's `trim_end_matches('\0')` must recover the real name.
        let mut record3 = vec![b' '; RECORD_BYTES];
        // Manually overwrite the first name slot so it starts with "SATURN"
        // followed by NULs (not spaces).
        let slot_bytes = (2 + 6_usize.div_ceil(2)) * DOUBLE_BYTES; // 5 * 8 = 40
        for b in &mut record3[..slot_bytes] {
            *b = 0;
        }
        record3[..6].copy_from_slice(b"SATURN");

        let mut record1 = vec![0u8; RECORD_BYTES];
        record1[0..8].copy_from_slice(b"DAF/SPK ");
        record1[8..12].copy_from_slice(&2u32.to_le_bytes());
        record1[12..16].copy_from_slice(&6u32.to_le_bytes());
        record1[76..80].copy_from_slice(&2u32.to_le_bytes());
        record1[80..84].copy_from_slice(&2u32.to_le_bytes());
        record1[88..96].copy_from_slice(b"LTL-IEEE");

        let mut record2 = vec![0u8; RECORD_BYTES];
        record2[0..8].copy_from_slice(&0.0f64.to_le_bytes()); // NEXT
        record2[8..16].copy_from_slice(&0.0f64.to_le_bytes()); // PREV
        record2[16..24].copy_from_slice(&1.0f64.to_le_bytes()); // NSUM
                                                                // Summary contents don't matter for this test; leave as zeros.

        let mut all = Vec::new();
        all.extend_from_slice(&record1);
        all.extend_from_slice(&record2);
        all.extend_from_slice(&record3);
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&all).unwrap();
        tmp.flush().unwrap();

        let daf = DafFile::open(tmp.path()).expect("open");
        let sums = daf.summaries().expect("summaries");
        assert_eq!(sums[0].name, "SATURN");
    }

    #[test]
    fn read_doubles_recovers_payload_words() {
        // Pack 8 payload doubles; they start at record 4 => address 1 + 3*128 = 385.
        let payload: Vec<f64> = (0..8).map(|i| i as f64 * 0.5).collect();
        let (tmp, _) = build_daf(
            b"DAF/SPK ",
            2,
            6,
            &[(vec![0.0, 1.0], vec![0; 6], "DATA".to_string())],
            &payload,
        );
        let daf = DafFile::open(tmp.path()).expect("open");
        let start = 1 + 3 * (RECORD_BYTES / DOUBLE_BYTES) as u32; // 385
        let got = daf.read_doubles(start, start + 7).expect("read");
        assert_eq!(got, payload);

        let got_n = daf.read_n_doubles(start, payload.len()).expect("read_n");
        assert_eq!(got_n, payload);

        // Zero-count read returns empty without touching the file.
        assert!(daf.read_n_doubles(start, 0).unwrap().is_empty());
    }

    #[test]
    fn read_doubles_returns_bounds_error_past_end() {
        let (tmp, all) = build_daf(
            b"DAF/SPK ",
            2,
            6,
            &[(vec![0.0, 1.0], vec![0; 6], "X".to_string())],
            &[1.0, 2.0],
        );
        let daf = DafFile::open(tmp.path()).expect("open");
        let past = (all.len() / DOUBLE_BYTES) as u32 + 10;
        let err = daf.read_doubles(past, past + 1).unwrap_err();
        assert!(matches!(err, DafError::AddressOutOfBounds { .. }));

        // Start=0 is illegal (DAF addresses are 1-indexed).
        let err0 = daf.read_doubles(0, 1).unwrap_err();
        assert!(matches!(err0, DafError::AddressOutOfBounds { .. }));

        // end < start.
        let err_inv = daf.read_doubles(10, 5).unwrap_err();
        assert!(matches!(err_inv, DafError::AddressOutOfBounds { .. }));
    }

    #[test]
    fn double_slice_is_zero_copy_and_byte_aligned() {
        let payload: Vec<f64> = vec![std::f64::consts::PI, std::f64::consts::E, 42.0];
        let (tmp, _) = build_daf(
            b"DAF/SPK ",
            2,
            6,
            &[(vec![0.0, 1.0], vec![0; 6], "X".to_string())],
            &payload,
        );
        let daf = DafFile::open(tmp.path()).expect("open");
        let start = 1 + 3 * (RECORD_BYTES / DOUBLE_BYTES) as u32;
        let slice = daf.double_slice(start, start + 2).expect("slice");
        assert_eq!(slice.len(), payload.len() * DOUBLE_BYTES);
        // Hand-decode two doubles and confirm they match the input.
        let x0 = f64::from_le_bytes(slice[0..8].try_into().unwrap());
        let x1 = f64::from_le_bytes(slice[8..16].try_into().unwrap());
        assert_eq!(x0, payload[0]);
        assert_eq!(x1, payload[1]);
    }
}
