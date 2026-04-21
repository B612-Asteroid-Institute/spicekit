//! Bit-for-bit-within-tolerance parity tests against CSpice.
//!
//! Ported verbatim from adam-core's
//! `src/adam_core/utils/tests/test_spice_backend.py`. The adam-core
//! suite uses `RustBackend` as the thing under test; here we target
//! spicekit directly via the local `Backend` struct in `lib.rs`. Same
//! inputs, same tolerances, same kernels (resolved at runtime from
//! the `naif-*` PyPI packages via `kernels::default_kernel_paths`).
//!
//! Skipped when the `cspice` feature is off (e.g. Apple Silicon).

#![cfg(feature = "cspice")]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once};

use spicekit_bench::kernels::default_kernel_paths;
use spicekit_bench::{cspice_wrap, parity_sample_ets, Backend, BackendError};

/// All CSpice parity tests share the kernel pool (furnsh is global in
/// CSpice). We serialize tests with a Mutex — they each lock a fresh
/// `Backend` on the spicekit side, so spicekit isolation is free, but
/// CSpice needs the lock so the pool doesn't churn during a test.
static CSPICE_LOADED: Once = Once::new();
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn ensure_cspice_loaded(kernels: &[PathBuf]) {
    CSPICE_LOADED.call_once(|| {
        for k in kernels {
            cspice_wrap::furnsh(k.to_str().expect("kernel path UTF-8"))
                .unwrap_or_else(|e| panic!("cspice furnsh {}: {e}", k.display()));
        }
    });
}

fn fresh_rust_backend(kernels: &[PathBuf]) -> Backend {
    let mut b = Backend::new();
    for k in kernels {
        b.furnsh(k)
            .unwrap_or_else(|e| panic!("spicekit furnsh {}: {e}", k.display()));
    }
    b
}

/// `numpy.testing.assert_allclose(actual, desired, rtol, atol)` logic.
fn assert_allclose(actual: f64, desired: f64, rtol: f64, atol: f64, context: &str) {
    let diff = (actual - desired).abs();
    let tol = atol + rtol * desired.abs();
    assert!(
        diff <= tol,
        "{context}: |{actual} - {desired}| = {diff} > atol({atol}) + rtol({rtol})*|desired| = {tol}"
    );
}

// ---------------------------------------------------------------------
// spkez_batch_parity  (rtol=1e-14, atol=1e-7)
// ---------------------------------------------------------------------

fn spkez_case(target: i32, observer: i32, frame: &str) {
    let _guard = TEST_LOCK.lock().unwrap();
    let kernels = default_kernel_paths();
    ensure_cspice_loaded(&kernels);
    let rust = fresh_rust_backend(&kernels);

    let ets = parity_sample_ets();
    let rust_out = rust
        .spkez_batch(target, observer, frame, &ets)
        .expect("spicekit spkez_batch");
    for (i, &et) in ets.iter().enumerate() {
        let (c, _) = cspice_wrap::spkez(target, et, frame, "NONE", observer).expect("cspice spkez");
        for (k, &ck) in c.iter().enumerate() {
            assert_allclose(
                rust_out[i][k],
                ck,
                1e-14,
                1e-7,
                &format!(
                    "spkez target={target} observer={observer} frame={frame} et={et} i={i} k={k}"
                ),
            );
        }
    }
}

#[test]
fn spkez_batch_parity_sun_wrt_ssb_j2000() {
    spkez_case(10, 0, "J2000");
}

#[test]
fn spkez_batch_parity_sun_wrt_ssb_eclipj2000() {
    spkez_case(10, 0, "ECLIPJ2000");
}

#[test]
fn spkez_batch_parity_earth_wrt_ssb_j2000() {
    spkez_case(399, 0, "J2000");
}

#[test]
fn spkez_batch_parity_earth_wrt_sun_eclipj2000() {
    spkez_case(399, 10, "ECLIPJ2000");
}

#[test]
fn spkez_batch_parity_moon_wrt_earth_j2000() {
    spkez_case(301, 399, "J2000");
}

// ---------------------------------------------------------------------
// pxform_batch_parity  (atol=1e-12)
// ---------------------------------------------------------------------

fn pxform_case(frame_from: &str, frame_to: &str) {
    let _guard = TEST_LOCK.lock().unwrap();
    let kernels = default_kernel_paths();
    ensure_cspice_loaded(&kernels);
    let rust = fresh_rust_backend(&kernels);

    let ets = parity_sample_ets();
    let rust_out = rust
        .pxform_batch(frame_from, frame_to, &ets)
        .expect("spicekit pxform_batch");
    for (i, &et) in ets.iter().enumerate() {
        let c = cspice_wrap::pxform(frame_from, frame_to, et).expect("cspice pxform");
        for (r, row) in c.iter().enumerate() {
            for (col, &c_rc) in row.iter().enumerate() {
                assert_allclose(
                    rust_out[i][r][col],
                    c_rc,
                    0.0,
                    1e-12,
                    &format!("pxform {frame_from}->{frame_to} et={et} i={i} [{r}][{col}]"),
                );
            }
        }
    }
}

