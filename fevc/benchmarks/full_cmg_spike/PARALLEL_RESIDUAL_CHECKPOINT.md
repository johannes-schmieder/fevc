# Parallel residual-certification checkpoint

VCkss source `e6f6b89856d0f5edcdd5a579dfe6c5a01a57dc32` runs firm
extraction, worker recovery, reduced-residual reconstruction, and independent
complete original-system residual certification across bounded ordered RHS
chunks on the existing isolated CMG pool. The caller thread polls UserBreak
between chunks; Rayon workers call no Stata APIs. Per-column arithmetic,
failure order, solution order, and receipt order are unchanged.

The clean source-bound 4-thread, 8,192-firm, 200-probe macOS run passed in
90.102 seconds. This is 14.227 seconds (13.6%) faster than the packed-counter
checkpoint and 47.5% faster than the registered maintained-MATLAB comparator.
VCkss is now 1.91 times as fast as MATLAB on this development run, but remains
4.236 seconds above the private 2x promotion gate.

| Source-bound headline run | Packed counter | Parallel residual | New / prior |
| --- | ---: | ---: | ---: |
| Complete command seconds | 104.329 | 90.102 | 0.864 |
| Native solve seconds | 93.564 | 79.481 | 0.849 |
| Extraction/recovery/residual seconds | 17.128 | 4.238 | 0.247 |
| Standalone CMG solve seconds | 57.837 | 56.957 | 0.985 |
| Maximum complete residual | `6.97e-6` | `6.97e-6` | identical |
| Process peak-footprint bytes | 5,227,599,152 | 5,230,400,720 | 1.001 |

All four corrected targets are bit-identical to the prior source-bound run.
Target identity, accounting, `e(sample)`, data, caller RNG, and sort restoration
pass. The optimization changes no tolerance or statistical acceptance rule.

The retained phase receipt identifies about 8.7 seconds in target direction/RHS
construction and leverage/target accumulation. Fusing per-column target
construction on the same ordered pool is the shortest remaining path to the
4.236-second 2x gap; no simplified-hierarchy tuning is implicated.

Exact identities, commands, timings, hashes, gates, and temporary evidence
paths are in
[`parallel_residual_2026-08-25.json`](parallel_residual_2026-08-25.json). This
is one development run, not the alternating five-run qualification.
