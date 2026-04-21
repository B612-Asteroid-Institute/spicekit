//! Frame rotations used by NAIF kernel consumers.
//!
//! - J2000 ↔ ECLIPJ2000: the time-independent static rotation at the
//!   J2000 obliquity (ε = 84381.448″ ≈ 23°26′21.448″).
//! - J2000 ↔ ITRF93 (time-varying): body-fixed Earth frame, assembled
//!   from PCK Type-2 Chebyshev Euler angles `(t1, t2, t3)` and their
//!   time derivatives via the NAIF 3-1-3 convention
//!   `R_body_ref = Rz(t3) · Rx(t2) · Rz(t1)`,
//!   where `ref` is the PCK segment's reference inertial frame (for
//!   the standard Earth PCKs this is ECLIPJ2000, so the caller
//!   composes with the J2000 ↔ ECLIPJ2000 static rotation to land in
//!   J2000). We bit-reproduce `pxform("J2000","ITRF93",et)` /
//!   `sxform("J2000","ITRF93",et)` given the same PCK data CSPICE
//!   would see.

pub const OBLIQUITY_J2000_RAD: f64 = 0.409_092_804_222_328_97;

/// NAIF frame identifiers we natively support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaifFrame {
    J2000,
    EclipJ2000,
}

/// J2000 → ECLIPJ2000 rotation matrix (row-major 3×3).
///
/// Matches `pxform("J2000", "ECLIPJ2000", et)` from CSPICE.
pub fn j2000_to_eclipj2000() -> [[f64; 3]; 3] {
    let (s, c) = OBLIQUITY_J2000_RAD.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]]
}

/// Apply a 3×3 rotation to a 6-vector state (position + velocity) in
/// an inertial frame (no ω×r term).
pub fn rotate_state_inertial(rot: &[[f64; 3]; 3], state: &[f64; 6]) -> [f64; 6] {
    let p = rotate_vec3(rot, &[state[0], state[1], state[2]]);
    let v = rotate_vec3(rot, &[state[3], state[4], state[5]]);
    [p[0], p[1], p[2], v[0], v[1], v[2]]
}

fn rotate_vec3(rot: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        rot[0][0] * v[0] + rot[0][1] * v[1] + rot[0][2] * v[2],
        rot[1][0] * v[0] + rot[1][1] * v[1] + rot[1][2] * v[2],
        rot[2][0] * v[0] + rot[2][1] * v[1] + rot[2][2] * v[2],
    ]
}

/// Rotate a state from `from` to `to`. Both must be inertial.
pub fn rotate_state(from: NaifFrame, to: NaifFrame, state: &[f64; 6]) -> [f64; 6] {
    if from == to {
        return *state;
    }
    match (from, to) {
        (NaifFrame::J2000, NaifFrame::EclipJ2000) => {
            rotate_state_inertial(&j2000_to_eclipj2000(), state)
        }
        (NaifFrame::EclipJ2000, NaifFrame::J2000) => {
            // Inverse is transpose (orthogonal rotation).
            let r = j2000_to_eclipj2000();
            let rt = [
                [r[0][0], r[1][0], r[2][0]],
                [r[0][1], r[1][1], r[2][1]],
                [r[0][2], r[1][2], r[2][2]],
            ];
            rotate_state_inertial(&rt, state)
        }
        _ => unreachable!(),
    }
}

