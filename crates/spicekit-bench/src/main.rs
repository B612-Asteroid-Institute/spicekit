//! Side-by-side microbenchmark: spicekit vs. CSpice.
//!
//! Loads the canonical benchmark kernel set (leapseconds + DE440 + three
//! Earth EOP PCKs + Earth ITRF93 binary PCK, resolved from the
//! `naif-*` PyPI packages via `crates/spicekit-bench/src/kernels.rs`),
//! then measures the same operations through both backends on matched
//! inputs. The case matrix covers ephemerides, ITRF93 Earth-rotation
//! transforms, ITRF93-state output, leapseconds `DELTET/*` kernel-pool
//! access, frame-name association lookups, and `KernelPool` dispatch
//! over one, two and three overlapping kernels.
//!
//! Build/run locally (Linux — CSpice auto-downloads via cspice-sys):
//!     cargo run --release -p spicekit-bench
//!
//! Apple Silicon: cspice-sys 1.0.4's `downloadcspice` feature checks
//! `target_arch = "arm"` and fetches the x86_64 archive, so the
//! default feature set fails to link. Point `CSPICE_DIR` at an arm64
//! CSpice archive and it skips the download:
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
use std::hint::black_box;
#[cfg(feature = "cspice")]
use std::path::{Path, PathBuf};
#[cfg(feature = "cspice")]
use std::time::Instant;

#[cfg(feature = "cspice")]
use spicekit::text_kernel::{KernelDate, LeapSecondsKernel};
#[cfg(feature = "cspice")]
use spicekit::{KernelPool, NaifFrame};
#[cfg(feature = "cspice")]
use spicekit_bench::{kernel_date_to_et_seconds, make_ets, Backend};

#[cfg(feature = "cspice")]
const TIMED_ITERS: usize = 20;
#[cfg(feature = "cspice")]
const WARMUP_ITERS: usize = 2;
#[cfg(feature = "cspice")]
const BODN2C_CALLS_PER_ITER: usize = 10_000;
#[cfg(feature = "cspice")]
const LSK_POOL_CALLS_PER_ITER: usize = 10_000;
#[cfg(feature = "cspice")]
const FRAME_LOOKUP_CALLS_PER_ITER: usize = 10_000;
#[cfg(feature = "cspice")]
const BATCH_SIZES: &[usize] = &[1, 100, 1_000, 10_000];

/// Binary-PCK body-fixed frame id for ITRF93.
#[cfg(feature = "cspice")]
const ITRF93_BODY_FRAME: i32 = 3000;
/// Positions in `default_kernel_paths()`, which resolves leapseconds,
/// DE440, EOP predict, EOP historical, EOP high precision and the
/// ITRF93 frame kernel, in that order.
#[cfg(feature = "cspice")]
const KERNEL_DE440: usize = 1;
#[cfg(feature = "cspice")]
const KERNEL_EOP_PREDICT: usize = 2;
#[cfg(feature = "cspice")]
const KERNEL_EOP_HISTORICAL: usize = 3;
#[cfg(feature = "cspice")]
const KERNEL_EOP_HIGH_PREC: usize = 4;

#[cfg(feature = "cspice")]
const SPK_CASES: &[(&str, i32, i32, &str)] = &[
    ("sun_wrt_ssb_j2000", 10, 0, "J2000"),
    ("sun_wrt_ssb_ecliptic", 10, 0, "ECLIPJ2000"),
    ("earth_wrt_sun_ecliptic", 399, 10, "ECLIPJ2000"),
    ("moon_wrt_earth_j2000", 301, 399, "J2000"),
    ("moon_wrt_earth_itrf93", 301, 399, "ITRF93"),
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
const LSK_NUMERIC_VARS: &[&str] = &[
    "DELTET/DELTA_T_A",
    "DELTET/K",
    "DELTET/EB",
    "DELTET/M",
    "DELTET/DELTA_AT",
];

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
fn lsk_metadata(lsk: &LeapSecondsKernel) -> [(usize, char); 5] {
    [
        (1, 'N'),
        (1, 'N'),
        (1, 'N'),
        (lsk.m.len(), 'N'),
        (lsk.delta_at.len() * 2, 'N'),
    ]
}

#[cfg(feature = "cspice")]
fn lsk_pool_values(lsk: &LeapSecondsKernel) -> [Vec<f64>; 5] {
    [
        vec![lsk.delta_t_a],
        vec![lsk.k],
        vec![lsk.eb],
        lsk.m.to_vec(),
        lsk.delta_at
            .iter()
            .flat_map(|entry| {
                [
                    entry.leap_seconds as f64,
                    kernel_date_to_et_seconds(entry.date),
                ]
            })
            .collect(),
    ]
}

/// `n` epochs spread evenly from `start` to `end`, both included.
#[cfg(feature = "cspice")]
fn ets_between(start: f64, end: f64, n: usize) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![start],
        _ => {
            let step = (end - start) / ((n - 1) as f64);
            (0..n).map(|i| start + step * i as f64).collect()
        }
    }
}

