//! Furnished kernel pool with CSPICE `furnsh` semantics. [`SpkFile`]
//! and [`PckFile`] read one file. [`KernelPool`] owns a set of them.

use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::frame::{
    j2000_to_eclipj2000, pck_euler_rotation_and_derivative, rotate_state, sxform_from_rotation,
    NaifFrame,
};
use crate::pck::{PckError, PckFile};
use crate::spk::{SpkError, SpkFile};

/// Longest body chain `spk_state` will walk. NAIF chains are shallow,
/// so this only bounds a malformed kernel set.
const MAX_CHAIN: usize = 32;

#[derive(Debug, Error)]
pub enum PoolError {
    /// The typed form of CSPICE `SPICE(SPKINSUFFDATA)`.
    #[error("no loaded SPK covers target {target} relative to center {center} at epoch index {index} (et {et} TDB)")]
    SpkInsufficientData {
        target: i32,
        center: i32,
        index: usize,
        et: f64,
    },
    /// No loaded binary PCK covers the frame at this epoch.
    #[error("no loaded binary PCK covers frame {frame} at epoch index {index} (et {et} TDB)")]
    PckInsufficientData { frame: i32, index: usize, et: f64 },
    /// The angles are stated against a reference frame this crate
    /// cannot compose with the requested inertial frame.
    #[error("binary PCK frame {frame} is stated against unsupported reference frame {reference}")]
    UnsupportedPckReference { frame: i32, reference: i32 },
    /// Text kernels are not part of the pool.
    #[error("{path} is not an SPK or a binary PCK")]
    UnsupportedKernel { path: PathBuf },
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Spk(#[from] SpkError),
    #[error(transparent)]
    Pck(#[from] PckError),
}

/// Merged epoch windows per key, the key whose last segment comes
/// latest in the file first. That is the order CSPICE prefers inside
/// one file, and across files the pool walks newest furnished first.
type Coverage<K> = Vec<(K, Vec<(f64, f64)>)>;

/// A furnished SPK and the `(target, center)` pairs it covers.
struct SpkEntry {
    file: SpkFile,
    pairs: Coverage<(i32, i32)>,
}

/// A furnished binary PCK and the body-fixed frames it covers.
struct PckEntry {
    file: PckFile,
    frames: Coverage<i32>,
}

enum Kernel {
    Spk(SpkEntry),
    Pck(PckEntry),
}

/// A set of furnished kernels resolved the way CSPICE resolves one.
///
/// Priority is load order, so the most recently furnished file is
/// searched first and wins wherever two files overlap. Resolution is
/// per epoch and never per batch, so a batch succeeds whenever the
/// union of the loaded files covers it, and an epoch nothing covers is
/// a typed error naming that epoch. SPK lookups chain across files the
/// way `spkgeo_c` does, walking target and centre toward the solar
/// system barycentre and meeting at the first common node. Evaluation
/// runs through the single-file readers, which are unchanged and stay
/// the right choice when one file is in play.
#[derive(Default)]
pub struct KernelPool {
    kernels: Vec<(PathBuf, Kernel)>,
}

impl KernelPool {
    /// An empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a kernel and give it the highest priority. The type comes
    /// from the DAF id word, so SPK and binary PCK need no hint and
    /// anything else is rejected. A path already in the pool keeps the
    /// priority it has, like `furnsh_c` on an open DAF.
    pub fn furnish<P: AsRef<Path>>(&mut self, path: P) -> Result<(), PoolError> {
        let path = path.as_ref();
        if self.kernels.iter().any(|(loaded, _)| loaded == path) {
            return Ok(());
        }
        let kernel = match daf_idword(path)? {
            Some(word) if word.starts_with(b"DAF/SPK") => {
                Kernel::Spk(SpkEntry::new(SpkFile::open(path)?))
            }
            Some(word) if word.starts_with(b"DAF/PCK") => {
                Kernel::Pck(PckEntry::new(PckFile::open(path)?))
            }
            _ => {
                return Err(PoolError::UnsupportedKernel {
                    path: path.to_path_buf(),
                })
            }
        };
        self.kernels.push((path.to_path_buf(), kernel));
        Ok(())
    }

    /// Remove a kernel. True when the path was loaded. Everything else
    /// keeps its relative priority.
    pub fn unload<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let path = path.as_ref();
        let before = self.kernels.len();
        self.kernels.retain(|(loaded, _)| loaded != path);
        self.kernels.len() != before
    }

    /// Unload everything, like `kclear_c`.
    pub fn clear(&mut self) {
        self.kernels.clear();
    }

    /// Furnished paths in load order, lowest priority first.
    pub fn loaded(&self) -> Vec<&Path> {
        self.kernels.iter().map(|(p, _)| p.as_path()).collect()
    }

    /// State of `target` relative to `center` at `et`, in the frame the
    /// segments are stated in, which is J2000 for DE-series kernels.
    pub fn spk_state(&self, target: i32, center: i32, et: f64) -> Result<[f64; 6], PoolError> {
        self.state(target, center, et, 0)
    }

    /// States for a batch of epochs, in input order, rotated into
    /// `frame`. The error names the first epoch nothing covers.
    pub fn spk_state_batch(
        &self,
        target: i32,
        center: i32,
        frame: NaifFrame,
        ets: &[f64],
    ) -> Result<Vec<[f64; 6]>, PoolError> {
        ets.iter()
            .enumerate()
            .map(|(index, &et)| {
                let state = self.state(target, center, et, index)?;
                Ok(rotate_state(NaifFrame::J2000, frame, &state))
            })
            .collect()
    }

    /// Merged coverage of `body` as an SPK target, like `spkcov_c`
    /// taken over every loaded file.
    pub fn spk_coverage(&self, body: i32) -> Vec<(f64, f64)> {
        let mut raw = Vec::new();
        for entry in self.spks() {
            for (_, windows) in entry.pairs.iter().filter(|((t, _), _)| *t == body) {
                raw.extend(windows.iter().copied());
            }
        }
        merge(raw)
    }

    /// Transformation from `inertial` to the body-fixed frame
    /// `body_frame` at `et`, in CSPICE `sxform_c` block layout. For the
    /// other direction use [`crate::frame::invert_sxform`].
    pub fn pck_sxform(
        &self,
        body_frame: i32,
        inertial: NaifFrame,
        et: f64,
    ) -> Result<[[f64; 6]; 6], PoolError> {
        let (reference, euler) = self.euler(body_frame, et, 0)?;
        sxform(body_frame, inertial, reference, &euler)
    }

    /// Rotation-only form of [`KernelPool::pck_sxform`], matching
    /// `pxform_c`.
    pub fn pck_pxform(
        &self,
        body_frame: i32,
        inertial: NaifFrame,
        et: f64,
    ) -> Result<[[f64; 3]; 3], PoolError> {
        Ok(rotation_block(&self.pck_sxform(body_frame, inertial, et)?))
    }

    /// Transformations for a batch of epochs, in input order. The error
    /// names the first epoch nothing covers.
    pub fn pck_sxform_batch(
        &self,
        body_frame: i32,
        inertial: NaifFrame,
        ets: &[f64],
    ) -> Result<Vec<[[f64; 6]; 6]>, PoolError> {
        ets.iter()
            .enumerate()
            .map(|(index, &et)| {
                let (reference, euler) = self.euler(body_frame, et, index)?;
                sxform(body_frame, inertial, reference, &euler)
            })
            .collect()
    }

    /// Rotation-only form of [`KernelPool::pck_sxform_batch`].
    pub fn pck_pxform_batch(
        &self,
        body_frame: i32,
        inertial: NaifFrame,
        ets: &[f64],
    ) -> Result<Vec<[[f64; 3]; 3]>, PoolError> {
        let sxforms = self.pck_sxform_batch(body_frame, inertial, ets)?;
        Ok(sxforms.iter().map(rotation_block).collect())
    }

    /// Merged coverage of `body_frame`, like `pckcov_c` taken over
    /// every loaded file.
    pub fn pck_coverage(&self, body_frame: i32) -> Vec<(f64, f64)> {
        let mut raw = Vec::new();
        for entry in self.pcks() {
            for (_, windows) in entry.frames.iter().filter(|(f, _)| *f == body_frame) {
                raw.extend(windows.iter().copied());
            }
        }
        merge(raw)
    }

    /// Loaded SPKs, highest priority first.
    fn spks(&self) -> impl Iterator<Item = &SpkEntry> {
        self.kernels.iter().rev().filter_map(|(_, k)| match k {
            Kernel::Spk(entry) => Some(entry),
            Kernel::Pck(_) => None,
        })
    }

    /// Loaded binary PCKs, highest priority first.
    fn pcks(&self) -> impl Iterator<Item = &PckEntry> {
        self.kernels.iter().rev().filter_map(|(_, k)| match k {
            Kernel::Pck(entry) => Some(entry),
            Kernel::Spk(_) => None,
        })
    }

    /// The highest priority file that holds the pair at `et`.
    fn spk_for(&self, target: i32, center: i32, et: f64) -> Option<&SpkEntry> {
        self.spks().find(|entry| {
            (entry.pairs.iter())
                .any(|(key, windows)| *key == (target, center) && covers(windows, et))
        })
    }

    fn state(
        &self,
        target: i32,
        center: i32,
        et: f64,
        index: usize,
    ) -> Result<[f64; 6], PoolError> {
        if target == center {
            return Ok([0.0; 6]);
        }
        if let Some(entry) = self.spk_for(target, center, et) {
            return Ok(entry.file.state(target, center, et)?);
        }
        if let Some(entry) = self.spk_for(center, target, et) {
            let mut state = entry.file.state(center, target, et)?;
            state.iter_mut().for_each(|v| *v = -*v);
            return Ok(state);
        }
        self.chain(target, center, et)?
            .ok_or(PoolError::SpkInsufficientData {
                target,
                center,
                index,
                et,
            })
    }

    /// Walk both bodies toward the barycentre and meet at the first
    /// common node, the way `spkgeo_c` does.
    fn chain(&self, target: i32, center: i32, et: f64) -> Result<Option<[f64; 6]>, PoolError> {
        let (up, up_states) = self.walk(target, center, et)?;
        let (down, down_states) = self.walk(center, target, et)?;
        for (m, node) in down.iter().enumerate() {
            let Some(k) = up.iter().position(|n| n == node) else {
                continue;
            };
            let mut out = [0.0f64; 6];
            for state in &up_states[..k] {
                for (slot, value) in out.iter_mut().zip(state) {
                    *slot += value;
                }
            }
            for state in &down_states[..m] {
                for (slot, value) in out.iter_mut().zip(state) {
                    *slot -= value;
                }
            }
            return Ok(Some(out));
        }
        Ok(None)
    }

    /// Nodes visited walking from `body` toward the barycentre, and the
    /// state of each node relative to the next. The walk stops at the
    /// barycentre, at `stop`, at a repeated node, or where no loaded
    /// SPK covers the current node.
    fn walk(&self, body: i32, stop: i32, et: f64) -> Result<(Vec<i32>, Vec<[f64; 6]>), PoolError> {
        let mut nodes = vec![body];
        let mut states = Vec::new();
        while nodes.len() <= MAX_CHAIN {
            let current = nodes[nodes.len() - 1];
            if current == 0 || current == stop {
                break;
            }
            let Some((entry, next)) = self.step(current, et) else {
                break;
            };
            if nodes.contains(&next) {
                break;
            }
            states.push(entry.file.state(current, next, et)?);
            nodes.push(next);
        }
        Ok((nodes, states))
    }

    /// The file and centre a chain advances to from `body` at `et`.
    fn step(&self, body: i32, et: f64) -> Option<(&SpkEntry, i32)> {
        self.spks().find_map(|entry| {
            let (key, _) = (entry.pairs.iter())
                .find(|((target, _), windows)| *target == body && covers(windows, et))?;
            Some((entry, key.1))
        })
    }

    /// Euler angles and rates for `body_frame`, with the reference
    /// frame id they are stated against.
    fn euler(&self, body_frame: i32, et: f64, index: usize) -> Result<(i32, [f64; 6]), PoolError> {
        let entry = self
            .pcks()
            .find(|entry| {
                (entry.frames.iter()).any(|(f, windows)| *f == body_frame && covers(windows, et))
            })
            .ok_or(PoolError::PckInsufficientData {
                frame: body_frame,
                index,
                et,
            })?;
        Ok(entry.file.euler_state_with_ref(body_frame, et)?)
    }
}

impl SpkEntry {
    fn new(file: SpkFile) -> Self {
        let rows = (file.segments().iter()).map(|s| ((s.target, s.center), (s.start_et, s.end_et)));
        let pairs = group(rows);
        Self { file, pairs }
    }
}

impl PckEntry {
    fn new(file: PckFile) -> Self {
        let rows = (file.segments().iter()).map(|s| (s.body_frame, (s.start_et, s.end_et)));
        let frames = group(rows);
        Self { file, frames }
    }
}

/// Collect one file's segment windows into merged windows per key.
/// Reading the segments backwards puts the key whose last segment
/// comes latest in the file first.
fn group<K: Copy + PartialEq>(
    rows: impl DoubleEndedIterator<Item = (K, (f64, f64))>,
) -> Coverage<K> {
    let mut out: Coverage<K> = Vec::new();
    for (key, window) in rows.rev() {
        match out.iter_mut().find(|(k, _)| *k == key) {
            Some((_, windows)) => windows.push(window),
            None => out.push((key, vec![window])),
        }
    }
    out.into_iter().map(|(k, w)| (k, merge(w))).collect()
}

/// True when `et` lands in one of `windows`, which are sorted by start
/// and disjoint. One binary search per file is the whole resolution
/// cost, and a pool holds a handful of files.
fn covers(windows: &[(f64, f64)], et: f64) -> bool {
    match windows.binary_search_by(|w| w.0.partial_cmp(&et).unwrap_or(Ordering::Equal)) {
        Ok(_) => true,
        Err(0) => false,
        Err(next) => windows[next - 1].1 >= et,
    }
}

/// Sort and merge raw segment windows into disjoint intervals.
fn merge(mut raw: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    raw.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(raw.len());
    for (start, end) in raw {
        match out.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => out.push((start, end)),
        }
    }
    out
}

