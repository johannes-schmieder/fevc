# Accepted SCC comparative-scaling evidence

This directory contains the accepted compact evidence and standalone report
for the two-generation SCC study completed on 29 August 2026.

- Measurement source: `ca162022833c3d6e6906c11cf4860baf0d52fd86`
- Source bundle: `02d6eb6ffc96e0eb7eea1252605663e3e206c6ce177f3cf7f0e860c2b25f40f9`
- Base array: `7354867`, carrying 228 complete tasks
- Replacement preparation/array: `7358809` / `7358890`, contributing 69
  complete tasks and three registered right-censored MATLAB attempts
- Accepted evidence: 297 complete same-host tasks, 891 successful estimator
  calls, 99 rankable graph--size--core cells, and one excluded censored cell

VCkss--Rust is fastest in all 99 rankable cells. The median paired
Rust/MATLAB command-time ratio is 0.207 and the median paired Rust/Mata ratio
is 0.065. Ninety-five of the 99 Rust/MATLAB cell ratios are below 0.5. These
are measured-grid results, not a universal language ranking. Rust/Mata target
cells 8 and 16 compare full-target Rust with four-core-capped Mata.

Historical task IDs 61--63 are the largest `strong_d2` one-core cell.
Maintained MATLAB reached the registered 10,800-second limit in all three
position-balanced repetitions. Their lower bounds are in
`collection/censored_matlab_3.tsv`; the entire cell is excluded from rankings,
ratios, and scaling. Future execution is prohibited by
[`future_exclusions.json`](../../../future_exclusions.json).

The compact machine-readable packet is under `collection/`. The generated
tables, figures, Markdown summary, and visually inspected 16-page PDF are
under `report/`.
