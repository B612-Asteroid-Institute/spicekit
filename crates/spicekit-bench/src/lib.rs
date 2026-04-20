//! Shared helpers for the spicekit-vs-CSpice bench binary and the
//! integration parity tests.
//!
//! The only dispatch logic intentionally duplicated here (rather than
//! exposed from the `spicekit` library) is the multi-reader backend
//! that spkez/pxform/sxform go through. In production, adam-core's
//! `RustBackend` owns that dispatch on the Python side; this crate
//! re-implements it in Rust so the comparison lines up with what
//! CSpice does after a batch of `furnsh_c` calls. Keeping it here
//! (and not in the library) avoids baking a specific kernel-list
//! semantic into the public spicekit surface.

#[cfg(feature = "cspice")]
pub mod cspice_wrap;
pub mod kernels;

use std::path::{Path, PathBuf};

use spicekit::frame::{
    invert_sxform, j2000_to_eclipj2000, pck_euler_rotation_and_derivative, sxform_from_rotation,
    NaifFrame,
};
use spicekit::naif_ids;
use spicekit::pck::{PckError, PckFile};
use spicekit::spk::{SpkError, SpkFile};
use spicekit::text_kernel::{parse_body_bindings, BodyBinding};

/// MJD of J2000 epoch in TDB scale.
const J2000_TDB_MJD: f64 = 51544.5;
/// Seconds per day.
const SECONDS_PER_DAY: f64 = 86_400.0;
/// NAIF frame code for ITRF93.
const ITRF93_FRAME_CODE: i32 = 3000;

/// Deterministic ET grid spanning an interval inside the overlap of
/// DE440 and the three Earth PCKs (MJD TDB 59000..60500, ≈2020–2024).
/// Matches `_make_ets` in `migration/scripts/spice_backend_benchmark.py`
/// and `_sample_ets` in `tests/test_spice_backend.py`.
pub fn make_ets(n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![mjd_tdb_to_et(59000.0)];
    }
    let start = 59000.0_f64;
    let end = 60500.0_f64;
    let step = (end - start) / ((n - 1) as f64);
    (0..n)
        .map(|i| mjd_tdb_to_et(start + step * i as f64))
        .collect()
}

/// Exact MJDs from `_sample_ets` in adam-core's test_spice_backend.py,
/// used for parity comparisons.
pub fn parity_sample_ets() -> Vec<f64> {
    [60000.0, 60000.5, 60001.0, 60100.0, 59500.0]
        .into_iter()
        .map(mjd_tdb_to_et)
        .collect()
}

fn mjd_tdb_to_et(mjd_tdb: f64) -> f64 {
    (mjd_tdb - J2000_TDB_MJD) * SECONDS_PER_DAY
}

#[derive(Debug)]
pub enum BackendError {
    NotCovered(String),
    Spk(SpkError),
    Pck(PckError),
    Io(std::io::Error),
    Text(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::NotCovered(s) => write!(f, "not covered: {s}"),
            BackendError::Spk(e) => write!(f, "spk: {e}"),
            BackendError::Pck(e) => write!(f, "pck: {e}"),
            BackendError::Io(e) => write!(f, "io: {e}"),
            BackendError::Text(s) => write!(f, "text kernel: {s}"),
        }
    }
}

impl std::error::Error for BackendError {}

enum Loaded {
    Spk(SpkFile),
    Pck(PckFile),
    Text(Vec<BodyBinding>),
    Ignored,
}

struct Kernel {
    path: PathBuf,
    content: Loaded,
}

/// Minimal spicekit-side analogue of adam-core's `RustBackend` — tracks
/// a list of loaded kernels in furnsh order and routes batch queries
/// through them using the same last-loaded-wins semantics. Used by the
/// bench binary and the integration parity tests.
#[derive(Default)]
pub struct Backend {
    kernels: Vec<Kernel>,
}

