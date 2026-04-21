//! Side-by-side microbenchmark: spicekit vs. CSpice.
//!
//! Loads adam-core's canonical kernel set (leapseconds + DE440 + three
//! Earth EOP PCKs + Earth ITRF93 binary PCK, resolved from the
//! `naif-*` PyPI packages via `crates/spicekit-bench/src/kernels.rs`),
//! then measures the same operations through both backends on matched
//! inputs. The case matrix mirrors
//! `adam-core/migration/scripts/spice_backend_benchmark.py` so results
//! are directly comparable across the two repos.
//!
//! Build/run locally (Linux — CSpice auto-downloads via cspice-sys):
//!     cargo run --release -p spicekit-bench
//!
//! Apple Silicon: cspice-sys 1.0.4's `downloadcspice` feature checks
//! `target_arch = "arm"` and fetches the x86_64 archive, so the
//! default feature set fails to link. Point `CSPICE_DIR` at an arm64
//! CSpice archive (e.g. adam-core's `vendor/cspice/`) and it skips the
//! download:
//!     CSPICE_DIR=/path/to/cspice \
//!       SPICEKIT_BENCH_KERNEL_LEAPSECONDS=... \
//!       ... (5 more SPICEKIT_BENCH_KERNEL_* env vars) \
//!       cargo run --release --bin spicekit-bench
//!
//! The env vars can also be omitted — the kernel resolver falls back
//! to `python3 -c "from naif_de440 import de440; print(de440)"` (and
//! the other five `naif-*` packages) as long as a venv with those
//! packages is active. Fully compile-checkable without CSpice at all:
//!     cargo check -p spicekit-bench --no-default-features

#[cfg(not(feature = "cspice"))]
fn main() {
    eprintln!(
        "spicekit-bench was built without the `cspice` feature; the \
         side-by-side comparison requires CSpice. Rebuild with \
         `--features cspice` (default) on a platform where cspice-sys \
         can install CSpice."
    );
    std::process::exit(2);
}

#[cfg(feature = "cspice")]
use std::time::Instant;

#[cfg(feature = "cspice")]
use spicekit_bench::{make_ets, Backend};

#[cfg(feature = "cspice")]
const TIMED_ITERS: usize = 20;
#[cfg(feature = "cspice")]
const WARMUP_ITERS: usize = 2;
#[cfg(feature = "cspice")]
const BODN2C_CALLS_PER_ITER: usize = 10_000;
#[cfg(feature = "cspice")]
const BATCH_SIZES: &[usize] = &[1, 100, 1_000, 10_000];

#[cfg(feature = "cspice")]
const SPK_CASES: &[(&str, i32, i32, &str)] = &[
    ("sun_wrt_ssb_j2000", 10, 0, "J2000"),
    ("sun_wrt_ssb_ecliptic", 10, 0, "ECLIPJ2000"),
    ("earth_wrt_sun_ecliptic", 399, 10, "ECLIPJ2000"),
    ("moon_wrt_earth_j2000", 301, 399, "J2000"),
    ("mars_bc_wrt_sun_j2000", 4, 10, "J2000"),
    ("saturn_bc_wrt_sun_j2000", 6, 10, "J2000"),
];

#[cfg(feature = "cspice")]
const ROT_PAIRS: &[(&str, &str)] = &[
    ("ITRF93", "J2000"),
    ("J2000", "ITRF93"),
    ("ITRF93", "ECLIPJ2000"),
    ("ECLIPJ2000", "ITRF93"),
];

#[cfg(feature = "cspice")]
const BODN2C_NAMES: &[&str] = &["SUN", "EARTH", "MARS BARYCENTER", "MOON", "JWST", "HST"];

#[cfg(feature = "cspice")]
#[derive(Debug, Clone, Copy)]
struct Timings {
    p50_us: f64,
    p95_us: f64,
}

#[cfg(feature = "cspice")]
fn run(iters: usize, warmup: usize, mut f: impl FnMut()) -> Timings {
    for _ in 0..warmup {
        f();
    }
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_nanos() as f64 / 1_000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Timings {
        p50_us: samples[samples.len() / 2],
        p95_us: samples[(samples.len() as f64 * 0.95) as usize],
    }
}

#[cfg(feature = "cspice")]
fn print_table_header() {
    println!(
        "| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
}

#[cfg(feature = "cspice")]
fn print_row(op: &str, case: &str, n: usize, cspice: Timings, spicekit: Timings) {
    let speedup_p50 = cspice.p50_us / spicekit.p50_us;
    let speedup_p95 = cspice.p95_us / spicekit.p95_us;
    println!(
        "| {op} | {case} | {n} | {:.2} | {:.2} | {:.2}x | {:.2} | {:.2} | {:.2}x |",
        cspice.p50_us, spicekit.p50_us, speedup_p50, cspice.p95_us, spicekit.p95_us, speedup_p95,
    );
}

#[cfg(feature = "cspice")]
fn main() {
    use spicekit_bench::cspice_wrap;
    use spicekit_bench::kernels::default_kernel_paths;

    let kernels = default_kernel_paths();
    eprintln!("Loading {} kernels into both backends…", kernels.len());
    let mut spicekit = Backend::new();
    for k in &kernels {
        spicekit
            .furnsh(k)
            .unwrap_or_else(|e| panic!("spicekit furnsh {}: {e}", k.display()));
        cspice_wrap::furnsh(k.to_str().expect("kernel path must be UTF-8"))
            .unwrap_or_else(|e| panic!("cspice furnsh {}: {e}", k.display()));
    }

    print_table_header();

    // spkez_batch — 6 cases × 4 batch sizes.
    for &(label, target, observer, frame) in SPK_CASES {
        for &n in BATCH_SIZES {
            let ets = make_ets(n);
            let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                for &et in &ets {
                    cspice_wrap::spkez(target, et, frame, "NONE", observer).unwrap();
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                spicekit.spkez_batch(target, observer, frame, &ets).unwrap();
            });
            print_row("spkez_batch", label, n, cspice_t, spicekit_t);
        }
    }

    // pxform_batch + sxform_batch — 4 frame pairs × 4 batch sizes, each op.
    for &(frame_from, frame_to) in ROT_PAIRS {
        let label_pair = format!("{frame_from}->{frame_to}");
        for &n in BATCH_SIZES {
            let ets = make_ets(n);

            let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                for &et in &ets {
                    cspice_wrap::pxform(frame_from, frame_to, et).unwrap();
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                spicekit.pxform_batch(frame_from, frame_to, &ets).unwrap();
            });
            print_row("pxform_batch", &label_pair, n, cspice_t, spicekit_t);

            let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                for &et in &ets {
                    cspice_wrap::sxform(frame_from, frame_to, et).unwrap();
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                spicekit.sxform_batch(frame_from, frame_to, &ets).unwrap();
            });
            print_row("sxform_batch", &label_pair, n, cspice_t, spicekit_t);
        }
    }

    // bodn2c — 10 000 calls per iter, 6 names.
    for &name in BODN2C_NAMES {
        let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
            for _ in 0..BODN2C_CALLS_PER_ITER {
                let _ = cspice_wrap::bodn2c(name).unwrap();
            }
        });
        let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
            for _ in 0..BODN2C_CALLS_PER_ITER {
                let _ = spicekit.bodn2c(name).unwrap();
            }
        });
        print_row("bodn2c", name, BODN2C_CALLS_PER_ITER, cspice_t, spicekit_t);
    }
}
