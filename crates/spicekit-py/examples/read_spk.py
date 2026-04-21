"""Read positions and velocities from an SPK kernel.

Run with:
    python read_spk.py path/to/de440.bsp

Prints the state (km, km/s) of Earth (NAIF ID 399) relative to the
Solar System Barycenter (NAIF ID 0) at J2000 epoch (et = 0.0).
"""

from __future__ import annotations

import sys

import numpy as np

import spicekit


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(f"usage: {sys.argv[0]} <path-to-spk>")
    spk = spicekit.NaifSpk(sys.argv[1])

    x, y, z, vx, vy, vz = spk.state(399, 0, 0.0)
    print(
        "Earth (399) rel SSB (0) at ET=0.0:\n"
        f"  position = ({x:+.3f}, {y:+.3f}, {z:+.3f}) km\n"
        f"  velocity = ({vx:+.6f}, {vy:+.6f}, {vz:+.6f}) km/s"
    )

    ets = np.linspace(0.0, 86_400.0 * 7, 8, dtype=np.float64)
    states = spk.state_batch(399, 0, ets)
    print(f"\nBatched: {states.shape=} over one week from J2000.")


if __name__ == "__main__":
    main()