/// Construct the inertial-reference → body-fixed rotation (3×3) and its
/// time derivative (3×3) from PCK Type 2 Chebyshev Euler-angle channels
/// `(t1, t2, t3)` and rates `(dt1, dt2, dt3)` (units: rad and rad/s).
///
/// Convention (NAIF 3-1-3 Euler, per `EUL2M(t3, t2, t1, 3, 1, 3)`):
///   R = Rz(t3) · Rx(t2) · Rz(t1)
/// where the PCK stores the three angles as raw Chebyshev channels
/// (NOT the historical `(RA, DEC, W)` pole+meridian parameterization —
/// the raw channels are 3-1-3 Euler angles of the body-fixed frame wrt
/// the segment's reference inertial frame). Rz and Rx are right-handed
/// active rotations; a vector `v` in the reference frame maps to `R v`
/// in the body-fixed frame.
pub fn pck_euler_rotation_and_derivative(
    t1: f64,
    t2: f64,
    t3: f64,
    dt1: f64,
    dt2: f64,
    dt3: f64,
) -> ([[f64; 3]; 3], [[f64; 3]; 3]) {
    let (rz_t1, drz_t1) = rotz_and_deriv(t1, dt1);
    let (rx_t2, drx_t2) = rotx_and_deriv(t2, dt2);
    let (rz_t3, drz_t3) = rotz_and_deriv(t3, dt3);

    // R = Rz(t3) · (Rx(t2) · Rz(t1))
    let a = matmul3(&rx_t2, &rz_t1);
    let r = matmul3(&rz_t3, &a);

    // dR/dt = dRz(t3) · A + Rz(t3) · dA
    // dA    = dRx(t2) · Rz(t1) + Rx(t2) · dRz(t1)
    let da = add3(&matmul3(&drx_t2, &rz_t1), &matmul3(&rx_t2, &drz_t1));
    let dr = add3(&matmul3(&drz_t3, &a), &matmul3(&rz_t3, &da));
    (r, dr)
}

/// Assemble a 6×6 state-transform matrix for an inertial → body-fixed
/// rotation with angular velocity, in the CSPICE `sxform` block layout:
///
/// ```text
/// [ R     0 ]
/// [ dR/dt R ]
/// ```
pub fn sxform_from_rotation(r: &[[f64; 3]; 3], dr: &[[f64; 3]; 3]) -> [[f64; 6]; 6] {
    let mut m = [[0.0f64; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = r[i][j];
            m[i + 3][j + 3] = r[i][j];
            m[i + 3][j] = dr[i][j];
        }
    }
    m
}

/// Apply a 6×6 state transform (as produced by `sxform_from_rotation`)
/// to a 6-vector state.
pub fn apply_sxform(m: &[[f64; 6]; 6], state: &[f64; 6]) -> [f64; 6] {
    let mut out = [0.0f64; 6];
    for (i, out_i) in out.iter_mut().enumerate() {
        let mut acc = 0.0;
        for (j, &s) in state.iter().enumerate() {
            acc += m[i][j] * s;
        }
        *out_i = acc;
    }
    out
}

/// Invert a 6×6 state-transform with the CSPICE `sxform` block layout
/// `[[R, 0], [dR, R]]`. The inverse is `[[R^T, 0], [dR^T, R^T]]` — the
/// two 3×3 diagonal blocks transpose normally, and the lower-left block
/// also transposes (not the full 6×6 transpose, which would put a
/// non-zero block in the upper-right).
///
/// Why this matches CSPICE's `sxform(to, from, et)`: for an orthogonal
/// rotation `R(t)`, the angular-velocity tensor `Ω = dR · R^T` is
/// skew-symmetric, so `-R^T · dR · R^T = dR^T`. Hence the algebraic
/// block inverse `[[R^T, 0], [-R^T dR R^T, R^T]]` reduces to the form
/// above.
pub fn invert_sxform(m: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut inv = [[0.0f64; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            // R^T in top-left.
            inv[i][j] = m[j][i];
            // R^T in bottom-right.
            inv[i + 3][j + 3] = m[j + 3][i + 3];
            // dR^T in bottom-left.
            inv[i + 3][j] = m[j + 3][i];
        }
    }
    inv
}

fn rotz_and_deriv(angle: f64, rate: f64) -> ([[f64; 3]; 3], [[f64; 3]; 3]) {
    let (s, c) = angle.sin_cos();
    let r = [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]];
    let dr = [
        [-s * rate, c * rate, 0.0],
        [-c * rate, -s * rate, 0.0],
        [0.0, 0.0, 0.0],
    ];
    (r, dr)
}

fn rotx_and_deriv(angle: f64, rate: f64) -> ([[f64; 3]; 3], [[f64; 3]; 3]) {
    let (s, c) = angle.sin_cos();
    let r = [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]];
    let dr = [
        [0.0, 0.0, 0.0],
        [0.0, -s * rate, c * rate],
        [0.0, -c * rate, -s * rate],
    ];
    (r, dr)
}

