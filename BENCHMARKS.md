# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `3dbac8db5429aa8241e1093103638933f1790e7d`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-10-20 16:02:14 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 0.76 | 0.14 | 5.44x | 1.95 | 0.19 | 10.28x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 75.18 | 5.76 | 13.05x | 91.94 | 5.84 | 15.74x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 753.57 | 56.73 | 13.28x | 772.52 | 66.05 | 11.70x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7445.75 | 579.44 | 12.85x | 7499.90 | 615.50 | 12.19x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.52 | 0.17 | 8.91x | 18.02 | 0.20 | 90.12x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 80.87 | 6.25 | 12.94x | 137.68 | 6.32 | 21.78x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 819.12 | 61.61 | 13.30x | 896.75 | 70.42 | 12.73x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 8132.02 | 626.11 | 12.99x | 8472.15 | 634.05 | 13.36x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.55 | 0.40 | 8.85x | 7.51 | 0.46 | 16.30x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 849.71 | 23.91 | 35.53x | 862.34 | 24.64 | 35.00x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2933.57 | 237.33 | 12.36x | 3227.41 | 252.35 | 12.79x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 23588.30 | 2399.09 | 9.83x | 25944.20 | 2442.01 | 10.62x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 2.49 | 0.36 | 6.91x | 8.57 | 0.45 | 18.99x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1206.75 | 30.24 | 39.91x | 1222.88 | 44.93 | 27.22x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3473.12 | 298.04 | 11.65x | 3516.50 | 310.23 | 11.34x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 25334.98 | 3002.16 | 8.44x | 25566.56 | 3026.63 | 8.45x |
| spkez_batch | moon_wrt_earth_itrf93 | 1 | 7.37 | 0.93 | 7.92x | 29.51 | 1.11 | 26.51x |
| spkez_batch | moon_wrt_earth_itrf93 | 100 | 1971.02 | 54.23 | 36.34x | 2026.93 | 68.85 | 29.44x |
| spkez_batch | moon_wrt_earth_itrf93 | 1000 | 8040.18 | 536.42 | 14.99x | 8131.32 | 543.25 | 14.97x |
| spkez_batch | moon_wrt_earth_itrf93 | 10000 | 47722.01 | 5365.89 | 8.89x | 61382.87 | 5455.23 | 11.25x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 1.56 | 0.27 | 5.77x | 6.12 | 0.31 | 19.75x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 155.98 | 15.96 | 9.77x | 190.13 | 25.58 | 7.43x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1554.08 | 157.36 | 9.88x | 1577.09 | 164.89 | 9.56x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 15538.58 | 1578.07 | 9.85x | 15860.80 | 1593.26 | 9.95x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 1.55 | 0.21 | 7.36x | 1.64 | 0.23 | 7.11x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 144.81 | 14.61 | 9.91x | 178.15 | 23.04 | 7.73x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1455.32 | 145.35 | 10.01x | 1533.27 | 179.84 | 8.53x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 14520.52 | 1465.80 | 9.91x | 16372.76 | 1472.54 | 11.12x |
| pxform_batch | ITRF93->J2000 | 1 | 1.93 | 0.28 | 6.88x | 2.07 | 0.33 | 6.26x |
| sxform_batch | ITRF93->J2000 | 1 | 1.98 | 0.36 | 5.50x | 6.22 | 0.63 | 9.86x |
| pxform_batch | ITRF93->J2000 | 100 | 718.27 | 21.78 | 32.98x | 732.29 | 33.04 | 22.16x |
| sxform_batch | ITRF93->J2000 | 100 | 725.38 | 21.01 | 34.53x | 727.28 | 21.13 | 34.42x |
| pxform_batch | ITRF93->J2000 | 1000 | 4544.41 | 217.44 | 20.90x | 4616.74 | 229.37 | 20.13x |
| sxform_batch | ITRF93->J2000 | 1000 | 4645.65 | 207.16 | 22.43x | 4897.67 | 219.84 | 22.28x |
| pxform_batch | ITRF93->J2000 | 10000 | 23304.51 | 2201.43 | 10.59x | 24140.61 | 2259.24 | 10.69x |
| sxform_batch | ITRF93->J2000 | 10000 | 24166.53 | 2100.18 | 11.51x | 25246.49 | 2146.86 | 11.76x |
| pxform_batch | J2000->ITRF93 | 1 | 2.02 | 0.28 | 7.20x | 2.15 | 0.32 | 6.71x |
| sxform_batch | J2000->ITRF93 | 1 | 3.93 | 0.36 | 10.91x | 6.57 | 0.54 | 12.15x |
| pxform_batch | J2000->ITRF93 | 100 | 725.73 | 22.51 | 32.24x | 737.02 | 33.95 | 21.71x |
| sxform_batch | J2000->ITRF93 | 100 | 745.98 | 21.59 | 34.55x | 749.11 | 37.97 | 19.73x |
| pxform_batch | J2000->ITRF93 | 1000 | 4592.53 | 225.69 | 20.35x | 4836.66 | 241.55 | 20.02x |
| sxform_batch | J2000->ITRF93 | 1000 | 4767.04 | 215.69 | 22.10x | 4922.44 | 226.65 | 21.72x |
| pxform_batch | J2000->ITRF93 | 10000 | 22982.10 | 2284.12 | 10.06x | 23986.19 | 2528.31 | 9.49x |
| sxform_batch | J2000->ITRF93 | 10000 | 24850.89 | 2182.54 | 11.39x | 26031.87 | 2253.54 | 11.55x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.21 | 0.37 | 5.99x | 8.28 | 0.42 | 19.66x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.30 | 0.34 | 12.64x | 4.75 | 0.38 | 12.50x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 737.06 | 21.69 | 33.98x | 750.67 | 22.22 | 33.78x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 759.20 | 20.73 | 36.62x | 945.87 | 32.24 | 29.34x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4798.32 | 217.13 | 22.10x | 6849.45 | 238.25 | 28.75x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 5001.03 | 209.00 | 23.93x | 5481.95 | 222.76 | 24.61x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 25843.35 | 2199.34 | 11.75x | 26461.44 | 2268.76 | 11.66x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 27705.36 | 2098.73 | 13.20x | 28526.65 | 2177.48 | 13.10x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 4.28 | 0.36 | 11.85x | 8.38 | 0.43 | 19.43x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 4.82 | 0.43 | 11.18x | 5.58 | 0.46 | 12.10x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 745.05 | 22.47 | 33.15x | 752.10 | 33.39 | 22.52x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 766.69 | 21.52 | 35.63x | 772.90 | 21.69 | 35.63x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4817.31 | 226.78 | 21.24x | 5127.06 | 248.41 | 20.64x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 5043.11 | 216.92 | 23.25x | 5109.80 | 231.38 | 22.08x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 25686.31 | 2281.49 | 11.26x | 26451.91 | 2430.16 | 10.88x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 28055.80 | 2182.99 | 12.85x | 28918.80 | 2294.41 | 12.60x |
| pool_sxform_batch | eop_1_file | 1 | 4.16 | 0.50 | 8.30x | 28.04 | 0.59 | 47.45x |
| pool_sxform_batch | eop_1_file | 100 | 229.83 | 30.04 | 7.65x | 259.84 | 38.95 | 6.67x |
| pool_sxform_batch | eop_1_file | 1000 | 2299.28 | 287.77 | 7.99x | 2329.31 | 298.95 | 7.79x |
| pool_sxform_batch | eop_1_file | 10000 | 22909.55 | 2909.30 | 7.87x | 23229.84 | 3036.78 | 7.65x |
| pool_sxform_batch | eop_2_files | 1 | 2.33 | 0.35 | 6.65x | 2.71 | 0.44 | 6.13x |
| pool_sxform_batch | eop_2_files | 100 | 230.76 | 28.91 | 7.98x | 255.83 | 39.20 | 6.53x |
| pool_sxform_batch | eop_2_files | 1000 | 2331.65 | 297.34 | 7.84x | 2436.49 | 310.31 | 7.85x |
| pool_sxform_batch | eop_2_files | 10000 | 23223.63 | 3000.71 | 7.74x | 24081.34 | 3041.80 | 7.92x |
| pool_sxform_batch | eop_3_files | 1 | 2.17 | 0.35 | 6.19x | 2.48 | 0.40 | 6.20x |
| pool_sxform_batch | eop_3_files | 100 | 221.37 | 28.95 | 7.65x | 238.85 | 49.92 | 4.78x |
| pool_sxform_batch | eop_3_files | 1000 | 2228.63 | 297.33 | 7.50x | 2314.64 | 308.78 | 7.50x |
| pool_sxform_batch | eop_3_files | 10000 | 22136.86 | 2998.11 | 7.38x | 22789.91 | 3011.88 | 7.57x |
| pool_sxform_batch | eop_3_files_straddle | 1 | 3.68 | 0.53 | 6.92x | 4.48 | 0.63 | 7.10x |
| pool_sxform_batch | eop_3_files_straddle | 100 | 865.14 | 30.03 | 28.81x | 901.02 | 31.31 | 28.78x |
| pool_sxform_batch | eop_3_files_straddle | 1000 | 7770.37 | 320.69 | 24.23x | 7837.36 | 344.08 | 22.78x |
| pool_sxform_batch | eop_3_files_straddle | 10000 | 52320.13 | 3324.66 | 15.74x | 53512.03 | 3690.25 | 14.50x |
| pool_spkez_batch | de440_x1 | 1 | 1.47 | 0.37 | 3.98x | 1.65 | 18.15 | 0.09x |
| pool_spkez_batch | de440_x1 | 100 | 690.06 | 31.12 | 22.18x | 704.74 | 31.62 | 22.29x |
| pool_spkez_batch | de440_x1 | 1000 | 2232.06 | 309.66 | 7.21x | 2265.25 | 332.27 | 6.82x |
| pool_spkez_batch | de440_x1 | 10000 | 17425.47 | 3111.11 | 5.60x | 17697.65 | 3127.51 | 5.66x |
| pool_spkez_batch | de440_x2 | 1 | 1.51 | 0.37 | 4.08x | 1.71 | 0.40 | 4.27x |
| pool_spkez_batch | de440_x2 | 100 | 674.92 | 31.15 | 21.67x | 713.17 | 42.06 | 16.96x |
| pool_spkez_batch | de440_x2 | 1000 | 2237.32 | 308.18 | 7.26x | 2867.66 | 324.37 | 8.84x |
| pool_spkez_batch | de440_x2 | 10000 | 17128.81 | 3115.59 | 5.50x | 17270.65 | 3138.94 | 5.50x |
| pool_spkez_batch | de440_x3 | 1 | 1.50 | 0.46 | 3.26x | 1.65 | 0.61 | 2.71x |
| pool_spkez_batch | de440_x3 | 100 | 669.59 | 31.10 | 21.53x | 701.42 | 34.29 | 20.45x |
| pool_spkez_batch | de440_x3 | 1000 | 2122.96 | 309.07 | 6.87x | 2157.21 | 321.38 | 6.71x |
| pool_spkez_batch | de440_x3 | 10000 | 16498.37 | 3112.44 | 5.30x | 16960.89 | 3142.12 | 5.40x |
| bodn2c | SUN | 10000 | 3021.51 | 1306.26 | 2.31x | 3312.68 | 1323.12 | 2.50x |
| bodn2c | EARTH | 10000 | 3118.69 | 1555.19 | 2.01x | 3185.49 | 1590.91 | 2.00x |
| bodn2c | MARS BARYCENTER | 10000 | 3999.19 | 2609.15 | 1.53x | 4144.58 | 2664.08 | 1.56x |
| bodn2c | MOON | 10000 | 3098.62 | 1437.80 | 2.16x | 3170.98 | 1465.99 | 2.16x |
| bodn2c | JWST | 10000 | 3073.98 | 1482.49 | 2.07x | 3136.10 | 1500.45 | 2.09x |
| bodn2c | HST | 10000 | 3129.58 | 1245.03 | 2.51x | 3174.08 | 1413.03 | 2.25x |
| lsk_dtpool | DELTET/* | 50000 | 13123.99 | 15.46 | 848.95x | 13206.37 | 24.10 | 548.07x |
| lsk_gdpool | DELTET/* | 50000 | 27006.08 | 2284.07 | 11.82x | 27101.52 | 2307.21 | 11.75x |
| cnmfrm | EARTH | 10000 | 9238.82 | 1765.73 | 5.23x | 9569.50 | 1782.67 | 5.37x |
| cidfrm | EARTH(399) | 10000 | 14418.63 | 1953.69 | 7.38x | 14506.69 | 1964.28 | 7.39x |
| namfrm | ITRF93 | 10000 | 2879.14 | 228.21 | 12.62x | 2960.80 | 261.70 | 11.31x |
