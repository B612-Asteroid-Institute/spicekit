//! Python bindings for the spicekit crate.
//!
//! Exposes pure-Rust NAIF kernel readers (SPK, PCK, text kernels) and
//! the SPK writer to Python via PyO3. This is the extension module that
//! backs the `spicekit` Python package — there is no CSpice linkage.

#![allow(clippy::useless_conversion, clippy::too_many_arguments)]

use numpy::{IntoPyArray, PyArray2, PyArray3, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use spicekit::frame::{
    apply_sxform, invert_sxform, j2000_to_eclipj2000, pck_euler_rotation_and_derivative,
    sxform_from_rotation,
};
use spicekit::naif_ids;
use spicekit::spk_writer::{
    SpkWriter as SpkWriterRs, SpkWriterError, Type3Record as Type3RecordRs,
    Type3Segment as Type3SegmentRs, Type9Segment as Type9SegmentRs,
};
use spicekit::text_kernel;
use spicekit::{NaifFrame, PckError, PckFile, SpkError, SpkFile};

fn spk_err_to_py(err: SpkError) -> PyErr {
    PyRuntimeError::new_err(format!("{err}"))
}

fn parse_naif_frame(name: &str) -> PyResult<NaifFrame> {
    match name {
        "J2000" => Ok(NaifFrame::J2000),
        "ECLIPJ2000" => Ok(NaifFrame::EclipJ2000),
        _ => Err(PyValueError::new_err(format!(
            "unsupported NAIF frame: {name} (supported: J2000, ECLIPJ2000)"
        ))),
    }
}

/// Pure-Rust SPK reader. Opens a DAF/SPK once (mmap-backed, cheap to
/// clone internally) and evaluates states at TDB ephemeris-time seconds.
#[pyclass]
struct NaifSpk {
    inner: SpkFile,
}

#[pymethods]
impl NaifSpk {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let inner = SpkFile::open(path).map_err(spk_err_to_py)?;
        Ok(NaifSpk { inner })
    }

    fn state(&self, target: i32, center: i32, et: f64) -> PyResult<(f64, f64, f64, f64, f64, f64)> {
        let s = self
            .inner
            .state(target, center, et)
            .map_err(spk_err_to_py)?;
        Ok((s[0], s[1], s[2], s[3], s[4], s[5]))
    }

    fn state_batch<'py>(
        &self,
        py: Python<'py>,
        target: i32,
        center: i32,
        ets: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let ets = ets.as_slice()?;
        let mut out = ndarray::Array2::<f64>::zeros((ets.len(), 6));
        for (i, &et) in ets.iter().enumerate() {
            let s = self
                .inner
                .state(target, center, et)
                .map_err(spk_err_to_py)?;
            for k in 0..6 {
                out[[i, k]] = s[k];
            }
        }
        Ok(out.into_pyarray_bound(py))
    }

    /// Batched state lookup with an explicit NAIF output frame. Accepts
    /// "J2000" or "ECLIPJ2000" (case-sensitive to match CSPICE).
    fn state_batch_in_frame<'py>(
        &self,
        py: Python<'py>,
        target: i32,
        center: i32,
        ets: PyReadonlyArray1<'py, f64>,
        frame: &str,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let out_frame = parse_naif_frame(frame)?;
        let ets = ets.as_slice()?;
        let mut out = ndarray::Array2::<f64>::zeros((ets.len(), 6));
        for (i, &et) in ets.iter().enumerate() {
            let s = self
                .inner
                .state_in_frame(target, center, et, out_frame)
                .map_err(spk_err_to_py)?;
            for k in 0..6 {
                out[[i, k]] = s[k];
            }
        }
        Ok(out.into_pyarray_bound(py))
    }

    fn segments(&self) -> Vec<(i32, i32, i32, i32, f64, f64, String)> {
        self.inner
            .segments()
            .iter()
            .map(|s| {
                (
                    s.target,
                    s.center,
                    s.frame,
                    s.data_type,
                    s.start_et,
                    s.end_et,
                    s.name.clone(),
                )
            })
            .collect()
    }
}