fn matmul3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let mut acc = 0.0;
            for (k, b_row) in b.iter().enumerate() {
                acc += a[i][k] * b_row[j];
            }
            c[i][j] = acc;
        }
    }
    c
}

fn add3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][j] + b[i][j];
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_matrix_entries_match_cspice() {
        // Values from pxform('J2000', 'ECLIPJ2000', 0.0).
        let m = j2000_to_eclipj2000();
        assert_eq!(m[0], [1.0, 0.0, 0.0]);
        assert!((m[1][1] - 9.174_820_620_691_818e-1).abs() < 1e-15);
        assert!((m[1][2] - 3.977_771_559_319_137e-1).abs() < 1e-15);
        assert!((m[2][1] - (-3.977_771_559_319_137e-1)).abs() < 1e-15);
        assert!((m[2][2] - 9.174_820_620_691_818e-1).abs() < 1e-15);
    }

    #[test]
    fn roundtrip_recovers_state() {
        let s = [1.0, 2.0, 3.0, 0.1, -0.2, 0.3];
        let eclip = rotate_state(NaifFrame::J2000, NaifFrame::EclipJ2000, &s);
        let back = rotate_state(NaifFrame::EclipJ2000, NaifFrame::J2000, &eclip);
        for i in 0..6 {
            assert!((back[i] - s[i]).abs() < 1e-14);
        }
    }

    #[test]
    fn j2000_to_eclipj2000_is_orthogonal() {
        // R · R^T == I within fp noise.
        let r = j2000_to_eclipj2000();
        let rt = transpose3(&r);
        let prod = matmul3(&r, &rt);
        for (i, row) in prod.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((v - expected).abs() < 1e-15);
            }
        }
    }

    #[test]
    fn sxform_from_rotation_block_layout() {
        // Top-left and bottom-right must be R; bottom-left must be dR;
        // top-right must be zero.
        let r = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let dr = [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6], [0.7, 0.8, 0.9]];
        let m = sxform_from_rotation(&r, &dr);
        for (i, (r_row, dr_row)) in r.iter().zip(dr.iter()).enumerate() {
            for (j, (&r_ij, &dr_ij)) in r_row.iter().zip(dr_row.iter()).enumerate() {
                assert_eq!(m[i][j], r_ij);
                assert_eq!(m[i + 3][j + 3], r_ij);
                assert_eq!(m[i + 3][j], dr_ij);
                assert_eq!(m[i][j + 3], 0.0);
            }
        }
    }

    #[test]
    fn invert_sxform_roundtrip_to_identity() {
        // For any orthogonal R with skew-symmetric ω·R (i.e. dR = ω·R
        // where ω^T = -ω), M · invert(M) must equal I_6x6.
        // Use a nontrivial body-fixed Earth-like rotation.
        let (r, dr) = pck_euler_rotation_and_derivative(
            0.7, 1.1, -0.3, // angles
            1e-5, 2e-7, 7.3e-5, // rates (~Earth sidereal angular velocity scale)
        );
        let m = sxform_from_rotation(&r, &dr);
        let inv = invert_sxform(&m);
        let prod = matmul6_test(&m, &inv);
        for (i, row) in prod.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (v - expected).abs() < 1e-12,
                    "M·inv at [{i},{j}] = {v}  (expected {expected})",
                );
            }
        }
    }

    #[test]
    fn pck_euler_rotation_is_orthogonal() {
        // For any angle triple, R should be orthogonal.
        for &(t1, t2, t3) in &[
            (0.0, 0.0, 0.0),
            (0.7, 1.2, -0.3),
            (-1.5, 0.4, 2.9),
            (3.1, 0.01, -0.7),
        ] {
            let (r, _) = pck_euler_rotation_and_derivative(t1, t2, t3, 0.0, 0.0, 0.0);
            let rt = transpose3(&r);
            let prod = matmul3(&r, &rt);
            for (i, row) in prod.iter().enumerate() {
                for (j, &v) in row.iter().enumerate() {
                    let expected = if i == j { 1.0 } else { 0.0 };
                    assert!(
                        (v - expected).abs() < 1e-14,
                        "R·R^T[{i},{j}] = {v} at angles ({t1},{t2},{t3})",
                    );
                }
            }
        }
    }

    #[test]
    fn pck_euler_derivative_matches_finite_difference() {
        // Compare analytic dR against a centered finite difference on R
        // at perturbed angles. Derivative should match to ~h^2 truncation.
        let (t1, t2, t3) = (0.42, 1.15, -0.7);
        let (dt1, dt2, dt3) = (3e-6, -2e-6, 7.3e-5);
        let (_, dr_analytic) = pck_euler_rotation_and_derivative(t1, t2, t3, dt1, dt2, dt3);
        let h = 1e-3;
        let (r_plus, _) = pck_euler_rotation_and_derivative(
            t1 + dt1 * h,
            t2 + dt2 * h,
            t3 + dt3 * h,
            0.0,
            0.0,
            0.0,
        );
        let (r_minus, _) = pck_euler_rotation_and_derivative(
            t1 - dt1 * h,
            t2 - dt2 * h,
            t3 - dt3 * h,
            0.0,
            0.0,
            0.0,
        );
        for i in 0..3 {
            for j in 0..3 {
                let fd = (r_plus[i][j] - r_minus[i][j]) / (2.0 * h);
                assert!(
                    (dr_analytic[i][j] - fd).abs() < 1e-11,
                    "dR[{i},{j}] analytic={}  fd={}",
                    dr_analytic[i][j],
                    fd
                );
            }
        }
    }

    #[test]
    fn apply_sxform_on_identity_is_identity() {
        // 6×6 identity must reproduce the state.
        let mut ident = [[0.0f64; 6]; 6];
        for (i, row) in ident.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        let state = [1.0, -2.0, 3.5, 0.01, 0.02, -0.03];
        let out = apply_sxform(&ident, &state);
        assert_eq!(out, state);
    }

    #[test]
    fn apply_sxform_matches_block_math() {
        // Hand-check apply_sxform: out_pos = R·r, out_vel = dR·r + R·v.
        let r = [[0.6, 0.8, 0.0], [-0.8, 0.6, 0.0], [0.0, 0.0, 1.0]];
        let dr = [[0.0, 1e-4, 0.0], [-1e-4, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let m = sxform_from_rotation(&r, &dr);
        let pos = [1.0_f64, 2.0, 3.0];
        let vel = [0.1_f64, 0.2, 0.3];
        let state = [pos[0], pos[1], pos[2], vel[0], vel[1], vel[2]];
        let out = apply_sxform(&m, &state);

        let expected_pos = [
            r[0][0] * pos[0] + r[0][1] * pos[1] + r[0][2] * pos[2],
            r[1][0] * pos[0] + r[1][1] * pos[1] + r[1][2] * pos[2],
            r[2][0] * pos[0] + r[2][1] * pos[1] + r[2][2] * pos[2],
        ];
        let expected_vel = [
            dr[0][0] * pos[0]
                + dr[0][1] * pos[1]
                + dr[0][2] * pos[2]
                + r[0][0] * vel[0]
                + r[0][1] * vel[1]
                + r[0][2] * vel[2],
            dr[1][0] * pos[0]
                + dr[1][1] * pos[1]
                + dr[1][2] * pos[2]
                + r[1][0] * vel[0]
                + r[1][1] * vel[1]
                + r[1][2] * vel[2],
            dr[2][0] * pos[0]
                + dr[2][1] * pos[1]
                + dr[2][2] * pos[2]
                + r[2][0] * vel[0]
                + r[2][1] * vel[1]
                + r[2][2] * vel[2],
        ];
        for i in 0..3 {
            assert!((out[i] - expected_pos[i]).abs() < 1e-14);
            assert!((out[i + 3] - expected_vel[i]).abs() < 1e-14);
        }
    }

    fn transpose3(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
        [
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ]
    }

    fn matmul6_test(a: &[[f64; 6]; 6], b: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
        let mut c = [[0.0f64; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                let mut acc = 0.0;
                for (k, b_row) in b.iter().enumerate() {
                    acc += a[i][k] * b_row[j];
                }
                c[i][j] = acc;
            }
        }
        c
    }
}
