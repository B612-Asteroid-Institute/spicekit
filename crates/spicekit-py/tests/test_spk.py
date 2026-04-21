"""Tests for `NaifSpk` against the DE440 kernel from `naif-de440`."""

from __future__ import annotations

import numpy as np
import pytest

from spicekit import NaifSpk

J2000_TDB_SECONDS = 0.0
ONE_DAY_S = 86400.0


@pytest.fixture(scope="module")
def de440(de440_path: str) -> NaifSpk:
    return NaifSpk(de440_path)


def test_open_missing_file_raises(tmp_path) -> None:
    with pytest.raises(Exception):
        NaifSpk(str(tmp_path / "does-not-exist.bsp"))


def test_state_returns_six_tuple(de440: NaifSpk) -> None:
    s = de440.state(target=399, center=0, et=J2000_TDB_SECONDS)
    assert len(s) == 6
    assert all(isinstance(v, float) for v in s)
    # Earth wrt SSB at J2000: position magnitude ~1 AU (1.496e8 km).
    pos = np.array(s[:3])
    assert 1.4e8 < np.linalg.norm(pos) < 1.6e8


def test_state_batch_shape_and_dtype(de440: NaifSpk) -> None:
    ets = np.linspace(-10 * ONE_DAY_S, 10 * ONE_DAY_S, 17)
    out = de440.state_batch(target=399, center=10, ets=ets)
    assert out.shape == (17, 6)
    assert out.dtype == np.float64


def test_state_batch_matches_scalar(de440: NaifSpk) -> None:
    ets = np.linspace(-5 * ONE_DAY_S, 5 * ONE_DAY_S, 11)
    batch = de440.state_batch(target=301, center=399, ets=ets)
    for i, et in enumerate(ets):
        scalar = np.array(de440.state(target=301, center=399, et=float(et)))
        np.testing.assert_allclose(batch[i], scalar, rtol=0.0, atol=0.0)


def test_state_batch_in_frame_j2000_matches_default(de440: NaifSpk) -> None:
    # `state_batch` returns states in the segment's native frame. For
    # DE440 that is J2000, so `state_batch_in_frame(..., "J2000")` is
    # identical.
    ets = np.array([-ONE_DAY_S, 0.0, ONE_DAY_S], dtype=np.float64)
    default = de440.state_batch(target=399, center=10, ets=ets)
    j2000 = de440.state_batch_in_frame(
        target=399, center=10, ets=ets, frame="J2000"
    )
    np.testing.assert_allclose(default, j2000, rtol=0.0, atol=0.0)


def test_state_batch_in_frame_eclipj2000_differs_from_j2000(de440: NaifSpk) -> None:
    ets = np.array([0.0], dtype=np.float64)
    j2000 = de440.state_batch_in_frame(target=399, center=10, ets=ets, frame="J2000")
    eclip = de440.state_batch_in_frame(
        target=399, center=10, ets=ets, frame="ECLIPJ2000"
    )
    # Same vector length, different orientation.
    np.testing.assert_allclose(
        np.linalg.norm(j2000[0, :3]), np.linalg.norm(eclip[0, :3]), rtol=1e-12
    )
    assert not np.allclose(j2000[0, :3], eclip[0, :3])


def test_state_batch_in_frame_unsupported_raises(de440: NaifSpk) -> None:
    ets = np.array([0.0], dtype=np.float64)
    with pytest.raises(ValueError):
        de440.state_batch_in_frame(
            target=399, center=10, ets=ets, frame="ITRF93"
        )


def test_segments_nonempty_and_typed(de440: NaifSpk) -> None:
    segs = de440.segments()
    assert len(segs) > 0
    target, center, frame, data_type, start_et, end_et, name = segs[0]
    assert isinstance(target, int)
    assert isinstance(center, int)
    assert isinstance(frame, int)
    assert isinstance(data_type, int)
    assert isinstance(start_et, float)
    assert isinstance(end_et, float)
    assert isinstance(name, str)
    assert start_et < end_et


def test_state_outside_coverage_raises(de440: NaifSpk) -> None:
    # DE440 does not cover the year 10000 BC-ish.
    far_past_et = -1.0e18
    with pytest.raises(Exception):
        de440.state(target=399, center=0, et=far_past_et)