fn spk_writer_err_to_py(err: SpkWriterError) -> PyErr {
    PyRuntimeError::new_err(format!("{err}"))
}

/// Look up a NAIF body ID by name in the built-in body-code table
/// (CSpice's full 692-entry zzidmap). Raises `ValueError` when the name
/// is not present; kernel-supplied bindings are handled separately on
/// the Python side and take precedence over this table.
#[pyfunction]
fn naif_bodn2c(name: &str) -> PyResult<i32> {
    naif_ids::bodn2c(name).map_err(|e| PyValueError::new_err(format!("{e}")))
}

/// Reverse: NAIF ID → canonical built-in body name. Raises `ValueError`
/// for IDs outside the built-in table.
#[pyfunction]
fn naif_bodc2n(code: i32) -> PyResult<String> {
    naif_ids::bodc2n(code)
        .map(|s| s.to_string())
        .map_err(|e| PyValueError::new_err(format!("{e}")))
}

/// Parse a SPICE text kernel (.tk/.tf/.tpc/.ti) and return the ordered
/// list of `NAIF_BODY_NAME` ↔ `NAIF_BODY_CODE` bindings it declares, as
/// `[(name, code), ...]`. Non-body assignments are ignored. Returns an
/// empty list if the file contains no `\begindata` blocks with body
/// bindings. Raises `ValueError` on parse errors or mismatched array
/// lengths.
#[pyfunction]
fn naif_parse_text_kernel_bindings(path: &str) -> PyResult<Vec<(String, i32)>> {
    text_kernel::parse_body_bindings(std::path::Path::new(path))
        .map(|v| v.into_iter().map(|b| (b.name, b.code)).collect())
        .map_err(|e| PyValueError::new_err(format!("{e}")))
}

/// Pure-Rust SPK writer. Emits a DAF/SPK file with one or more Type 3
/// (Chebyshev position+velocity) and/or Type 9 (Lagrange) segments. All
/// bytes are assembled in memory and committed with an atomic rename,
/// so partial files never survive a crash.
#[pyclass]
struct NaifSpkWriter {
    inner: SpkWriterRs,
}

#[pymethods]
impl NaifSpkWriter {
    #[new]
    #[pyo3(signature = (locifn = "adam-core"))]
    fn new(locifn: &str) -> Self {
        NaifSpkWriter {
            inner: SpkWriterRs::new_spk(locifn),
        }
    }