/// Seconds past J2000 for a calendar date, the way the kernel pool
/// stores a `@YYYY-MMM-D` literal.
#[cfg(feature = "cspice")]
fn et_at(year: i32, month: u8, day: u8) -> f64 {
    kernel_date_to_et_seconds(KernelDate { year, month, day })
}

#[cfg(feature = "cspice")]
fn path_str(path: &Path) -> &str {
    path.to_str().expect("kernel path must be UTF-8")
}

/// Dispatch over overlapping Earth orientation PCKs. Every case answers
/// the same epochs out of a furnished set of one, two or three files, so
/// a row shows what resolving the set costs rather than what the data
/// costs. CSPICE holds exactly the files under test: the three EOP
/// kernels furnished at startup are unloaded up front, then each set is
/// furnshed before its rows and unloaded after. The startup kernels that
/// stay loaded (leapseconds, DE440, the ITRF93 frame kernel) hold no
/// binary PCK data and so take no part in this lookup.
#[cfg(feature = "cspice")]
fn bench_pck_dispatch(kernels: &[PathBuf]) {
    use spicekit_bench::cspice_wrap;

    let predict = kernels[KERNEL_EOP_PREDICT].as_path();
    let historical = kernels[KERNEL_EOP_HISTORICAL].as_path();
    let high_prec = kernels[KERNEL_EOP_HIGH_PREC].as_path();
    for k in [predict, historical, high_prec] {
        cspice_wrap::unload(path_str(k))
            .unwrap_or_else(|e| panic!("cspice unload {}: {e}", k.display()));
    }

    // Predict starts 2026-01-01, historical ends 2026-08-08, high
    // precision runs 2000-01-01 to 2027-01-09, so 2026-02-01 to
    // 2026-07-01 sits inside all three and only the set size varies.
    let window = (et_at(2026, 2, 1), et_at(2026, 7, 1));
    // 1978-01-01 predates both the high precision and the predict
    // file, so the one file set cannot answer a grid from there to
    // 2026-07-01 at all, and the full set resolves the old end out of
    // the historical file and the recent end out of the high precision
    // file. That row is run for the full set only.
    let straddle = (et_at(1978, 1, 1), et_at(2026, 7, 1));

    let configs: [(&str, &[&Path]); 3] = [
        ("eop_1_file", &[high_prec]),
        ("eop_2_files", &[historical, high_prec]),
        ("eop_3_files", &[predict, historical, high_prec]),
    ];
    for (case, set) in configs {
        let mut pool = KernelPool::new();
        for &k in set {
            pool.furnish(k)
                .unwrap_or_else(|e| panic!("pool furnish {}: {e}", k.display()));
            cspice_wrap::furnsh(path_str(k))
                .unwrap_or_else(|e| panic!("cspice furnsh {}: {e}", k.display()));
        }

        let mut grids = vec![(case, window)];
        if set.len() == 3 {
            grids.push(("eop_3_files_straddle", straddle));
        }
        for (row_case, (start, end)) in grids {
            for &n in BATCH_SIZES {
                let ets = ets_between(start, end, n);
                let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                    for &et in &ets {
                        black_box(cspice_wrap::sxform("J2000", "ITRF93", et).unwrap());
                    }
                });
                let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                    black_box(
                        pool.pck_sxform_batch(ITRF93_BODY_FRAME, NaifFrame::J2000, &ets)
                            .unwrap(),
                    );
                });
                print_row("pool_sxform_batch", row_case, n, cspice_t, spicekit_t);
            }
        }

        for &k in set {
            cspice_wrap::unload(path_str(k))
                .unwrap_or_else(|e| panic!("cspice unload {}: {e}", k.display()));
        }
    }

    // Leave CSPICE as the startup loop left it, in the same order.
    for k in [predict, historical, high_prec] {
        cspice_wrap::furnsh(path_str(k))
            .unwrap_or_else(|e| panic!("cspice furnsh {}: {e}", k.display()));
    }
}

