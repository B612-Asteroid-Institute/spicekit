//! SPK (Spacecraft and Planet Kernel) reader.
//!
//! Parses DAF segments into typed `SpkSegment`s and evaluates each
//! supported NAIF data type at a requested TDB ephemeris time. Returned
//! states are 6-vectors (km, km/s) in the segment's native frame, which
//! the caller may rotate to another frame. All times are TDB seconds
//! past J2000.
//!
//! Supported data types:
//! - Type 2: Chebyshev (position only). Velocity is the analytic
//!   derivative of the position polynomial (the standard CSPICE
//!   definition). This covers DE440 and other planetary ephemerides.
//!
//! Types 3, 9, and 13 will be added in a follow-up to cover SPKs we
//! generate ourselves and the JWST Horizons fixture. Calling `evaluate`
//! on an unsupported segment returns `SpkError::UnsupportedType`.

use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;

use crate::daf::{DafError, DafFile};
use crate::frame::{rotate_state, NaifFrame};

#[derive(Debug, Error)]
pub enum SpkError {
    #[error(transparent)]
    Daf(#[from] DafError),
    #[error("no segment covers target {target} wrt center {center} at et {et}")]
    NoCoverage { target: i32, center: i32, et: f64 },
    #[error("unsupported SPK data type {0}")]
    UnsupportedType(i32),
    #[error("malformed Type 2 segment: {0}")]
    BadType2(&'static str),
}

/// Fully-parsed SPK segment. Payload is lazily evaluable; the metadata
/// block is enough to route a lookup without reading coefficients.
#[derive(Clone)]
pub struct SpkSegment {
    pub target: i32,
    pub center: i32,
    pub frame: i32,
    pub data_type: i32,
    pub start_et: f64,
    pub end_et: f64,
    pub start_addr: u32,
    pub end_addr: u32,
    pub name: String,
    payload: SpkPayload,
}

#[derive(Clone)]
enum SpkPayload {
    Type2(SpkType2),
    Type3(SpkType3),
    Type9(SpkType9),
    Type13(SpkType13),
    Unsupported,
}

/// Type 2 segment: Chebyshev position, analytic-derivative velocity.
///
/// Record layout (all f64, little-endian):
///   [MID, RADIUS, Xcoef[0..N], Ycoef[0..N], Zcoef[0..N]]
/// where RSIZE = 2 + 3*N and N = polynomial_degree + 1.
///
/// Segment trailer (last 4 doubles of the segment):
///   [INIT, INTLEN, RSIZE, N_RECORDS]
/// INIT and INTLEN are the TDB epoch / length (seconds) of each record.
#[derive(Clone)]
struct SpkType2 {
    file: DafFile,
    init: f64,
    intlen: f64,
    rsize: usize,
    n_records: usize,
    n_coef: usize,
    start_addr: u32,
}

impl SpkType2 {
    fn from_segment(file: &DafFile, start_addr: u32, end_addr: u32) -> Result<Self, SpkError> {
        // Trailer: last 4 doubles of the segment.
        let trailer = file.read_doubles(end_addr - 3, end_addr)?;
        let init = trailer[0];
        let intlen = trailer[1];
        let rsize = trailer[2] as usize;
        let n_records = trailer[3] as usize;
        if rsize < 2 || (rsize - 2) % 3 != 0 {
            return Err(SpkError::BadType2("RSIZE not 2 + 3N"));
        }
        let n_coef = (rsize - 2) / 3;
        if n_coef == 0 {
            return Err(SpkError::BadType2("degree < 0"));
        }
        if intlen <= 0.0 {
            return Err(SpkError::BadType2("INTLEN <= 0"));
        }
        Ok(SpkType2 {
            file: file.clone(),
            init,
            intlen,
            rsize,
            n_records,
            n_coef,
            start_addr,
        })
    }

    fn evaluate(&self, et: f64) -> Result<[f64; 6], SpkError> {
        // Record selection: clamp to last record at the right edge to
        // match CSPICE semantics (the upper bound is inclusive).
        let raw_idx = ((et - self.init) / self.intlen).floor() as isize;
        let idx = raw_idx.clamp(0, self.n_records as isize - 1) as usize;
        let rec_start = self.start_addr + (idx * self.rsize) as u32;
        let rec_end = rec_start + self.rsize as u32 - 1;
        let rec = self.file.doubles_native(rec_start, rec_end)?;

        let mid = rec[0];
        let radius = rec[1];
        if radius == 0.0 {
            return Err(SpkError::BadType2("RADIUS == 0"));
        }
        let s = (et - mid) / radius;

        let n = self.n_coef;
        let xc = &rec[2..2 + n];
        let yc = &rec[2 + n..2 + 2 * n];
        let zc = &rec[2 + 2 * n..2 + 3 * n];

        let (pos, vel) = cheby3_val_and_deriv(xc, yc, zc, s);
        // Derivative is d/ds; chain rule to d/dt: ds/dt = 1/radius.
        let inv_r = 1.0 / radius;
        Ok([
            pos[0],
            pos[1],
            pos[2],
            vel[0] * inv_r,
            vel[1] * inv_r,
            vel[2] * inv_r,
        ])
    }
}

/// Type 3 segment: Chebyshev with separately-stored velocity
/// coefficients.
///
/// Record layout:
///   [MID, RADIUS, Xpos[N], Ypos[N], Zpos[N], Xvel[N], Yvel[N], Zvel[N]]
/// RSIZE = 2 + 6*N. The trailer `[INIT, INTLEN, RSIZE, N_RECORDS]` is
/// shared with Type 2.
///
/// Unlike Type 2, velocity is evaluated from its own coefficient block
/// (no chain-rule scaling by `1/RADIUS` on the position derivative).
#[derive(Clone)]
struct SpkType3 {
    file: DafFile,
    init: f64,
    intlen: f64,
    rsize: usize,
    n_records: usize,
    n_coef: usize,
    start_addr: u32,
}

impl SpkType3 {
    fn from_segment(file: &DafFile, start_addr: u32, end_addr: u32) -> Result<Self, SpkError> {
        let trailer = file.read_doubles(end_addr - 3, end_addr)?;
        let init = trailer[0];
        let intlen = trailer[1];
        let rsize = trailer[2] as usize;
        let n_records = trailer[3] as usize;
        if rsize < 2 || (rsize - 2) % 6 != 0 {
            return Err(SpkError::BadType2("RSIZE not 2 + 6N"));
        }
        let n_coef = (rsize - 2) / 6;
        if n_coef == 0 || intlen <= 0.0 {
            return Err(SpkError::BadType2("degree<0 or INTLEN<=0"));
        }
        Ok(SpkType3 {
            file: file.clone(),
            init,
            intlen,
            rsize,
            n_records,
            n_coef,
            start_addr,
        })
    }

    fn evaluate(&self, et: f64) -> Result<[f64; 6], SpkError> {
        let raw_idx = ((et - self.init) / self.intlen).floor() as isize;
        let idx = raw_idx.clamp(0, self.n_records as isize - 1) as usize;
        let rec_start = self.start_addr + (idx * self.rsize) as u32;
        let rec_end = rec_start + self.rsize as u32 - 1;
        let rec = self.file.doubles_native(rec_start, rec_end)?;

        let mid = rec[0];
        let radius = rec[1];
        let s = (et - mid) / radius;
        let n = self.n_coef;
        let xc = &rec[2..2 + n];
        // SAFETY: zero-cost shadow guard so the slice constructions
        // below match the validated record layout.
        debug_assert_eq!(rec.len(), self.rsize);
        let yc = &rec[2 + n..2 + 2 * n];
        let zc = &rec[2 + 2 * n..2 + 3 * n];
        let vxc = &rec[2 + 3 * n..2 + 4 * n];
        let vyc = &rec[2 + 4 * n..2 + 5 * n];
        let vzc = &rec[2 + 5 * n..2 + 6 * n];
        // Type 3 stores velocity as a separate Chebyshev series — no
        // derivative needed for either evaluation. Two value-only
        // 3-channel evaluations replace six scalar val-and-deriv calls.
        let pos = cheby3_val_only(xc, yc, zc, s);
        let vel = cheby3_val_only(vxc, vyc, vzc, s);
        Ok([pos[0], pos[1], pos[2], vel[0], vel[1], vel[2]])
    }
}

/// Shared trailer read for discrete-state segments (Types 8, 9, 12, 13).
///
/// Last two doubles of the segment are `[window_or_degree, N_states]`.
/// The epoch table sits immediately before the directory entries (one
/// directory entry per full group of 100 states; directory follows
/// the epoch table).
struct DiscreteMeta {
    window_or_degree: usize,
    n_states: usize,
    states_start: u32,
    epochs_start: u32,
}

impl DiscreteMeta {
    fn from_segment(file: &DafFile, start_addr: u32, end_addr: u32) -> Result<Self, SpkError> {
        let tail = file.read_doubles(end_addr - 1, end_addr)?;
        let window_or_degree = tail[0] as usize;
        let n_states = tail[1] as usize;
        if n_states == 0 {
            return Err(SpkError::BadType2("empty discrete-state segment"));
        }
        // Directory entries: 1 per completed group of 100 states.
        let n_dir = n_states / 100;
        // Segment body (indexed 1-based DAF addresses):
        //   start_addr .. start_addr + 6*N - 1   : states
        //   then N epochs
        //   then n_dir directory entries
        //   then 2 trailer doubles
        let states_start = start_addr;
        let epochs_start = start_addr + (6 * n_states) as u32;
        // Sanity check against end_addr.
        let expected_end = epochs_start + n_states as u32 + n_dir as u32 + 2 - 1;
        if expected_end != end_addr {
            return Err(SpkError::BadType2("segment size does not match trailer"));
        }
        Ok(DiscreteMeta {
            window_or_degree,
            n_states,
            states_start,
            epochs_start,
        })
    }
}

/// Type 9 segment: Lagrange interpolation of discrete states with
/// unequal time steps. Position and velocity components are
/// interpolated independently from their stored values.
#[derive(Clone)]
struct SpkType9 {
    file: DafFile,
    meta_degree: usize,
    n_states: usize,
    states_start: u32,
    epochs_start: u32,
}

impl SpkType9 {
    fn from_segment(file: &DafFile, start_addr: u32, end_addr: u32) -> Result<Self, SpkError> {
        let meta = DiscreteMeta::from_segment(file, start_addr, end_addr)?;
        Ok(SpkType9 {
            file: file.clone(),
            meta_degree: meta.window_or_degree,
            n_states: meta.n_states,
            states_start: meta.states_start,
            epochs_start: meta.epochs_start,
        })
    }

    fn evaluate(&self, et: f64) -> Result<[f64; 6], SpkError> {
        let window = self.meta_degree + 1;
        let (i0, count) = pick_window(&self.file, self.epochs_start, self.n_states, window, et)?;
        let epochs = self.file.doubles_native(
            self.epochs_start + i0 as u32,
            self.epochs_start + (i0 + count - 1) as u32,
        )?;
        let states = self.file.doubles_native(
            self.states_start + (6 * i0) as u32,
            self.states_start + (6 * (i0 + count) - 1) as u32,
        )?;
        let mut out = [0.0_f64; 6];
        let mut comp = vec![0.0_f64; count];
        for k in 0..6 {
            for j in 0..count {
                comp[j] = states[6 * j + k];
            }
            out[k] = lagrange_eval(epochs, &comp, et);
        }
        Ok(out)
    }
}

/// Type 13 segment: Hermite interpolation of discrete states (position
/// + velocity used as constraints) with unequal time steps.
#[derive(Clone)]
struct SpkType13 {
    file: DafFile,
    window_size: usize,
    n_states: usize,
    states_start: u32,
    epochs_start: u32,
}

impl SpkType13 {
    fn from_segment(file: &DafFile, start_addr: u32, end_addr: u32) -> Result<Self, SpkError> {
        let meta = DiscreteMeta::from_segment(file, start_addr, end_addr)?;
        // Type 13 trailer stores WINSIZ - 1 (not WINSIZ itself; this
        // differs from Type 9, which stores the polynomial degree =
        // WINSIZ - 1 as well but is consumed as degree).
        let window_size = meta.window_or_degree + 1;
        if window_size < 2 {
            return Err(SpkError::BadType2("Type 13 window size < 2"));
        }
        Ok(SpkType13 {
            file: file.clone(),
            window_size,
            n_states: meta.n_states,
            states_start: meta.states_start,
            epochs_start: meta.epochs_start,
        })
    }

    fn evaluate(&self, et: f64) -> Result<[f64; 6], SpkError> {
        let (i0, count) = pick_window(
            &self.file,
            self.epochs_start,
            self.n_states,
            self.window_size,
            et,
        )?;
        let epochs = self.file.doubles_native(
            self.epochs_start + i0 as u32,
            self.epochs_start + (i0 + count - 1) as u32,
        )?;
        let states = self.file.doubles_native(
            self.states_start + (6 * i0) as u32,
            self.states_start + (6 * (i0 + count) - 1) as u32,
        )?;
        let mut out = [0.0_f64; 6];
        // Each component uses position + velocity as Hermite
        // constraints, so a single call produces f = position and
        // df/dt = velocity. We only take the position output; the
        // velocity output we reuse by running Hermite again on the
        // other three components is avoided: instead we get both at
        // once per spatial axis.
        let mut pos_vals = vec![0.0_f64; count];
        let mut vel_vals = vec![0.0_f64; count];
        for axis in 0..3 {
            for j in 0..count {
                pos_vals[j] = states[6 * j + axis];
                vel_vals[j] = states[6 * j + 3 + axis];
            }
            let (p, v) = hermite_eval(epochs, &pos_vals, &vel_vals, et);
            out[axis] = p;
            out[3 + axis] = v;
        }
        Ok(out)
    }
}

/// Pick the contiguous window of `window` epoch indices centered on
/// the requested `et`. Binary-searches the epoch array and clamps to
/// the segment bounds (leading/trailing cases).
fn pick_window(
    file: &DafFile,
    epochs_start: u32,
    n_states: usize,
    window: usize,
    et: f64,
) -> Result<(usize, usize), SpkError> {
    if window == 0 {
        return Err(SpkError::BadType2("window size 0"));
    }
    let count = window.min(n_states);
    // Binary search for the greatest epoch <= et.
    let epochs = file.doubles_native(epochs_start, epochs_start + n_states as u32 - 1)?;
    let mut lo = 0usize;
    let mut hi = n_states;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if epochs[mid] <= et {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    // lo is first index with epochs[lo] > et; predecessor is lo-1.
    let left_idx = lo.saturating_sub(1);
    // CSPICE convention: window starts at `left_idx - floor((W-1)/2)`
    // and contains `W` consecutive knots. For odd W, two extra knots
    // are placed after `left_idx`; for even W, knots are split evenly
    // across the [left_idx, left_idx+1] target interval.
    let half = (count - 1) / 2;
    let start = left_idx.saturating_sub(half);
    let start = start.min(n_states - count);
    Ok((start, count))
}

/// Barycentric Lagrange evaluation on arbitrary nodes.
fn lagrange_eval(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let n = xs.len();
    // Exact-node shortcut.
    for i in 0..n {
        if xs[i] == x {
            return ys[i];
        }
    }
    // Precompute weights w_i = 1 / prod_{j!=i} (x_i - x_j).
    let mut w = vec![1.0_f64; n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                w[i] *= xs[i] - xs[j];
            }
        }
        w[i] = 1.0 / w[i];
    }
    let mut num = 0.0_f64;
    let mut den = 0.0_f64;
    for i in 0..n {
        let term = w[i] / (x - xs[i]);
        num += term * ys[i];
        den += term;
    }
    num / den
}

/// Hermite interpolation at arbitrary nodes with paired value +
/// derivative constraints. Returns `(f(x), f'(x))`.
///
/// Uses the standard "doubled nodes" divided-difference scheme: treat
/// each node as repeated, seed divided differences with the derivative
/// across each doubled pair, and then apply the Newton form.
fn hermite_eval(xs: &[f64], ys: &[f64], dys: &[f64], x: f64) -> (f64, f64) {
    let n = xs.len();
    debug_assert_eq!(ys.len(), n);
    debug_assert_eq!(dys.len(), n);
    let m = 2 * n;
    // z = doubled nodes, f = divided-difference table (column-major: we
    // overwrite f in place).
    let mut z = vec![0.0_f64; m];
    let mut f = vec![0.0_f64; m];
    for i in 0..n {
        z[2 * i] = xs[i];
        z[2 * i + 1] = xs[i];
        f[2 * i] = ys[i];
        f[2 * i + 1] = ys[i];
    }
    // First-order differences: for paired nodes, use the derivative;
    // otherwise the standard divided difference.
    let prev = f.clone();
    let mut col = vec![0.0_f64; m];
    for i in 0..m - 1 {
        if (i % 2) == 0 {
            col[i] = dys[i / 2];
        } else {
            col[i] = (prev[i + 1] - prev[i]) / (z[i + 1] - z[i]);
        }
    }
    // Keep diagonal entries as Newton coefficients.
    let mut coeffs = vec![0.0_f64; m];
    coeffs[0] = f[0];
    coeffs[1] = col[0];
    // Higher-order columns: col_k[i] = (col_{k-1}[i+1] - col_{k-1}[i]) / (z[i+k] - z[i]).
    let mut cur = col.clone();
    for k in 2..m {
        let mut next = vec![0.0_f64; m - k];
        for i in 0..m - k {
            next[i] = (cur[i + 1] - cur[i]) / (z[i + k] - z[i]);
        }
        coeffs[k] = next[0];
        cur = next;
    }
    // Evaluate Newton form: f(x) = sum coeffs[k] * prod_{j<k} (x - z[j]).
    // Derivative via product-rule accumulation alongside.
    let mut val = coeffs[0];
    let mut der = 0.0_f64;
    let mut prod = 1.0_f64; // prod_{j<k} (x - z[j]) at step k
    let mut dprod = 0.0_f64; // d/dx of prod up to k
    for k in 1..m {
        // Update derivative of prod: d[(x - z[k-1]) * prod_old]/dx
        //                        = prod_old + (x - z[k-1]) * dprod_old
        let dprod_new = prod + (x - z[k - 1]) * dprod;
        let prod_new = (x - z[k - 1]) * prod;
        val += coeffs[k] * prod_new;
        der += coeffs[k] * dprod_new;
        prod = prod_new;
        dprod = dprod_new;
    }
    (val, der)
}

/// Evaluate a Chebyshev-T series and its first derivative at `s`.
///
/// Returns `(f(s), f'(s))` where f = sum_{k=0}^{n-1} c[k] T_k(s) and the
/// derivative is with respect to `s` (caller applies the chain-rule
/// scaling). Uses the direct three-term recurrence.
///
/// Kept as a single-channel reference implementation only; production
/// callers in SPK/PCK Type 2 use the shared 3-channel evaluators
/// ([`cheby3_val_and_deriv`] / [`cheby3_val_only`]). The parity tests
/// in `cheby3_parity_tests` verify the 3-channel evaluators reproduce
/// this scalar version bit-for-bit.
#[cfg(test)]
pub(crate) fn cheby_val_and_deriv(c: &[f64], s: f64) -> (f64, f64) {
    let n = c.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    if n == 1 {
        return (c[0], 0.0);
    }
    let mut t_prev = 1.0;
    let mut t_curr = s;
    let mut dt_prev = 0.0;
    let mut dt_curr = 1.0;
    let mut val = c[0] * t_prev + c[1] * t_curr;
    let mut der = c[1] * dt_curr;
    for &ck in &c[2..] {
        let t_next = 2.0 * s * t_curr - t_prev;
        let dt_next = 2.0 * t_curr + 2.0 * s * dt_curr - dt_prev;
        val += ck * t_next;
        der += ck * dt_next;
        t_prev = t_curr;
        t_curr = t_next;
        dt_prev = dt_curr;
        dt_curr = dt_next;
    }
    (val, der)
}

/// Three-channel Chebyshev evaluation with shared basis-function
/// recurrence. Equivalent to calling [`cheby_val_and_deriv`] three
/// times on `(cx, cy, cz)` at the same `s`, but computes `T_k(s)` and
/// `dT_k/ds` once per iteration and applies them to all three
/// channels. The per-channel arithmetic order is identical to the
/// scalar variant, so the output is bit-for-bit equivalent.
///
/// All three slices must have the same length (a Type 2/3 record
/// invariant; debug-asserted).
#[inline]
pub(crate) fn cheby3_val_and_deriv(
    cx: &[f64],
    cy: &[f64],
    cz: &[f64],
    s: f64,
) -> ([f64; 3], [f64; 3]) {
    let n = cx.len();
    debug_assert_eq!(cy.len(), n);
    debug_assert_eq!(cz.len(), n);
    if n == 0 {
        return ([0.0; 3], [0.0; 3]);
    }
    if n == 1 {
        return ([cx[0], cy[0], cz[0]], [0.0; 3]);
    }
    let mut t_prev = 1.0;
    let mut t_curr = s;
    let mut dt_prev = 0.0_f64;
    let mut dt_curr = 1.0;
    let mut val = [
        cx[0] * t_prev + cx[1] * t_curr,
        cy[0] * t_prev + cy[1] * t_curr,
        cz[0] * t_prev + cz[1] * t_curr,
    ];
    let mut der = [cx[1] * dt_curr, cy[1] * dt_curr, cz[1] * dt_curr];
    let two_s = 2.0 * s;
    for k in 2..n {
        let t_next = two_s * t_curr - t_prev;
        let dt_next = 2.0 * t_curr + two_s * dt_curr - dt_prev;
        val[0] += cx[k] * t_next;
        val[1] += cy[k] * t_next;
        val[2] += cz[k] * t_next;
        der[0] += cx[k] * dt_next;
        der[1] += cy[k] * dt_next;
        der[2] += cz[k] * dt_next;
        t_prev = t_curr;
        t_curr = t_next;
        dt_prev = dt_curr;
        dt_curr = dt_next;
    }
    (val, der)
}

/// Three-channel Chebyshev value-only evaluation (no derivative).
/// Used by SPK Type 3, which stores velocity as a separate Chebyshev
/// series so the position-polynomial derivative is unused. Saves the
/// `dT_n/ds` recurrence and one fma per iteration per channel.
#[inline]
pub(crate) fn cheby3_val_only(cx: &[f64], cy: &[f64], cz: &[f64], s: f64) -> [f64; 3] {
    let n = cx.len();
    debug_assert_eq!(cy.len(), n);
    debug_assert_eq!(cz.len(), n);
    if n == 0 {
        return [0.0; 3];
    }
    if n == 1 {
        return [cx[0], cy[0], cz[0]];
    }
    let mut t_prev = 1.0;
    let mut t_curr = s;
    let mut val = [
        cx[0] * t_prev + cx[1] * t_curr,
        cy[0] * t_prev + cy[1] * t_curr,
        cz[0] * t_prev + cz[1] * t_curr,
    ];
    let two_s = 2.0 * s;
    for k in 2..n {
        let t_next = two_s * t_curr - t_prev;
        val[0] += cx[k] * t_next;
        val[1] += cy[k] * t_next;
        val[2] += cz[k] * t_next;
        t_prev = t_curr;
        t_curr = t_next;
    }
    val
}

/// A loaded SPK file: DAF plus per-segment metadata and a
/// (target, center) index for O(1) segment lookup.
pub struct SpkFile {
    segments: Vec<SpkSegment>,
    index: HashMap<(i32, i32), Vec<usize>>,
}

impl SpkFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, SpkError> {
        let daf = DafFile::open(path)?;
        Self::from_daf(daf)
    }

