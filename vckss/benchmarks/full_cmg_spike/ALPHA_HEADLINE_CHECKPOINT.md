# Alpha headline performance checkpoint

VCkss source `598a08d5c0792519b3d87d6f56f743cacbf93a24` combines
ordered parallel Counter-V1 generation, direct leverage-RHS construction,
parallel leverage-moment accumulation, independent target preparation, and
parallel recovery and complete-residual certification on the private direct
hybrid-Laplacian full-CMG route. The route retains mathematically independent
PCGs and deterministic per-column ordering. Worker threads call no Stata APIs.

The clean source-bound 4-thread, 8,192-firm, 200-probe macOS run completed in
81.145 seconds. The registered maintained-MATLAB comparator completed in
171.733 seconds, so this development run is 2.12 times as fast and passes the
private end-to-end 2x threshold with 4.722 seconds of margin.

| Source-bound headline run | Parallel residual | Alpha headline | New / prior |
| --- | ---: | ---: | ---: |
| Complete command seconds | 90.102 | 81.145 | 0.901 |
| Native solve seconds | 79.481 | 70.523 | 0.887 |
| Standalone CMG solve seconds | 56.957 | 57.660 | 1.012 |
| Counter-V1 generation seconds | 3.715 | 0.990 | 0.267 |
| Extraction/recovery/residual seconds | 4.238 | 4.310 | 1.017 |
| Maximum complete residual | `6.97e-6` | `6.97e-6` | identical |
| Complete command / MATLAB | 0.525 | 0.473 | — |

The remaining end-to-end cost is dominated by the 57.660-second full-CMG
solve phase. All 600 randomized solve columns converge in eight iterations.
The four corrected targets are bit-identical to the prior source-bound run,
the complete residual remains below the `1e-5` randomized-phase gate, and
target identity, accounting, `e(sample)`, data, caller RNG, and sort
restoration pass. The optimization changes no tolerance or acceptance rule.

This is the first clean single-run crossing of the performance threshold. It
does not promote or qualify the private route: the registered requirement is
one warm-up followed by at least five alternating timed runs, and the fixed
CZ18 hard case and SCC Linux gate remain pending.

Exact identities, commands, phase timings, hashes, numerical results, gates,
and retained temporary evidence paths are in
[`alpha_headline_2026-08-25.json`](alpha_headline_2026-08-25.json).
