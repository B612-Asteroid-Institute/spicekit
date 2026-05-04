# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `084edd626e31af164799fbaeadcc8e7a46d7903b`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-05-04 20:25:34 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 0.68 | 0.16 | 4.26x | 1.15 | 0.20 | 5.75x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 69.80 | 6.51 | 10.72x | 85.80 | 6.57 | 13.06x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 715.64 | 63.59 | 11.25x | 731.24 | 71.92 | 10.17x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7106.51 | 643.14 | 11.05x | 7183.10 | 656.77 | 10.94x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.20 | 0.17 | 7.07x | 1.58 | 0.20 | 7.91x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 76.22 | 7.19 | 10.60x | 87.85 | 7.22 | 12.17x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 755.62 | 69.95 | 10.80x | 793.81 | 78.24 | 10.15x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 7519.40 | 708.35 | 10.62x | 7552.32 | 712.61 | 10.60x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 2.72 | 0.42 | 6.49x | 3.27 | 0.44 | 7.40x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 852.01 | 25.68 | 33.18x | 878.78 | 37.74 | 23.29x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2789.69 | 255.04 | 10.94x | 2831.72 | 264.98 | 10.69x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 21793.27 | 2570.87 | 8.48x | 22024.20 | 2622.42 | 8.40x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 2.28 | 0.50 | 4.56x | 2.43 | 1.12 | 2.17x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1201.30 | 33.85 | 35.49x | 1211.45 | 34.35 | 35.27x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3211.15 | 326.08 | 9.85x | 3340.51 | 336.94 | 9.91x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 22117.59 | 3272.86 | 6.76x | 24085.44 | 3357.07 | 7.17x |
| spkez_batch | moon_wrt_earth_itrf93 | 1 | 6.24 | 0.88 | 7.08x | 6.44 | 0.94 | 6.84x |
| spkez_batch | moon_wrt_earth_itrf93 | 100 | 1993.75 | 58.01 | 34.37x | 2138.18 | 67.63 | 31.61x |
| spkez_batch | moon_wrt_earth_itrf93 | 1000 | 7791.66 | 579.86 | 13.44x | 8015.10 | 594.60 | 13.48x |
| spkez_batch | moon_wrt_earth_itrf93 | 10000 | 43558.02 | 5730.85 | 7.60x | 45002.63 | 5800.02 | 7.76x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 2.09 | 0.29 | 7.19x | 2.44 | 0.35 | 6.96x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 147.87 | 16.86 | 8.77x | 156.98 | 17.07 | 9.20x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1504.21 | 168.63 | 8.92x | 1556.86 | 177.28 | 8.78x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 15025.98 | 1699.50 | 8.84x | 15238.26 | 1711.25 | 8.90x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 1.21 | 0.29 | 4.18x | 1.91 | 0.32 | 5.98x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 135.83 | 16.52 | 8.22x | 155.47 | 16.87 | 9.22x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1386.10 | 164.49 | 8.43x | 1440.12 | 173.86 | 8.28x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 13751.59 | 1653.89 | 8.31x | 13859.82 | 1687.72 | 8.21x |
| pxform_batch | ITRF93->J2000 | 1 | 2.59 | 0.29 | 8.91x | 3.52 | 0.34 | 10.31x |
| sxform_batch | ITRF93->J2000 | 1 | 2.84 | 0.35 | 8.13x | 3.28 | 0.41 | 8.01x |
| pxform_batch | ITRF93->J2000 | 100 | 728.71 | 23.88 | 30.52x | 753.80 | 24.23 | 31.12x |
| sxform_batch | ITRF93->J2000 | 100 | 732.49 | 22.84 | 32.06x | 736.61 | 32.48 | 22.68x |
| pxform_batch | ITRF93->J2000 | 1000 | 4459.04 | 240.77 | 18.52x | 4572.14 | 267.19 | 17.11x |
| sxform_batch | ITRF93->J2000 | 1000 | 4634.65 | 229.28 | 20.21x | 4856.56 | 241.51 | 20.11x |
| pxform_batch | ITRF93->J2000 | 10000 | 21824.19 | 2396.28 | 9.11x | 22458.26 | 2445.06 | 9.19x |
| sxform_batch | ITRF93->J2000 | 10000 | 22628.09 | 2298.55 | 9.84x | 28883.70 | 2528.80 | 11.42x |
| pxform_batch | J2000->ITRF93 | 1 | 2.57 | 0.37 | 6.95x | 3.47 | 0.43 | 8.06x |
| sxform_batch | J2000->ITRF93 | 1 | 3.33 | 0.34 | 9.81x | 8.34 | 0.42 | 19.86x |
| pxform_batch | J2000->ITRF93 | 100 | 739.84 | 24.07 | 30.74x | 767.62 | 28.70 | 26.74x |
| sxform_batch | J2000->ITRF93 | 100 | 745.11 | 22.95 | 32.46x | 769.92 | 24.01 | 32.07x |
| pxform_batch | J2000->ITRF93 | 1000 | 4417.21 | 239.93 | 18.41x | 5811.73 | 251.84 | 23.08x |
| sxform_batch | J2000->ITRF93 | 1000 | 4671.31 | 230.19 | 20.29x | 4732.46 | 247.98 | 19.08x |
| pxform_batch | J2000->ITRF93 | 10000 | 21551.10 | 2402.76 | 8.97x | 22084.75 | 2440.93 | 9.05x |
| sxform_batch | J2000->ITRF93 | 10000 | 23559.81 | 2300.35 | 10.24x | 24136.51 | 2334.63 | 10.34x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 1.99 | 0.30 | 6.64x | 3.91 | 0.33 | 11.80x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 3.42 | 0.34 | 10.04x | 17.21 | 0.36 | 47.66x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 750.00 | 24.04 | 31.20x | 890.54 | 33.64 | 26.47x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 776.54 | 22.91 | 33.89x | 783.41 | 32.63 | 24.01x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4605.82 | 239.51 | 19.23x | 4654.62 | 251.05 | 18.54x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4917.65 | 268.72 | 18.30x | 8042.62 | 289.93 | 27.74x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 23453.44 | 2396.00 | 9.79x | 24243.80 | 2407.88 | 10.07x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 25729.51 | 2297.26 | 11.20x | 26269.80 | 2352.35 | 11.17x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 1.88 | 0.36 | 5.23x | 6.46 | 0.60 | 10.75x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 3.42 | 0.35 | 9.76x | 4.26 | 0.37 | 11.47x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 747.27 | 24.09 | 31.02x | 756.12 | 34.14 | 22.15x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 779.42 | 22.91 | 34.02x | 936.38 | 33.75 | 27.74x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4640.43 | 240.24 | 19.32x | 4776.96 | 278.47 | 17.15x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4900.05 | 229.84 | 21.32x | 5212.90 | 245.90 | 21.20x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 23537.08 | 2404.74 | 9.79x | 24222.88 | 2428.17 | 9.98x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 25973.02 | 2300.72 | 11.29x | 27177.34 | 2316.64 | 11.73x |
| bodn2c | SUN | 10000 | 2723.76 | 1314.49 | 2.07x | 2751.34 | 1360.90 | 2.02x |
| bodn2c | EARTH | 10000 | 2815.78 | 1614.79 | 1.74x | 2971.04 | 1625.73 | 1.83x |
| bodn2c | MARS BARYCENTER | 10000 | 3811.55 | 2958.83 | 1.29x | 3854.01 | 3009.51 | 1.28x |
| bodn2c | MOON | 10000 | 2794.36 | 1507.30 | 1.85x | 2889.97 | 1523.40 | 1.90x |
| bodn2c | JWST | 10000 | 2762.61 | 1508.00 | 1.83x | 2787.42 | 1562.02 | 1.78x |
| bodn2c | HST | 10000 | 2821.99 | 1319.10 | 2.14x | 2971.04 | 1486.11 | 2.00x |
| lsk_dtpool | DELTET/* | 50000 | 12374.70 | 17.46 | 708.91x | 12623.30 | 19.20 | 657.50x |
| lsk_gdpool | DELTET/* | 50000 | 25844.29 | 2386.28 | 10.83x | 30011.49 | 2417.79 | 12.41x |
| cnmfrm | EARTH | 10000 | 8850.88 | 2066.63 | 4.28x | 13053.31 | 3231.90 | 4.04x |
| cidfrm | EARTH(399) | 10000 | 13704.76 | 1959.03 | 7.00x | 18811.20 | 2015.47 | 9.33x |
| namfrm | ITRF93 | 10000 | 2637.32 | 272.61 | 9.67x | 3556.20 | 361.40 | 9.84x |