    pub fn from_daf(daf: DafFile) -> Result<Self, SpkError> {
        let mut segments = Vec::new();
        let mut index: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for summary in daf.summaries()? {
            if summary.doubles.len() < 2 || summary.integers.len() < 6 {
                continue;
            }
            let start_et = summary.doubles[0];
            let end_et = summary.doubles[1];
            let target = summary.integers[0];
            let center = summary.integers[1];
            let frame = summary.integers[2];
            let data_type = summary.integers[3];
            let start_addr = summary.integers[4] as u32;
            let end_addr = summary.integers[5] as u32;

            let payload = match data_type {
                2 => SpkPayload::Type2(SpkType2::from_segment(&daf, start_addr, end_addr)?),
                3 => SpkPayload::Type3(SpkType3::from_segment(&daf, start_addr, end_addr)?),
                9 => SpkPayload::Type9(SpkType9::from_segment(&daf, start_addr, end_addr)?),
                13 => SpkPayload::Type13(SpkType13::from_segment(&daf, start_addr, end_addr)?),
                _ => SpkPayload::Unsupported,
            };

            index
                .entry((target, center))
                .or_default()
                .push(segments.len());
            segments.push(SpkSegment {
                target,
                center,
                frame,
                data_type,
                start_et,
                end_et,
                start_addr,
                end_addr,
                name: summary.name,
                payload,
            });
        }
        Ok(SpkFile { segments, index })
    }