/// Compose the reference to body rotation with the static inertial to
/// reference rotation, then assemble the 6x6.
fn sxform(
    body_frame: i32,
    inertial: NaifFrame,
    reference: i32,
    euler: &[f64; 6],
) -> Result<[[f64; 6]; 6], PoolError> {
    let (r, dr) = pck_euler_rotation_and_derivative(
        euler[0], euler[1], euler[2], euler[3], euler[4], euler[5],
    );
    let to_reference =
        inertial_to_reference(inertial, reference).ok_or(PoolError::UnsupportedPckReference {
            frame: body_frame,
            reference,
        })?;
    Ok(sxform_from_rotation(
        &matmul3(&r, &to_reference),
        &matmul3(&dr, &to_reference),
    ))
}

/// Static rotation from an inertial frame to a PCK segment's reference
/// frame. Reference ids are 1 for J2000 and 17 for ECLIPJ2000.
fn inertial_to_reference(inertial: NaifFrame, reference: i32) -> Option<[[f64; 3]; 3]> {
    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let ecliptic = j2000_to_eclipj2000();
    match (inertial, reference) {
        (NaifFrame::J2000, 1) | (NaifFrame::EclipJ2000, 17) => Some(IDENTITY),
        (NaifFrame::J2000, 17) => Some(ecliptic),
        (NaifFrame::EclipJ2000, 1) => {
            let mut t = [[0.0f64; 3]; 3];
            for (i, row) in t.iter_mut().enumerate() {
                for (j, slot) in row.iter_mut().enumerate() {
                    *slot = ecliptic[j][i];
                }
            }
            Some(t)
        }
        _ => None,
    }
}

