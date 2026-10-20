//! Furnished kernel pool with CSPICE `furnsh` semantics. [`SpkFile`]
//! and [`PckFile`] read one file. [`KernelPool`] owns a set of them.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::daf::{DafError, DafFile};
use crate::frame::{
    j2000_to_eclipj2000, pck_euler_rotation_and_derivative, rotate_state, sxform_from_rotation,
    NaifFrame,
};
use crate::pck::{PckError, PckFile};
use crate::spk::{SpkError, SpkFile};

/// Longest body chain `spk_state` will walk, a bound on bad kernels.
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
    #[error("no loaded binary PCK covers frame {frame} at epoch index {index} (et {et} TDB)")]
    PckInsufficientData { frame: i32, index: usize, et: f64 },
    #[error("binary PCK frame {frame} is stated against unsupported reference frame {reference}")]
    UnsupportedPckReference { frame: i32, reference: i32 },
    #[error("{path} is not an SPK or a binary PCK")]
    UnsupportedKernel { path: PathBuf },
    #[error(transparent)]
    Daf(#[from] DafError),
    #[error(transparent)]
    Spk(#[from] SpkError),
    #[error(transparent)]
    Pck(#[from] PckError),
}

enum Kernel {
    Spk(SpkFile),
    Pck(PckFile),
}

/// A set of furnished kernels resolved the way CSPICE resolves one.
///
/// Priority is load order. A lookup takes the newest file holding a
/// segment that covers the body at the epoch, and the last such segment
/// in file order, the way `spksfs` does. SPK lookups chain like
/// `spkgeo_c`, meeting target and centre at the first common node.
/// Resolution is per epoch, so a batch succeeds whenever the set covers
/// it, and an uncovered epoch is a typed error naming that epoch.
#[derive(Default)]
pub struct KernelPool {
    kernels: Vec<(PathBuf, Kernel)>,
}

impl KernelPool {
    /// An empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a kernel at the top of the priority order, rereading and
    /// promoting a path already in the pool the way `furnsh_c` does.
    pub fn furnish<P: AsRef<Path>>(&mut self, path: P) -> Result<(), PoolError> {
        let path = path.as_ref();
        let unsupported = || PoolError::UnsupportedKernel {
            path: path.to_path_buf(),
        };
        let daf = DafFile::open(path).map_err(|e| match e {
            DafError::Io(_) => PoolError::Daf(e),
            _ => unsupported(),
        })?;
        let kernel = match daf.idword() {
            word if word.starts_with(b"DAF/SPK") => Kernel::Spk(SpkFile::from_daf(daf)?),
            word if word.starts_with(b"DAF/PCK") => Kernel::Pck(PckFile::from_daf(daf)?),
            _ => return Err(unsupported()),
        };
        self.kernels.retain(|(loaded, _)| loaded != path);
        self.kernels.push((path.to_path_buf(), kernel));
        Ok(())
    }

    /// Remove a kernel, true when the path was loaded, leaving every
    /// other kernel at its relative priority.
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
    /// `frame`.
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
        merge(
            self.spks()
                .flat_map(|file| file.segments())
                .filter(|s| s.target == body)
                .map(|s| (s.start_et, s.end_et))
                .collect(),
        )
    }

    /// Transformation from `inertial` to the body-fixed frame
    /// `body_frame` at `et`, in CSPICE `sxform_c` block layout.
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

    /// Transformations for a batch of epochs, in input order.
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
        Ok((self.pck_sxform_batch(body_frame, inertial, ets)?.iter())
            .map(rotation_block)
            .collect())
    }

    /// Merged coverage of `body_frame`, like `pckcov_c` taken over
    /// every loaded file.
    pub fn pck_coverage(&self, body_frame: i32) -> Vec<(f64, f64)> {
        merge(
            self.pcks()
                .flat_map(|file| file.segments())
                .filter(|s| s.body_frame == body_frame)
                .map(|s| (s.start_et, s.end_et))
                .collect(),
        )
    }

    /// Loaded SPKs, highest priority first.
    fn spks(&self) -> impl Iterator<Item = &SpkFile> {
        self.kernels.iter().rev().filter_map(|(_, k)| match k {
            Kernel::Spk(file) => Some(file),
            Kernel::Pck(_) => None,
        })
    }

    /// Loaded binary PCKs, highest priority first.
    fn pcks(&self) -> impl Iterator<Item = &PckFile> {
        self.kernels.iter().rev().filter_map(|(_, k)| match k {
            Kernel::Pck(file) => Some(file),
            Kernel::Spk(_) => None,
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
        self.chain(target, center, et)?
            .ok_or(PoolError::SpkInsufficientData {
                target,
                center,
                index,
                et,
            })
    }

    /// Meet the walks from both bodies at the first common node.
    fn chain(&self, target: i32, center: i32, et: f64) -> Result<Option<[f64; 6]>, PoolError> {
        let (up, up_states) = self.walk(target, &[center], et)?;
        let (down, down_states) = self.walk(center, &up, et)?;
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

    /// Nodes from `body` toward the barycentre, with pairwise states.
    fn walk(
        &self,
        body: i32,
        stop: &[i32],
        et: f64,
    ) -> Result<(Vec<i32>, Vec<[f64; 6]>), PoolError> {
        let mut nodes = vec![body];
        let mut states = Vec::new();
        while nodes.len() <= MAX_CHAIN {
            let current = nodes[nodes.len() - 1];
            if current == 0 || stop.contains(&current) {
                break;
            }
            let Some((file, next)) = self.step(current, et) else {
                break;
            };
            if nodes.contains(&next) {
                break;
            }
            states.push(file.state(current, next, et)?);
            nodes.push(next);
        }
        Ok((nodes, states))
    }

    /// The file and centre a chain advances to, the way `spksfs` picks.
    fn step(&self, body: i32, et: f64) -> Option<(&SpkFile, i32)> {
        (self.spks()).find_map(|file| center_for(file, body, et).map(|next| (file, next)))
    }

    /// Euler angles and rates for `body_frame`, with their reference id.
    fn euler(&self, body_frame: i32, et: f64, index: usize) -> Result<(i32, [f64; 6]), PoolError> {
        let file = self
            .pcks()
            .find(|file| {
                (file.segments().iter().rev())
                    .any(|s| s.body_frame == body_frame && s.start_et <= et && et <= s.end_et)
            })
            .ok_or(PoolError::PckInsufficientData {
                frame: body_frame,
                index,
                et,
            })?;
        Ok(file.euler_state_with_ref(body_frame, et)?)
    }
}

/// The centre `spksfs` picks, from the last covering segment in file
/// order. The reader lands on that same segment, so the chain reads the
/// state through [`SpkFile::state`].
fn center_for(file: &SpkFile, body: i32, et: f64) -> Option<i32> {
    (file.segments().iter().rev())
        .find(|s| s.target == body && s.start_et <= et && et <= s.end_et)
        .map(|s| s.center)
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

/// Reference ids are 1 for J2000 and 17 for ECLIPJ2000.
fn inertial_to_reference(inertial: NaifFrame, reference: i32) -> Option<[[f64; 3]; 3]> {
    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    match (inertial, reference) {
        (NaifFrame::J2000, 1) | (NaifFrame::EclipJ2000, 17) => Some(IDENTITY),
        (NaifFrame::J2000, 17) => Some(j2000_to_eclipj2000()),
        (NaifFrame::EclipJ2000, 1) => {
            let ecliptic = j2000_to_eclipj2000();
            Some(std::array::from_fn(|i| {
                std::array::from_fn(|j| ecliptic[j][i])
            }))
        }
        _ => None,
    }
}

fn matmul3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn rotation_block(m: &[[f64; 6]; 6]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| m[i][j]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_writer::{SpkWriter, Type3Record, Type3Segment};
    use tempfile::NamedTempFile;

    /// An SPK of constant-state segments, one per
    /// `(target, center, start, end, x)` tuple. A segment reads `x` on
    /// the x axis everywhere inside its window.
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

    /// Pins `furnsh` priority, where load order decides, furnishing a
    /// loaded path again puts it back on top, and only SPK and binary
    /// PCK belong in the pool.
    #[test]
    fn later_furnish_wins_and_refurnishing_promotes() {
        let a = spk(&[(1_000, 0, 0.0, 100.0, 1.0)]);
        let b = spk(&[(1_000, 0, 50.0, 200.0, 2.0)]);
        let mut pool = KernelPool::new();
        pool.furnish(a.path()).unwrap();
        pool.furnish(b.path()).unwrap();
        assert_eq!(pool.loaded(), vec![a.path(), b.path()]);
        assert_eq!(pool.spk_state(1_000, 0, 75.0).unwrap()[0], 2.0);
        assert_eq!(pool.spk_state(1_000, 0, 25.0).unwrap()[0], 1.0);
        assert_eq!(pool.spk_coverage(1_000), vec![(0.0, 200.0)]);

        pool.furnish(a.path()).unwrap();
        assert_eq!(pool.loaded(), vec![b.path(), a.path()]);
        assert_eq!(pool.spk_state(1_000, 0, 75.0).unwrap()[0], 1.0);

        // A text kernel is rejected instead of silently ignored.
        let text = NamedTempFile::new().unwrap();
        std::fs::write(
            text.path(),
            "KPL/FK\n\\begindata\nNAIF_BODY_CODE += ( 7777 )\n",
        )
        .unwrap();
        assert!(
            matches!(
                pool.furnish(text.path()),
                Err(PoolError::UnsupportedKernel { .. })
            ),
            "a text kernel is not pool content"
        );

        // The promotion moved the entry, it did not duplicate it, and
        // unloading the winner hands the epoch back to the other file.
        assert!(pool.unload(a.path()));
        assert_eq!(pool.loaded(), vec![b.path()]);
        assert_eq!(pool.spk_state(1_000, 0, 75.0).unwrap()[0], 2.0);
        assert!(!pool.unload(a.path()));
        pool.clear();
        assert!(pool.spk_state(1_000, 0, 75.0).is_err());
    }

    /// Pins per epoch resolution, where a batch spanning two files
    /// works and an uncovered epoch is named by index and by ET.
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
            for (i, &et) in ets.iter().enumerate() {
                assert_eq!(batch[i], pool.spk_state(1_000, 0, et).unwrap());
            }
            let err = pool
                .spk_state_batch(1_000, 0, NaifFrame::J2000, &[50.0, 150.0])
                .expect_err("150 falls in the gap between the two files");
            assert!(
                matches!(err, PoolError::SpkInsufficientData { index: 1, et, .. } if et == 150.0),
                "{err}"
            );
        }
    }

    /// Pins `spkgeo` chaining, where the newest file holding a segment
    /// for the body sets the route, so its chain beats an older direct
    /// pair and the two walks meet at the first common node.
    #[test]
    fn a_newer_chain_beats_an_older_direct_pair() {
        // CSpice N0067 answers 13 for (1000, 0) in this set.
        let older = spk(&[(1_000, 0, 0.0, 100.0, 1.0), (2_000, 399, 0.0, 100.0, 7.0)]);
        let newer = spk(&[(1_000, 399, 0.0, 100.0, 3.0), (399, 0, 0.0, 100.0, 10.0)]);
        let mut pool = KernelPool::new();
        pool.furnish(older.path()).unwrap();
        pool.furnish(newer.path()).unwrap();
        assert_eq!(pool.spk_state(1_000, 0, 50.0).unwrap()[0], 13.0);
        assert_eq!(pool.spk_state(0, 1_000, 50.0).unwrap()[0], -13.0);
        let batch = pool
            .spk_state_batch(1_000, 0, NaifFrame::J2000, &[25.0, 75.0])
            .expect("both epochs are covered");
        assert_eq!((batch[0][0], batch[1][0]), (13.0, 13.0));
        // 1000 and 2000 both hang off 399, so the walks cross files and
        // meet there, short of the barycentre.
        assert_eq!(pool.spk_state(1_000, 2_000, 50.0).unwrap()[0], -4.0);
        assert_eq!(pool.spk_state(399, 399, 50.0).unwrap(), [0.0; 6]);

        // Furnished the other way round the direct pair is newest, and
        // CSpice answers 1.
        let mut reversed = KernelPool::new();
        reversed.furnish(newer.path()).unwrap();
        reversed.furnish(older.path()).unwrap();
        assert_eq!(reversed.spk_state(1_000, 0, 50.0).unwrap()[0], 1.0);
        let batch = reversed
            .spk_state_batch(1_000, 0, NaifFrame::J2000, &[25.0, 75.0])
            .expect("both epochs are covered");
        assert_eq!((batch[0][0], batch[1][0]), (1.0, 1.0));
    }

    /// Pins the `spksfs` segment rule, where the last segment covering
    /// the epoch inside the winning file sets the centre.
    #[test]
    fn the_last_segment_covering_the_epoch_picks_the_centre() {
        let bodies = spk(&[
            (1_000, 399, 0.0, 100.0, 3.0),
            (1_000, 301, 0.0, 100.0, 5.0),
            (1_000, 399, 200.0, 300.0, 7.0),
        ]);
        let centers = spk(&[(399, 0, 0.0, 300.0, 10.0), (301, 0, 0.0, 300.0, 20.0)]);
        let mut pool = KernelPool::new();
        pool.furnish(bodies.path()).unwrap();
        pool.furnish(centers.path()).unwrap();
        // At ET 50 the last covering segment routes through 301.
        assert_eq!(pool.spk_state(1_000, 0, 50.0).unwrap()[0], 25.0);
        // Past 100 only the third segment covers, so the centre is 399.
        assert_eq!(pool.spk_state(1_000, 0, 250.0).unwrap()[0], 17.0);
        // Coverage is still the union of the windows.
        assert_eq!(pool.spk_coverage(1_000), vec![(0.0, 100.0), (200.0, 300.0)]);
    }
}
