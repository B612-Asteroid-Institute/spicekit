# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `55d5530224dd108a8729199a08c5e9c25f7adea3`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-05-02 12:45:08 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 0.74 | 0.14 | 5.29x | 1.20 | 0.17 | 7.07x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 76.79 | 6.37 | 12.05x | 92.64 | 6.43 | 14.40x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 774.03 | 62.88 | 12.31x | 801.37 | 71.89 | 11.15x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 7701.07 | 639.10 | 12.05x | 10839.81 | 652.15 | 16.62x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.66 | 0.17 | 9.78x | 1.87 | 0.34 | 5.49x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 82.86 | 7.12 | 11.63x | 112.60 | 7.19 | 15.65x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 831.28 | 70.46 | 11.80x | 842.98 | 80.09 | 10.53x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 8273.36 | 714.51 | 11.58x | 8946.85 | 719.04 | 12.44x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 4.21 | 0.34 | 12.37x | 4.44 | 0.38 | 11.68x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 843.81 | 28.48 | 29.63x | 878.57 | 39.60 | 22.18x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 3040.99 | 281.22 | 10.81x | 3081.57 | 290.27 | 10.62x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 24716.69 | 2832.15 | 8.73x | 24947.38 | 2845.43 | 8.77x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 2.67 | 0.52 | 5.12x | 6.34 | 0.60 | 10.55x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1186.02 | 36.36 | 32.62x | 1335.85 | 37.20 | 35.91x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3578.25 | 357.12 | 10.02x | 3626.19 | 367.70 | 9.86x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 26746.75 | 3607.32 | 7.41x | 27222.58 | 3650.38 | 7.46x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 3.32 | 0.31 | 10.70x | 7.26 | 0.36 | 20.18x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 179.24 | 18.29 | 9.80x | 192.93 | 18.36 | 10.51x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1792.37 | 182.30 | 9.83x | 1796.43 | 191.25 | 9.39x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 17908.57 | 1844.14 | 9.71x | 18373.72 | 1859.09 | 9.88x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 3.82 | 0.28 | 13.59x | 4.50 | 0.43 | 10.44x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 181.48 | 16.35 | 11.10x | 218.44 | 25.08 | 8.71x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1826.25 | 162.87 | 11.21x | 1872.85 | 172.17 | 10.88x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 18260.96 | 1646.97 | 11.09x | 18518.42 | 1669.03 | 11.10x |
| pxform_batch | ITRF93->J2000 | 1 | 3.85 | 0.29 | 13.22x | 6.84 | 0.58 | 11.78x |
| sxform_batch | ITRF93->J2000 | 1 | 4.00 | 0.27 | 14.75x | 4.52 | 0.31 | 14.57x |
| pxform_batch | ITRF93->J2000 | 100 | 713.96 | 23.91 | 29.87x | 717.66 | 34.87 | 20.58x |
| sxform_batch | ITRF93->J2000 | 100 | 726.90 | 23.12 | 31.44x | 807.70 | 23.19 | 34.82x |
| pxform_batch | ITRF93->J2000 | 1000 | 4553.12 | 238.47 | 19.09x | 4718.48 | 250.10 | 18.87x |
| sxform_batch | ITRF93->J2000 | 1000 | 4630.75 | 229.78 | 20.15x | 4715.39 | 244.51 | 19.29x |
| pxform_batch | ITRF93->J2000 | 10000 | 23569.40 | 2400.18 | 9.82x | 24569.12 | 2515.49 | 9.77x |
| sxform_batch | ITRF93->J2000 | 10000 | 24621.72 | 2318.61 | 10.62x | 25513.53 | 2701.85 | 9.44x |
| pxform_batch | J2000->ITRF93 | 1 | 3.70 | 0.39 | 9.46x | 6.97 | 0.60 | 11.60x |
| sxform_batch | J2000->ITRF93 | 1 | 3.72 | 0.42 | 8.85x | 8.69 | 0.44 | 19.70x |
| pxform_batch | J2000->ITRF93 | 100 | 711.55 | 24.34 | 29.23x | 714.66 | 34.80 | 20.53x |
| sxform_batch | J2000->ITRF93 | 100 | 734.25 | 23.41 | 31.36x | 757.66 | 34.17 | 22.17x |
| pxform_batch | J2000->ITRF93 | 1000 | 4524.28 | 243.43 | 18.59x | 4765.61 | 269.04 | 17.71x |
| sxform_batch | J2000->ITRF93 | 1000 | 4738.02 | 234.13 | 20.24x | 4872.17 | 250.22 | 19.47x |
| pxform_batch | J2000->ITRF93 | 10000 | 23162.69 | 2441.68 | 9.49x | 24462.37 | 2461.73 | 9.94x |
| sxform_batch | J2000->ITRF93 | 10000 | 25420.91 | 2350.42 | 10.82x | 26840.62 | 2364.51 | 11.35x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.22 | 0.30 | 7.39x | 2.37 | 0.32 | 7.42x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.56 | 0.34 | 13.41x | 8.31 | 16.51 | 0.50x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 736.83 | 23.86 | 30.87x | 753.80 | 24.37 | 30.94x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 762.67 | 23.01 | 33.14x | 769.67 | 34.14 | 22.54x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4764.69 | 238.34 | 19.99x | 4857.90 | 254.63 | 19.08x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4968.51 | 229.65 | 21.64x | 5072.66 | 246.67 | 20.56x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 25994.68 | 2391.96 | 10.87x | 26804.54 | 2421.01 | 11.07x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 28137.70 | 2303.78 | 12.21x | 29170.92 | 2320.24 | 12.57x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 2.10 | 0.39 | 5.38x | 7.83 | 0.58 | 13.47x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 4.53 | 0.38 | 11.89x | 5.00 | 0.39 | 12.79x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 731.15 | 24.20 | 30.22x | 813.93 | 35.19 | 23.13x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 755.39 | 23.29 | 32.43x | 778.84 | 23.38 | 33.31x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4758.60 | 242.10 | 19.66x | 4877.06 | 255.86 | 19.06x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 5075.10 | 233.35 | 21.75x | 5171.82 | 245.46 | 21.07x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 25762.78 | 2432.50 | 10.59x | 26857.87 | 2443.43 | 10.99x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 28454.06 | 2345.74 | 12.13x | 35401.82 | 2352.49 | 15.05x |
| bodn2c | SUN | 10000 | 2935.39 | 1307.49 | 2.25x | 2991.78 | 1434.32 | 2.09x |
| bodn2c | EARTH | 10000 | 3065.81 | 1579.06 | 1.94x | 3128.45 | 1673.08 | 1.87x |
| bodn2c | MARS BARYCENTER | 10000 | 3925.57 | 2797.40 | 1.40x | 3962.30 | 2836.07 | 1.40x |
| bodn2c | MOON | 10000 | 3016.22 | 1497.95 | 2.01x | 3090.42 | 1513.18 | 2.04x |
| bodn2c | JWST | 10000 | 2995.56 | 1489.59 | 2.01x | 3010.45 | 1538.35 | 1.96x |
| bodn2c | HST | 10000 | 3079.31 | 1317.11 | 2.34x | 3142.85 | 1336.85 | 2.35x |
