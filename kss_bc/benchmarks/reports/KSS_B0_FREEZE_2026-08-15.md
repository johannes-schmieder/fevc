# KSS numerical-optimization B0 freeze — 2026-08-15

## Frozen implementation

B0 is the scalar-RHS matrix-solve implementation at commit
`9f06a2f2ea2dba449289f35012a88067ec8447f7`.  This record does not alter or
reinterpret any B0 run.  Its source hashes are:

- `kss_bc/kss_bc.mata`:
  `60b8ae8d9b550ddb9fe34751070a0923acc288e7fcdeacb7a8ca464353b2ff32`;
- `kss_bc/kss_bc.ado`:
  `27aaebcead8f528a94efdd2ee4f1c59f87e1ef54486f193602b5202b7d8402d8`;
- `kss_bc/benchmarks/synthetic_benchmark.do`:
  `9d56cb3b22013d3ce082ef5bd443b22cb62a3c22248ddefe69238109abd36d5a`.

The prior handover is preserved byte-for-byte at
`kss_bc/docs/HANDOVER_BATCH_CMG_2026-08-15.md`, SHA-256
`d3281ac00ea9b53547410f1ad7260d1419a543e9b646226c397353c99ddd72f8`.
It is the detailed source, scheduler, application, and failure ledger for B0.

## Accepted evidence before the live large run

Run `20260814T183348Z-f4182b1` records accepted Stata 19 portability, paired
oracle, smoke, and medium jobs.  The medium job used 50,000 workers, 2,500
firms, 200,000 stored rows, 300,000 physical observations, and 100 probes.  It
completed in 3,870 seconds with 430,956 KB GNU-time peak RSS, 1,250 maximum
solver iterations, and maximum complete residual `5.16314065594e-9`.

The first B0 large job, `7183395`, reached its 12-hour scheduler limit with
`failed=100`, `exit_status=137`, 172,292.480 CPU seconds, and 1.329 GB
`maxvmem`.  It produced no accepted result or success marker.  This failed
attempt remains evidence and is not converted to success.

## Frozen live run

SCC job `7185180` is an unchanged rerun of the B0 large scientific design:

- run: `20260815T081233Z-9f06a2f`;
- source commit: `9f06a2f2ea2dba449289f35012a88067ec8447f7`;
- dimensions: 250,000 workers, 10,000 firms, 1,000,000 stored rows,
  1,500,000 physical observations, 200 probes;
- seed/tolerance: `20260814` / `1e-8`;
- request: four slots, 8 GB per slot, 18 hours, `stata-mp/19`;
- remote root:
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f`.

At `2026-08-15T12:14:00Z` it remained running on
`econ@scc-gr4.scc.bu.edu`; scheduler usage was 15:05:32 aggregate CPU,
1.270 GB current virtual memory, and 1.270 GB maximum virtual memory.  No
accepted marker or structured result existed yet.  The job has not been
cancelled, modified, resubmitted, or otherwise altered by the numerical-
optimization work.

Absence from `qstat` is not success.  When it finishes, acceptance still
requires collected `qacct`, the application and wrapper markers, structured
CSV validation, GNU-time peak RSS, the reported Stata version/flavor, and the
recorded source commit.
