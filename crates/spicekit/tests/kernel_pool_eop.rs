//! Kernel-backed test for [`spicekit::KernelPool`].
//!
//! The three NAIF Earth orientation PCKs are too large to vendor, so
//! their paths come from `SPICEKIT_TEST_KERNEL_EOP_HISTORICAL`,
//! `SPICEKIT_TEST_KERNEL_EOP_HIGH_PREC` and
//! `SPICEKIT_TEST_KERNEL_EOP_PREDICT`, or the matching
//! `SPICEKIT_BENCH_KERNEL_` names a bench machine already has. Without
//! them the test prints what to set and returns.

use std::path::PathBuf;

use spicekit::{KernelPool, NaifFrame, PoolError};

/// NAIF frame id of ITRF93, the frame the Earth PCKs describe.
const ITRF93: i32 = 3_000;
/// 1978-12-23, 2020-06-01 and 2026-10-11 TDB, one epoch per file.
const ETS: [f64; 3] = [-663_508_800.0, 644_241_600.0, 844_948_800.0];
/// About 1952, before every Earth orientation kernel starts.
const ET_1952: f64 = -1_500_000_000.0;
/// The six furnish orders of three files.
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

#[test]
fn the_three_earth_orientation_pcks_resolve_as_one_set() {
    let find = |name: &str| {
        ["TEST", "BENCH"]
            .iter()
            .filter_map(|k| std::env::var(format!("SPICEKIT_{k}_KERNEL_{name}")).ok())
            .map(PathBuf::from)
            .find(|path| path.exists())
    };
    let found = ["EOP_HISTORICAL", "EOP_HIGH_PREC", "EOP_PREDICT"].map(find);
    if found.iter().any(Option::is_none) {
        eprintln!("skipping: the three SPICEKIT_TEST_KERNEL_EOP_ paths are not set");
        return;
    }
    let files = found.map(|path| path.expect("checked above"));

    // Every furnish order resolves the whole batch, each entry equals
    // the single epoch answer, and the rotation is its upper left block.
    let mut pool = KernelPool::new();
    for order in ORDERS {
        pool.clear();
        for i in order {
            pool.furnish(&files[i]).unwrap();
        }
        let batch = pool
            .pck_sxform_batch(ITRF93, NaifFrame::J2000, &ETS)
            .unwrap_or_else(|e| panic!("batch failed for order {order:?}: {e}"));
        let rotations = pool
            .pck_pxform_batch(ITRF93, NaifFrame::J2000, &ETS)
            .unwrap();
        for (i, &et) in ETS.iter().enumerate() {
            let single = pool.pck_sxform(ITRF93, NaifFrame::J2000, et).unwrap();
            assert_eq!(batch[i], single, "order {order:?} epoch {i}");
            for (row, values) in rotations[i].iter().enumerate() {
                assert_eq!(values[..], batch[i][row][..3]);
            }
        }
    }

    // An epoch no file covers is a typed error naming its index.
    let err = pool
        .pck_sxform_batch(ITRF93, NaifFrame::J2000, &[ETS[1], ET_1952])
        .expect_err("1952 predates every Earth orientation kernel");
    let named = matches!(err, PoolError::PckInsufficientData { frame: ITRF93, index: 1, et }
        if et == ET_1952);
    assert!(named && err.to_string().contains("3000"), "{err}");

    // Coverage is the merged union, so dropping a file shortens it.
    let all = pool.pck_coverage(ITRF93);
    assert_eq!(all.len(), 1, "the three files form one continuous window");
    assert!(ETS.iter().all(|&et| all[0].0 <= et && et <= all[0].1));
    assert!(pool.unload(&files[2]));
    let shorter = pool.pck_coverage(ITRF93);
    assert!(shorter[0].1 < all[0].1);
    assert!(pool.unload(&files[1]));
    assert_eq!(pool.pck_coverage(ITRF93)[0].0, all[0].0);
    assert!(pool.pck_coverage(ITRF93)[0].1 < shorter[0].1);
}