    /// Append a Type 3 segment.
    ///
    /// `records_coeffs` is shape (n_records, 2 + 6*(degree+1)) where each
    /// row is [mid, radius, x[0..N], y[0..N], z[0..N], vx[0..N], vy[0..N], vz[0..N]].
    #[pyo3(signature = (target, center, frame_id, start_et, end_et, segment_id, init, intlen, records_coeffs))]
    fn add_type3<'py>(
        &mut self,
        target: i32,
        center: i32,
        frame_id: i32,
        start_et: f64,
        end_et: f64,
        segment_id: &str,
        init: f64,
        intlen: f64,
        records_coeffs: PyReadonlyArray2<'py, f64>,
    ) -> PyResult<()> {
        let arr = records_coeffs.as_array();
        let row_len = arr.ncols();
        if row_len < 2 + 6 {
            return Err(PyValueError::new_err(
                "records_coeffs must have ≥ 2+6 columns (mid, radius, then ≥1 coef per component)",
            ));
        }
        let coef_block = row_len - 2;
        if coef_block % 6 != 0 {
            return Err(PyValueError::new_err(
                "records_coeffs row length - 2 must be a multiple of 6",
            ));
        }
        let n_coef = coef_block / 6;
        let mut records = Vec::with_capacity(arr.nrows());
        for row in arr.rows() {
            let mid = row[0];
            let radius = row[1];
            let slice = |start: usize| -> Vec<f64> {
                row.slice(ndarray::s![start..start + n_coef]).to_vec()
            };
            records.push(Type3RecordRs {
                mid,
                radius,
                x: slice(2),
                y: slice(2 + n_coef),
                z: slice(2 + 2 * n_coef),
                vx: slice(2 + 3 * n_coef),
                vy: slice(2 + 4 * n_coef),
                vz: slice(2 + 5 * n_coef),
            });
        }
        self.inner
            .add_type3(Type3SegmentRs {
                target,
                center,
                frame_id,
                start_et,
                end_et,
                segment_id: segment_id.to_string(),
                init,
                intlen,
                records,
            })
            .map_err(spk_writer_err_to_py)
    }

    /// Append a Type 9 (Lagrange, unequal time steps) segment.
    ///
    /// `states` is shape (N, 6) [x, y, z, vx, vy, vz]; `epochs` is shape (N,).
    #[pyo3(signature = (target, center, frame_id, start_et, end_et, segment_id, degree, states, epochs))]
    fn add_type9<'py>(
        &mut self,
        target: i32,
        center: i32,
        frame_id: i32,
        start_et: f64,
        end_et: f64,
        segment_id: &str,
        degree: i32,
        states: PyReadonlyArray2<'py, f64>,
        epochs: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<()> {
        let states_arr = states.as_array();
        if states_arr.ncols() != 6 {
            return Err(PyValueError::new_err("states must have shape (N, 6)"));
        }
        let epochs_slice = epochs.as_slice()?;
        if states_arr.nrows() != epochs_slice.len() {
            return Err(PyValueError::new_err(
                "states rows must match epochs length",
            ));
        }
        let mut flat = Vec::with_capacity(6 * epochs_slice.len());
        for row in states_arr.rows() {
            flat.extend_from_slice(row.as_slice().unwrap_or(&row.to_vec()));
        }
        self.inner
            .add_type9(Type9SegmentRs {
                target,
                center,
                frame_id,
                start_et,
                end_et,
                segment_id: segment_id.to_string(),
                degree,
                states: flat,
                epochs: epochs_slice.to_vec(),
            })
            .map_err(spk_writer_err_to_py)
    }

    /// Write the assembled SPK to `path` (atomic rename).
    fn write(&self, path: &str) -> PyResult<()> {
        self.inner.write(path).map_err(spk_writer_err_to_py)
    }
}

fn pck_err_to_py(err: PckError) -> PyErr {
    PyRuntimeError::new_err(format!("{err}"))
}

/// Numeric body-frame code for a NAIF name. We only recognize the
/// frames adam-core actually uses.
fn body_frame_code(name: &str) -> PyResult<i32> {
    match name {
        "ITRF93" => Ok(3000),
        _ => Err(PyValueError::new_err(format!(
            "unsupported body-fixed frame: {name} (supported: ITRF93)"
        ))),
    }
}

/// Pure-Rust binary PCK reader. Opens a DAF/PCK once (mmap-backed,
/// cheap to clone internally) and evaluates Euler angles / state
/// transforms at TDB ephemeris-time seconds.
#[pyclass]
struct NaifPck {
    inner: PckFile,
}

