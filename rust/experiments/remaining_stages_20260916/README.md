# Remaining campaign stages — September 16

Owner authorization extends the passing first supported-path screen through
400k, 1.6m and private Veneto. This is a harness-only extension; no new native
source, binary, installed package, paper edit, branch, commit or push.

The exact qualified `b14496bc` candidate and accepted September 15 baseline
packages are reused from SCC `20260916T005700Z-pipeline-screen-b14496bc`.
First-stage report SHA256 is
`16150baa2acbf81e494ccaad5967268cf304a2948910a44c2035ab274873c92a`.
The source-verified adapter and native binaries are unchanged. The historical
baseline execution behavior is preserved. The public point request is exactly
the accepted paper request (implicit worker–firm matches, both populations,
JLA/200 probes, omitted tolerance and memory, Counter-V1 and registered seed).
Inputs are reused by hash; consumers never generate or modify them.

## Inventory and execution

| Stage | Cells | Measured | Warm-ups | Native threads |
|---|---:|---:|---:|---|
| 400k, all five classes × both deletions | 20 | 150 | 50 | 4, 28 |
| 1.6m, three selected classes × both deletions | 6 | 90 | 18 | 28 |
| Private Veneto, both deletions | 2 | 12 | 4 | 4 |

Baseline/candidate run in all cells; pinned KSS Matlab additionally runs in
28-thread synthetic cells. No new Matlab Veneto arm. Each cell's warm-ups and
rotated measured calls run sequentially on one host. Stata remains at min(4,T),
with native T checked against the allocation. Each substantive task reserves
28 × 3G/45 minutes, without queue/host restriction. FEVC physical-RSS guards
are 20 GiB at 400k and 45 GiB for large/private; Matlab's is 72 GiB. The task
deadline leaves 30 seconds for receipt cleanup. Pool startup is capped at 240
seconds. Resource overrun stops, with no automatic increase.

The six-cell small workflow reserves 4 × 3G/25 minutes and a 10 GiB guard.
It covers both deletions on degree-four bottleneck and mixed well-mixed with
all three variants, plus both FEVC variants on a public pooled-stayer fixture
in the private CSV schema. It deliberately fails both Stata and Matlab.
The local generator-to-validator check exercises eight FEVC calls and failure
propagation. Stata/native qualification is carried forward only because every
estimator/package byte is unchanged; this harness still requires the SCC smoke.
Unit regressions reject malformed inventories, missing/stale/duplicate/partial
outputs, numerical/nonfinite changes, resource and timer inconsistencies.

Primary time includes package/pool initialization after common data import;
native estimator-boundary and whole-process times are separate. The complete
full-precision export preserves residuals, iterations/refinement, phase timings,
thread/batch/allocation receipts and target MCSE. Physical RSS uses tracked
process identities, including reparented MATLAB children. Scheduler virtual
memory is distinct. Every attempted, failed and unattempted call is enumerated.

The first-screen corrected-target gate remains unchanged. Stage performance
requires at least 3% geometric complete-command improvement and no cell median
regression above 5%. Matlab timings and normalized corrected targets are
reported separately, with independent-RNG and finite-projection-formula
caveats; no exact finite-probe equality or paper claim is asserted.

## Guarded launch

`deploy.py` verifies the local workflow, creates a fresh run namespace, copies
only this harness plus the public small fixture, and verifies transferred hashes.
`remaining.py freeze` binds all packages, support modules, comparator/MEX bytes,
input metadata, options, seeds, resources and expected outputs before any calls.
`submit-smoke` submits the real smoke and guarded accounting/validation job.
Only a passing smoke can open `submit`, which submits all three arrays and
their aggregate jobs together as SGE dependencies. Consumers require both
the previous report's PASS and clean aggregate accounting, not merely scheduler
completion. Scientific/resource failure blocks advancement. Three additionally
authorized narrow operational repairs are available; attempts are never replaced.

Private raw data stays in its existing owner-only SCC area; new private logs,
TSV outputs and detailed report use a separate owner-only private run directory.
Public manifests contain paths/hashes and aggregate row counts, not private rows.

## Narrow operational repair 1

Smoke `7589873` completed all sixteen normal calls successfully, but its final
deliberate Matlab error attempt reused the successful call's scratch path and
failed in Python before launching Matlab. The original failed scheduler and
aggregate receipts remain unchanged. Scratch paths now additionally identify
the output directory; a focused regression reproduces the former collision.
The focused SCC retest runs one baseline/candidate/Matlab cell plus both real
deliberate failures. Its clean accounting is required. An explicit compatibility
receipt revalidates all sixteen original normal calls, all original source,
input and output hashes, and the specific prelaunch FileExistsError. It carries
only numerical/thread/input-format evidence, never a scheduler-PASS claim.
No command, native binary, input, RNG, numerical gate or resource limit changes.
The remaining stage inventories are unchanged. Two further repairs remain.
