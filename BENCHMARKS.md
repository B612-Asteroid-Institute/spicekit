# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `801ce5d323b7ab7bb0d3c9dc0fde3ddc7b5ef698`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-04-20 15:51:16 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 1.42 | 0.31 | 4.57x | 35.49 | 0.38 | 93.14x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 76.83 | 17.62 | 4.36x | 164.68 | 17.70 | 9.30x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 767.72 | 175.18 | 4.38x | 790.11 | 184.33 | 4.29x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7646.25 | 1783.31 | 4.29x | 13311.72 | 2328.34 | 5.72x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.29 | 0.29 | 4.46x | 1.39 | 0.31 | 4.49x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 83.28 | 18.49 | 4.50x | 120.10 | 27.78 | 4.32x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 830.97 | 184.50 | 4.50x | 1245.62 | 193.33 | 6.44x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 8259.62 | 1857.24 | 4.45x | 8509.70 | 1885.05 | 4.51x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.17 | 0.87 | 3.65x | 3.60 | 0.94 | 3.82x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 807.52 | 50.31 | 16.05x | 810.86 | 61.83 | 13.12x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2779.55 | 510.21 | 5.45x | 3014.26 | 547.23 | 5.51x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 22283.07 | 5069.59 | 4.40x | 36392.70 | 5609.34 | 6.49x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 4.17 | 1.12 | 3.71x | 4.41 | 1.17 | 3.76x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1135.48 | 66.88 | 16.98x | 1338.10 | 77.45 | 17.28x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3217.96 | 676.65 | 4.76x | 3265.41 | 912.75 | 3.58x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 23424.86 | 6741.98 | 3.47x | 25033.58 | 7001.41 | 3.58x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 2.68 | 0.58 | 4.61x | 20.88 | 0.64 | 32.57x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 157.73 | 31.96 | 4.94x | 183.20 | 32.74 | 5.60x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1578.69 | 321.72 | 4.91x | 1612.27 | 379.17 | 4.25x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 15726.40 | 3211.61 | 4.90x | 16308.62 | 3248.86 | 5.02x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 2.73 | 0.47 | 5.81x | 2.92 | 0.60 | 4.85x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 157.62 | 28.24 | 5.58x | 205.30 | 37.66 | 5.45x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1593.70 | 279.80 | 5.70x | 1652.48 | 289.13 | 5.72x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 15942.86 | 2823.85 | 5.65x | 16921.98 | 4570.36 | 3.70x |
| pxform_batch | ITRF93->J2000 | 1 | 3.24 | 0.66 | 4.90x | 3.63 | 0.71 | 5.10x |
| sxform_batch | ITRF93->J2000 | 1 | 3.43 | 0.62 | 5.51x | 3.84 | 0.70 | 5.47x |
| pxform_batch | ITRF93->J2000 | 100 | 1169.86 | 54.45 | 21.48x | 1199.79 | 71.07 | 16.88x |
| sxform_batch | ITRF93->J2000 | 100 | 1202.50 | 53.45 | 22.50x | 1240.92 | 69.00 | 17.98x |
| pxform_batch | ITRF93->J2000 | 1000 | 4596.33 | 361.66 | 12.71x | 7775.43 | 406.90 | 19.11x |
| sxform_batch | ITRF93->J2000 | 1000 | 4437.14 | 350.39 | 12.66x | 4662.96 | 436.76 | 10.68x |
| pxform_batch | ITRF93->J2000 | 10000 | 22063.81 | 3644.33 | 6.05x | 23063.75 | 3748.43 | 6.15x |
| sxform_batch | ITRF93->J2000 | 10000 | 23346.20 | 3528.67 | 6.62x | 24405.41 | 3796.02 | 6.43x |
| pxform_batch | J2000->ITRF93 | 1 | 3.15 | 0.66 | 4.76x | 3.68 | 0.69 | 5.31x |
| sxform_batch | J2000->ITRF93 | 1 | 3.42 | 0.64 | 5.33x | 4.03 | 0.65 | 6.18x |
| pxform_batch | J2000->ITRF93 | 100 | 686.93 | 36.97 | 18.58x | 692.30 | 47.76 | 14.50x |
| sxform_batch | J2000->ITRF93 | 100 | 704.79 | 35.90 | 19.63x | 719.49 | 46.91 | 15.34x |
| pxform_batch | J2000->ITRF93 | 1000 | 4313.11 | 360.90 | 11.95x | 4387.95 | 373.57 | 11.75x |
| sxform_batch | J2000->ITRF93 | 1000 | 4507.96 | 355.34 | 12.69x | 4858.18 | 367.65 | 13.21x |
| pxform_batch | J2000->ITRF93 | 10000 | 22141.65 | 3662.18 | 6.05x | 25042.20 | 3787.82 | 6.61x |
| sxform_batch | J2000->ITRF93 | 10000 | 23990.68 | 3545.72 | 6.77x | 24858.85 | 3651.49 | 6.81x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 3.82 | 0.67 | 5.69x | 4.25 | 0.75 | 5.66x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.30 | 0.51 | 8.39x | 4.52 | 0.55 | 8.22x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 712.23 | 36.74 | 19.39x | 719.56 | 47.44 | 15.17x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 732.22 | 35.83 | 20.44x | 1056.90 | 46.25 | 22.85x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4568.77 | 360.89 | 12.66x | 4834.00 | 378.33 | 12.78x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4762.26 | 350.52 | 13.59x | 5004.08 | 369.71 | 13.54x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 24617.61 | 3638.19 | 6.77x | 25888.00 | 3797.18 | 6.82x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 26829.51 | 3528.40 | 7.60x | 27919.73 | 3682.76 | 7.58x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 3.63 | 0.60 | 6.02x | 4.26 | 0.77 | 5.52x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 4.13 | 0.52 | 7.92x | 4.32 | 0.57 | 7.56x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 709.57 | 36.94 | 19.21x | 721.32 | 47.73 | 15.11x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 728.19 | 35.74 | 20.38x | 739.26 | 46.36 | 15.95x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4588.29 | 365.25 | 12.56x | 4734.37 | 383.27 | 12.35x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4777.32 | 354.60 | 13.47x | 5126.50 | 374.90 | 13.67x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 24925.45 | 3656.67 | 6.82x | 28902.94 | 3715.28 | 7.78x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 26947.23 | 3558.05 | 7.57x | 27802.52 | 5949.47 | 4.67x |
| bodn2c | SUN | 10000 | 2969.71 | 1265.46 | 2.35x | 3802.82 | 1360.95 | 2.79x |
| bodn2c | EARTH | 10000 | 3111.71 | 1529.83 | 2.03x | 3352.70 | 2099.24 | 1.60x |
| bodn2c | MARS BARYCENTER | 10000 | 3962.96 | 2705.56 | 1.46x | 3993.66 | 2797.77 | 1.43x |
| bodn2c | MOON | 10000 | 3065.15 | 1456.26 | 2.10x | 4270.40 | 1488.35 | 2.87x |
| bodn2c | JWST | 10000 | 3037.95 | 1531.98 | 1.98x | 3094.95 | 1564.65 | 1.98x |
| bodn2c | HST | 10000 | 3144.26 | 1260.78 | 2.49x | 3206.29 | 1338.25 | 2.40x |