#[test]
fn pxform_batch_parity_itrf93_to_j2000() {
    pxform_case("ITRF93", "J2000");
}

#[test]
fn pxform_batch_parity_j2000_to_itrf93() {
    pxform_case("J2000", "ITRF93");
}

#[test]
fn pxform_batch_parity_itrf93_to_eclipj2000() {
    pxform_case("ITRF93", "ECLIPJ2000");
}

#[test]
fn pxform_batch_parity_eclipj2000_to_itrf93() {
    pxform_case("ECLIPJ2000", "ITRF93");
}

// ---------------------------------------------------------------------
// sxform_batch_parity  (atol=1e-11)
// ---------------------------------------------------------------------

fn sxform_case(frame_from: &str, frame_to: &str) {
    let _guard = TEST_LOCK.lock().unwrap();
    let kernels = default_kernel_paths();
    ensure_cspice_loaded(&kernels);
    let rust = fresh_rust_backend(&kernels);

    let ets = parity_sample_ets();
    let rust_out = rust
        .sxform_batch(frame_from, frame_to, &ets)
        .expect("spicekit sxform_batch");
    for (i, &et) in ets.iter().enumerate() {
        let c = cspice_wrap::sxform(frame_from, frame_to, et).expect("cspice sxform");
        for (r, row) in c.iter().enumerate() {
            for (col, &c_rc) in row.iter().enumerate() {
                assert_allclose(
                    rust_out[i][r][col],
                    c_rc,
                    0.0,
                    1e-11,
                    &format!("sxform {frame_from}->{frame_to} et={et} i={i} [{r}][{col}]"),
                );
            }
        }
    }
}

#[test]
fn sxform_batch_parity_itrf93_to_j2000() {
    sxform_case("ITRF93", "J2000");
}

#[test]
fn sxform_batch_parity_j2000_to_itrf93() {
    sxform_case("J2000", "ITRF93");
}

// ---------------------------------------------------------------------
// bodn2c_parity
// ---------------------------------------------------------------------

fn bodn2c_case(name: &str) {
    let _guard = TEST_LOCK.lock().unwrap();
    let kernels = default_kernel_paths();
    ensure_cspice_loaded(&kernels);
    let rust = Backend::new();
    let r = rust.bodn2c(name).expect("spicekit bodn2c");
    let c = cspice_wrap::bodn2c(name)
        .expect("cspice bodn2c call")
        .expect("cspice bodn2c not-found");
    assert_eq!(r, c, "bodn2c mismatch for {name}");
}

#[test]
fn bodn2c_parity_sun() {
    bodn2c_case("SUN");
}
#[test]
fn bodn2c_parity_earth() {
    bodn2c_case("EARTH");
}
#[test]
fn bodn2c_parity_mars_barycenter() {
    bodn2c_case("MARS BARYCENTER");
}
#[test]
fn bodn2c_parity_moon() {
    bodn2c_case("MOON");
}
#[test]
fn bodn2c_parity_ssb() {
    bodn2c_case("SOLAR SYSTEM BARYCENTER");
}
#[test]
fn bodn2c_parity_jwst() {
    bodn2c_case("JWST");
}
#[test]
fn bodn2c_parity_jwst_full() {
    bodn2c_case("JAMES WEBB SPACE TELESCOPE");
}
#[test]
fn bodn2c_parity_hst() {
    bodn2c_case("HST");
}
#[test]
fn bodn2c_parity_hst_full() {
    bodn2c_case("HUBBLE SPACE TELESCOPE");
}

// ---------------------------------------------------------------------
// bodc2n_parity — canonical reverse-lookup spelling must match CSpice
// for every code in the built-in table.
// ---------------------------------------------------------------------

