"""Round-trip tests for `NaifSpkWriter`.

Write a synthetic SPK segment to disk, re-open it with `NaifSpk`, and
assert the reader recovers the same states at the sample epochs.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest

from spicekit import NaifSpk, NaifSpkWriter

J2000_FRAME_ID = 1
ONE_HOUR_S = 3600.0


def _synthetic_states(n: int, seed: int = 0) -> tuple[np.ndarray, np.ndarray]:
    """N epochs spanning 24 hours and a smooth analytic state trajectory."""
    rng = np.random.default_rng(seed=seed)
    epochs = np.linspace(0.0, 24 * ONE_HOUR_S, n)
    # Smooth function of time so Lagrange interpolation is well-behaved.
    t = epochs / ONE_HOUR_S
    states = np.column_stack(
        [
            1e6 + 10.0 * t + 0.1 * t * t,
            2e6 - 5.0 * t + 0.05 * t * t,
            3e6 + 7.0 * t,
            1.0 + 0.01 * t,
            2.0 - 0.02 * t,
            3.0 + 0.001 * t,
        ]
    )
    # Add a tiny deterministic jitter to prove we're reading written bytes.
    states = states + 1e-3 * rng.standard_normal(states.shape)
    return epochs, states


def test_type9_roundtrip_samples_match(tmp_path: Path) -> None:
    epochs, states = _synthetic_states(n=32, seed=1)

    writer = NaifSpkWriter("spicekit-py-test")
    writer.add_type9(
        target=-900001,
        center=0,
        frame_id=J2000_FRAME_ID,
        start_et=float(epochs[0]),
        end_et=float(epochs[-1]),
        segment_id="SYN-T9",
        degree=5,
        states=states,
        epochs=epochs,
    )
    out = tmp_path / "synthetic_type9.bsp"
    writer.write(str(out))
    assert out.exists() and out.stat().st_size > 0

    reader = NaifSpk(str(out))
    segs = reader.segments()
    assert len(segs) == 1
    target, center, frame, data_type, start_et, end_et, name = segs[0]
    assert target == -900001
    assert center == 0
    assert frame == J2000_FRAME_ID
    assert data_type == 9
    assert name.strip() == "SYN-T9"
    assert start_et == pytest.approx(epochs[0])
    assert end_et == pytest.approx(epochs[-1])

    # At each sample epoch, Lagrange interpolation is exact.
    recovered = reader.state_batch(target=-900001, center=0, ets=epochs)
    np.testing.assert_allclose(recovered, states, rtol=0.0, atol=1e-9)


def test_type9_rejects_bad_shapes(tmp_path: Path) -> None:
    writer = NaifSpkWriter()
    states_5col = np.zeros((4, 5))
    epochs = np.linspace(0.0, 3.0, 4)
    with pytest.raises(ValueError):
        writer.add_type9(
            target=-1,
            center=0,
            frame_id=J2000_FRAME_ID,
            start_et=0.0,
            end_et=3.0,
            segment_id="BAD",
            degree=3,
            states=states_5col,
            epochs=epochs,
        )

    states = np.zeros((4, 6))
    short_epochs = np.linspace(0.0, 3.0, 3)
    with pytest.raises(ValueError):
        writer.add_type9(
            target=-1,
            center=0,
            frame_id=J2000_FRAME_ID,
            start_et=0.0,
            end_et=3.0,
            segment_id="BAD",
            degree=3,
            states=states,
            epochs=short_epochs,
        )


def test_type3_rejects_bad_row_length(tmp_path: Path) -> None:
    writer = NaifSpkWriter()
    # Need mid, radius, then multiple-of-6 coefs — 2 + 7 = 9 is invalid.
    bad = np.zeros((2, 9))
    with pytest.raises(ValueError):
        writer.add_type3(
            target=-1,
            center=0,
            frame_id=J2000_FRAME_ID,
            start_et=0.0,
            end_et=10.0,
            segment_id="BAD-T3",
            init=0.0,
            intlen=5.0,
            records_coeffs=bad,
        )


def test_multiple_segments_in_one_file(tmp_path: Path) -> None:
    epochs_a, states_a = _synthetic_states(n=16, seed=1)
    epochs_b, states_b = _synthetic_states(n=16, seed=2)

    writer = NaifSpkWriter()
    writer.add_type9(
        target=-900001,
        center=0,
        frame_id=J2000_FRAME_ID,
        start_et=float(epochs_a[0]),
        end_et=float(epochs_a[-1]),
        segment_id="SEG-A",
        degree=5,
        states=states_a,
        epochs=epochs_a,
    )
    writer.add_type9(
        target=-900002,
        center=0,
        frame_id=J2000_FRAME_ID,
        start_et=float(epochs_b[0]),
        end_et=float(epochs_b[-1]),
        segment_id="SEG-B",
        degree=5,
        states=states_b,
        epochs=epochs_b,
    )
    out = tmp_path / "two_segments.bsp"
    writer.write(str(out))

    reader = NaifSpk(str(out))
    segs = reader.segments()
    assert len(segs) == 2
    assert {s[0] for s in segs} == {-900001, -900002}
