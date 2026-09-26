# Original deletion-unit mover eligibility — September 25, 2026

This is a development repair in the owner-authorized isolated worktree above
`7167d23aaa9bdb66c00501d3f2db7758b1c91279`. It is not a tag, binary refresh,
release, downstream integration, or new inference-coverage qualification.
The Separations vendored copy and its safety guard remain unchanged.

## Scientific change and boundary audit

For match deletion, freeze worker eligibility from distinct original deletion
units on the complete-case input. Use supplied `deletionid()` values, or the
ordinary worker–firm pair otherwise. Multiple original blocks at a pooled
model firm are mover blocks; their observations must not become synthetic
stayer observation units. A repeated spell with the same ID remains in its
original block. Only an original one-block worker can enter the existing
eligible-stayer augmentation. Graph-dropped movers remain excluded.

The population helper validates each declared ID within one coefficient cell,
counts histories, and reports original stayer counts. The Mata graph selector
uses deletion-unit support at initial filtering and every fixed-point pass.
It reuses the existing distinct-pair counter, with the deletion ID as the
second index. Worker-articulation, bridge, full-model, nuisance, deleted-rank,
residual, and PSD gates are unchanged. Thus two blocks are necessary, not a
sufficient identification certificate. The graph build token changes so a
cached old selector cannot silently survive a source update.

Default match IDs still give the former firm-count classification. Observation
population selection still counts model firms, and observation/both still uses
physical-observation deletion. The raw native implicit-match shortcut cannot
receive a supplied deletion ID, so its existing default semantics remain valid.

Exact, generic JLA, compressed JLA, and exact projection already receive the
selected deletion IDs and frozen stayer mask; their numerical formulas need no
change. Projection's dense literal-block oracle also checks the covariance's
PSD decision. Both projection fixtures here are withheld as independently
indefinite, not counted as successful inference estimates.

Existing native graph preparation still counts firms, and its native stayer
augmentation can therefore disagree. The public boundary conservatively gates
**every parallel supplied partition**, including multi-firm workers whose support
could change during pruning. Automatic backend/RNG requests fall back to Mata
before native preparation and estimator RNG. Strict Rust or Counter requests
return `RUST_PARALLEL_DELETION_UNSUPPORTED`. This includes structured component
inference and native projection; no native inference extension is claimed.
Direct private `fevc_rust` preparation APIs retain their historical semantics.

This assumes independence across declared blocks, including blocks of one
worker, and a correctly specified pooled-effect model. Pooling coefficient IDs
does not establish block independence. Actual one-block stayers retain the
existing non-match-robust observation correction under `stayers(both)`.

## Focused evidence

`tests/stata/test_pooled_deletion.do` adapts the synthetic Separations failure
reproducer into a success regression. Its original 40-row case failed on the
baseline because worker 99 was excluded. The repaired result retains all 40
rows and 20 original blocks in both populations. Literal deleted-design QR
refits remain full rank and give worker variance `44.902233`, firm variance
`.01665509`, covariance `.36216391`, and total `45.643216` at displayed precision.
The former hybrid had 22 blocks and worker variance `44.902401`.

The independent extended oracle physically expands frequencies, divides each
stored target mass across its copies, constructs all four quadratic targets,
and literally refits every deletion using QR. It does not call production
inverse-action or graph routines. The controlled fixture uses unequal target
and frequency weights, repeated noncontiguous matches, parallel blocks,
one-block stayers, a physical singleton, a smaller component, and a worker
articulation joining it to the main component. It checks retained membership,
20 mover blocks versus 26 mixed physical/block units, and prevents dropped
original movers from returning as stayers. Both nuisance conventions and both
population choices agree for plugin, correction and all four corrected targets
within `1e-8` relative difference (observed relative discrepancies below `1e-12`).

Generic JLA uses 4,096 probes; the no-control generic/compressed comparison uses
8,192, seed `9252026`. All four targets lie within six reported numerical
MCSEs plus `1e-8*max(1,abs(exact))` of exact. These are numerical checks, not
sampling-SE or finite-projection unbiasedness claims. Tests also cover string
IDs, row permutation, if restrictions, caller data/order/RNG restoration,
cross-cell ID rejection, full/deleted-control rank failures, automatic fallback,
strict native withholding and idle native state, and unchanged ordinary/default
match and observation samples. Existing graph tests retain disconnected/tied
component and bridge/articulation failures.

The full profile adds **256 attempted, 256 successful** exact joint-control
replications, seed `9252026`, with stored-row errors `0.8*b_g + 0.6*u_r`:
independent block shocks and independent row shocks, hence within-block
correlation .64; literal frequency copies share their stored-row error. The
unequal target masses differ from regression frequencies. All outcomes and
return codes are saved to `.local/pooled-deletion/monte_carlo.csv`; failures are
listed and a 100% fit-success gate precedes any mean-error gate. This is focused
development evidence, not a registered confirmation campaign.

| Target | Truth | Mean error | Simulation MCSE |
|---|---:|---:|---:|
| Worker variance | 36.139506 | −0.12503808 | 0.17057927 |
| Firm variance | 0.01592431 | −0.00530918 | 0.00701040 |
| Covariance | 0.29118519 | 0.03924942 | 0.03344495 |
| Total variance | 36.737801 | −0.05184835 | 0.15627297 |

All four errors are below 1.2 simulation MCSEs; the fixed diagnostic gate is
six MCSEs. No threshold was changed after inspecting results.

## Validation and limits

Commands and complete diagnostic logs are in ignored `.local/pooled-deletion/`.
The focused Stata/MP 18 and 19 regression passes, including its full Monte Carlo branch:

