# PREP-BND-1 qualification evidence

This directory contains compact, nonrestricted receipts for the PREP-BND-1
implementation and MATLAB comparison completed on 2026-08-19. Row-level CZ18
data, synthetic row inputs, MATLAB detail rows, compiled MEX objects, and the
licensed maintained MATLAB source are deliberately excluded.

Source bindings:

- measurement baseline: `72179fb3627ac9699fb892ca721105930d560fc1`;
- PREP-MAP-1 candidate: `a83f902e71a77b205bb63976ce6df584f01480fc`;
- cumulative runtime candidate: `f06e29e3a5bbb27cfddf3ac48e9596d60b95dbfa`;
- benchmark tool commit: `b4c784b3de59cbb4642d8eddc55e92dd84fc53aa`;
- maintained-MATLAB matrix source commit: `5718ab59d5618ad2b1bd502fe04a5e199484147e`;
- fixed-CZ18 MATLAB source commit: `fe50a90f9ec18438b613a0d06e5d7b178ebf45db`.

Contents:

- `local/summary.json`: four-process archive-isolated AB/BA local result;
- `stata/map_only_summary.json`: ten-job PREP-MAP-only SCC analysis;
- `stata/cumulative_summary.json`: ten-job cumulative PREP-MAP/PREP-SEM SCC
  analysis, including the fixed CZ18 AB/BA holdout; and
- `stata/cumulative_jobs.tsv`: the accepted job-to-result/scheduler-log ledger
  used to construct that analysis (SHA-256
  `65f5a17e607e0ab0a9cfac09fbb4a1bb93ae5c838478afcfe88f0285b849f484`);
- `matlab/dense_oracle/`: exact clean-room MATLAB/Stata numerical receipts;
- `matlab/matrix/`: 30-job same-host, source-order-reversed maintained-MATLAB
  matrix manifest, aggregate summaries, and per-job validation records; and
- `matlab/cz18/`: three fresh fixed-CZ18 MATLAB validations plus the derived
  phase, timing, memory, and descriptive numerical summary.

Every SCC summary or per-job validation requires clean qacct, exact source and
input identities, application and wrapper terminal markers, resource-policy
checks, and the registered scientific/state contract. Maintained MATLAB is a
descriptive JLA comparator; `corrected_estimate_equality_gate` is intentionally
`NONE_DESCRIPTIVE_ONLY`. The independent dense oracle supplies the hard
plug-in/correction/corrected-target equality gate.

The complete SCC evidence remains under:

- `/projectnb/welfgr/varcomp-kss/prep-bnd1/runs/20260819T2048Z-map1`;
- `/projectnb/welfgr/varcomp-kss/prep-bnd1/runs/20260819T2118Z-final`;
- `/projectnb/welfgr/varcomp-kss/prep-bnd1-matlab/runs/20260819T2129Z-final-inputfix`;
- `/projectnb/welfgr/varcomp-kss/runs/20260819T220956Z-prep-bnd1-cz18-matlab`.

At the 18:46 collection cutoff, original F8192 AB job 7236971 was incomplete.
Replacement job 7237620 was submitted before 7236971 completed and is the
accepted AB source in the frozen ledger. Job 7236971 later completed, but its
result is superseded and excluded from the cumulative summary.