#[pymethods]
impl NaifPck {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let inner = PckFile::open(path).map_err(pck_err_to_py)?;
        Ok(NaifPck { inner })
    }

    /// Raw Euler-angle evaluation: returns `[RA, DEC, W, dRA, dDEC, dW]`
    /// (radians and radians/second) at ET for the requested body-fixed
    /// frame. Frame name may be a NAIF string ("ITRF93") or the raw
    /// numeric body-frame code.
    fn euler_state(&self, body_frame: i32, et: f64) -> PyResult<(f64, f64, f64, f64, f64, f64)> {
        let s = self
            .inner
            .euler_state(body_frame, et)
            .map_err(pck_err_to_py)?;
        Ok((s[0], s[1], s[2], s[3], s[4], s[5]))
    }

    /// Assemble a 6×6 state-transform matrix mapping a state from
    /// `from` to `to` at ET. Equivalent to `sp.sxform(from, to, et)` for
    /// the (J2000 ↔ ITRF93) pair.
    fn sxform<'py>(
        &self,
        py: Python<'py>,
        from: &str,
        to: &str,
        et: f64,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let m = self.sxform_matrix(from, to, et)?;
        let flat: Vec<f64> = m.iter().flatten().copied().collect();
        let arr = ndarray::Array2::from_shape_vec((6, 6), flat)
            .map_err(|e| PyValueError::new_err(format!("sxform shape error: {e}")))?;
        Ok(arr.into_pyarray_bound(py))
    }

    /// Assemble a 3×3 rotation matrix (position-only) mapping from
    /// `from` to `to` at ET. Equivalent to `sp.pxform(from, to, et)`.
    fn pxform<'py>(
        &self,
        py: Python<'py>,
        from: &str,
        to: &str,
        et: f64,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let m = self.sxform_matrix(from, to, et)?;
        let mut rot = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                rot[i][j] = m[i][j];
            }
        }
        let flat: Vec<f64> = rot.iter().flatten().copied().collect();
        let arr = ndarray::Array2::from_shape_vec((3, 3), flat)
            .map_err(|e| PyValueError::new_err(format!("pxform shape error: {e}")))?;
        Ok(arr.into_pyarray_bound(py))
    }

    /// Batched 3×3 rotation evaluation. Returns shape `(N, 3, 3)` where
    /// row `i` is `pxform(from, to, ets[i])`. One Rust call amortizes the
    /// PyO3 boundary cost across all epochs — intended for per-epoch
    /// loops that currently pay that cost per call.
    fn pxform_batch<'py>(
        &self,
        py: Python<'py>,
        from: &str,
        to: &str,
        ets: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray3<f64>>> {
        let ets = ets.as_slice()?;
        let mut out = ndarray::Array3::<f64>::zeros((ets.len(), 3, 3));
        for (i, &et) in ets.iter().enumerate() {
            let m = self.sxform_matrix(from, to, et)?;
            for r in 0..3 {
                for c in 0..3 {
                    out[[i, r, c]] = m[r][c];
                }
            }
        }
        Ok(out.into_pyarray_bound(py))
    }

    /// Batched 6×6 state-transform evaluation. Returns shape `(N, 6, 6)`.
    fn sxform_batch<'py>(
        &self,
        py: Python<'py>,
        from: &str,
        to: &str,
        ets: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray3<f64>>> {
        let ets = ets.as_slice()?;
        let mut out = ndarray::Array3::<f64>::zeros((ets.len(), 6, 6));
        for (i, &et) in ets.iter().enumerate() {
            let m = self.sxform_matrix(from, to, et)?;
            for r in 0..6 {
                for c in 0..6 {
                    out[[i, r, c]] = m[r][c];
                }
            }
        }
        Ok(out.into_pyarray_bound(py))
    }

    /// Apply the 6×6 state transform to a batch of 6-vector states.
    /// `ets[i]` and `states[i,:]` are paired; output matches shape of
    /// `states`.
    fn rotate_state_batch<'py>(
        &self,
        py: Python<'py>,
        from: &str,
        to: &str,
        ets: PyReadonlyArray1<'py, f64>,
        states: PyReadonlyArray2<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let ets = ets.as_slice()?;
        let s = states.as_array();
        if s.ncols() != 6 {
            return Err(PyValueError::new_err("states must have shape (N, 6)"));
        }
        if s.nrows() != ets.len() {
            return Err(PyValueError::new_err(
                "states and ets must have the same length",
            ));
        }
        let mut out = ndarray::Array2::<f64>::zeros((s.nrows(), 6));
        for (i, &et) in ets.iter().enumerate() {
            let m = self.sxform_matrix(from, to, et)?;
            let st = [
                s[[i, 0]],
                s[[i, 1]],
                s[[i, 2]],
                s[[i, 3]],
                s[[i, 4]],
                s[[i, 5]],
            ];
            let o = apply_sxform(&m, &st);
            for k in 0..6 {
                out[[i, k]] = o[k];
            }
        }
        Ok(out.into_pyarray_bound(py))
    }

    fn segments(&self) -> Vec<(i32, i32, i32, f64, f64, String)> {
        self.inner
            .segments()
            .iter()
            .map(|s| {
                (
                    s.body_frame,
                    s.ref_frame,
                    s.data_type,
                    s.start_et,
                    s.end_et,
                    s.name.clone(),
                )
            })
            .collect()
    }
}