```
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do \
  fevc/tests/stata/test_pooled_deletion.do fevc full
'/Applications/Stata 18/StataMP.app/Contents/MacOS/stata-mp' -q do \
  fevc/tests/stata/test_pooled_deletion.do fevc full
```

Stata/MP 18.0.130 and 19.0.115 pass the complete focused
regression, including the full Monte Carlo branch. Eight separate numerical
point cells (exact/JLA × joint/fixedoffset × movers/both) also preserve samples,
partitions and caller states, and their four exported targets agree across
versions. During development, the initial oracle passed repeated physical-copy
indices to `st_data()`; subsequent JLA calls could crash in Stata's allocator,
on either version and also on the unchanged baseline. The oracle now imports
stored rows once and expands them entirely inside Mata. Literal deleted QR
refits, all acceptance thresholds and production estimation are unchanged.

The ordinary Python gate was attempted. One pre-existing deployment test
requires `git symbolic-ref --short HEAD == main` and fails in the explicitly
authorized detached app worktree. Its external-deployment guard was not changed.
The integrated rerun deselects only that test:

```
PYTEST_ADDOPTS='-k not\ test_deployer_check_only_never_calls_remote_tools' \
  ./.venv/bin/python fevc/tools/run_checks.py
./ci/run_ci_profile.sh plugin-build
```

The source/Python/CMG/quick gates pass, including **828 Python tests, one
deselected**, and `fevc/cmg/tools/assemble.py --all --check`. The full suite
initially stopped at the unchanged `test_forced_cmg.do` assertion
`cmg.preconditioner_seconds > 0`, after its residual, coefficient, convergence
and iteration checks passed. That test passed unchanged in isolation. A local
driver then resumed the unchanged integrated runner at the full suite:

```
./.venv/bin/python .local/pooled-deletion/resume_integrated.py
```

This rerun passes the same CMG timing assertion and the complete full suite,
then the clean installation, both helper-migration layouts, synthetic and paired
B1/CMG benchmark validators, Separations preparation fixtures and MATLAB-sample
bridge audit. It exits 0 with `FEVC LOCAL QUALIFICATION PASS`. The full profile
again has 256/256 successful Monte Carlo replications and the same values in
the table above. The local driver executes the remaining statements directly
from `run_checks.py`; it changes no assertion, fixture, solver or threshold.
`integrated-final-counters.log` preserves the initial run and timing-only
failure; `forced-cmg-timer-recheck.log` and `integrated-resumed.log` preserve
the successful recheck and completion. A strictly positive subroutine duration
is sensitive to timer resolution; this transient failure is not erased.

The native qualifier completed successfully on Stata/MP 19.0.115, macOS
26.6.2, Rust 1.85.1: Rust formatting, clippy, 14 unit and six build-boundary
tests, C transport/ABI checks, thin and universal arm64 and Rosetta x86-64
routes, and both isolated clean installations passed. The new regression has
explicit terminal PASS markers on all four artifact/architecture combinations and both installed
packages. Native route fixtures now use actual one-block stayers and
one-block-per-cell supported comparisons, with separate assertions that
parallel requests are withheld. Prior native receipts remain unchanged.

The outer `plugin-build` CI wrapper nevertheless exits 1 with
`failure_kind=dirty_checkout`: this uncommitted repair cannot qualify the
baseline commit. The inner qualifier exits 0 and records
`classification=LOCAL_CHECKPOINT_DIRTY_TREE`, with 221 source files bound to
manifest SHA-256
`ca4f89b3a021726a980828e5f2aa58a588a28d08e0a790c25f6ca2d2686380fa`.
The source hash is unchanged across compilation and testing. This is successful
source-local validation, not a clean-SHA candidate receipt. Its receipt,
qualification text and manifest are copied to
`.local/pooled-deletion/plugin-final-{receipt.json,qualification.txt,source-manifest.sha256}`;
sanitized Stata logs and temporary candidate binaries are in ignored
`.ci/stata/run/plugin-evidence/`.

The qualifier automatically stages its three candidates into the package
directory on success, so the subsequent integrated full-suite completion used
those tested candidates. At final review, all three repository binaries were
restored byte for byte from the original clean `HEAD`; candidate copies remain
in ignored evidence. `.local/pooled-deletion/binary-restoration.json` records
both identities. The complete focused regression, including all 256 Monte
Carlo draws and public automatic/strict native routing, then passed again
against the restored binaries (`final-restored-binaries.log`). The final tree
contains no native binary change. Full candidate-platform evidence remains
bound to its recorded candidate hashes.

Development failures remain logged: the initial public-SDK fetch was blocked
by the network sandbox; old native fixtures expected unsupported parallel
partitions; the literal-oracle import needed the correction described above;
and the clean-install test initially assumed zero native release history.
The latter now compares history before and after the rejected call and passes
in both fresh and reused sessions. Preparation-counter fixtures now include
the two additional validation sorts; observation-mode counters are unchanged.
No numerical threshold was relaxed. No Linux, Windows, native Intel hardware,
new-scale, or unrestricted inference-coverage claim is made.

## Preparation cost diagnostic

Supplied deletion IDs add two Stata validation sorts; omitted IDs and
observation requests do not. An 80,000-row, 2,000-worker, 20-firm ordinary-match
smoke (40,000 blocks; native automatic JLA, 64 probes, seed `9252026`, four
processors) compares the baseline package with the repaired boundary. After
one warm-up per version, four complete-command runs have medians 0.458 and
0.494 seconds respectively. All four targets agree within the deterministic
`1e-8` gate. Raw `timing-*.csv` and the generator are in the diagnostic directory.
These runs overlap other qualification work; they bound this small operational
check, not a performance or scale qualification claim.