    pub fn segments(&self) -> &[SpkSegment] {
        &self.segments
    }

    /// State in the requested inertial frame. SPK segments are read in
    /// their native frame (J2000 for DE-series kernels) and rotated to
    /// `out_frame`.
    pub fn state_in_frame(
        &self,
        target: i32,
        center: i32,
        et: f64,
        out_frame: NaifFrame,
    ) -> Result<[f64; 6], SpkError> {
        let s = self.state(target, center, et)?;
        // Assume segment frame == J2000 (true for DE-series; other
        // cases will be flagged when we add non-J2000 SPKs).
        Ok(rotate_state(NaifFrame::J2000, out_frame, &s))
    }

    /// State of `target` relative to `center` at `et` (TDB seconds past
    /// J2000). Walks the body chain if no direct segment is available
    /// for the pair: computes state(target -> SSB) and state(center ->
    /// SSB) by recursively summing segments rooted at body 0 (SSB),
    /// then subtracts. All segments must share the same inertial frame;
    /// this holds for DE-series planetary ephemerides (all J2000).
    pub fn state(&self, target: i32, center: i32, et: f64) -> Result<[f64; 6], SpkError> {
        if target == center {
            return Ok([0.0; 6]);
        }
        if let Some(s) = self.try_direct(target, center, et)? {
            return Ok(s);
        }
        let t_ssb = self.state_wrt_ssb(target, et)?;
        let c_ssb = self.state_wrt_ssb(center, et)?;
        let mut out = [0.0_f64; 6];
        for i in 0..6 {
            out[i] = t_ssb[i] - c_ssb[i];
        }
        Ok(out)
    }

