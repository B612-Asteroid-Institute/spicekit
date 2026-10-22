# Benchmark results

spicekit vs CSpice, side-by-side on matched inputs.

- Commit: `392ab32a873c64eee782d88d34b31e938caeeed2`
- Runner: `Linux` (`x86_64`)
- Generated: 2026-10-22 16:03:20 UTC
- Source: `cargo run --release --bin spicekit-bench`

Speedup = cspice / spicekit (higher is better for spicekit).

| op | case | n | cspice p50 (µs) | spicekit p50 (µs) | speedup p50 | cspice p95 (µs) | spicekit p95 (µs) | speedup p95 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| spkez_batch | sun_wrt_ssb_j2000 | 1 | 1.22 | 0.16 | 7.64x | 1.87 | 0.22 | 8.52x |
| spkez_batch | sun_wrt_ssb_j2000 | 100 | 70.13 | 5.59 | 12.55x | 109.56 | 5.71 | 19.19x |
| spkez_batch | sun_wrt_ssb_j2000 | 1000 | 706.04 | 54.98 | 12.84x | 761.82 | 64.72 | 11.77x |
| spkez_batch | sun_wrt_ssb_j2000 | 10000 | 6943.89 | 560.64 | 12.39x | 7020.16 | 574.77 | 12.21x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1 | 1.40 | 0.16 | 8.76x | 1.58 | 0.18 | 8.79x |
| spkez_batch | sun_wrt_ssb_ecliptic | 100 | 76.38 | 6.28 | 12.16x | 141.06 | 6.33 | 22.28x |
| spkez_batch | sun_wrt_ssb_ecliptic | 1000 | 767.36 | 60.39 | 12.71x | 810.97 | 68.76 | 11.79x |
| spkez_batch | sun_wrt_ssb_ecliptic | 10000 | 7625.49 | 615.84 | 12.38x | 7978.91 | 631.96 | 12.63x |
| spkez_batch | earth_wrt_sun_ecliptic | 1 | 3.82 | 0.29 | 13.12x | 7.13 | 0.36 | 19.76x |
| spkez_batch | earth_wrt_sun_ecliptic | 100 | 830.33 | 23.75 | 34.96x | 836.74 | 33.50 | 24.98x |
| spkez_batch | earth_wrt_sun_ecliptic | 1000 | 2865.32 | 234.69 | 12.21x | 5502.25 | 243.59 | 22.59x |
| spkez_batch | earth_wrt_sun_ecliptic | 10000 | 23016.70 | 2373.64 | 9.70x | 23211.28 | 2406.38 | 9.65x |
| spkez_batch | moon_wrt_earth_j2000 | 1 | 4.65 | 0.44 | 10.54x | 5.93 | 0.56 | 10.55x |
| spkez_batch | moon_wrt_earth_j2000 | 100 | 1181.75 | 30.15 | 39.20x | 1195.80 | 40.34 | 29.65x |
| spkez_batch | moon_wrt_earth_j2000 | 1000 | 3424.86 | 296.66 | 11.54x | 3454.05 | 306.43 | 11.27x |
| spkez_batch | moon_wrt_earth_j2000 | 10000 | 25103.80 | 2996.54 | 8.38x | 25339.60 | 3027.90 | 8.37x |
| spkez_batch | moon_wrt_earth_itrf93 | 1 | 6.96 | 0.98 | 7.10x | 9.06 | 1.04 | 8.69x |
| spkez_batch | moon_wrt_earth_itrf93 | 100 | 1913.12 | 53.74 | 35.60x | 1986.65 | 66.14 | 30.04x |
| spkez_batch | moon_wrt_earth_itrf93 | 1000 | 7679.25 | 532.83 | 14.41x | 7882.73 | 554.89 | 14.21x |
| spkez_batch | moon_wrt_earth_itrf93 | 10000 | 45669.81 | 5272.51 | 8.66x | 62574.68 | 6008.75 | 10.41x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1 | 2.36 | 0.28 | 8.44x | 3.48 | 0.30 | 11.55x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 100 | 148.29 | 15.57 | 9.52x | 170.74 | 15.66 | 10.90x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 1000 | 1478.48 | 153.97 | 9.60x | 2279.30 | 174.15 | 13.09x |
| spkez_batch | mars_bc_wrt_sun_j2000 | 10000 | 14740.40 | 1559.88 | 9.45x | 15165.61 | 1629.03 | 9.31x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1 | 1.48 | 0.21 | 7.06x | 1.56 | 0.24 | 6.49x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 100 | 138.36 | 14.21 | 9.74x | 152.67 | 22.69 | 6.73x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 1000 | 1383.16 | 142.84 | 9.68x | 1589.53 | 150.71 | 10.55x |
| spkez_batch | saturn_bc_wrt_sun_j2000 | 10000 | 13775.67 | 1456.39 | 9.46x | 21827.01 | 1936.58 | 11.27x |
| pxform_batch | ITRF93->J2000 | 1 | 1.74 | 0.41 | 4.24x | 12.55 | 0.63 | 19.86x |
| sxform_batch | ITRF93->J2000 | 1 | 3.48 | 0.37 | 9.37x | 3.79 | 0.57 | 6.63x |
| pxform_batch | ITRF93->J2000 | 100 | 696.18 | 21.84 | 31.87x | 733.83 | 32.57 | 22.53x |
| sxform_batch | ITRF93->J2000 | 100 | 712.79 | 20.92 | 34.07x | 738.76 | 34.01 | 21.72x |
| pxform_batch | ITRF93->J2000 | 1000 | 4447.95 | 221.05 | 20.12x | 7890.58 | 240.64 | 32.79x |
| sxform_batch | ITRF93->J2000 | 1000 | 4546.78 | 215.14 | 21.13x | 8297.38 | 232.64 | 35.67x |
| pxform_batch | ITRF93->J2000 | 10000 | 22187.99 | 2198.31 | 10.09x | 23304.26 | 2237.26 | 10.42x |
| sxform_batch | ITRF93->J2000 | 10000 | 22900.22 | 2102.40 | 10.89x | 24464.35 | 2155.00 | 11.35x |
| pxform_batch | J2000->ITRF93 | 1 | 1.92 | 0.28 | 6.87x | 2.10 | 0.32 | 6.58x |
| sxform_batch | J2000->ITRF93 | 1 | 3.68 | 0.42 | 8.75x | 6.08 | 0.45 | 13.51x |
| pxform_batch | J2000->ITRF93 | 100 | 700.54 | 22.33 | 31.37x | 707.76 | 22.95 | 30.84x |
| sxform_batch | J2000->ITRF93 | 100 | 721.31 | 21.36 | 33.77x | 726.59 | 32.07 | 22.66x |
| pxform_batch | J2000->ITRF93 | 1000 | 4398.61 | 223.66 | 19.67x | 4528.76 | 233.53 | 19.39x |
| sxform_batch | J2000->ITRF93 | 1000 | 4588.79 | 217.09 | 21.14x | 4681.84 | 226.75 | 20.65x |
| pxform_batch | J2000->ITRF93 | 10000 | 21999.59 | 2242.81 | 9.81x | 23129.70 | 2265.53 | 10.21x |
| sxform_batch | J2000->ITRF93 | 10000 | 24008.50 | 2144.28 | 11.20x | 25146.57 | 2170.99 | 11.58x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1 | 2.10 | 0.39 | 5.39x | 6.89 | 0.64 | 10.75x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1 | 4.50 | 0.36 | 12.46x | 7.45 | 0.39 | 19.06x |
| pxform_batch | ITRF93->ECLIPJ2000 | 100 | 713.73 | 21.82 | 32.71x | 731.29 | 23.60 | 30.98x |
| sxform_batch | ITRF93->ECLIPJ2000 | 100 | 741.17 | 20.85 | 35.55x | 869.32 | 21.68 | 40.10x |
| pxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4607.47 | 218.05 | 21.13x | 4650.17 | 232.19 | 20.03x |
| sxform_batch | ITRF93->ECLIPJ2000 | 1000 | 4815.37 | 209.62 | 22.97x | 4896.09 | 237.12 | 20.65x |
| pxform_batch | ITRF93->ECLIPJ2000 | 10000 | 24060.21 | 2205.74 | 10.91x | 24819.43 | 2285.51 | 10.86x |
| sxform_batch | ITRF93->ECLIPJ2000 | 10000 | 26238.39 | 2104.86 | 12.47x | 27414.09 | 2123.51 | 12.91x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1 | 4.02 | 0.36 | 11.13x | 7.25 | 0.40 | 18.09x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1 | 5.00 | 0.26 | 19.16x | 5.46 | 0.31 | 17.56x |
| pxform_batch | ECLIPJ2000->ITRF93 | 100 | 735.96 | 22.37 | 32.90x | 855.49 | 34.94 | 24.49x |
| sxform_batch | ECLIPJ2000->ITRF93 | 100 | 1266.92 | 33.03 | 38.35x | 2048.78 | 38.07 | 53.81x |
| pxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4641.40 | 224.38 | 20.69x | 5692.53 | 237.87 | 23.93x |
| sxform_batch | ECLIPJ2000->ITRF93 | 1000 | 4828.37 | 213.68 | 22.60x | 4893.33 | 226.40 | 21.61x |
| pxform_batch | ECLIPJ2000->ITRF93 | 10000 | 24362.35 | 2253.55 | 10.81x | 25204.66 | 2316.28 | 10.88x |
| sxform_batch | ECLIPJ2000->ITRF93 | 10000 | 26800.78 | 2149.01 | 12.47x | 27768.92 | 2186.31 | 12.70x |
| pool_sxform_batch | eop_1_file | 1 | 2.05 | 0.37 | 5.54x | 4.53 | 0.42 | 10.76x |
| pool_sxform_batch | eop_1_file | 100 | 220.39 | 30.01 | 7.34x | 254.69 | 41.46 | 6.14x |
| pool_sxform_batch | eop_1_file | 1000 | 2217.75 | 289.84 | 7.65x | 2299.66 | 313.13 | 7.34x |
| pool_sxform_batch | eop_1_file | 10000 | 22063.26 | 2928.88 | 7.53x | 22456.56 | 2955.03 | 7.60x |
| pool_sxform_batch | eop_2_files | 1 | 2.11 | 0.35 | 6.02x | 2.35 | 0.39 | 6.04x |
| pool_sxform_batch | eop_2_files | 100 | 223.74 | 29.18 | 7.67x | 241.51 | 39.19 | 6.16x |
| pool_sxform_batch | eop_2_files | 1000 | 2243.82 | 295.75 | 7.59x | 2306.73 | 315.92 | 7.30x |
| pool_sxform_batch | eop_2_files | 10000 | 22258.50 | 2994.95 | 7.43x | 22437.82 | 3027.36 | 7.41x |
| pool_sxform_batch | eop_3_files | 1 | 2.05 | 0.35 | 5.85x | 2.29 | 0.39 | 5.84x |
| pool_sxform_batch | eop_3_files | 100 | 214.55 | 29.12 | 7.37x | 228.62 | 38.74 | 5.90x |
| pool_sxform_batch | eop_3_files | 1000 | 2135.47 | 296.54 | 7.20x | 2267.93 | 316.65 | 7.16x |
| pool_sxform_batch | eop_3_files | 10000 | 21239.37 | 2989.09 | 7.11x | 21596.51 | 3029.25 | 7.13x |
| pool_sxform_batch | eop_3_files_straddle | 1 | 3.35 | 0.50 | 6.68x | 4.54 | 0.60 | 7.55x |
| pool_sxform_batch | eop_3_files_straddle | 100 | 835.62 | 30.15 | 27.72x | 1007.84 | 42.33 | 23.81x |
| pool_sxform_batch | eop_3_files_straddle | 1000 | 7501.62 | 326.10 | 23.00x | 7604.78 | 333.45 | 22.81x |
| pool_sxform_batch | eop_3_files_straddle | 10000 | 50559.76 | 3149.36 | 16.05x | 51206.63 | 3608.34 | 14.19x |
| pool_spkez_batch | de440_x1 | 1 | 1.33 | 0.34 | 3.91x | 1.51 | 0.45 | 3.35x |
| pool_spkez_batch | de440_x1 | 100 | 669.76 | 28.98 | 23.11x | 691.90 | 39.14 | 17.68x |
| pool_spkez_batch | de440_x1 | 1000 | 2178.57 | 288.03 | 7.56x | 2222.90 | 297.27 | 7.48x |
| pool_spkez_batch | de440_x1 | 10000 | 17104.70 | 2900.05 | 5.90x | 17421.47 | 2934.93 | 5.94x |
| pool_spkez_batch | de440_x2 | 1 | 1.34 | 0.37 | 3.63x | 1.49 | 0.44 | 3.38x |
| pool_spkez_batch | de440_x2 | 100 | 661.74 | 29.05 | 22.78x | 683.62 | 29.77 | 22.96x |
| pool_spkez_batch | de440_x2 | 1000 | 2137.59 | 288.36 | 7.41x | 2201.23 | 298.23 | 7.38x |
| pool_spkez_batch | de440_x2 | 10000 | 16682.09 | 2898.45 | 5.76x | 16981.81 | 2906.53 | 5.84x |
| pool_spkez_batch | de440_x3 | 1 | 1.34 | 0.34 | 3.95x | 2.03 | 0.36 | 5.63x |
| pool_spkez_batch | de440_x3 | 100 | 654.97 | 29.06 | 22.54x | 680.46 | 40.09 | 16.98x |
| pool_spkez_batch | de440_x3 | 1000 | 2064.39 | 288.78 | 7.15x | 2129.16 | 297.75 | 7.15x |
| pool_spkez_batch | de440_x3 | 10000 | 16044.07 | 2896.99 | 5.54x | 16387.21 | 2921.46 | 5.61x |
| bodn2c | SUN | 10000 | 2744.79 | 1304.00 | 2.10x | 2925.94 | 1433.98 | 2.04x |
| bodn2c | EARTH | 10000 | 2839.06 | 1581.81 | 1.79x | 2915.28 | 1626.73 | 1.79x |
| bodn2c | MARS BARYCENTER | 10000 | 3855.90 | 2597.92 | 1.48x | 4261.96 | 2665.80 | 1.60x |
| bodn2c | MOON | 10000 | 2848.32 | 1433.82 | 1.99x | 2888.16 | 1462.39 | 1.97x |
| bodn2c | JWST | 10000 | 2781.34 | 1468.53 | 1.89x | 2861.35 | 1505.42 | 1.90x |
| bodn2c | HST | 10000 | 2800.89 | 1246.36 | 2.25x | 2875.18 | 1279.46 | 2.25x |
| lsk_dtpool | DELTET/* | 50000 | 11982.82 | 15.45 | 775.64x | 12115.95 | 17.14 | 706.80x |
| lsk_gdpool | DELTET/* | 50000 | 25353.86 | 2246.73 | 11.28x | 25535.77 | 2260.40 | 11.30x |
| cnmfrm | EARTH | 10000 | 8538.83 | 1829.92 | 4.67x | 8901.42 | 1938.78 | 4.59x |
| cidfrm | EARTH(399) | 10000 | 13287.49 | 2038.57 | 6.52x | 13484.13 | 2115.63 | 6.37x |
| namfrm | ITRF93 | 10000 | 2658.18 | 225.08 | 11.81x | 2730.88 | 233.85 | 11.68x |