impl Backend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn furnsh(&mut self, path: &Path) -> Result<(), BackendError> {
        for k in &self.kernels {
            if k.path == path {
                return Ok(());
            }
        }
        let idword = peek_daf_idword(path).map_err(BackendError::Io)?;
        if let Some(w) = &idword {
            if w.starts_with(b"DAF/SPK") {
                let spk = SpkFile::open(path).map_err(BackendError::Spk)?;
                self.kernels.push(Kernel {
                    path: path.to_path_buf(),
                    content: Loaded::Spk(spk),
                });
                return Ok(());
            }
            if w.starts_with(b"DAF/PCK") {
                let pck = PckFile::open(path).map_err(BackendError::Pck)?;
                self.kernels.push(Kernel {
                    path: path.to_path_buf(),
                    content: Loaded::Pck(pck),
                });
                return Ok(());
            }
        }
        match parse_body_bindings(path) {
            Ok(bindings) if !bindings.is_empty() => {
                self.kernels.push(Kernel {
                    path: path.to_path_buf(),
                    content: Loaded::Text(bindings),
                });
            }
            Ok(_) => {
                self.kernels.push(Kernel {
                    path: path.to_path_buf(),
                    content: Loaded::Ignored,
                });
            }
            Err(e) => return Err(BackendError::Text(e.to_string())),
        }
        Ok(())
    }

    pub fn unload(&mut self, path: &Path) {
        self.kernels.retain(|k| k.path != path);
    }

    fn spk_readers_newest_first(&self) -> impl Iterator<Item = &SpkFile> {
        self.kernels.iter().rev().filter_map(|k| match &k.content {
            Loaded::Spk(s) => Some(s),
            _ => None,
        })
    }

    fn pck_readers_newest_first(&self) -> impl Iterator<Item = &PckFile> {
        self.kernels.iter().rev().filter_map(|k| match &k.content {
            Loaded::Pck(p) => Some(p),
            _ => None,
        })
    }

    pub fn spkez_batch(
        &self,
        target: i32,
        observer: i32,
        frame: &str,
        ets: &[f64],
    ) -> Result<Vec<[f64; 6]>, BackendError> {
        match frame {
            "J2000" | "ECLIPJ2000" => {
                let out_frame = if frame == "J2000" {
                    NaifFrame::J2000
                } else {
                    NaifFrame::EclipJ2000
                };
                // Newest-first — mirror RustBackend's fallback loop over
                // readers. First reader that can satisfy every ET wins.
                let mut last_err: Option<SpkError> = None;
                for reader in self.spk_readers_newest_first() {
                    match try_states_in_frame(reader, target, observer, ets, out_frame) {
                        Ok(v) => return Ok(v),
                        Err(e) => last_err = Some(e),
                    }
                }
                Err(match last_err {
                    Some(e) => BackendError::Spk(e),
                    None => BackendError::NotCovered(format!(
                        "no SPK loaded (target={target} observer={observer} frame={frame})"
                    )),
                })
            }
            "ITRF93" => {
                let sxform_stack = self.sxform_batch("J2000", "ITRF93", ets)?;
                let j2000 = self.spkez_batch(target, observer, "J2000", ets)?;
                let mut out = Vec::with_capacity(ets.len());
                for (i, m) in sxform_stack.iter().enumerate() {
                    out.push(apply6(m, &j2000[i]));
                }
                Ok(out)
            }
            other => Err(BackendError::NotCovered(format!(
                "frame {other} not supported (expected J2000 / ECLIPJ2000 / ITRF93)"
            ))),
        }
    }

    pub fn pxform_batch(
        &self,
        frame_from: &str,
        frame_to: &str,
        ets: &[f64],
    ) -> Result<Vec<[[f64; 3]; 3]>, BackendError> {
        let sxforms = self.sxform_batch(frame_from, frame_to, ets)?;
        Ok(sxforms.iter().map(extract_rotation).collect())
    }

    pub fn sxform_batch(
        &self,
        frame_from: &str,
        frame_to: &str,
        ets: &[f64],
    ) -> Result<Vec<[[f64; 6]; 6]>, BackendError> {
        if frame_from != "ITRF93" && frame_to != "ITRF93" {
            return Err(BackendError::NotCovered(format!(
                "sxform({frame_from},{frame_to}): at least one side must be ITRF93"
            )));
        }
        let (inertial_name, body_is_to) = if is_inertial(frame_from) && frame_to == "ITRF93" {
            (frame_from, true)
        } else if is_inertial(frame_to) && frame_from == "ITRF93" {
            (frame_to, false)
        } else {
            return Err(BackendError::NotCovered(format!(
                "sxform({frame_from},{frame_to}): one side must be J2000 / ECLIPJ2000, the other ITRF93"
            )));
        };

        let mut last_err: Option<PckError> = None;
        for reader in self.pck_readers_newest_first() {
            match build_sxform_stack(reader, ets, inertial_name, body_is_to) {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }
        Err(match last_err {
            Some(e) => BackendError::Pck(e),
            None => BackendError::NotCovered("no PCK loaded".to_string()),
        })
    }

    /// Custom-name lookups use last-loaded-wins (text kernels walked in
    /// forward order, later assignments overwrite earlier ones).
    pub fn bodn2c(&self, name: &str) -> Result<i32, BackendError> {
        let key = normalize_body_name(name);
        let mut custom: Option<i32> = None;
        for k in &self.kernels {
            if let Loaded::Text(bindings) = &k.content {
                for b in bindings {
                    if normalize_body_name(&b.name) == key {
                        custom = Some(b.code);
                    }
                }
            }
        }
        if let Some(c) = custom {
            return Ok(c);
        }
        naif_ids::bodn2c(name).map_err(|e| BackendError::NotCovered(e.to_string()))
    }
}