impl NaifPck {
    /// Build a 6×6 state transform between an inertial frame and a
    /// body-fixed frame. One side of the pair must be a body-fixed
    /// frame ("ITRF93"), the other must be an inertial frame
    /// ("J2000" or "ECLIPJ2000"). The PCK segment's reference inertial
    /// frame can be either J2000 or ECLIPJ2000; a static inter-inertial
    /// rotation is composed when the requested inertial differs from
    /// the segment's reference.
    fn sxform_matrix(&self, from: &str, to: &str, et: f64) -> PyResult<[[f64; 6]; 6]> {
        let (inertial_name, body_name, body_is_to) = match (is_inertial(from), is_inertial(to)) {
            (true, false) => (from, to, true),
            (false, true) => (to, from, false),
            (true, true) => {
                return Err(PyValueError::new_err(format!(
                    "frame pair {from} -> {to}: both sides are inertial; this reader only \
                     handles body-fixed↔inertial pairs"
                )));
            }
            (false, false) => {
                return Err(PyValueError::new_err(format!(
                    "frame pair {from} -> {to}: neither side is inertial; one must be J2000 or \
                     ECLIPJ2000"
                )));
            }
        };
        let body_frame = body_frame_code(body_name)?;
        let (ref_frame, euler) = self
            .inner
            .euler_state_with_ref(body_frame, et)
            .map_err(pck_err_to_py)?;
        let (r, dr) = pck_euler_rotation_and_derivative(
            euler[0], euler[1], euler[2], euler[3], euler[4], euler[5],
        );
        // `r`/`dr` describe the rotation from `ref_frame` → body.
        let ref_to_body = sxform_from_rotation(&r, &dr);

        // Build the static inter-inertial 3×3 that maps the REQUESTED
        // inertial frame to the segment's REFERENCE inertial frame.
        let target_to_ref = static_inter_inertial(inertial_name, ref_frame)?;
        let target_to_ref_6x6 = sxform_from_rotation(&target_to_ref, &[[0.0f64; 3]; 3]);

        // M_target→body = M_ref→body · M_target→ref
        let target_to_body = matmul6(&ref_to_body, &target_to_ref_6x6);

        Ok(if body_is_to {
            target_to_body
        } else {
            invert_sxform(&target_to_body)
        })
    }
}

fn is_inertial(name: &str) -> bool {
    matches!(name, "J2000" | "ECLIPJ2000")
}

/// 3×3 rotation from a requested inertial frame (name) into a PCK
/// segment's reference inertial frame (by NAIF frame ID).
fn static_inter_inertial(target_name: &str, ref_id: i32) -> PyResult<[[f64; 3]; 3]> {
    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    match (target_name, ref_id) {
        ("J2000", 1) | ("ECLIPJ2000", 17) => Ok(IDENTITY),
        ("J2000", 17) => Ok(j2000_to_eclipj2000()),
        ("ECLIPJ2000", 1) => {
            let s = j2000_to_eclipj2000();
            Ok([
                [s[0][0], s[1][0], s[2][0]],
                [s[0][1], s[1][1], s[2][1]],
                [s[0][2], s[1][2], s[2][2]],
            ])
        }
        _ => Err(PyValueError::new_err(format!(
            "unsupported PCK reference frame {ref_id}; expected 1 (J2000) or 17 (ECLIPJ2000) \
             with target {target_name}"
        ))),
    }
}

#[allow(clippy::needless_range_loop)]
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

#[pymodule]
fn _rust_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(naif_bodn2c, m)?)?;
    m.add_function(wrap_pyfunction!(naif_bodc2n, m)?)?;
    m.add_function(wrap_pyfunction!(naif_parse_text_kernel_bindings, m)?)?;
    m.add_class::<NaifSpk>()?;
    m.add_class::<NaifSpkWriter>()?;
    m.add_class::<NaifPck>()?;
    Ok(())
}