/// Dispatch over overlapping SPKs. DE440 plus copies of it under other
/// names are distinct kernels to a pool and to CSPICE alike, with
/// identical coverage, so the newest file answers every epoch and a row
/// shows what walking the set costs. DE440 is unloaded from CSPICE up
/// front and each configuration is furnshed and unloaded around its
/// rows, so the CSPICE SPK set is exactly the set under test.
#[cfg(feature = "cspice")]
fn bench_spk_dispatch(kernels: &[PathBuf]) {
    use spicekit_bench::cspice_wrap;

    let de440 = kernels[KERNEL_DE440].as_path();
    let dir = std::env::temp_dir().join("spicekit-bench-spk-dispatch");
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
    let copies: Vec<PathBuf> = (1..=2)
        .map(|i| {
            let dst = dir.join(format!("de440_copy{i}.bsp"));
            std::fs::copy(de440, &dst)
                .unwrap_or_else(|e| panic!("copy DE440 to {}: {e}", dst.display()));
            dst
        })
        .collect();
    cspice_wrap::unload(path_str(de440))
        .unwrap_or_else(|e| panic!("cspice unload {}: {e}", de440.display()));

    for (case, count) in [("de440_x1", 1), ("de440_x2", 2), ("de440_x3", 3)] {
        let set: Vec<&Path> = std::iter::once(de440)
            .chain(copies.iter().map(PathBuf::as_path))
            .take(count)
            .collect();
        let mut pool = KernelPool::new();
        for &k in &set {
            pool.furnish(k)
                .unwrap_or_else(|e| panic!("pool furnish {}: {e}", k.display()));
            cspice_wrap::furnsh(path_str(k))
                .unwrap_or_else(|e| panic!("cspice furnsh {}: {e}", k.display()));
        }

        // Earth wrt the solar system barycentre, which chains 399 to 3
        // to 0 inside DE440.
        for &n in BATCH_SIZES {
            let ets = make_ets(n);
            let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                for &et in &ets {
                    black_box(cspice_wrap::spkez(399, et, "J2000", "NONE", 0).unwrap());
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                black_box(
                    pool.spk_state_batch(399, 0, NaifFrame::J2000, &ets)
                        .unwrap(),
                );
            });
            print_row("pool_spkez_batch", case, n, cspice_t, spicekit_t);
        }

        for &k in &set {
            cspice_wrap::unload(path_str(k))
                .unwrap_or_else(|e| panic!("cspice unload {}: {e}", k.display()));
        }
    }

    cspice_wrap::furnsh(path_str(de440))
        .unwrap_or_else(|e| panic!("cspice furnsh {}: {e}", de440.display()));
    if let Err(e) = std::fs::remove_dir_all(&dir) {
        eprintln!("warning: could not remove {}: {e}", dir.display());
    }
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
                    black_box(cspice_wrap::spkez(target, et, frame, "NONE", observer).unwrap());
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                black_box(spicekit.spkez_batch(target, observer, frame, &ets).unwrap());
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
                    black_box(cspice_wrap::pxform(frame_from, frame_to, et).unwrap());
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                black_box(spicekit.pxform_batch(frame_from, frame_to, &ets).unwrap());
            });
            print_row("pxform_batch", &label_pair, n, cspice_t, spicekit_t);

            let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                for &et in &ets {
                    black_box(cspice_wrap::sxform(frame_from, frame_to, et).unwrap());
                }
            });
            let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
                black_box(spicekit.sxform_batch(frame_from, frame_to, &ets).unwrap());
            });
            print_row("sxform_batch", &label_pair, n, cspice_t, spicekit_t);
        }
    }

    // Pool dispatch, the cost of resolving a query against a furnished
    // set of one, two or three overlapping kernels of the same kind.
    bench_pck_dispatch(&kernels);
    bench_spk_dispatch(&kernels);

    // bodn2c — 10 000 calls per iter, 6 names.
    for &name in BODN2C_NAMES {
        let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
            for _ in 0..BODN2C_CALLS_PER_ITER {
                black_box(cspice_wrap::bodn2c(name).unwrap());
            }
        });
        let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
            for _ in 0..BODN2C_CALLS_PER_ITER {
                black_box(spicekit.bodn2c(name).unwrap());
            }
        });
        print_row("bodn2c", name, BODN2C_CALLS_PER_ITER, cspice_t, spicekit_t);
    }

    let lsk = spicekit
        .leapseconds()
        .expect("default kernel set includes leapseconds");

    // LSK kernel-pool metadata and values — CSpice dtpool/gdpool vs.
    // spicekit's retained structured DELTET content.
    let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..LSK_POOL_CALLS_PER_ITER {
            for &name in LSK_NUMERIC_VARS {
                black_box(cspice_wrap::dtpool(name).unwrap().unwrap());
            }
        }
    });
    let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..LSK_POOL_CALLS_PER_ITER {
            black_box(lsk_metadata(lsk));
        }
    });
    print_row(
        "lsk_dtpool",
        "DELTET/*",
        LSK_POOL_CALLS_PER_ITER * LSK_NUMERIC_VARS.len(),
        cspice_t,
        spicekit_t,
    );

    let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..LSK_POOL_CALLS_PER_ITER {
            for &name in LSK_NUMERIC_VARS {
                black_box(cspice_wrap::gdpool(name).unwrap().unwrap());
            }
        }
    });
    let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..LSK_POOL_CALLS_PER_ITER {
            black_box(lsk_pool_values(lsk));
        }
    });
    print_row(
        "lsk_gdpool",
        "DELTET/*",
        LSK_POOL_CALLS_PER_ITER * LSK_NUMERIC_VARS.len(),
        cspice_t,
        spicekit_t,
    );

    // Earth frame-association lookups provided by naif-earth-itrf93.
    let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..FRAME_LOOKUP_CALLS_PER_ITER {
            black_box(cspice_wrap::cnmfrm("EARTH").unwrap());
        }
    });
    let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..FRAME_LOOKUP_CALLS_PER_ITER {
            black_box(spicekit.cnmfrm("EARTH").unwrap());
        }
    });
    print_row(
        "cnmfrm",
        "EARTH",
        FRAME_LOOKUP_CALLS_PER_ITER,
        cspice_t,
        spicekit_t,
    );

    let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..FRAME_LOOKUP_CALLS_PER_ITER {
            black_box(cspice_wrap::cidfrm(399).unwrap());
        }
    });
    let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..FRAME_LOOKUP_CALLS_PER_ITER {
            black_box(spicekit.cidfrm(399).unwrap());
        }
    });
    print_row(
        "cidfrm",
        "EARTH(399)",
        FRAME_LOOKUP_CALLS_PER_ITER,
        cspice_t,
        spicekit_t,
    );

    let cspice_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..FRAME_LOOKUP_CALLS_PER_ITER {
            black_box(cspice_wrap::namfrm("ITRF93").unwrap());
        }
    });
    let spicekit_t = run(TIMED_ITERS, WARMUP_ITERS, || {
        for _ in 0..FRAME_LOOKUP_CALLS_PER_ITER {
            black_box(spicekit.namfrm("ITRF93").unwrap());
        }
    });
    print_row(
        "namfrm",
        "ITRF93",
        FRAME_LOOKUP_CALLS_PER_ITER,
        cspice_t,
        spicekit_t,
    );
}
