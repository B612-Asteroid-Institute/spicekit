# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `00232ccef4cd13b8eaeac9bc2b44ebe0611abc62`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-05-02 00:51:14 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 1.38 | 0.16 | 8.64x | 2.01 | 0.20 | 10.02x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 77.28 | 6.35 | 12.17x | 127.77 | 6.45 | 19.80x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 771.41 | 62.99 | 12.25x | 794.16 | 72.16 | 11.01x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7660.20 | 640.08 | 11.97x | 8121.31 | 711.73 | 11.41x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.35 | 0.16 | 8.40x | 1.47 | 0.17 | 8.66x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 83.11 | 7.11 | 11.68x | 173.58 | 10.79 | 16.09x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 828.36 | 70.21 | 11.80x | 831.28 | 81.09 | 10.25x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 8232.92 | 718.48 | 11.46x | 8340.99 | 815.93 | 10.22x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.86 | 0.43 | 8.95x | 7.01 | 0.52 | 13.46x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 838.74 | 28.33 | 29.60x | 859.75 | 28.40 | 30.27x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 3026.84 | 280.71 | 10.78x | 3209.41 | 303.51 | 10.57x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 24655.95 | 2832.28 | 8.71x | 28386.36 | 2861.33 | 9.92x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 5.32 | 0.42 | 12.64x | 5.85 | 0.45 | 12.97x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1190.67 | 36.18 | 32.91x | 1203.78 | 46.61 | 25.83x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3576.25 | 358.31 | 9.98x | 3659.94 | 379.14 | 9.65x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 26747.87 | 3605.31 | 7.42x | 27409.44 | 3633.88 | 7.54x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 1.76 | 0.30 | 5.86x | 6.65 | 0.32 | 20.79x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 178.58 | 18.29 | 9.76x | 188.25 | 26.65 | 7.06x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1789.51 | 183.02 | 9.78x | 1861.14 | 191.21 | 9.73x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 17855.42 | 1840.26 | 9.70x | 18054.09 | 1860.48 | 9.70x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 3.62 | 0.28 | 12.87x | 3.97 | 0.96 | 4.12x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 181.11 | 16.42 | 11.03x | 194.91 | 16.55 | 11.78x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1822.28 | 163.31 | 11.16x | 1910.17 | 175.56 | 10.88x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 18205.86 | 1650.51 | 11.03x | 18484.19 | 1670.46 | 11.07x |
| pxform_batch | ITRF93->J2000 | 1 | 3.33 | 0.40 | 8.29x | 7.59 | 18.04 | 0.42x |
| sxform_batch | ITRF93->J2000 | 1 | 3.59 | 0.36 | 9.94x | 3.90 | 0.40 | 9.72x |
| pxform_batch | ITRF93->J2000 | 100 | 709.45 | 23.90 | 29.68x | 716.20 | 24.23 | 29.56x |
| sxform_batch | ITRF93->J2000 | 100 | 722.04 | 23.02 | 31.36x | 790.93 | 33.96 | 23.29x |
| pxform_batch | ITRF93->J2000 | 1000 | 4530.18 | 238.04 | 19.03x | 4767.30 | 255.43 | 18.66x |
| sxform_batch | ITRF93->J2000 | 1000 | 4591.73 | 230.48 | 19.92x | 4698.99 | 264.77 | 17.75x |
| pxform_batch | ITRF93->J2000 | 10000 | 23844.65 | 2395.90 | 9.95x | 26622.96 | 2418.16 | 11.01x |
| sxform_batch | ITRF93->J2000 | 10000 | 24907.96 | 2314.95 | 10.76x | 25804.69 | 2356.57 | 10.95x |
| pxform_batch | J2000->ITRF93 | 1 | 1.86 | 0.39 | 4.76x | 6.88 | 0.57 | 12.05x |
| sxform_batch | J2000->ITRF93 | 1 | 3.52 | 0.34 | 10.31x | 3.82 | 0.43 | 8.86x |
| pxform_batch | J2000->ITRF93 | 100 | 714.16 | 24.39 | 29.29x | 719.26 | 36.11 | 19.92x |
| sxform_batch | J2000->ITRF93 | 100 | 734.87 | 23.40 | 31.40x | 737.36 | 23.70 | 31.12x |
| pxform_batch | J2000->ITRF93 | 1000 | 4549.71 | 242.70 | 18.75x | 4637.97 | 255.64 | 18.14x |
| sxform_batch | J2000->ITRF93 | 1000 | 4721.80 | 233.72 | 20.20x | 7348.12 | 257.19 | 28.57x |
| pxform_batch | J2000->ITRF93 | 10000 | 23304.78 | 2441.43 | 9.55x | 24678.19 | 2461.07 | 10.03x |
| sxform_batch | J2000->ITRF93 | 10000 | 25282.93 | 2347.77 | 10.77x | 26808.81 | 2360.43 | 11.36x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.30 | 0.35 | 6.56x | 7.65 | 0.38 | 20.09x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.88 | 0.28 | 17.42x | 23.82 | 0.34 | 70.07x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 741.27 | 23.86 | 31.06x | 795.07 | 34.55 | 23.02x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 763.67 | 22.95 | 33.27x | 768.70 | 34.94 | 22.00x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4797.38 | 238.07 | 20.15x | 5209.11 | 276.23 | 18.86x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4968.21 | 232.11 | 21.40x | 5148.50 | 243.42 | 21.15x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 26210.06 | 2391.73 | 10.96x | 27074.98 | 2401.41 | 11.27x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 28084.71 | 2307.53 | 12.17x | 29108.48 | 2329.00 | 12.50x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 2.12 | 0.31 | 6.85x | 2.27 | 0.32 | 7.08x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 4.10 | 0.41 | 9.97x | 7.82 | 0.44 | 17.74x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 734.37 | 24.23 | 30.31x | 743.47 | 34.77 | 21.39x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 754.49 | 23.29 | 32.39x | 760.15 | 34.70 | 21.90x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4775.76 | 242.49 | 19.69x | 4921.06 | 253.70 | 19.40x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4982.70 | 233.55 | 21.33x | 5077.10 | 244.25 | 20.79x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 25904.13 | 2430.26 | 10.66x | 26974.44 | 2444.31 | 11.04x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 28345.89 | 2344.35 | 12.09x | 30723.38 | 2356.35 | 13.04x |
| bodn2c | SUN | 10000 | 2970.60 | 1351.07 | 2.20x | 3043.10 | 1502.86 | 2.02x |
| bodn2c | EARTH | 10000 | 3097.43 | 1601.89 | 1.93x | 3139.37 | 1630.43 | 1.93x |
| bodn2c | MARS BARYCENTER | 10000 | 3959.80 | 2836.16 | 1.40x | 4008.54 | 2854.86 | 1.40x |
| bodn2c | MOON | 10000 | 3053.57 | 1526.00 | 2.00x | 3263.56 | 1561.06 | 2.09x |
| bodn2c | JWST | 10000 | 3029.08 | 1522.78 | 1.99x | 3066.36 | 1575.40 | 1.95x |
| bodn2c | HST | 10000 | 3102.71 | 1334.95 | 2.32x | 3190.42 | 1372.44 | 2.32x |