    /// Try to satisfy the pair from a single segment (or a single
    /// reverse segment by negation). Returns `Ok(None)` when neither
    /// direction has coverage at `et`.
    fn try_direct(&self, target: i32, center: i32, et: f64) -> Result<Option<[f64; 6]>, SpkError> {
        if let Some(indices) = self.index.get(&(target, center)) {
            for &i in indices {
                let seg = &self.segments[i];
                if et >= seg.start_et && et <= seg.end_et {
                    return Ok(Some(Self::eval_segment(seg, et)?));
                }
            }
        }
        if let Some(indices) = self.index.get(&(center, target)) {
            for &i in indices {
                let seg = &self.segments[i];
                if et >= seg.start_et && et <= seg.end_et {
                    let mut s = Self::eval_segment(seg, et)?;
                    for v in s.iter_mut() {
                        *v = -*v;
                    }
                    return Ok(Some(s));
                }
            }
        }
        Ok(None)
    }

    fn eval_segment(seg: &SpkSegment, et: f64) -> Result<[f64; 6], SpkError> {
        match &seg.payload {
            SpkPayload::Type2(t) => t.evaluate(et),
            SpkPayload::Type3(t) => t.evaluate(et),
            SpkPayload::Type9(t) => t.evaluate(et),
            SpkPayload::Type13(t) => t.evaluate(et),
            SpkPayload::Unsupported => Err(SpkError::UnsupportedType(seg.data_type)),
        }
    }

    /// State of `body` wrt SSB (NAIF 0) at `et`, by walking
    /// `body -> next_center -> ... -> 0`. Detects cycles and bails if
    /// the chain exceeds a conservative depth bound.
    fn state_wrt_ssb(&self, body: i32, et: f64) -> Result<[f64; 6], SpkError> {
        if body == 0 {
            return Ok([0.0; 6]);
        }
        let mut total = [0.0_f64; 6];
        let mut cur = body;
        // NAIF body chains are shallow (planet -> barycenter -> SSB is
        // depth 2); 32 is far beyond any realistic depth.
        for _ in 0..32 {
            let (delta, next_center) = self.step_toward_ssb(cur, et)?;
            for i in 0..6 {
                total[i] += delta[i];
            }
            if next_center == 0 {
                return Ok(total);
            }
            if next_center == body {
                return Err(SpkError::NoCoverage {
                    target: body,
                    center: 0,
                    et,
                });
            }
            cur = next_center;
        }
        Err(SpkError::NoCoverage {
            target: body,
            center: 0,
            et,
        })
    }

