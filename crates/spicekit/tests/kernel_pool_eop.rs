//! Kernel-backed tests for [`spicekit::KernelPool`].
//!
//! The three NAIF Earth orientation PCKs are too large to vendor, so
//! their paths come from `SPICEKIT_TEST_KERNEL_EOP_HISTORICAL`,
//! `SPICEKIT_TEST_KERNEL_EOP_HIGH_PREC` and
//! `SPICEKIT_TEST_KERNEL_EOP_PREDICT`, or the matching
//! `SPICEKIT_BENCH_KERNEL_` names a bench machine already has. Without
//! them each test prints what to set and returns.

use std::path::PathBuf;

use spicekit::{KernelPool, NaifFrame, PoolError};

/// NAIF frame id of ITRF93, the frame the Earth PCKs describe.
const ITRF93: i32 = 3_000;
/// 1978-12-23 TDB. Only the historical file reaches it.
const ET_1978: f64 = -663_508_800.0;
/// 2020-06-01 TDB. Historical and high precision both reach it.
const ET_2020: f64 = 644_241_600.0;
/// 2026-10-11 TDB. High precision and predict both reach it.
const ET_2026: f64 = 844_948_800.0;
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

/// The historical, high precision and predict PCKs, in that order.
fn eop() -> Option<[PathBuf; 3]> {
    let find = |name: &str| {
        let vars = [
            format!("SPICEKIT_TEST_KERNEL_{name}"),
            format!("SPICEKIT_BENCH_KERNEL_{name}"),
        ];
        vars.iter()
            .filter_map(|var| std::env::var(var).ok())
            .map(PathBuf::from)
            .find(|path| path.exists())
    };
    let found = [
        find("EOP_HISTORICAL"),
        find("EOP_HIGH_PREC"),
        find("EOP_PREDICT"),
    ];
    if found.iter().any(Option::is_none) {
        eprintln!(
            "skipping: set SPICEKIT_TEST_KERNEL_EOP_HISTORICAL, \
             SPICEKIT_TEST_KERNEL_EOP_HIGH_PREC and \
             SPICEKIT_TEST_KERNEL_EOP_PREDICT to the three NAIF Earth \
             orientation binary PCKs"
        );
        return None;
    }
    Some(found.map(|path| path.expect("checked above")))
}

#[test]
fn a_batch_across_the_three_files_works_in_every_furnish_order() {
    let Some(files) = eop() else { return };
    let ets = [ET_1978, ET_2020, ET_2026];
    for order in ORDERS {
        let mut pool = KernelPool::new();
        for i in order {
            pool.furnish(&files[i]).unwrap();
        }
        let batch = pool
            .pck_sxform_batch(ITRF93, NaifFrame::J2000, &ets)
            .unwrap_or_else(|e| panic!("batch failed for order {order:?}: {e}"));

        // Each batch entry is exactly the single-epoch answer.
        for (i, &et) in ets.iter().enumerate() {
            let single = pool.pck_sxform(ITRF93, NaifFrame::J2000, et).unwrap();
            assert_eq!(batch[i], single, "order {order:?} epoch {i}");
        }
        // The rotation-only form is the upper left block.
        let rotations = pool
            .pck_pxform_batch(ITRF93, NaifFrame::J2000, &ets)
            .unwrap();
        for (i, rotation) in rotations.iter().enumerate() {
            for (row, values) in rotation.iter().enumerate() {
                assert_eq!(values[..], batch[i][row][..3]);
            }
        }
    }
}

#[test]
fn an_epoch_before_1962_fails_and_names_its_index() {
    let Some(files) = eop() else { return };
    let mut pool = KernelPool::new();
    for path in &files {
        pool.furnish(path).unwrap();
    }
    let err = pool
        .pck_sxform_batch(ITRF93, NaifFrame::J2000, &[ET_2020, ET_1952])
        .expect_err("1952 predates every Earth orientation kernel");
    assert!(
        matches!(err, PoolError::PckInsufficientData { frame: ITRF93, index: 1, et } if et == ET_1952),
        "{err}"
    );
    assert!(err.to_string().contains("3000"), "{err}");
}

#[test]
fn coverage_is_the_merged_union_of_the_loaded_files() {
    let Some(files) = eop() else { return };
    let mut pool = KernelPool::new();
    pool.furnish(&files[0]).unwrap();
    let historical = pool.pck_coverage(ITRF93);
    assert_eq!(historical.len(), 1);

    for path in &files[1..] {
        pool.furnish(path).unwrap();
    }
    let all = pool.pck_coverage(ITRF93);
    assert_eq!(all.len(), 1, "the three files form one continuous window");
    assert!(all[0].0 <= historical[0].0 && all[0].1 > historical[0].1);
    for et in [ET_1978, ET_2020, ET_2026] {
        assert!(all[0].0 <= et && et <= all[0].1);
    }

    assert!(pool.unload(&files[2]));
    assert!(pool.pck_coverage(ITRF93)[0].1 < all[0].1);
}
