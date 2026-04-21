"""Tests for `naif_parse_text_kernel_bindings`."""

from __future__ import annotations

from pathlib import Path

import pytest

from spicekit import naif_parse_text_kernel_bindings


def write_tk(path: Path, body: str) -> None:
    path.write_text(body)


def test_parses_simple_bindings(tmp_path: Path) -> None:
    tk = tmp_path / "custom_names.tk"
    write_tk(
        tk,
        "KPL/FK\n"
        "\\begindata\n"
        "NAIF_BODY_NAME += ( 'ADAM_PROBE', 'APROBE' )\n"
        "NAIF_BODY_CODE += ( -900001, -900001 )\n"
        "\\begintext\n",
    )
    bindings = naif_parse_text_kernel_bindings(str(tk))
    assert bindings == [("ADAM_PROBE", -900001), ("APROBE", -900001)]


def test_preserves_declaration_order(tmp_path: Path) -> None:
    tk = tmp_path / "order.tk"
    write_tk(
        tk,
        "\\begindata\n"
        "NAIF_BODY_NAME += ( 'FIRST', 'SECOND', 'THIRD' )\n"
        "NAIF_BODY_CODE += ( -1, -2, -3 )\n"
        "\\begintext\n",
    )
    bindings = naif_parse_text_kernel_bindings(str(tk))
    assert bindings == [("FIRST", -1), ("SECOND", -2), ("THIRD", -3)]


def test_empty_file_returns_empty_list(tmp_path: Path) -> None:
    tk = tmp_path / "empty.tk"
    write_tk(tk, "KPL/FK\nno data blocks here\n")
    assert naif_parse_text_kernel_bindings(str(tk)) == []


def test_mismatched_lengths_raises(tmp_path: Path) -> None:
    tk = tmp_path / "bad.tk"
    write_tk(
        tk,
        "\\begindata\n"
        "NAIF_BODY_NAME += ( 'A', 'B' )\n"
        "NAIF_BODY_CODE += ( -1 )\n"
        "\\begintext\n",
    )
    with pytest.raises(ValueError):
        naif_parse_text_kernel_bindings(str(tk))


def test_missing_file_raises(tmp_path: Path) -> None:
    with pytest.raises(ValueError):
        naif_parse_text_kernel_bindings(str(tmp_path / "nope.tk"))


def test_real_leapseconds_kernel_has_no_body_bindings(leapseconds_path: str) -> None:
    # LSK files declare leapseconds, not body bindings — parser should
    # accept them and return an empty list.
    assert naif_parse_text_kernel_bindings(leapseconds_path) == []
