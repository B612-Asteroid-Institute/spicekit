//! Resolve NAIF kernel paths for the parity + bench suite.
//!
//! The six kernels mirror adam-core's `DEFAULT_KERNELS`: a leapseconds
//! file, DE440 ephemeris, three Earth EOP PCKs (predict / historical /
//! high-precision), and the Earth ITRF93 binary PCK. Paths are obtained
//! from the `naif-*` PyPI packages — each package exports a single
//! absolute-path string to the kernel it vendors.
//!
//! Resolution order per kernel:
//!   1. Env var (e.g. `SPICEKIT_BENCH_KERNEL_DE440`) — CI sets these
//!      after `uv pip install` so tests don't spawn Python.
//!   2. `python -c "from naif_de440 import de440; print(de440)"`
//!      fallback for local runs.
//!
//! If neither is available the function panics with a message telling
//! the developer which env var to set or which pip package to install.

use std::path::PathBuf;
use std::process::Command;

/// All kernels in the order the bench / parity tests furnsh them.
/// Last-loaded-wins applies to the Earth PCKs, matching how adam-core
/// loads them (predict → historical → high-prec → ITRF93 binary).
pub fn default_kernel_paths() -> Vec<PathBuf> {
    vec![
        resolve("LEAPSECONDS", "naif_leapseconds", "leapseconds"),
        resolve("DE440", "naif_de440", "de440"),
        resolve("EOP_PREDICT", "naif_eop_predict", "eop_predict"),
        resolve("EOP_HISTORICAL", "naif_eop_historical", "eop_historical"),
        resolve("EOP_HIGH_PREC", "naif_eop_high_prec", "eop_high_prec"),
        resolve("EARTH_ITRF93", "naif_earth_itrf93", "earth_itrf93"),
    ]
}

fn resolve(env_key: &str, module: &str, attr: &str) -> PathBuf {
    let env_var = format!("SPICEKIT_BENCH_KERNEL_{env_key}");
    if let Ok(p) = std::env::var(&env_var) {
        let pb = PathBuf::from(p);
        assert!(
            pb.exists(),
            "{env_var} points at missing file: {}",
            pb.display()
        );
        return pb;
    }

    let out = Command::new("python")
        .arg("-c")
        .arg(format!("from {module} import {attr}; print({attr})"))
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "failed to spawn `python -c` to resolve {module}.{attr}: {e}. \
                 Either `uv pip install {module}` first, or set the env var \
                 {env_var} to the kernel path directly."
            )
        });
    assert!(
        out.status.success(),
        "python -c failed resolving {module}.{attr}:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let path = String::from_utf8(out.stdout)
        .expect("python kernel-path output was not valid UTF-8")
        .trim()
        .to_string();
    let pb = PathBuf::from(&path);
    assert!(
        pb.exists(),
        "{module}.{attr} returned non-existent path: {path}"
    );
    pb
}