fn peek_daf_idword(path: &Path) -> std::io::Result<Option<[u8; 8]>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = [0u8; 8];
    let n = f.read(&mut buf)?;
    if n < 8 || !buf.starts_with(b"DAF/") {
        return Ok(None);
    }
    Ok(Some(buf))
}

fn is_inertial(name: &str) -> bool {
    matches!(name, "J2000" | "ECLIPJ2000")
}

/// Apply a 6×6 to a 6-vector. Kept local so the lib has no extra math
/// dep besides what spicekit already exposes.
fn apply6(m: &[[f64; 6]; 6], v: &[f64; 6]) -> [f64; 6] {
    let mut out = [0.0; 6];
    for i in 0..6 {
        let mut acc = 0.0;
        for j in 0..6 {
            acc += m[i][j] * v[j];
        }
        out[i] = acc;
    }
    out
}

fn extract_rotation(m: &[[f64; 6]; 6]) -> [[f64; 3]; 3] {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m[i][j];
        }
    }
    r
}

fn matmul6(a: &[[f64; 6]; 6], b: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut c = [[0.0f64; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            let mut acc = 0.0;
            for k in 0..6 {
                acc += a[i][k] * b[k][j];
            }
            c[i][j] = acc;
        }
    }
    c
}

fn try_states_in_frame(
    reader: &SpkFile,
    target: i32,
    center: i32,
    ets: &[f64],
    out_frame: NaifFrame,
) -> Result<Vec<[f64; 6]>, SpkError> {
    let mut out = Vec::with_capacity(ets.len());
    for &et in ets {
        out.push(reader.state_in_frame(target, center, et, out_frame)?);
    }
    Ok(out)
}

/// Matches the `sxform_matrix` method on adam-core's `NaifPck`:
/// compose the static inter-inertial 3×3 with the PCK's Euler-angle
/// `ref_frame → body` rotation, and invert if the inertial side is
/// the "to" argument.
fn build_sxform_stack(
    reader: &PckFile,
    ets: &[f64],
    inertial_name: &str,
    body_is_to: bool,
) -> Result<Vec<[[f64; 6]; 6]>, PckError> {
    let mut out = Vec::with_capacity(ets.len());
    for &et in ets {
        let (ref_frame, euler) = reader.euler_state_with_ref(ITRF93_FRAME_CODE, et)?;
        let (r, dr) = pck_euler_rotation_and_derivative(
            euler[0], euler[1], euler[2], euler[3], euler[4], euler[5],
        );
        let ref_to_body = sxform_from_rotation(&r, &dr);
        let target_to_ref = static_inter_inertial(inertial_name, ref_frame);
        let target_to_ref_6x6 = sxform_from_rotation(&target_to_ref, &[[0.0f64; 3]; 3]);
        let target_to_body = matmul6(&ref_to_body, &target_to_ref_6x6);
        out.push(if body_is_to {
            target_to_body
        } else {
            invert_sxform(&target_to_body)
        });
    }
    Ok(out)
}

fn static_inter_inertial(target_name: &str, ref_id: i32) -> [[f64; 3]; 3] {
    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    match (target_name, ref_id) {
        ("J2000", 1) | ("ECLIPJ2000", 17) => IDENTITY,
        ("J2000", 17) => j2000_to_eclipj2000(),
        ("ECLIPJ2000", 1) => {
            let s = j2000_to_eclipj2000();
            [
                [s[0][0], s[1][0], s[2][0]],
                [s[0][1], s[1][1], s[2][1]],
                [s[0][2], s[1][2], s[2][2]],
            ]
        }
        _ => panic!(
            "unsupported PCK reference frame id={ref_id} for target={target_name} \
             (expected 1=J2000 or 17=ECLIPJ2000)"
        ),
    }
}

/// Mirror adam-core's `_normalize_body_name`: uppercase, collapse
/// whitespace, strip trailing/leading spaces. Used only for custom
/// bindings — built-in lookups go straight through `naif_ids::bodn2c`.
fn normalize_body_name(raw: &str) -> String {
    let upper = raw.to_uppercase();
    let mut out = String::with_capacity(upper.len());
    let mut last_space = true;
    for c in upper.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.trim().to_string()
}