    /// Pick one segment whose target is `body`, whose covers `et`, and
    /// whose center is closer to SSB (prefer center = 0 when available;
    /// otherwise the first matching segment). Returns the segment state
    /// and the center body we advanced to.
    fn step_toward_ssb(&self, body: i32, et: f64) -> Result<([f64; 6], i32), SpkError> {
        let mut preferred: Option<(&SpkSegment, [f64; 6])> = None;
        for seg in &self.segments {
            if seg.target != body {
                continue;
            }
            if et < seg.start_et || et > seg.end_et {
                continue;
            }
            let s = Self::eval_segment(seg, et)?;
            if seg.center == 0 {
                return Ok((s, 0));
            }
            if preferred.is_none() {
                preferred = Some((seg, s));
            }
        }
        match preferred {
            Some((seg, s)) => Ok((s, seg.center)),
            None => Err(SpkError::NoCoverage {
                target: body,
                center: 0,
                et,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermite_reproduces_cubic() {
        // f(x) = x^3, so f'(x) = 3x^2. Two knots should reproduce exactly.
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let dys = [0.0, 3.0];
        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let (v, d) = hermite_eval(&xs, &ys, &dys, x);
            assert!((v - x * x * x).abs() < 1e-14, "v={v}");
            assert!((d - 3.0 * x * x).abs() < 1e-14, "d={d}");
        }
    }

    #[test]
    fn hermite_reproduces_quartic_with_three_knots() {
        // f(x) = x^4 - 2x^2 + 3, f'(x) = 4x^3 - 4x.
        let xs = [-1.0, 0.0, 1.0];
        let f = |x: f64| x.powi(4) - 2.0 * x * x + 3.0;
        let df = |x: f64| 4.0 * x.powi(3) - 4.0 * x;
        let ys: Vec<_> = xs.iter().map(|&x| f(x)).collect();
        let dys: Vec<_> = xs.iter().map(|&x| df(x)).collect();
        for x in [-0.9, -0.5, 0.1, 0.7] {
            let (v, d) = hermite_eval(&xs, &ys, &dys, x);
            assert!((v - f(x)).abs() < 1e-12, "v={v} expected {}", f(x));
            assert!((d - df(x)).abs() < 1e-12, "d={d} expected {}", df(x));
        }
    }

    #[test]
    fn chebyshev_value_matches_reference() {
        // f(s) = 1*T0 + 2*T1 + 3*T2 = 1 + 2s + 3(2s^2 - 1) = 6s^2 + 2s - 2
        // f'(s) = 12s + 2
        let c = [1.0, 2.0, 3.0];
        let (v, d) = cheby_val_and_deriv(&c, 0.5);
        assert!((v - (6.0 * 0.25 + 1.0 - 2.0)).abs() < 1e-14);
        assert!((d - (12.0 * 0.5 + 2.0)).abs() < 1e-14);
    }

    #[test]
    fn chebyshev_derivative_matches_finite_difference() {
        // Nontrivial coefficient set; compare analytic derivative to a
        // symmetric finite difference. Uses a small step so truncation
        // error dominates rounding.
        let c = [0.3, -1.2, 0.7, 0.4, -0.15, 0.02];
        let h = 1e-6;
        for &s in &[-0.9_f64, -0.3, 0.0, 0.25, 0.7] {
            let (_, d_analytic) = cheby_val_and_deriv(&c, s);
            let (vp, _) = cheby_val_and_deriv(&c, s + h);
            let (vm, _) = cheby_val_and_deriv(&c, s - h);
            let d_fd = (vp - vm) / (2.0 * h);
            assert!(
                (d_analytic - d_fd).abs() < 1e-8,
                "analytic={d_analytic} fd={d_fd}"
            );
        }
    }

    #[test]
    fn chebyshev_degree_zero_is_constant() {
        let (v, d) = cheby_val_and_deriv(&[2.5], 0.42);
        assert_eq!(v, 2.5);
        assert_eq!(d, 0.0);
    }

    #[test]
    fn lagrange_reproduces_quartic_exactly() {
        // With 5 knots a 4th-degree polynomial is exact.
        let xs = [-2.0_f64, -0.5, 0.1, 1.3, 2.7];
        let f = |x: f64| 1.0 + 2.0 * x - 3.0 * x * x + 0.5 * x.powi(3) - 0.1 * x.powi(4);
        let ys: Vec<_> = xs.iter().map(|&x| f(x)).collect();
        for x in [-1.7_f64, -0.2, 0.0, 0.8, 2.5] {
            let v = lagrange_eval(&xs, &ys, x);
            assert!((v - f(x)).abs() < 1e-12, "lagrange={v} expected={}", f(x));
        }
    }

    #[test]
    fn hermite_derivative_matches_finite_difference() {
        // f(x) = sin(x), f'(x) = cos(x). With many knots the Hermite
        // interpolant is accurate to well beyond our finite-difference
        // step; compare derivative at an interior point.
        let xs: Vec<f64> = (0..9).map(|i| -2.0 + 0.5 * (i as f64)).collect();
        let ys: Vec<f64> = xs.iter().map(|&x| x.sin()).collect();
        let dys: Vec<f64> = xs.iter().map(|&x| x.cos()).collect();
        let x0 = 0.37;
        let h = 1e-5;
        let (v0, d_analytic) = hermite_eval(&xs, &ys, &dys, x0);
        let (vp, _) = hermite_eval(&xs, &ys, &dys, x0 + h);
        let (vm, _) = hermite_eval(&xs, &ys, &dys, x0 - h);
        let d_fd = (vp - vm) / (2.0 * h);
        assert!((v0 - x0.sin()).abs() < 1e-8);
        assert!(
            (d_analytic - d_fd).abs() < 1e-6,
            "analytic={d_analytic} fd={d_fd}"
        );
    }
}

#[cfg(test)]
mod cheby3_parity_tests {
    //! Pin down that the 3-axis shared evaluators produce bit-identical
    //! output to the scalar `cheby_val_and_deriv` called once per axis.
    //! Same per-channel arithmetic order, so equality must hold even at
    //! `rtol=atol=0` on every f64 bit.
    use super::{cheby3_val_and_deriv, cheby3_val_only, cheby_val_and_deriv};

    fn coeffs(seed: u64, n: usize) -> Vec<f64> {
        // Deterministic pseudo-random f64s in roughly [-1, 1] without
        // pulling in a `rand` dep — sufficient to trip any FP reorder.
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        (0..n)
            .map(|_| {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let bits = x >> 11;
                (bits as f64) * (1.0 / (1_u64 << 53) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn cheby3_val_and_deriv_matches_scalar_bit_for_bit() {
        for &n in &[2usize, 3, 8, 11, 13, 14, 27] {
            let cx = coeffs(0xA, n);
            let cy = coeffs(0xB, n);
            let cz = coeffs(0xC, n);
            for &s in &[-1.0, -0.7, -0.123, 0.0, 0.25, 0.5, 0.999, 1.0] {
                let (vx, dx) = cheby_val_and_deriv(&cx, s);
                let (vy, dy) = cheby_val_and_deriv(&cy, s);
                let (vz, dz) = cheby_val_and_deriv(&cz, s);
                let (val, der) = cheby3_val_and_deriv(&cx, &cy, &cz, s);
                assert_eq!(val[0].to_bits(), vx.to_bits(), "val.x n={n} s={s}");
                assert_eq!(val[1].to_bits(), vy.to_bits(), "val.y n={n} s={s}");
                assert_eq!(val[2].to_bits(), vz.to_bits(), "val.z n={n} s={s}");
                assert_eq!(der[0].to_bits(), dx.to_bits(), "der.x n={n} s={s}");
                assert_eq!(der[1].to_bits(), dy.to_bits(), "der.y n={n} s={s}");
                assert_eq!(der[2].to_bits(), dz.to_bits(), "der.z n={n} s={s}");
            }
        }
    }

    #[test]
    fn cheby3_val_only_matches_scalar_bit_for_bit() {
        for &n in &[2usize, 3, 8, 11, 13, 14, 27] {
            let cx = coeffs(0x1, n);
            let cy = coeffs(0x2, n);
            let cz = coeffs(0x3, n);
            for &s in &[-1.0, -0.7, -0.123, 0.0, 0.25, 0.5, 0.999, 1.0] {
                let (vx, _) = cheby_val_and_deriv(&cx, s);
                let (vy, _) = cheby_val_and_deriv(&cy, s);
                let (vz, _) = cheby_val_and_deriv(&cz, s);
                let v3 = cheby3_val_only(&cx, &cy, &cz, s);
                assert_eq!(v3[0].to_bits(), vx.to_bits(), "x n={n} s={s}");
                assert_eq!(v3[1].to_bits(), vy.to_bits(), "y n={n} s={s}");
                assert_eq!(v3[2].to_bits(), vz.to_bits(), "z n={n} s={s}");
            }
        }
    }

    #[test]
    fn cheby3_handles_degenerate_lengths() {
        // n=0 and n=1 short-circuit — must agree with three scalar calls.
        let s = 0.3_f64;
        let (val, der) = cheby3_val_and_deriv(&[], &[], &[], s);
        assert_eq!(val, [0.0; 3]);
        assert_eq!(der, [0.0; 3]);
        let v3 = cheby3_val_only(&[], &[], &[], s);
        assert_eq!(v3, [0.0; 3]);

        let cx = [1.5];
        let cy = [-0.25];
        let cz = [42.0];
        let (val, der) = cheby3_val_and_deriv(&cx, &cy, &cz, s);
        assert_eq!(val, [1.5, -0.25, 42.0]);
        assert_eq!(der, [0.0; 3]);
        let v3 = cheby3_val_only(&cx, &cy, &cz, s);
        assert_eq!(v3, [1.5, -0.25, 42.0]);
    }
}

#[cfg(test)]
mod synthetic_spk_tests {
    //! End-to-end tests over a synthesized DAF/SPK file. Each test
    //! constructs a minimal kernel in a tempfile with hand-chosen
    //! coefficients, then exercises `SpkFile` through its public API.

    use super::*;
    use crate::daf::{DOUBLE_BYTES, RECORD_BYTES};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// One SPK segment descriptor for the synthetic builder. The
    /// builder lays out data records sequentially in the order
    /// segments are supplied; every segment produces a single Type 2
    /// (or Type 3, disambiguated by `data_type`) record.
    struct SegSpec {
        target: i32,
        center: i32,
        frame: i32,
        data_type: i32, // 2 or 3
        start_et: f64,
        end_et: f64,
        mid: f64,
        radius: f64,
        /// For Type 2: three position coefficient vectors.
        /// For Type 3: six coefficient vectors (pos x/y/z, vel x/y/z).
        coefs: Vec<Vec<f64>>,
        name: String,
    }

    /// Build a DAF/SPK file containing the supplied segments in one
    /// summary record. Each segment gets its own one-record Type 2 or
    /// Type 3 payload block; address math is computed so segments are
    /// packed back-to-back after the name record.
    fn build_spk(segs: &[SegSpec]) -> NamedTempFile {
        // nd=2, ni=6 is the SPK layout.
        let nd = 2u32;
        let ni = 6u32;
        let ss_doubles = nd as usize + (ni as usize).div_ceil(2); // 5
        let summary_bytes = ss_doubles * DOUBLE_BYTES; // 40

        // Lay out data records. Address space is 1-indexed in doubles,
        // record 4 (= first payload record) starts at 1 + 3*128 = 385.
        let doubles_per_record = (RECORD_BYTES / DOUBLE_BYTES) as u32;
        let mut per_segment_payload: Vec<(u32, u32, Vec<f64>)> = Vec::new();
        let mut cursor_doubles: u32 = 1 + 3 * doubles_per_record;
        for s in segs {
            let n_coef_each = s.coefs[0].len();
            let expected_blocks = if s.data_type == 2 { 3 } else { 6 };
            assert_eq!(s.coefs.len(), expected_blocks, "wrong coef block count");
            for c in &s.coefs {
                assert_eq!(c.len(), n_coef_each, "unequal coef block lengths");
            }
            let rsize = 2 + expected_blocks * n_coef_each;
            let mut data: Vec<f64> = Vec::with_capacity(rsize + 4);
            data.push(s.mid);
            data.push(s.radius);
            for block in &s.coefs {
                data.extend_from_slice(block);
            }
            // Trailer [INIT, INTLEN, RSIZE, N_RECORDS]. One record per segment.
            data.push(s.start_et);
            data.push(s.end_et - s.start_et);
            data.push(rsize as f64);
            data.push(1.0);
            let start_addr = cursor_doubles;
            let end_addr = start_addr + data.len() as u32 - 1;
            cursor_doubles = end_addr + 1;
            per_segment_payload.push((start_addr, end_addr, data));
        }

        // ---- record 1: file record ----
        let mut record1 = vec![0u8; RECORD_BYTES];
        record1[0..8].copy_from_slice(b"DAF/SPK ");
        record1[8..12].copy_from_slice(&nd.to_le_bytes());
        record1[12..16].copy_from_slice(&ni.to_le_bytes());
        for b in &mut record1[16..76] {
            *b = b' ';
        }
        record1[76..80].copy_from_slice(&2u32.to_le_bytes()); // fward
        record1[80..84].copy_from_slice(&2u32.to_le_bytes()); // bward
        record1[84..88].copy_from_slice(&0u32.to_le_bytes()); // free
        record1[88..96].copy_from_slice(b"LTL-IEEE");

        // ---- record 2: summary ----
        let mut record2 = vec![0u8; RECORD_BYTES];
        record2[0..8].copy_from_slice(&0.0f64.to_le_bytes()); // NEXT
        record2[8..16].copy_from_slice(&0.0f64.to_le_bytes()); // PREV
        record2[16..24].copy_from_slice(&(segs.len() as f64).to_le_bytes()); // NSUM
        for (i, s) in segs.iter().enumerate() {
            let (start_addr, end_addr, _) = &per_segment_payload[i];
            let soff = 24 + i * summary_bytes;
            record2[soff..soff + 8].copy_from_slice(&s.start_et.to_le_bytes());
            record2[soff + 8..soff + 16].copy_from_slice(&s.end_et.to_le_bytes());
            let int_start = soff + 2 * DOUBLE_BYTES;
            record2[int_start..int_start + 4].copy_from_slice(&s.target.to_le_bytes());
            record2[int_start + 4..int_start + 8].copy_from_slice(&s.center.to_le_bytes());
            record2[int_start + 8..int_start + 12].copy_from_slice(&s.frame.to_le_bytes());
            record2[int_start + 12..int_start + 16].copy_from_slice(&s.data_type.to_le_bytes());
            record2[int_start + 16..int_start + 20]
                .copy_from_slice(&(*start_addr as i32).to_le_bytes());
            record2[int_start + 20..int_start + 24]
                .copy_from_slice(&(*end_addr as i32).to_le_bytes());
        }

        // ---- record 3: name ----
        let mut record3 = vec![b' '; RECORD_BYTES];
        for (i, s) in segs.iter().enumerate() {
            let noff = i * summary_bytes;
            let nbytes = s.name.as_bytes();
            let n = nbytes.len().min(summary_bytes);
            record3[noff..noff + n].copy_from_slice(&nbytes[..n]);
        }

        // ---- data records ----
        let mut data_bytes = Vec::new();
        for (_, _, data) in &per_segment_payload {
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

    fn one_record_type2(
        target: i32,
        center: i32,
        start_et: f64,
        end_et: f64,
        coefs_x: &[f64],
        coefs_y: &[f64],
        coefs_z: &[f64],
    ) -> SegSpec {
        SegSpec {
            target,
            center,
            frame: 1, // J2000
            data_type: 2,
            start_et,
            end_et,
            mid: 0.5 * (start_et + end_et),
            radius: 0.5 * (end_et - start_et),
            coefs: vec![coefs_x.to_vec(), coefs_y.to_vec(), coefs_z.to_vec()],
            name: format!("SYN {target} FROM {center}"),
        }
    }

    #[test]
    fn type2_state_at_midpoint_returns_degree_zero_position() {
        // Single segment, target=3 from center=0 (SSB). At record
        // midpoint (ET=0) the Chebyshev evaluator yields the
        // degree-zero coefficient; velocity is the analytic derivative
        // of the position polynomial (T1' = 1 contribution).
        let seg = one_record_type2(3, 0, -100.0, 100.0, &[1.0, 2.0], &[3.0, 0.0], &[4.0, -1.0]);
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let s = spk.state(3, 0, 0.0).expect("state");
        assert!((s[0] - 1.0).abs() < 1e-14); // x
        assert!((s[1] - 3.0).abs() < 1e-14); // y
        assert!((s[2] - 4.0).abs() < 1e-14); // z
                                             // radius = 100 so vx = coef_x[1] * 1 / 100 = 0.02
        assert!((s[3] - 0.02).abs() < 1e-14);
        assert!((s[4] - 0.0).abs() < 1e-14);
        assert!((s[5] - (-0.01)).abs() < 1e-14);
    }

    #[test]
    fn type2_hand_computed_position_and_velocity() {
        // X-coefs [1, 2, 3] at s=0.5 (et=25, mid=0, radius=50).
        // f(s) = 6s^2 + 2s - 2  → f(0.5) = 0.5
        // df/ds = 12s + 2        → 8.0 at s=0.5
        // dx/dt = (df/ds)/radius → 0.16
        let seg = one_record_type2(
            5,
            0,
            -50.0,
            50.0,
            &[1.0, 2.0, 3.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
        );
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let s = spk.state(5, 0, 25.0).expect("state");
        assert!((s[0] - 0.5).abs() < 1e-14, "x={}", s[0]);
        assert!((s[3] - 0.16).abs() < 1e-14, "vx={}", s[3]);
    }

    #[test]
    fn reverse_direction_negates_returned_state() {
        // Segment is target=3, center=10. Querying state(10, 3, et)
        // should reuse the same segment and flip all six components.
        let seg = one_record_type2(3, 10, -10.0, 10.0, &[7.0, 0.5], &[-2.0, 1.5], &[3.0, -0.25]);
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let forward = spk.state(3, 10, 0.0).expect("forward");
        let reverse = spk.state(10, 3, 0.0).expect("reverse");
        for i in 0..6 {
            assert!((forward[i] + reverse[i]).abs() < 1e-14, "axis {i}");
        }
    }

    #[test]
    fn chain_walk_composes_through_intermediate_center() {
        // Build a two-segment chain:
        //   segment A: target=399 (Earth), center=3 (EMB)
        //   segment B: target=3   (EMB),   center=0 (SSB)
        // Query state(399, 0) should return the sum; state(399, 10) of
        // Sun (10) also chains via SSB.
        let seg_earth_from_emb =
            one_record_type2(399, 3, -100.0, 100.0, &[1.0, 0.0], &[2.0, 0.0], &[3.0, 0.0]);
        let seg_emb_from_ssb = one_record_type2(
            3,
            0,
            -100.0,
            100.0,
            &[100.0, 0.0],
            &[200.0, 0.0],
            &[300.0, 0.0],
        );
        let seg_sun_from_ssb =
            one_record_type2(10, 0, -100.0, 100.0, &[0.1, 0.0], &[0.2, 0.0], &[0.3, 0.0]);
        let tmp = build_spk(&[seg_earth_from_emb, seg_emb_from_ssb, seg_sun_from_ssb]);
        let spk = SpkFile::open(tmp.path()).expect("open");

        // Earth wrt SSB = (Earth wrt EMB) + (EMB wrt SSB).
        let earth_ssb = spk.state(399, 0, 0.0).expect("earth ssb");
        assert!((earth_ssb[0] - 101.0).abs() < 1e-14);
        assert!((earth_ssb[1] - 202.0).abs() < 1e-14);
        assert!((earth_ssb[2] - 303.0).abs() < 1e-14);

        // Earth wrt Sun = earth_ssb - sun_ssb.
        let earth_sun = spk.state(399, 10, 0.0).expect("earth sun");
        assert!((earth_sun[0] - (101.0 - 0.1)).abs() < 1e-14);
        assert!((earth_sun[1] - (202.0 - 0.2)).abs() < 1e-14);
        assert!((earth_sun[2] - (303.0 - 0.3)).abs() < 1e-14);
    }

    #[test]
    fn no_coverage_when_et_outside_segment() {
        let seg = one_record_type2(5, 0, -10.0, 10.0, &[1.0, 0.0], &[2.0, 0.0], &[3.0, 0.0]);
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let err = spk.state(5, 0, 1000.0).unwrap_err();
        assert!(matches!(err, SpkError::NoCoverage { .. }));
    }

    #[test]
    fn no_coverage_when_target_missing() {
        let seg = one_record_type2(5, 0, -10.0, 10.0, &[1.0, 0.0], &[2.0, 0.0], &[3.0, 0.0]);
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let err = spk.state(9999, 0, 0.0).unwrap_err();
        assert!(matches!(err, SpkError::NoCoverage { .. }));
    }

    #[test]
    fn state_target_equals_center_is_zero_vector() {
        let seg = one_record_type2(5, 0, -10.0, 10.0, &[1.0, 2.0], &[3.0, 4.0], &[5.0, 6.0]);
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        // Short-circuits before any segment lookup.
        let s = spk.state(7, 7, 0.0).expect("identity");
        assert_eq!(s, [0.0; 6]);
    }

    #[test]
    fn type3_uses_separate_velocity_coefficients() {
        // Type 3 separates velocity from position. Pick constant
        // velocity coefs and verify they bypass the 1/radius scaling
        // that Type 2 applies to d(position)/ds.
        let spec = SegSpec {
            target: 7,
            center: 0,
            frame: 1,
            data_type: 3,
            start_et: -100.0,
            end_et: 100.0,
            mid: 0.0,
            radius: 100.0,
            coefs: vec![
                vec![1.0, 0.0], // X pos
                vec![2.0, 0.0], // Y pos
                vec![3.0, 0.0], // Z pos
                vec![0.5, 0.0], // X vel — constant 0.5, no 1/radius scaling
                vec![0.25, 0.0],
                vec![-0.125, 0.0],
            ],
            name: "SYN TYPE 3".to_string(),
        };
        let tmp = build_spk(&[spec]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let s = spk.state(7, 0, 0.0).expect("state");
        assert!((s[0] - 1.0).abs() < 1e-14);
        assert!((s[1] - 2.0).abs() < 1e-14);
        assert!((s[2] - 3.0).abs() < 1e-14);
        // Velocity comes directly from its own coefs — no /radius.
        assert!((s[3] - 0.5).abs() < 1e-14, "vx={}", s[3]);
        assert!((s[4] - 0.25).abs() < 1e-14, "vy={}", s[4]);
        assert!((s[5] - (-0.125)).abs() < 1e-14, "vz={}", s[5]);
    }

    #[test]
    fn state_selects_segment_by_epoch_range() {
        // Two adjacent segments for the same (target, center). Each
        // covers a disjoint ET range and reports its own constant
        // position. Verify that lookups route to the correct segment.
        let early = one_record_type2(5, 0, -100.0, 0.0, &[1.0, 0.0], &[0.0, 0.0], &[0.0, 0.0]);
        let late = one_record_type2(5, 0, 0.0, 100.0, &[2.0, 0.0], &[0.0, 0.0], &[0.0, 0.0]);
        let tmp = build_spk(&[early, late]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        let s_early = spk.state(5, 0, -50.0).expect("early");
        assert!((s_early[0] - 1.0).abs() < 1e-14);
        let s_late = spk.state(5, 0, 50.0).expect("late");
        assert!((s_late[0] - 2.0).abs() < 1e-14);
    }

    #[test]
    fn derivative_matches_central_finite_difference() {
        // Non-trivial coefficients — confirm the analytic velocity
        // agrees with a symmetric finite difference of position.
        let seg = one_record_type2(
            5,
            0,
            -1000.0,
            1000.0,
            &[0.1, -0.05, 0.02, -0.003],
            &[0.2, 0.04, -0.01, 0.005],
            &[0.3, 0.08, 0.015, -0.004],
        );
        let tmp = build_spk(&[seg]);
        let spk = SpkFile::open(tmp.path()).expect("open");
        for &et in &[-500.0_f64, -100.0, 0.0, 250.0, 750.0] {
            let h = 1e-3_f64;
            let s = spk.state(5, 0, et).unwrap();
            let sp = spk.state(5, 0, et + h).unwrap();
            let sm = spk.state(5, 0, et - h).unwrap();
            for axis in 0..3 {
                let fd = (sp[axis] - sm[axis]) / (2.0 * h);
                assert!(
                    (s[3 + axis] - fd).abs() < 1e-9,
                    "axis {axis} et={et}: analytic={} fd={}",
                    s[3 + axis],
                    fd
                );
            }
        }
    }

    /// Build a DAF/SPK file with a single Type 2 segment that contains
    /// multiple records. Each record covers `intlen` seconds of ET,
    /// has its own (mid, radius) pair, and its own 3×N coefficient
    /// blocks. Used to verify the evaluator's record-selection math
    /// (`idx = floor((et - init) / intlen)`).
    type Type2Record = (f64, f64, Vec<f64>, Vec<f64>, Vec<f64>);

    fn build_spk_multi_record_type2(
        target: i32,
        center: i32,
        init: f64,
        intlen: f64,
        records: &[Type2Record],
    ) -> NamedTempFile {
        let n_records = records.len();
        assert!(n_records > 0);
        let n_coef = records[0].2.len();
        for (_, _, xc, yc, zc) in records {
            assert_eq!(xc.len(), n_coef);
            assert_eq!(yc.len(), n_coef);
            assert_eq!(zc.len(), n_coef);
        }
        let rsize = 2 + 3 * n_coef;

        let mut data: Vec<f64> = Vec::with_capacity(rsize * n_records + 4);
        for (mid, radius, xc, yc, zc) in records {
            data.push(*mid);
            data.push(*radius);
            data.extend_from_slice(xc);
            data.extend_from_slice(yc);
            data.extend_from_slice(zc);
        }
        data.push(init);
        data.push(intlen);
        data.push(rsize as f64);
        data.push(n_records as f64);

        let doubles_per_record = (RECORD_BYTES / DOUBLE_BYTES) as u32;
        let start_addr = 1 + 3 * doubles_per_record;
        let end_addr = start_addr + data.len() as u32 - 1;

        let mut record1 = vec![0u8; RECORD_BYTES];
        record1[0..8].copy_from_slice(b"DAF/SPK ");
        record1[8..12].copy_from_slice(&2u32.to_le_bytes());
        record1[12..16].copy_from_slice(&6u32.to_le_bytes());
        for b in &mut record1[16..76] {
            *b = b' ';
        }
        record1[76..80].copy_from_slice(&2u32.to_le_bytes());
        record1[80..84].copy_from_slice(&2u32.to_le_bytes());
        record1[88..96].copy_from_slice(b"LTL-IEEE");

        let mut record2 = vec![0u8; RECORD_BYTES];
        record2[0..8].copy_from_slice(&0.0f64.to_le_bytes());
        record2[8..16].copy_from_slice(&0.0f64.to_le_bytes());
        record2[16..24].copy_from_slice(&1.0f64.to_le_bytes());
        let start_et = init;
        let end_et = init + intlen * n_records as f64;
        record2[24..32].copy_from_slice(&start_et.to_le_bytes());
        record2[32..40].copy_from_slice(&end_et.to_le_bytes());
        let int_start = 40;
        record2[int_start..int_start + 4].copy_from_slice(&target.to_le_bytes());
        record2[int_start + 4..int_start + 8].copy_from_slice(&center.to_le_bytes());
        record2[int_start + 8..int_start + 12].copy_from_slice(&1i32.to_le_bytes());
        record2[int_start + 12..int_start + 16].copy_from_slice(&2i32.to_le_bytes());
        record2[int_start + 16..int_start + 20].copy_from_slice(&(start_addr as i32).to_le_bytes());
        record2[int_start + 20..int_start + 24].copy_from_slice(&(end_addr as i32).to_le_bytes());

        let mut record3 = vec![b' '; RECORD_BYTES];
        let name = b"MULTI REC";
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
    fn multi_record_segment_selects_correct_record_by_et() {
        // One segment with three records, each returning a distinct
        // constant position. Record i covers [init + i*intlen, init + (i+1)*intlen).
        let tmp = build_spk_multi_record_type2(
            5,
            0,
            0.0,   // init
            100.0, // intlen
            &[
                (50.0, 50.0, vec![10.0, 0.0], vec![0.0, 0.0], vec![0.0, 0.0]),
                (150.0, 50.0, vec![20.0, 0.0], vec![0.0, 0.0], vec![0.0, 0.0]),
                (250.0, 50.0, vec![30.0, 0.0], vec![0.0, 0.0], vec![0.0, 0.0]),
            ],
        );
        let spk = SpkFile::open(tmp.path()).expect("open");
        // ET 25 → record 0 → x=10.
        let s0 = spk.state(5, 0, 25.0).expect("rec 0");
        assert!((s0[0] - 10.0).abs() < 1e-14, "record 0 x={}", s0[0]);
        // ET 125 → record 1 → x=20.
        let s1 = spk.state(5, 0, 125.0).expect("rec 1");
        assert!((s1[0] - 20.0).abs() < 1e-14, "record 1 x={}", s1[0]);
        // ET 250 → record 2 → x=30.
        let s2 = spk.state(5, 0, 250.0).expect("rec 2");
        assert!((s2[0] - 30.0).abs() < 1e-14, "record 2 x={}", s2[0]);
        // ET at the right edge should clamp to the last record (CSPICE
        // semantics — upper bound inclusive).
        let s3 = spk.state(5, 0, 300.0).expect("right edge");
        assert!((s3[0] - 30.0).abs() < 1e-14, "edge x={}", s3[0]);
    }
}
