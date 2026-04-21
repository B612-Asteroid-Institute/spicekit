"""Tests for `NaifPck` against the earth_itrf93 binary PCK."""

from __future__ import annotations

import numpy as np
import pytest

from spicekit import NaifPck

ONE_DAY_S = 86400.0
# `earth_latest_high_prec.bpc` coverage starts near ET = -43135 and runs
# forward; pick epochs comfortably inside that window so every test
# targets a valid interpolation interval.
SAFE_ET_0 = 3600.0


@pytest.fixture(scope="module")
def earth_pck(eop_high_prec_path: str) -> NaifPck:
    return NaifPck(eop_high_prec_path)


def test_open_missing_file_raises(tmp_path) -> None:
    with pytest.raises(Exception):
        NaifPck(str(tmp_path / "does-not-exist.bpc"))


def test_euler_state_returns_six_tuple(earth_pck: NaifPck) -> None:
    # body_frame=3000 is ITRF93.
    s = earth_pck.euler_state(3000, SAFE_ET_0)
    assert len(s) == 6
    assert all(isinstance(v, float) for v in s)


def test_pxform_is_orthogonal(earth_pck: NaifPck) -> None:
    r = earth_pck.pxform("ITRF93", "J2000", SAFE_ET_0)
    assert r.shape == (3, 3)
    np.testing.assert_allclose(r @ r.T, np.eye(3), rtol=0.0, atol=1e-12)


def test_pxform_batch_matches_scalar(earth_pck: NaifPck) -> None:
    ets = np.linspace(SAFE_ET_0, SAFE_ET_0 + ONE_DAY_S, 5)
    batch = earth_pck.pxform_batch("J2000", "ITRF93", ets)
    assert batch.shape == (5, 3, 3)
    for i, et in enumerate(ets):
        scalar = earth_pck.pxform("J2000", "ITRF93", float(et))
        np.testing.assert_allclose(batch[i], scalar, rtol=0.0, atol=0.0)


def test_sxform_top_left_equals_pxform(earth_pck: NaifPck) -> None:
    et = SAFE_ET_0
    s = earth_pck.sxform("ITRF93", "J2000", et)
    p = earth_pck.pxform("ITRF93", "J2000", et)
    assert s.shape == (6, 6)
    np.testing.assert_allclose(s[:3, :3], p, rtol=0.0, atol=1e-14)


def test_sxform_batch_shape(earth_pck: NaifPck) -> None:
    ets = np.array([SAFE_ET_0, SAFE_ET_0 + 100.0, SAFE_ET_0 + 200.0], dtype=np.float64)
    batch = earth_pck.sxform_batch("ITRF93", "J2000", ets)
    assert batch.shape == (3, 6, 6)


def test_pxform_forward_inverse_roundtrip(earth_pck: NaifPck) -> None:
    et = SAFE_ET_0 + 1234.5
    fwd = earth_pck.pxform("ITRF93", "J2000", et)
    inv = earth_pck.pxform("J2000", "ITRF93", et)
    np.testing.assert_allclose(fwd @ inv, np.eye(3), rtol=0.0, atol=1e-12)


def test_rotate_state_batch_matches_sxform_apply(earth_pck: NaifPck) -> None:
    ets = np.array(
        [SAFE_ET_0, SAFE_ET_0 + 3600.0, SAFE_ET_0 + 7200.0], dtype=np.float64
    )
    rng = np.random.default_rng(seed=42)
    states = rng.normal(size=(3, 6))
    rotated = earth_pck.rotate_state_batch("ITRF93", "J2000", ets, states)
    assert rotated.shape == (3, 6)
    sx = earth_pck.sxform_batch("ITRF93", "J2000", ets)
    for i in range(3):
        np.testing.assert_allclose(
            rotated[i], sx[i] @ states[i], rtol=1e-14, atol=1e-14
        )


def test_unsupported_body_frame_raises(earth_pck: NaifPck) -> None:
    with pytest.raises(ValueError):
        earth_pck.pxform("IAU_MARS", "J2000", SAFE_ET_0)


def test_both_inertial_pair_raises(earth_pck: NaifPck) -> None:
    with pytest.raises(ValueError):
        earth_pck.pxform("J2000", "ECLIPJ2000", SAFE_ET_0)


def test_segments_nonempty_and_typed(earth_pck: NaifPck) -> None:
    segs = earth_pck.segments()
    assert len(segs) > 0
    body_frame, ref_frame, data_type, start_et, end_et, name = segs[0]
    assert isinstance(body_frame, int)
    assert isinstance(ref_frame, int)
    assert isinstance(data_type, int)
    assert isinstance(start_et, float)
    assert isinstance(end_et, float)
    assert isinstance(name, str)
    assert start_et < end_et