#[test]
fn bodc2n_parity_all_builtin_codes() {
    let _guard = TEST_LOCK.lock().unwrap();
    // `bodc2n` needs no kernels — it only reads the built-in table.
    let rust_backend = spicekit_bench::Backend::new();
    let _ = rust_backend; // silence unused in case Backend grows state

    // Unique codes preserving first-seen order (same order spicekit's
    // `bodc2n` resolves canonical-per-code).
    let mut seen: std::collections::HashSet<i32> = std::collections::HashSet::new();
    let mut codes: Vec<i32> = Vec::new();
    for &(_, code) in spicekit::naif_ids::builtin_entries() {
        if seen.insert(code) {
            codes.push(code);
        }
    }
    assert!(!codes.is_empty(), "built-in table should contain codes");

    let mut mismatches: Vec<String> = Vec::new();
    for &code in &codes {
        let rust_name = spicekit::naif_ids::bodc2n(code)
            .unwrap_or_else(|e| panic!("spicekit bodc2n({code}) failed: {e}"));
        let cspice_name = match cspice_wrap::bodc2n(code).expect("cspice bodc2n call") {
            Some(n) => n,
            None => {
                // CSpice doesn't know this code; spicekit has a name for
                // it but we can't parity-check that — skip.
                continue;
            }
        };
        if rust_name != cspice_name {
            mismatches.push(format!(
                "code {code}: spicekit={rust_name:?} cspice={cspice_name:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "bodc2n canonical-name mismatches:\n  {}",
        mismatches.join("\n  ")
    );
}

// ---------------------------------------------------------------------
// Text-kernel binding semantics (spicekit-only; no CSpice needed)
// ---------------------------------------------------------------------

fn write_tk(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write tk");
}

#[test]
fn text_kernel_pickup_and_unload() {
    let dir = tempfile::tempdir().unwrap();
    let tk = dir.path().join("custom_names.tk");
    write_tk(
        &tk,
        "KPL/FK\n\
         \\begindata\n\
         NAIF_BODY_NAME += ( 'ADAM_PROBE', 'APROBE' )\n\
         NAIF_BODY_CODE += ( -900001, -900001 )\n\
         \\begintext\n",
    );
    let mut b = Backend::new();
    assert!(
        matches!(b.bodn2c("ADAM_PROBE"), Err(BackendError::NotCovered(_))),
        "custom name must be NotCovered before furnsh"
    );
    b.furnsh(&tk).unwrap();
    assert_eq!(b.bodn2c("ADAM_PROBE").unwrap(), -900001);
    assert_eq!(b.bodn2c("APROBE").unwrap(), -900001);
    b.unload(&tk);
    assert!(
        matches!(b.bodn2c("ADAM_PROBE"), Err(BackendError::NotCovered(_))),
        "custom name must be NotCovered after unload"
    );
}

#[test]
fn text_kernel_last_loaded_wins() {
    let dir = tempfile::tempdir().unwrap();
    let k1 = dir.path().join("first.tk");
    let k2 = dir.path().join("second.tk");
    write_tk(
        &k1,
        "\\begindata\n\
         NAIF_BODY_NAME += ( 'OVERRIDE_ME' )\n\
         NAIF_BODY_CODE += ( -1 )\n\
         \\begintext\n",
    );
    write_tk(
        &k2,
        "\\begindata\n\
         NAIF_BODY_NAME += ( 'OVERRIDE_ME' )\n\
         NAIF_BODY_CODE += ( -2 )\n\
         \\begintext\n",
    );
    let mut b = Backend::new();
    b.furnsh(&k1).unwrap();
    assert_eq!(b.bodn2c("OVERRIDE_ME").unwrap(), -1);
    b.furnsh(&k2).unwrap();
    assert_eq!(b.bodn2c("OVERRIDE_ME").unwrap(), -2);
    b.unload(&k2);
    assert_eq!(b.bodn2c("OVERRIDE_ME").unwrap(), -1);
}

#[test]
fn text_kernel_overrides_builtin() {
    let dir = tempfile::tempdir().unwrap();
    let k = dir.path().join("shadow.tk");
    write_tk(
        &k,
        "\\begindata\n\
         NAIF_BODY_NAME += ( 'EARTH' )\n\
         NAIF_BODY_CODE += ( -9999 )\n\
         \\begintext\n",
    );
    let mut b = Backend::new();
    assert_eq!(b.bodn2c("EARTH").unwrap(), 399);
    b.furnsh(&k).unwrap();
    assert_eq!(b.bodn2c("EARTH").unwrap(), -9999);
    b.unload(&k);
    assert_eq!(b.bodn2c("EARTH").unwrap(), 399);
}

#[test]
fn unknown_bodn2c_is_not_covered() {
    let b = Backend::new();
    assert!(matches!(
        b.bodn2c("NOT-A-NAIF-NAME"),
        Err(BackendError::NotCovered(_))
    ));
}

#[test]
fn itrf93_without_pck_is_not_covered() {
    let b = Backend::new();
    let ets = [0.0f64];
    assert!(matches!(
        b.pxform_batch("ITRF93", "J2000", &ets),
        Err(BackendError::NotCovered(_))
    ));
    assert!(matches!(
        b.sxform_batch("ITRF93", "J2000", &ets),
        Err(BackendError::NotCovered(_))
    ));
}
