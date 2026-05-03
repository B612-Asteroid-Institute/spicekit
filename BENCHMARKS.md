# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `5c384ac98ea03a288375b0f379cb257a0757f469`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-05-03 12:59:18 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 1.24 | 0.16 | 7.72x | 1.90 | 0.24 | 7.93x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 74.61 | 6.29 | 11.86x | 87.21 | 6.37 | 13.69x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 747.72 | 62.20 | 12.02x | 775.07 | 71.36 | 10.86x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7475.52 | 633.40 | 11.80x | 11064.14 | 662.15 | 16.71x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.38 | 0.17 | 8.09x | 4.90 | 0.19 | 25.78x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 80.13 | 7.10 | 11.28x | 133.09 | 7.16 | 18.58x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 800.21 | 70.23 | 11.39x | 809.52 | 78.62 | 10.30x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 7967.39 | 717.41 | 11.11x | 9127.02 | 743.87 | 12.27x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.56 | 0.43 | 8.25x | 7.11 | 0.59 | 12.04x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 790.18 | 28.17 | 28.05x | 1137.93 | 43.94 | 25.90x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2891.35 | 279.89 | 10.33x | 3108.61 | 289.49 | 10.74x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 23419.44 | 2824.15 | 8.29x | 24068.80 | 2863.75 | 8.40x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 4.24 | 0.53 | 7.98x | 8.47 | 0.58 | 14.55x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1118.04 | 36.03 | 31.03x | 1156.56 | 36.81 | 31.42x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3372.18 | 357.09 | 9.44x | 3431.73 | 426.01 | 8.06x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 25237.29 | 3601.48 | 7.01x | 25457.66 | 3707.33 | 6.87x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 3.15 | 0.30 | 10.45x | 3.61 | 0.47 | 7.66x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 169.07 | 18.13 | 9.32x | 194.33 | 30.18 | 6.44x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1685.50 | 181.96 | 9.26x | 1734.25 | 190.45 | 9.11x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 16876.57 | 1860.60 | 9.07x | 27393.77 | 2211.85 | 12.38x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 2.93 | 0.28 | 10.41x | 3.61 | 0.31 | 11.60x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 169.31 | 16.15 | 10.48x | 191.83 | 27.49 | 6.98x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1723.48 | 160.99 | 10.71x | 2935.26 | 178.70 | 16.43x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 17655.82 | 1629.33 | 10.84x | 19880.69 | 1875.01 | 10.60x |
| pxform_batch | ITRF93->J2000 | 1 | 1.85 | 0.29 | 6.37x | 20.61 | 0.35 | 58.88x |
| sxform_batch | ITRF93->J2000 | 1 | 3.97 | 0.36 | 10.99x | 4.47 | 0.39 | 11.43x |
| pxform_batch | ITRF93->J2000 | 100 | 678.78 | 24.02 | 28.25x | 703.41 | 24.83 | 28.33x |
| sxform_batch | ITRF93->J2000 | 100 | 684.39 | 23.10 | 29.62x | 846.50 | 23.32 | 36.29x |
| pxform_batch | ITRF93->J2000 | 1000 | 4309.73 | 241.42 | 17.85x | 5985.59 | 258.47 | 23.16x |
| sxform_batch | ITRF93->J2000 | 1000 | 4398.67 | 232.25 | 18.94x | 5865.79 | 263.76 | 22.24x |
| pxform_batch | ITRF93->J2000 | 10000 | 22231.11 | 2412.66 | 9.21x | 24443.32 | 2453.90 | 9.96x |
| sxform_batch | ITRF93->J2000 | 10000 | 23384.06 | 2328.99 | 10.04x | 24251.15 | 2363.18 | 10.26x |
| pxform_batch | J2000->ITRF93 | 1 | 1.74 | 0.30 | 5.79x | 3.83 | 0.93 | 4.11x |
| sxform_batch | J2000->ITRF93 | 1 | 3.39 | 0.36 | 9.41x | 3.83 | 0.43 | 8.88x |
| pxform_batch | J2000->ITRF93 | 100 | 675.70 | 24.44 | 27.65x | 701.02 | 24.99 | 28.06x |
| sxform_batch | J2000->ITRF93 | 100 | 693.04 | 23.50 | 29.49x | 701.06 | 23.65 | 29.64x |
| pxform_batch | J2000->ITRF93 | 1000 | 4205.36 | 245.24 | 17.15x | 4396.76 | 260.95 | 16.85x |
| sxform_batch | J2000->ITRF93 | 1000 | 4442.06 | 236.38 | 18.79x | 4519.70 | 256.69 | 17.61x |
| pxform_batch | J2000->ITRF93 | 10000 | 21890.77 | 2465.75 | 8.88x | 22951.02 | 2518.67 | 9.11x |
| sxform_batch | J2000->ITRF93 | 10000 | 23986.70 | 2380.43 | 10.08x | 25286.72 | 2407.92 | 10.50x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.10 | 0.30 | 7.01x | 2.25 | 0.33 | 6.81x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.37 | 0.34 | 6.94x | 8.70 | 0.62 | 14.00x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 696.85 | 23.98 | 29.07x | 728.57 | 35.59 | 20.47x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 718.36 | 23.09 | 31.11x | 723.78 | 23.83 | 30.37x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4474.16 | 240.54 | 18.60x | 4533.33 | 251.63 | 18.02x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4672.74 | 232.01 | 20.14x | 4786.73 | 247.97 | 19.30x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 24348.55 | 2429.03 | 10.02x | 25121.94 | 3267.08 | 7.69x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 26622.84 | 2325.92 | 11.45x | 27505.12 | 2369.66 | 11.61x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 1.97 | 0.30 | 6.56x | 2.18 | 0.35 | 6.22x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 2.23 | 0.35 | 6.37x | 14.97 | 0.37 | 40.35x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 690.62 | 24.46 | 28.24x | 715.61 | 35.48 | 20.17x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 708.75 | 23.55 | 30.09x | 724.42 | 34.92 | 20.74x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4454.69 | 245.72 | 18.13x | 4731.65 | 267.77 | 17.67x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4691.45 | 236.16 | 19.87x | 5524.98 | 254.15 | 21.74x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 24260.50 | 2473.78 | 9.81x | 25455.04 | 2519.96 | 10.10x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 26680.84 | 2378.34 | 11.22x | 27480.26 | 2433.82 | 11.29x |
| bodn2c | SUN | 10000 | 2768.81 | 1421.71 | 1.95x | 2841.11 | 1459.54 | 1.95x |
| bodn2c | EARTH | 10000 | 2899.68 | 1569.08 | 1.85x | 2967.53 | 1673.68 | 1.77x |
| bodn2c | MARS BARYCENTER | 10000 | 3795.52 | 2690.57 | 1.41x | 3828.34 | 2710.23 | 1.41x |
| bodn2c | MOON | 10000 | 2868.12 | 1492.19 | 1.92x | 3260.37 | 1529.46 | 2.13x |
| bodn2c | JWST | 10000 | 2833.49 | 1496.60 | 1.89x | 2875.52 | 1514.82 | 1.90x |
| bodn2c | HST | 10000 | 2885.65 | 1303.65 | 2.21x | 2934.21 | 1515.28 | 1.94x |
