"""Tests for built-in NAIF body-code table lookups.

`naif_bodn2c` and `naif_bodc2n` front the pure-Rust port of CSpice's
zzidmap table — no kernels required.
"""

from __future__ import annotations

import pytest

from spicekit import naif_bodc2n, naif_bodn2c


@pytest.mark.parametrize(
    "name,code",
    [
        ("SUN", 10),
        ("EARTH", 399),
        ("MOON", 301),
        ("MARS BARYCENTER", 4),
        ("SOLAR SYSTEM BARYCENTER", 0),
        ("JUPITER BARYCENTER", 5),
    ],
)
def test_bodn2c_known_bodies(name: str, code: int) -> None:
    assert naif_bodn2c(name) == code


@pytest.mark.parametrize(
    "code,name",
    [
        (10, "SUN"),
        (399, "EARTH"),
        (301, "MOON"),
        # Canonical barycenter spellings use spaces to match CSpice's
        # `bodc2n`. The underscore spelling is still accepted on input
        # (`test_bodn2c_known_bodies`).
        (0, "SOLAR SYSTEM BARYCENTER"),
        (4, "MARS BARYCENTER"),
        (-170, "JAMES WEBB SPACE TELESCOPE"),
        (-48, "HST"),
    ],
)
def test_bodc2n_known_ids(code: int, name: str) -> None:
    assert naif_bodc2n(code) == name


@pytest.mark.parametrize("name", ["sun", "Sun", "EaRtH"])
def test_bodn2c_case_insensitive(name: str) -> None:
    # CSpice parity: body-name lookups collapse case and internal whitespace.
    assert naif_bodn2c(name) == naif_bodn2c(name.upper())


def test_bodn2c_spacecraft_aliases() -> None:
    # JWST and HST each have abbreviated + full-name entries that map
    # to the same ID.
    assert naif_bodn2c("JWST") == naif_bodn2c("JAMES WEBB SPACE TELESCOPE")
    assert naif_bodn2c("HST") == naif_bodn2c("HUBBLE SPACE TELESCOPE")


def test_bodn2c_unknown_raises() -> None:
    with pytest.raises(ValueError):
        naif_bodn2c("NOT-A-NAIF-NAME")


def test_bodc2n_unknown_raises() -> None:
    with pytest.raises(ValueError):
        naif_bodc2n(-987654321)


@pytest.mark.parametrize("code", [10, 399, 301, 0, 4, 5])
def test_bodc2n_bodn2c_roundtrip(code: int) -> None:
    assert naif_bodn2c(naif_bodc2n(code)) == code
