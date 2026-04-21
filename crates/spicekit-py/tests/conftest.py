"""Shared pytest fixtures for spicekit-py integration tests.

Kernel paths come from the `naif-*` PyPI packages (each vendors one
kernel and exports its absolute path as a module-level string).
"""

from __future__ import annotations

import pytest


@pytest.fixture(scope="session")
def de440_path() -> str:
    from naif_de440 import de440

    return de440


@pytest.fixture(scope="session")
def leapseconds_path() -> str:
    from naif_leapseconds import leapseconds

    return leapseconds


@pytest.fixture(scope="session")
def earth_itrf93_path() -> str:
    # Text frames kernel binding the name "ITRF93" to body-frame 3000.
    from naif_earth_itrf93 import earth_itrf93

    return earth_itrf93


@pytest.fixture(scope="session")
def eop_high_prec_path() -> str:
    # Binary PCK (`earth_latest_high_prec.bpc`) — contains the rotation
    # data that body-frame 3000 (ITRF93) refers to.
    from naif_eop_high_prec import eop_high_prec

    return eop_high_prec