fn matmul3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    c
}

fn rotation_block(m: &[[f64; 6]; 6]) -> [[f64; 3]; 3] {
    let mut r = [[0.0f64; 3]; 3];
    for i in 0..3 {
        r[i][..3].copy_from_slice(&m[i][..3]);
    }
    r
}

/// Read the DAF id word, or None when the file is not a DAF.
fn daf_idword(path: &Path) -> Result<Option<[u8; 8]>, PoolError> {
    let io = |source| PoolError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut buf = [0u8; 8];
    if file.read(&mut buf).map_err(io)? < 8 || !buf.starts_with(b"DAF/") {
        return Ok(None);
    }
    Ok(Some(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_writer::{SpkWriter, Type3Record, Type3Segment};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// An SPK of constant-state segments, one per
    /// `(target, center, start, end, x)` tuple. Coefficients past the
    /// constant term are zero, so a segment reads `x` on the x axis
    /// everywhere inside its window.
    fn spk(segments: &[(i32, i32, f64, f64, f64)]) -> NamedTempFile {
        let mut writer = SpkWriter::new_spk("pool-test");
        for &(target, center, start, end, x) in segments {
            let zero = || vec![0.0, 0.0];
            writer
                .add_type3(Type3Segment {
                    target,
                    center,
                    frame_id: 1,
                    start_et: start,
                    end_et: end,
                    segment_id: "pool-test".to_string(),
                    init: start,
                    intlen: end - start,
                    records: vec![Type3Record {
                        mid: 0.5 * (start + end),
                        radius: 0.5 * (end - start),
                        x: vec![x, 0.0],
                        y: zero(),
                        z: zero(),
                        vx: zero(),
                        vy: zero(),
                        vz: zero(),
                    }],
                })
                .expect("add segment");
        }
        let file = NamedTempFile::new().expect("tempfile");
        writer.write(file.path()).expect("write spk");
        file
    }

    #[test]
    fn later_furnish_wins_where_coverage_overlaps() {
        let low = spk(&[(1_000, 0, 0.0, 100.0, 1.0)]);
        let high = spk(&[(1_000, 0, 50.0, 200.0, 2.0)]);
        let mut pool = KernelPool::new();
        pool.furnish(low.path()).unwrap();
        pool.furnish(high.path()).unwrap();
        pool.furnish(low.path()).unwrap();
        assert_eq!(pool.loaded(), vec![low.path(), high.path()]);
        assert_eq!(pool.spk_state(1_000, 0, 75.0).unwrap()[0], 2.0);
        assert_eq!(pool.spk_state(1_000, 0, 25.0).unwrap()[0], 1.0);
        assert_eq!(pool.spk_coverage(1_000), vec![(0.0, 200.0)]);

        // Unloading the winner hands the epoch back to the other file.
        assert!(pool.unload(high.path()));
        assert_eq!(pool.spk_state(1_000, 0, 75.0).unwrap()[0], 1.0);
        assert!(!pool.unload(high.path()));
        pool.clear();
        assert!(pool.spk_state(1_000, 0, 75.0).is_err());
    }

    #[test]
    fn every_epoch_resolves_on_its_own() {
        let early = spk(&[(1_000, 0, 0.0, 100.0, 1.0)]);
        let late = spk(&[(1_000, 0, 200.0, 300.0, 2.0)]);
        let ets = [50.0, 250.0];
        for order in [[&early, &late], [&late, &early]] {
            let mut pool = KernelPool::new();
            for file in order {
                pool.furnish(file.path()).unwrap();
            }
            let batch = pool
                .spk_state_batch(1_000, 0, NaifFrame::J2000, &ets)
                .expect("the batch spans both files");
            assert_eq!((batch[0][0], batch[1][0]), (1.0, 2.0));
            // The batch answer is the single-epoch answer, epoch by epoch.
            for (i, &et) in ets.iter().enumerate() {
                assert_eq!(batch[i], pool.spk_state(1_000, 0, et).unwrap());
            }
            // An epoch no file reaches is named by index and by ET.
            let err = pool
                .spk_state_batch(1_000, 0, NaifFrame::J2000, &[50.0, 150.0])
                .expect_err("150 falls in the gap between the two files");
            assert!(
                matches!(err, PoolError::SpkInsufficientData { index: 1, et, .. } if et == 150.0),
                "{err}"
            );
        }
    }

    #[test]
    fn chains_across_files_and_meets_at_the_common_node() {
        let inner = spk(&[(1_000, 399, 0.0, 100.0, 3.0), (2_000, 399, 0.0, 100.0, 7.0)]);
        let outer = spk(&[(399, 0, 0.0, 100.0, 10.0)]);
        let mut pool = KernelPool::new();
        pool.furnish(inner.path()).unwrap();
        pool.furnish(outer.path()).unwrap();

        // No file holds the pair. The chain is 1000 to 399 to 0.
        assert_eq!(pool.spk_state(1_000, 0, 50.0).unwrap()[0], 13.0);
        // The reverse pair is the negation.
        assert_eq!(pool.spk_state(0, 1_000, 50.0).unwrap()[0], -13.0);
        // Both bodies hang off 399, so the walks meet short of the barycentre.
        assert_eq!(pool.spk_state(1_000, 2_000, 50.0).unwrap()[0], -4.0);
        // A direct pair answers directly, and a body is zero against itself.
        assert_eq!(pool.spk_state(1_000, 399, 50.0).unwrap()[0], 3.0);
        assert_eq!(pool.spk_state(399, 399, 50.0).unwrap(), [0.0; 6]);
        // One file alone cannot answer the chained pair.
        let mut partial = KernelPool::new();
        partial.furnish(inner.path()).unwrap();
        assert!(partial.spk_state(1_000, 0, 50.0).is_err());
    }

    #[test]
    fn a_text_kernel_is_rejected() {
        let mut text = NamedTempFile::new().unwrap();
        writeln!(text, "KPL/FK\n\\begindata\nNAIF_BODY_CODE += ( 7777 )").unwrap();
        text.flush().unwrap();
        let mut pool = KernelPool::new();
        let err = pool.furnish(text.path()).unwrap_err();
        assert!(matches!(err, PoolError::UnsupportedKernel { .. }), "{err}");
        assert!(pool.loaded().is_empty());
    }
}
