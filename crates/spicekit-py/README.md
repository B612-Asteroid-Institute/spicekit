# spicekit (Python)

[![PyPI](https://img.shields.io/pypi/v/spicekit.svg)](https://pypi.org/project/spicekit/)
[![CI](https://github.com/B612-Asteroid-Institute/spicekit/actions/workflows/ci.yml/badge.svg)](https://github.com/B612-Asteroid-Institute/spicekit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../../LICENSE)

Python bindings for [`spicekit`](../spicekit), a pure-Rust reader for
NASA/NAIF SPICE kernel formats (DAF, SPK, PCK, text kernels). Independent
of the CSpice toolkit.

## Install

```bash
pip install spicekit
```

Development install (from the workspace root):

```bash
maturin develop --release --manifest-path crates/spicekit-py/Cargo.toml
```

## Usage

```python
import spicekit

spk = spicekit.NaifSpk("/path/to/de440.bsp")
state = spk.state(target=399, center=0, et=0.0)  # (x, y, z, vx, vy, vz)

pck = spicekit.NaifPck("/path/to/earth_latest_high_prec.bpc")
m = pck.sxform("J2000", "ITRF93", et=0.0)  # 6x6 state-transform matrix

code = spicekit.naif_bodn2c("EARTH")  # 399
name = spicekit.naif_bodc2n(399)      # "EARTH"
bindings = spicekit.naif_parse_text_kernel_bindings("/path/to/ids.tf")
```

## Surface

- `NaifSpk(path)`: `state`, `state_batch`, `state_batch_in_frame`, `segments`.
- `NaifPck(path)`: `euler_state`, `sxform`, `pxform`, `sxform_batch`,
  `pxform_batch`, `rotate_state_batch`, `segments`.
- `NaifSpkWriter(locifn)`: `add_type3`, `add_type9`, `write`.
- `naif_bodn2c(name)` / `naif_bodc2n(code)`: built-in NAIF body-code table.
- `naif_parse_text_kernel_bindings(path)`: parse `NAIF_BODY_NAME` ↔
  `NAIF_BODY_CODE` assignments from text kernels.
