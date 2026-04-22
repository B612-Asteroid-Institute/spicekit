# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `b204dc47fcf53f02b289d167608520e19391e41b`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-04-22 01:15:57 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 1.34 | 0.31 | 4.33x | 1.92 | 0.45 | 4.27x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 142.03 | 17.63 | 8.05x | 160.17 | 17.72 | 9.04x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 1201.95 | 177.41 | 6.77x | 1562.50 | 226.45 | 6.90x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7444.11 | 1784.98 | 4.17x | 9602.73 | 1832.86 | 5.24x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.36 | 0.32 | 4.26x | 1.75 | 0.48 | 3.65x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 79.86 | 18.59 | 4.29x | 93.62 | 18.66 | 5.02x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 803.53 | 184.61 | 4.35x | 1161.64 | 195.07 | 5.96x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 7985.86 | 1866.01 | 4.28x | 8077.45 | 2183.74 | 3.70x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.81 | 0.55 | 6.91x | 7.02 | 0.60 | 11.69x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 788.46 | 50.60 | 15.58x | 1352.48 | 79.01 | 17.12x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2846.74 | 510.62 | 5.58x | 2973.80 | 616.59 | 4.82x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 23362.68 | 5069.01 | 4.61x | 23595.65 | 5172.94 | 4.56x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 2.48 | 0.95 | 2.61x | 9.26 | 1.01 | 9.15x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1113.81 | 66.93 | 16.64x | 1123.72 | 77.58 | 14.49x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3362.10 | 676.21 | 4.97x | 3484.13 | 682.49 | 5.11x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 25010.87 | 6742.09 | 3.71x | 25573.59 | 7152.93 | 3.58x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 1.66 | 0.37 | 4.48x | 1.77 | 0.40 | 4.42x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 167.64 | 31.98 | 5.24x | 205.68 | 40.87 | 5.03x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1679.92 | 317.93 | 5.28x | 1724.39 | 328.36 | 5.25x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 16781.81 | 3204.16 | 5.24x | 17169.67 | 3220.61 | 5.33x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 1.64 | 0.33 | 4.96x | 1.70 | 0.35 | 4.85x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 169.38 | 27.94 | 6.06x | 182.97 | 28.23 | 6.48x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1704.59 | 278.40 | 6.12x | 1813.03 | 288.65 | 6.28x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 17057.20 | 2807.18 | 6.08x | 17257.19 | 2829.26 | 6.10x |
| pxform_batch | ITRF93->J2000 | 1 | 1.77 | 0.40 | 4.42x | 1.98 | 0.45 | 4.39x |
| sxform_batch | ITRF93->J2000 | 1 | 3.80 | 0.59 | 6.42x | 22.58 | 0.99 | 22.76x |
| pxform_batch | ITRF93->J2000 | 100 | 667.88 | 37.32 | 17.90x | 676.70 | 37.67 | 17.96x |
| sxform_batch | ITRF93->J2000 | 100 | 680.03 | 36.30 | 18.73x | 779.88 | 36.43 | 21.41x |
| pxform_batch | ITRF93->J2000 | 1000 | 4230.31 | 367.09 | 11.52x | 4312.31 | 390.54 | 11.04x |
| sxform_batch | ITRF93->J2000 | 1000 | 4308.81 | 355.15 | 12.13x | 4443.57 | 380.78 | 11.67x |
| pxform_batch | ITRF93->J2000 | 10000 | 22320.01 | 3703.88 | 6.03x | 23565.34 | 3773.17 | 6.25x |
| sxform_batch | ITRF93->J2000 | 10000 | 23177.67 | 3588.45 | 6.46x | 23858.06 | 3644.19 | 6.55x |
| pxform_batch | J2000->ITRF93 | 1 | 1.74 | 0.59 | 2.95x | 6.55 | 0.75 | 8.71x |
| sxform_batch | J2000->ITRF93 | 1 | 3.57 | 0.55 | 6.47x | 4.52 | 4.17 | 1.08x |
| pxform_batch | J2000->ITRF93 | 100 | 664.84 | 37.64 | 17.66x | 677.92 | 48.85 | 13.88x |
| sxform_batch | J2000->ITRF93 | 100 | 686.77 | 36.58 | 18.78x | 724.06 | 36.63 | 19.77x |
| pxform_batch | J2000->ITRF93 | 1000 | 4187.04 | 370.01 | 11.32x | 4399.76 | 422.94 | 10.40x |
| sxform_batch | J2000->ITRF93 | 1000 | 4412.06 | 358.76 | 12.30x | 4692.99 | 390.05 | 12.03x |
| pxform_batch | J2000->ITRF93 | 10000 | 21926.58 | 3716.74 | 5.90x | 23004.76 | 3782.73 | 6.08x |
| sxform_batch | J2000->ITRF93 | 10000 | 23711.65 | 4468.94 | 5.31x | 25561.71 | 4938.63 | 5.18x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.08 | 0.40 | 5.20x | 2.36 | 0.43 | 5.48x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.54 | 0.59 | 7.68x | 5.09 | 0.62 | 8.20x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 998.24 | 55.70 | 17.92x | 1064.11 | 75.33 | 14.13x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 728.49 | 36.26 | 20.09x | 1255.46 | 53.53 | 23.45x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4439.80 | 366.68 | 12.11x | 5198.82 | 377.09 | 13.79x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4658.99 | 354.98 | 13.12x | 4769.37 | 384.30 | 12.41x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 24299.95 | 3716.65 | 6.54x | 33240.62 | 3753.95 | 8.85x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 26137.61 | 3599.30 | 7.26x | 27210.77 | 3708.85 | 7.34x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 1.99 | 0.41 | 4.85x | 13.95 | 0.49 | 28.46x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 3.73 | 0.51 | 7.29x | 7.20 | 0.61 | 11.77x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 684.93 | 37.65 | 18.19x | 688.61 | 48.24 | 14.27x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 706.95 | 36.71 | 19.26x | 711.79 | 52.49 | 13.56x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4449.38 | 368.75 | 12.07x | 4683.39 | 381.83 | 12.27x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4643.30 | 359.16 | 12.93x | 5217.60 | 371.97 | 14.03x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 24337.36 | 3719.17 | 6.54x | 25344.29 | 3751.80 | 6.76x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 26317.89 | 3615.60 | 7.28x | 27119.61 | 3647.20 | 7.44x |
| bodn2c | SUN | 10000 | 2812.28 | 1250.26 | 2.25x | 3034.03 | 1369.86 | 2.21x |
| bodn2c | EARTH | 10000 | 2883.89 | 1504.77 | 1.92x | 2892.60 | 1523.56 | 1.90x |
| bodn2c | MARS BARYCENTER | 10000 | 3788.00 | 2684.09 | 1.41x | 3804.74 | 2752.84 | 1.38x |
| bodn2c | MOON | 10000 | 2867.84 | 1445.33 | 1.98x | 3047.60 | 1513.37 | 2.01x |
| bodn2c | JWST | 10000 | 2824.83 | 1446.97 | 1.95x | 2871.88 | 1459.71 | 1.97x |
| bodn2c | HST | 10000 | 2884.49 | 1254.25 | 2.30x | 3071.01 | 1290.98 | 2.38x |
