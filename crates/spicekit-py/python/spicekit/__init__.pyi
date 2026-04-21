from __future__ import annotations

import numpy as np
from numpy.typing import NDArray

__all__ = [
    "NaifSpk",
    "NaifPck",
    "NaifSpkWriter",
    "naif_bodn2c",
    "naif_bodc2n",
    "naif_parse_text_kernel_bindings",
]

# --- SPK reader -----------------------------------------------------------

class NaifSpk:
    """Pure-Rust reader for a DAF/SPK ephemeris kernel."""

    def __init__(self, path: str) -> None: ...
    def state(
        self, target: int, center: int, et: float
    ) -> tuple[float, float, float, float, float, float]:
        """Return (x, y, z, vx, vy, vz) in km and km/s at TDB seconds past J2000."""
    def state_batch(
        self, target: int, center: int, ets: NDArray[np.float64]
    ) -> NDArray[np.float64]:
        """Return shape (N, 6) state vectors for each et in `ets`."""
    def state_batch_in_frame(
        self,
        target: int,
        center: int,
        ets: NDArray[np.float64],
        frame: str,
    ) -> NDArray[np.float64]:
        """`state_batch` with explicit output frame ("J2000" or "ECLIPJ2000")."""
    def segments(self) -> list[tuple[int, int, int, int, float, float, str]]:
        """Per-segment metadata: (target, center, frame_id, data_type, start_et, end_et, name)."""

# --- PCK reader -----------------------------------------------------------

class NaifPck:
    """Pure-Rust reader for a binary PCK (orientation) kernel."""

    def __init__(self, path: str) -> None: ...
    def euler_state(
        self, body_frame: int, et: float
    ) -> tuple[float, float, float, float, float, float]:
        """Raw (RA, DEC, W, dRA, dDEC, dW) in rad and rad/s at ET."""
    def sxform(self, from_: str, to: str, et: float) -> NDArray[np.float64]:
        """6×6 state-transform matrix for frame pair at ET."""
    def pxform(self, from_: str, to: str, et: float) -> NDArray[np.float64]:
        """3×3 rotation matrix for frame pair at ET."""
    def pxform_batch(
        self, from_: str, to: str, ets: NDArray[np.float64]
    ) -> NDArray[np.float64]:
        """Shape (N, 3, 3) batched `pxform`."""
    def sxform_batch(
        self, from_: str, to: str, ets: NDArray[np.float64]
    ) -> NDArray[np.float64]:
        """Shape (N, 6, 6) batched `sxform`."""
    def rotate_state_batch(
        self,
        from_: str,
        to: str,
        ets: NDArray[np.float64],
        states: NDArray[np.float64],
    ) -> NDArray[np.float64]:
        """Apply `sxform(from, to, ets[i])` to `states[i]`; returns shape (N, 6)."""
    def segments(self) -> list[tuple[int, int, int, float, float, str]]:
        """Per-segment metadata: (body_frame, ref_frame, data_type, start_et, end_et, name)."""

# --- SPK writer -----------------------------------------------------------

class NaifSpkWriter:
    """Pure-Rust writer that serializes DAF/SPK bytes with an atomic rename."""

    def __init__(self, locifn: str) -> None: ...
    def add_type3(
        self,
        target: int,
        center: int,
        frame_id: int,
        start_et: float,
        end_et: float,
        segment_id: str,
        init: float,
        intlen: float,
        records_coeffs: NDArray[np.float64],
    ) -> None:
        """Append a Type 3 (Chebyshev position+velocity) segment.

        `records_coeffs` is (n_records, 2 + 6*(degree+1)); each row is
        [mid, radius, x..., y..., z..., vx..., vy..., vz...].
        """
    def add_type9(
        self,
        target: int,
        center: int,
        frame_id: int,
        start_et: float,
        end_et: float,
        segment_id: str,
        degree: int,
        states: NDArray[np.float64],
        epochs: NDArray[np.float64],
    ) -> None:
        """Append a Type 9 (Lagrange, unequal time steps) segment.

        `states` is (N, 6), `epochs` is (N,).
        """
    def write(self, path: str) -> None:
        """Serialize to `path` using an atomic rename."""

# --- name/code helpers ----------------------------------------------------

def naif_bodn2c(name: str) -> int:
    """NAIF body-name → integer code (case-insensitive, whitespace-tolerant)."""

def naif_bodc2n(code: int) -> str:
    """NAIF integer code → canonical body name."""

def naif_parse_text_kernel_bindings(path: str) -> list[tuple[str, int]]:
    """Parse `NAIF_BODY_NAME` / `NAIF_BODY_CODE` pairs from a text kernel."""
