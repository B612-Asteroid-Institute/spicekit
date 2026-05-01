"""Python bindings for spicekit: pure-Rust NAIF kernel readers.

The public surface mirrors the underlying Rust crate one-to-one:

- ``NaifSpk``: DAF/SPK reader (state, state_batch, state_batch_in_frame, segments).
- ``NaifPck``: DAF/PCK reader (euler_state, sxform, pxform, sxform_batch,
  pxform_batch, rotate_state_batch, segments).
- ``NaifSpkWriter``: DAF/SPK writer (add_type3, add_type9, write).
- ``naif_bodn2c`` / ``naif_bodc2n``: built-in NAIF body-code table lookups.
- ``naif_parse_text_kernel_bindings``: parse ``NAIF_BODY_NAME`` ↔
  ``NAIF_BODY_CODE`` assignments from text kernels.
"""

from ._rust_native import (
    NaifPck,
    NaifSpk,
    NaifSpkWriter,
    naif_bodc2n,
    naif_bodn2c,
    naif_parse_text_kernel_bindings,
)

__all__ = [
    "NaifPck",
    "NaifSpk",
    "NaifSpkWriter",
    "naif_bodc2n",
    "naif_bodn2c",
    "naif_parse_text_kernel_bindings",
]
