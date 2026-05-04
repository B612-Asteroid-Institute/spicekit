# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `0501269d59815072afe403c054cedb51e5093566`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-05-04 20:39:31 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 0.71 | 0.15 | 4.75x | 1.12 | 0.19 | 5.88x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 71.76 | 6.38 | 11.24x | 99.37 | 6.45 | 15.40x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 716.52 | 62.93 | 11.39x | 758.37 | 73.12 | 10.37x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7109.29 | 638.08 | 11.14x | 8115.81 | 652.58 | 12.44x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.40 | 0.18 | 7.79x | 1.53 | 0.20 | 7.63x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 76.01 | 7.19 | 10.57x | 124.75 | 7.27 | 17.15x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 764.85 | 71.63 | 10.68x | 780.74 | 89.49 | 8.72x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 7580.57 | 718.06 | 10.56x | 7633.36 | 739.42 | 10.32x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.46 | 0.34 | 10.16x | 6.39 | 0.39 | 16.39x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 772.15 | 28.32 | 27.26x | 803.59 | 28.54 | 28.15x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2680.84 | 279.82 | 9.58x | 2751.54 | 300.05 | 9.17x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 21441.07 | 2827.49 | 7.58x | 21912.97 | 2986.99 | 7.34x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 4.07 | 0.54 | 7.52x | 4.33 | 0.58 | 7.45x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1088.39 | 36.34 | 29.95x | 1114.93 | 46.61 | 23.92x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3118.33 | 357.27 | 8.73x | 3417.93 | 369.34 | 9.25x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 22593.28 | 3602.53 | 6.27x | 22836.44 | 3629.03 | 6.29x |
| spkez_batch | moon_wrt_earth_itrf93 | 1 | 6.68 | 1.01 | 6.60x | 9.75 | 1.06 | 9.18x |
| spkez_batch | moon_wrt_earth_itrf93 | 100 | 1802.21 | 61.42 | 29.34x | 1833.69 | 76.53 | 23.96x |
| spkez_batch | moon_wrt_earth_itrf93 | 1000 | 7377.07 | 612.43 | 12.05x | 7600.98 | 628.67 | 12.09x |
| spkez_batch | moon_wrt_earth_itrf93 | 10000 | 43614.27 | 6123.54 | 7.12x | 48518.59 | 6237.07 | 7.78x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 1.39 | 0.31 | 4.49x | 6.02 | 0.34 | 17.66x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 149.47 | 18.22 | 8.20x | 162.23 | 26.98 | 6.01x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1484.66 | 181.44 | 8.18x | 1569.63 | 206.26 | 7.61x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 14640.18 | 1832.63 | 7.99x | 14811.35 | 1864.30 | 7.94x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 1.25 | 0.29 | 4.30x | 1.35 | 3.61 | 0.37x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 137.41 | 16.34 | 8.41x | 151.14 | 16.43 | 9.20x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1371.29 | 162.54 | 8.44x | 1472.13 | 177.28 | 8.30x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 13669.87 | 1644.68 | 8.31x | 13892.77 | 1685.75 | 8.24x |
| pxform_batch | ITRF93->J2000 | 1 | 1.72 | 0.31 | 5.56x | 2.00 | 0.33 | 6.05x |
| sxform_batch | ITRF93->J2000 | 1 | 3.73 | 0.34 | 10.93x | 7.34 | 0.43 | 17.04x |
| pxform_batch | ITRF93->J2000 | 100 | 651.89 | 23.89 | 27.28x | 657.29 | 34.34 | 19.14x |
| sxform_batch | ITRF93->J2000 | 100 | 662.74 | 22.91 | 28.92x | 671.06 | 23.34 | 28.75x |
| pxform_batch | ITRF93->J2000 | 1000 | 4105.52 | 241.03 | 17.03x | 4248.42 | 255.39 | 16.64x |
| sxform_batch | ITRF93->J2000 | 1000 | 4209.79 | 230.96 | 18.23x | 4314.53 | 243.55 | 17.71x |
| pxform_batch | ITRF93->J2000 | 10000 | 20840.96 | 2417.43 | 8.62x | 21348.32 | 2447.59 | 8.72x |
| sxform_batch | ITRF93->J2000 | 10000 | 21942.29 | 2323.51 | 9.44x | 22868.44 | 2342.54 | 9.76x |
| pxform_batch | J2000->ITRF93 | 1 | 1.71 | 0.31 | 5.51x | 1.83 | 0.33 | 5.54x |
| sxform_batch | J2000->ITRF93 | 1 | 3.22 | 0.39 | 8.23x | 4.77 | 0.54 | 8.82x |
| pxform_batch | J2000->ITRF93 | 100 | 659.76 | 24.25 | 27.21x | 666.04 | 35.53 | 18.75x |
| sxform_batch | J2000->ITRF93 | 100 | 681.77 | 28.57 | 23.86x | 687.22 | 52.51 | 13.09x |
| pxform_batch | J2000->ITRF93 | 1000 | 4108.30 | 243.93 | 16.84x | 4165.11 | 256.18 | 16.26x |
| sxform_batch | J2000->ITRF93 | 1000 | 4327.30 | 235.34 | 18.39x | 4387.46 | 248.71 | 17.64x |
| pxform_batch | J2000->ITRF93 | 10000 | 20993.79 | 2458.97 | 8.54x | 21373.58 | 2480.85 | 8.62x |
| sxform_batch | J2000->ITRF93 | 10000 | 22864.26 | 2376.70 | 9.62x | 23405.75 | 2400.56 | 9.75x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 1.99 | 0.38 | 5.23x | 7.21 | 0.44 | 16.36x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.43 | 0.28 | 15.76x | 4.84 | 0.29 | 16.63x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 681.34 | 23.95 | 28.44x | 688.19 | 35.13 | 19.59x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 709.05 | 22.95 | 30.89x | 755.66 | 34.81 | 21.70x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4322.12 | 240.33 | 17.98x | 4426.80 | 254.62 | 17.39x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4595.53 | 230.71 | 19.92x | 4679.51 | 261.70 | 17.88x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 23007.47 | 2420.01 | 9.51x | 23412.38 | 2457.86 | 9.53x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 25285.87 | 2319.42 | 10.90x | 25838.47 | 2335.74 | 11.06x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 3.45 | 0.31 | 11.08x | 4.14 | 0.35 | 11.79x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 2.95 | 0.29 | 10.12x | 4.82 | 0.30 | 16.01x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 680.91 | 24.23 | 28.10x | 687.34 | 35.53 | 19.35x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 706.31 | 23.26 | 30.36x | 717.81 | 33.91 | 21.17x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4337.27 | 243.41 | 17.82x | 4386.68 | 255.64 | 17.16x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4563.65 | 235.40 | 19.39x | 4756.77 | 245.73 | 19.36x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 23003.36 | 2458.51 | 9.36x | 23510.13 | 2485.06 | 9.46x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 25078.83 | 2368.49 | 10.59x | 25865.86 | 2385.08 | 10.84x |
| bodn2c | SUN | 10000 | 2832.79 | 1398.28 | 2.03x | 2909.44 | 1423.65 | 2.04x |
| bodn2c | EARTH | 10000 | 2943.01 | 1531.64 | 1.92x | 2959.47 | 1663.00 | 1.78x |
| bodn2c | MARS BARYCENTER | 10000 | 4645.92 | 2752.07 | 1.69x | 4817.80 | 2809.16 | 1.72x |
| bodn2c | MOON | 10000 | 2945.25 | 1463.61 | 2.01x | 2980.45 | 1727.28 | 1.73x |
| bodn2c | JWST | 10000 | 2908.89 | 1463.50 | 1.99x | 2927.25 | 1506.55 | 1.94x |
| bodn2c | HST | 10000 | 2944.31 | 1272.14 | 2.31x | 3009.92 | 1285.18 | 2.34x |
| lsk_dtpool | DELTET/* | 50000 | 12417.64 | 15.46 | 803.31x | 12562.92 | 27.83 | 451.38x |
| lsk_gdpool | DELTET/* | 50000 | 25715.63 | 2288.72 | 11.24x | 25947.65 | 2322.93 | 11.17x |
| cnmfrm | EARTH | 10000 | 9021.02 | 1834.51 | 4.92x | 9313.31 | 1874.20 | 4.97x |
| cidfrm | EARTH(399) | 10000 | 14125.68 | 2016.07 | 7.01x | 14267.28 | 2021.58 | 7.06x |
| namfrm | ITRF93 | 10000 | 2817.19 | 275.01 | 10.24x | 2969.28 | 286.58 | 10.36x |
