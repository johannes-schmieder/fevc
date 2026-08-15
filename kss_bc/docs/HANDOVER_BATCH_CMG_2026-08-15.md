# `kss_bc` batch/CMG handover — 2026-08-15

## KSS-NUMOPT follow-up addendum

The owner subsequently authorized bounded KSS numerical optimization and
read-only Separations wage work, then explicitly directed cancellation of
obsolete B0 job `7185180`. It was deleted and collected: wall 33,307 seconds,
CPU 132,808.540 seconds, four slots, maximum virtual memory 1.329 GiB,
`failed=100`, and exit 137. This is `USER_CANCELLED_OBSOLETE_B0`, not accepted
benchmark evidence. The historical instructions later in this handover to
leave that job active are superseded.

API 15 implements true lockstep diagonal B1 and one shared backend-driven PCG
kernel for B1 and forced test-only CMG. Source-bound Stata 19 SCC estimator
tests at 10,000 workers, 1,000 firms, and 200 probes give C/B1 command gains of
1.725x on moderate and 4.620x on weak, with estimator-matrix differences
`3.23e-12` and `2.01e-11` and every complete residual below `1e-10`. Easy C
fails closed as `HIERARCHY_STALLED`. Automatic routing remains disabled.

The owner-authorized SCC-only Separations ladder leaves the project checkout
and wage artifacts unchanged. A genuine 500-worker dense core and the larger
all-eligible-mover sample both fail the unchanged match-deletion estimability
gate under B1; CMG never converts that rejection into success. The maintained
MATLAB reference loader now checksum-binds its CMG entry point and nine MEX
hierarchy sources plus its double-preconditioner family, and compiles binaries
only below the KSS run directory. Final MATLAB job `7188235` passes on its own
smaller maintained leave-one-out set; it is descriptive and does not override
the API 15 withholding. A new benchmark-only adapter may reconstruct that
retained match set from checksum-bound SCC artifacts, restore all physical
rows, repeat KSS pruning, and audit match bridges before exact/B1/CMG runs.
This does not alter the public selector or create a MATLAB runtime dependency.
All estimator projections remain capped at 90 minutes.

The prior documentation thread explicitly released `kss_bc/PLAN.md`; the
current KSS-NUMOPT owner may edit it. No other thread owns `kss_bc/**` or the
CMG implementation at this addendum.

This is the durable handover from the implementation thread that began from
`varcomp_hdfe_specification.md`. It records repository state at
`2026-08-15T10:43:26Z` (`06:43:26-0400`) and releases the thread's ownership
after the validation recorded at the end.

## Historical immediate state at original handover

The following section records the original handover snapshot. Its prohibition
on cancelling `7185180` is superseded by the later owner direction and the
completed accounting above.

Substantive development has stopped. Do not cancel SCC job `7185180`, submit
another job, or change estimator algorithms while that job is active. The job
was still running (`r`) on `econ@scc-gr4.scc.bu.edu` at the handover snapshot.
The next thread should first read the repository `AGENTS.md`, `kss_bc/AGENTS.md`,
this file, and the `$cluster` skill. It should then monitor `7185180` read-only:

```bash
ssh scc 'qstat -u johannes | grep 7185180'
ssh scc qstat -j 7185180
```

Absence from `qstat` is not success. After the job leaves the queue, collect
`qacct`, copy the run evidence to a new temporary directory, and validate the
large job as described under “Pending SCC completion.”

## Repository state

- Branch/worktree: `main`, current worktree. No branch or worktree was created.
- Current HEAD: `9f06a2f2ea2dba449289f35012a88067ec8447f7`.
- Starting/base commit for this implementation:
  `2b3bb6b15b6a6d4d638c8ec6e7d3eea8641a7abb`.
- Relationship to remote before this handover file: `main...origin/main
  [ahead 5]`.
- The only new working-tree file made for this handover is this untracked
  Markdown file. It is intentionally not staged or committed.
- `tools/run_checks.py` remained unchanged; SHA-256 is
  `1195698665a97b1fbd41f97bac7b601e3fc4a4113fea656a1fe9e4cfb849fcaf`.
- No push was performed.

The implementation thread made these commits, in order:

1. `5c04aa4eb1681f5a5c475e2aa55770d4be04266e` — `feat: implement internal kss_bc estimator`
2. `adfd3add472571772157d430b98cda7f76ecd97a` — `test: preserve SCC oracle precision`
3. `f4182b1724d33de25c66ef18833e1211a6e4ee7b` — `test: stabilize SCC scale ladder`
4. `744ca8ed7271b791a721d1d05011d864d439801a` — `docs: record blocked API13 Pro review`
5. `9f06a2f2ea2dba449289f35012a88067ec8447f7` — `test: extend SCC large runtime`

Before this handover file was created, `git status --short --branch` was:

```text
## main...origin/main [ahead 5]
?? cmg_plan.md
?? reviews/gpt-pro/adjudications/CMG-MATA-PLAN.md
?? reviews/gpt-pro/requests/CMG-MATA-PLAN-A/
?? reviews/gpt-pro/requests/CMG-MATA-PLAN-B/
?? reviews/gpt-pro/responses/CMG-MATA-PLAN-A.md
?? reviews/gpt-pro/responses/CMG-MATA-PLAN-B.md
?? shared/
?? varcomp_naming.md
```

Those pre-existing/unrelated untracked paths are not owned by this thread and
were not read, edited, staged, deleted, or committed. After this handover is
written, status must additionally show:

```text
?? kss_bc/docs/HANDOVER_BATCH_CMG_2026-08-15.md
```

## Files changed and thread-owned scope

Relative to the base commit, the five commits add 526 tracked files and
128,413 lines. The large count is mostly immutable GPT Pro packet snapshots.
No tracked file outside the following scopes changed:

- all implementation, documentation, package, oracle, test, benchmark, and
  SCC harness files under `kss_bc/**`;
- KSS-specific review artifacts under
  `reviews/gpt-pro/requests/KSS-BC-*`,
  `reviews/gpt-pro/responses/KSS-BC-*`, and
  `reviews/gpt-pro/adjudications/KSS-BC-*`.

The live `kss_bc` files are:

```text
kss_bc/AGENTS.md
kss_bc/CHANGELOG.md
kss_bc/PLAN.md
kss_bc/README.md
kss_bc/TESTING.md
kss_bc/benchmarks/README.md
kss_bc/benchmarks/oracle/kss_bc_dense_oracle.m
kss_bc/benchmarks/oracle/stata_oracle.do
kss_bc/benchmarks/scc/collect_qacct.sh
kss_bc/benchmarks/scc/run_oracle.sge
kss_bc/benchmarks/scc/run_portability.sge
kss_bc/benchmarks/scc/run_scale.sge
kss_bc/benchmarks/scc/submit_one.sh
kss_bc/benchmarks/synthetic_benchmark.do
kss_bc/benchmarks/validate_scc.py
kss_bc/docs/BLOCK_CONTROL_DERIVATION.md
kss_bc/docs/DECISIONS.md
kss_bc/docs/ESTIMATOR_CONTRACT.md
kss_bc/docs/FAILURES_AND_RETURNS.md
kss_bc/docs/JLA_FINITE_PROJECTION.md
kss_bc/docs/NUMERICAL_ARCHITECTURE.md
kss_bc/docs/SOURCE_PROVENANCE.md
kss_bc/kss_bc.ado
kss_bc/kss_bc.mata
kss_bc/kss_bc.pkg
kss_bc/kss_bc.sthlp
kss_bc/stata.toc
kss_bc/tests/python/__init__.py
kss_bc/tests/python/oracle.py
kss_bc/tests/python/test_block_controls.py
kss_bc/tests/python/test_dense_oracle.py
kss_bc/tests/python/test_frequency_probes.py
kss_bc/tests/python/test_jla_formula.py
kss_bc/tests/python/test_monte_carlo_bias.py
kss_bc/tests/python/test_package_layout.py
kss_bc/tests/stata/run_all.do
kss_bc/tests/stata/test_control_anchor.do
kss_bc/tests/stata/test_exact_fixture.do
kss_bc/tests/stata/test_failures.do
kss_bc/tests/stata/test_frequency.do
kss_bc/tests/stata/test_graph_pruning.do
kss_bc/tests/stata/test_install.do
kss_bc/tests/stata/test_jla_convergence.do
kss_bc/tests/stata/test_jla_fixture.do
kss_bc/tests/stata/test_load.do
kss_bc/tests/stata/test_semantics.do
kss_bc/tests/stata/test_stale_runtime.do
kss_bc/tools/run_checks.py
```

The review series owned by the implementation thread comprises request,
response, and adjudication records for `KSS-BC-DERIVATION-A/B`,
`KSS-BC-REPAIRED-C/D`, `KSS-BC-API7-E/F`, `KSS-BC-API9-FINAL-G/H`,
`KSS-BC-API10-FINAL-I/J`, `KSS-BC-API11-FINAL-K/L`, and
`KSS-BC-API12-FINAL-M/N`; API13 has blocked response/adjudication O and local
request packets O/P. There is no API13 response P.

Use this command for the exact tracked inventory:

```bash
git diff --name-status 2b3bb6b15b6a6d4d638c8ec6e7d3eea8641a7abb..9f06a2f2ea2dba449289f35012a88067ec8447f7
```

## Implemented scope and fixed owner decisions

`kss_bc` is an internal sibling to the untouched `ppml_talo`; package
unification and a wrapper were deferred. It implements finite fixed-design KSS
point estimates for worker variance, firm variance, worker–firm covariance,
and total variance in a linear two-way fixed-effect model. It supports exact
and improved-JLA backends, observation and actual-match deletion, joint and
fixed-offset controls, literal positive-integer frequency weights, target
weights, mover-only match headlines, matrix-free quotient PCG, typed
fail-closed behavior, and no `e(V)`.

Preserve these owner decisions:

- work stays on `main`; do not create a branch or worktree unless the owner
  explicitly requests it;
- SCC qualification uses public/synthetic data only;
- match headlines are mover fit/mover target; `stayers(both)` is withheld;
- the corrected mixed fourth-moment coefficient is one;
- MATLAB coefficient two is audit-only legacy evidence;
- the maintained `LeaveOutTwoWay` source pin is
  `8b957ffeb10b8465a3584fceb0265cccc48379e1` and provides no public license;
- `LeaveOutKSS` 0.1.0 is secondary translation evidence, not the authority;
- the package is point-estimation only, internal, unlicensed for public
  redistribution, and not qualified on restricted data.

The current runtime/API token is API13 with build ID
`kss-bc-api13-invariant-order-dimension-certificate`. Controlled exact, JLA,
and auto paths use one outcome/per-copy-target semantic order that excludes
raw controls and encoded IDs; nonexchangeable semantic ties withhold. The
control count is capped at 32, the canonical-control basis carries a
dimensioned forward-error envelope through full and deletion conditioning,
literal totals above `2^53` fail before graph ranking, every JLA allocation is
gated by `physical_limit()`, and corrected-target subtraction is checked for
finiteness before posting.

## KB0–KB6 status

| Milestone | Status | Durable result or open gate |
|---|---|---|
| KB0 | COMPLETE | Governance, provenance, independent dense oracle, coefficient discrepancy evidence, and package skeleton are present. |
| KB1 | COMPLETE | Sample/target/graph semantics, exact estimator, literal frequency/target weights, match deletion, and typed failures are implemented and tested. |
| KB2 | COMPLETE | Matrix-free fit and inverse actions, quotient PCG, control Schur complement, normalization, and full residual checks are implemented and locally qualified. |
| KB3 | COMPLETE | Improved JLA, indexed Rademacher stream, coefficient-one moments, literal-copy aggregation, controls, target contractions, MCSE diagnostics, and fail-closed allocation gates are implemented and tested. |
| KB4 | COMPLETE | Ado/Mata command, help/package metadata, quick/full/install suites, local runner, oracle, and benchmark harness pass locally. |
| KB5 | BLOCKED / IN PROGRESS | API12 reviews M/N found real issues that were repaired in API13. Two fresh independent packet-bound API13 Pro reviews are still required. O was not transmitted because Chrome file upload failed; P is unsubmitted. No API13 mathematical verdict exists and status must not advance to `ai_reviewed`. |
| KB6 | IN PROGRESS | Portability, paired oracle, smoke, and medium SCC jobs pass. The 18-hour scalar-B0 rerun `7185180` was owner-cancelled as obsolete and is not accepted benchmark evidence. B1/CMG and bounded real-data qualification remain open. |

Overall plan status remains active. Do not mark the package complete,
`ai_reviewed`, `checked`, or `independently_checked`.

## Mathematical review state and pending findings

API12 reviews M and N were valid `false` reviews. Their accepted findings were:

- raw-control/encoded-ID ordering could choose different canonical anchors
  under invertible control maps or relabeling;
- the control-basis error proxy was not a demonstrated dimension-aware forward
  bound;
- ordinary double summation could rank exactly equal component masses
  differently above `2^53`;
- some match/target JLA paths could allocate literal signs without the public
  physical-copy limit.

API13 repaired those findings with the common semantic order, conservative tie
withholding, the exact incremental `2^53` gate, all-JLA `physical_limit()`
gates, the 32-control cap, dimensioned `gamma_m`/inverse/Cholesky/product
envelope, positive perturbation denominators, projector/anchor/span checks, and
full/deletion `E/(r-E) <= 1e-8` propagation. Local API13 regressions include the
reviewers' determinant-four, `Q` versus `-Q`, two-`K(2,2)`, relabeling,
allocation, and overflow witnesses. Local transform stress recorded:

- joint: 200 accepted / 200 withheld; accepted maximum gap about `4.14e-14`;
- fixed offset: 133 accepted / 267 withheld; accepted maximum gap about
  `3.86e-15`.

Those repairs have not received fresh external review. Packet O has SHA-256
`e8c6d3be0dbaec60e6973ba77c1e711422f942c34ab29d2dd8ca4c2702cb4969`.
Its blocked record has SHA-256
`41f24254d9f48bf044dda5fda75fbcbfdd9cccf03c7b2cf76079b6dda7b8f8a3`.
No packet or prompt was sent, and no model response was generated. Packet P has
SHA-256
`5f7dd6485934148cd8309c9bf3682db388911f4e3c83531b048b1e838320ffc3`
and remains unsubmitted.

Do not overwrite O or silently treat P as closure evidence. Once browser upload
works, prepare two new fresh packet IDs against the final source state, use
separate fresh Pro chats with no cross-contamination, store responses verbatim,
validate them, and adjudicate locally. To enable file upload, the owner must
open `chrome://extensions`, choose Details for the ChatGPT browser extension,
and enable “Allow access to file URLs”; see
<https://developers.openai.com/codex/app/chrome-extension#upload-files>.
GPT Pro evidence can set at most `ai_reviewed`; it cannot create human
independent review.

## SCC run and job ledger

All SCC data are public/synthetic. Every attempt has a unique immutable path
under `/projectnb/welfgr/kss-bc/runs/`; do not overwrite or delete any attempt.

| Run ID / source | Jobs and outcome | Diagnosis / evidence |
|---|---|---|
| `20260814T174421Z-5c04aa4` / `5c04aa4eb1681f5a5c475e2aa55770d4be04266e` | portability `7182360` PASS; oracle `7182484` FAIL (`failed=0`, `exit_status=1`, 161 s) | Stata `import delimited` inferred MATLAB numeric columns as float, losing about `1e-9`; MATLAB and Stata otherwise agreed near machine precision. Preserved remotely and at `/private/tmp/kss-bc-scc-20260814T174421Z-5c04aa4.UsJGBJ`. Archive SHA-256 `09ec62d7cd9a984f22f67b42b390b91702fda440e9d937a920920e44c543059c`. |
| `20260814T180248Z-adfd3ad` / `adfd3add472571772157d430b98cda7f76ecd97a` | portability `7182573` PASS; oracle `7182649` PASS; smoke `7182724` PASS; medium `7182738` FAIL (`failed=0`, `exit_status=1`, 10 s) | The sinusoidal second control landed on the canonical-anchor cutoff at medium scale. Repaired with the registered four-row discrete control pattern and `1e-8` scale tolerance/validator ceiling. Preserved remotely and at `/private/tmp/kss-bc-scc-20260814T180248Z-adfd3ad.F5tEu7`. Archive SHA-256 `ad5676676f00f077db4846ea4a15f39aaef4d83dc8db68d7aff08f6f131d1816`. |
| `20260814T183348Z-f4182b1` / `f4182b1724d33de25c66ef18833e1211a6e4ee7b` | portability `7182919` PASS; oracle `7182928` PASS; smoke `7182954` PASS; medium `7182968` PASS; large `7183395` FAIL (`failed=100`, `exit_status=137`) | Accepted four-job evidence plus the preserved failed 12-hour large attempt. Remote path `/projectnb/welfgr/kss-bc/runs/20260814T183348Z-f4182b1`; local evidence `/private/tmp/kss-bc-scc-20260814T183348Z-f4182b1.i9ac9G`; archive SHA-256 `ba5821dd7ded480c612191078278172d8c98c3220259455e731a8729274896ba`. |
| `20260815T081233Z-9f06a2f` / `9f06a2f2ea2dba449289f35012a88067ec8447f7` | large `7185180` ACTIVE at handover | Exact rerun of the large scientific design with only the scheduler-request/test/changelog repair. Remote path `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f`; archive SHA-256 `d1f37ae779cb4bb3c99b8fb169bc5d01d7d514d520ce8c25a87078daa4903b2c`. No local result evidence exists yet. |

### Accepted portability/oracle/smoke/medium evidence

The accepted evidence is source-bound to `f4182b1724d33de25c66ef18833e1211a6e4ee7b`.
The deployed archive records:

```text
ba5821dd7ded480c612191078278172d8c98c3220259455e731a8729274896ba  source.tar.gz
```

- Portability `7182919`: `failed=0`, `exit_status=0`, wallclock 15 s,
  `maxvmem=509.309M`; Stata full suite and isolated `net install` markers pass.
- Oracle `7182928`: `failed=0`, `exit_status=0`, wallclock 92 s,
  `maxvmem=9.045G`; MATLAB and Stata exact plugin/correction/corrected values
  agree for all four targets within about `1.4e-15`, well inside the `2e-9`
  validator ceiling.
- Smoke `7182954`: `failed=0`, `exit_status=0`, wallclock 14 s,
  `maxvmem=497.910M`; 5,000 workers, 250 firms, 20,000 stored rows, 30,000
  literal observations, 40 probes; command 12.405 s, GNU-time elapsed 12.89 s,
  peak RSS 82,876 KB, 125 solver iterations, maximum residual/inverse residual
  `2.13804178325e-11`, maximum leverage `0.8791858739671892`, control Schur
  rcond `0.5260532196386638`, deletion gap `0.9997998999995513`.
- Medium `7182968`: `failed=0`, `exit_status=0`, wallclock 3,870 s, CPU
  15,415.801 s, `maxvmem=769.918M`; 50,000 workers, 2,500 firms, 200,000
  stored rows, 300,000 literal observations, 100 probes; command 3,868.368 s,
  peak RSS 430,956 KB, 1,250 solver iterations, maximum residual/inverse
  residual `5.16314065594e-09`, maximum leverage `0.8328018096350546`, control
  Schur rcond `0.5260532196389142`, deletion gap `0.9999798999961999`.

The SCC application reports `c(stata_version)=19` and `c(flavor)=IC` even
though the loaded module/executable is `stata-mp/19` / `stata-mp`. Preserve
that observed distinction; do not relabel the application output as MP.

### Failed 12-hour large attempt and runtime diagnosis

Job `7183395` used 250,000 workers, 10,000 firms, 1,000,000 stored rows,
1,500,000 literal observations, 200 probes, seed `20260814`, four slots,
8 GB/core, and `h_rt=12h`. It ran on `scc-gr4` until exactly 43,200 seconds.
Final `qacct` was `failed=100`, `exit_status=137`, CPU `172292.480` seconds,
and `maxvmem=1.329G`. The Stata application log ends inside the `kss_bc` call;
no large CSV or success markers exist and `resources.txt` is empty. This is a
scheduler walltime failure, not accepted numerical evidence.

The production runtime currently serializes matrix right-hand sides.
`kssbc__fe_solve_matrix()` in `kss_bc/kss_bc.mata` loops over
`column=1..cols(right_hand_side)` and calls scalar `kssbc__fe_solve()` for each
column (currently lines 1211–1212). Thus the JLA matrix path performs separate
scalar PCG recurrences rather than a block/batched Krylov solve. The medium
case took about 1.07 hours for 100 probes and 200,000 rows; the large case has
five times the rows and twice the probes and exceeded 12 hours. Treat the
scalar-PCG loop as the explicit runtime diagnosis. Do not change it while
`7185180` is active. If the 18-hour run fails, do not submit again automatically;
preserve evidence and obtain owner direction before an algorithmic or
checkpointing redesign.

### Historical job `7185180` snapshot before cancellation

- Run/source: `20260815T081233Z-9f06a2f` /
  `9f06a2f2ea2dba449289f35012a88067ec8447f7`.
- Remote root:
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f`.
- Source/archive:
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/source` and
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/source.tar.gz`.
- Submission receipt:
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/submissions/large.job_id`.
- Log/output paths:
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/logs/large.stdout.txt`
  and
  `/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/scale/large/`.
- Dimensions: 250,000 workers, 10,000 firms, 1,000,000 stored rows,
  1,500,000 literal observations, 200 probes, seed `20260814`.
- Request: `-P welfgr`, `-pe omp 4`, `h_rt=18:00:00` (`64800` seconds),
  `mem_per_core=8G`, no explicit queue/host, Stata module `stata-mp/19`.
- Submitted `2026-08-15 04:23:23 EDT`; started `04:26:06 EDT`
  (`08:26:06Z`) on `econ@scc-gr4.scc.bu.edu`.
- Snapshot at `2026-08-15T10:43:26Z`: state `r`, four slots, aggregate CPU
  `08:58:27`, `vmem=1.255G`, `maxvmem=1.264G`, negligible I/O.
- The clean local source archive is
  `/private/tmp/kss-bc-20260815T081233Z-9f06a2f.tar.gz` (about 1.7 GB), SHA-256
  `d1f37ae779cb4bb3c99b8fb169bc5d01d7d514d520ce8c25a87078daa4903b2c`.
- Estimator bytes are identical to the accepted `f4182b1` run. Verified SHA-256:
  `kss_bc.ado` `27aaebcead8f528a94efdd2ee4f1c59f87e1ef54486f193602b5202b7d8402d8`;
  `kss_bc.mata` `60b8ae8d9b550ddb9fe34751070a0923acc288e7fcdeacb7a8ca464353b2ff32`;
  synthetic driver `9d56cb3b22013d3ce082ef5bd443b22cb62a3c22248ddefe69238109abd36d5a`.

The source difference from `f4182b1` is documentation of blocked review O and
the harness-only 12-to-18-hour request, its static regression assertion, and
changelog entry. The million-row design, probes, seed, tolerance, estimator,
and output validator were not weakened.

### Superseded SCC completion instructions

After `7185180` leaves `qstat`, do not infer success. Run:

```bash
ssh scc 'bash /projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/source/kss_bc/benchmarks/scc/collect_qacct.sh /projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f large'

EVIDENCE_DIR=$(mktemp -d /private/tmp/kss-bc-scc-20260815T081233Z-9f06a2f.XXXXXX)
rsync -rl --omit-dir-times --no-perms --no-owner --no-group \
  --exclude source --exclude source.tar.gz \
  scc:/projectnb/welfgr/kss-bc/runs/20260815T081233Z-9f06a2f/ \
  "$EVIDENCE_DIR/"

./.venv/bin/python kss_bc/benchmarks/validate_scc.py \
  --run-dir "$EVIDENCE_DIR" \
  --expected-commit 9f06a2f2ea2dba449289f35012a88067ec8447f7 \
  --jobs large
```

Accept only if all three layers agree: `qacct failed=0` and `exit_status=0`,
the wrapper/application markers are present, and `validate_scc.py` passes the
large CSV and diagnostics. If accepted, combine this source-bound large record
with the already accepted four-job `f4182b1` evidence, explicitly noting the
estimator-byte identity and harness-only commit difference. Then add compact
repository evidence and `kss_bc/benchmarks/scc/QUALIFICATION.md`, update KB6 in
`kss_bc/PLAN.md`, and rerun the appropriate gates. Do not include the 1.7 GB
archive or verbose logs in Git. If the job fails, preserve the run, document
the exact failure, and ask the owner before another submission or algorithmic
change.

## Validation commands and last outcomes

Repository startup/gates completed before implementation:

```bash
./.venv/bin/python tools/verify_handover.py
./.venv/bin/python tools/doctor.py
./.venv/bin/python tools/run_checks.py --scope handover
```

Last outcome: all PASS.

The complete package-local gate was rerun at current HEAD after the 18-hour
harness repair:

```bash
./.venv/bin/python kss_bc/tools/run_checks.py
```

Last outcome: `KSS_BC LOCAL QUALIFICATION PASS`; 44 Python tests passed, Stata
quick/full passed, isolated clean install passed, and the synthetic harness
smoke passed. The targeted static harness command also passed:

```bash
./.venv/bin/python -m pytest kss_bc/tests/python/test_package_layout.py -q
```

Last outcome: `15 passed`.

The full repository gate was run after API13 and before the later docs/harness-
only commits:

```bash
./.venv/bin/python tools/run_checks.py --scope full
```

Last outcome: PASS, including 209 M2–M6 tests, M7–M13 checks, paper/supplement
builds, source audit, and finite verification. It was not rerun after the
handover-only or SCC-runtime-request documentation changes.

The accepted four-job SCC evidence passes:

```bash
./.venv/bin/python kss_bc/benchmarks/validate_scc.py \
  --run-dir /private/tmp/kss-bc-scc-20260814T183348Z-f4182b1.i9ac9G \
  --expected-commit f4182b1724d33de25c66ef18833e1211a6e4ee7b \
  --jobs portability oracle smoke medium
```

Last outcome: `PASS portability`, `PASS oracle`, `PASS smoke`, `PASS medium`,
then `KSS_BC SCC EVIDENCE PASS`.

The same command with `large` appended fails closed after the first four passes:

```bash
./.venv/bin/python kss_bc/benchmarks/validate_scc.py \
  --run-dir /private/tmp/kss-bc-scc-20260814T183348Z-f4182b1.i9ac9G \
  --expected-commit f4182b1724d33de25c66ef18833e1211a6e4ee7b \
  --jobs portability oracle smoke medium large
```

Last outcome: `ValueError: qacct large: scheduler failure`, as required for
`failed=100`, `exit_status=137`.

The blocked O review record validates structurally:

```bash
./.venv/bin/python tools/reviews/validate_review_record.py \
  reviews/gpt-pro/responses/KSS-BC-API13-FINAL-O.md
```

Last outcome: `Review validation: PASS`. This validates the blocked record,
not a mathematical review.

## Prohibited changes and protected paths

At the original handover, before the later authorization, the next thread was
instructed not to:

- cancel `7185180`, submit another SCC job, overwrite/delete any run directory,
  or run sustained compute on an SCC login node;
- make algorithmic changes while `7185180` is active;
- create/switch branches or worktrees, push, or stage unrelated user files;
- edit `ppml_talo/**`, `application/**`, `software/**`, `paper/**`, `theory/**`,
  `proof-audit/**`, `state/**`, `archive/**`, `paper/releases/**`, or the frozen
  root `tools/run_checks.py`;
- edit or absorb the unrelated untracked `cmg_plan.md`, `CMG-MATA-PLAN*`
  records, `shared/`, or `varcomp_naming.md`;
- claim public licensing, restricted-data qualification, sampling inference,
  uniform asymptotics, a stayer-robust fallback, package unification, or
  independent review;
- treat executable tests, SCC synthetic evidence, or GPT Pro output as proof of
  an asymptotic theorem.

All immutable/frozen repository paths listed in the root `AGENTS.md` remain
untouched. `ppml_talo` remains untouched and separate.

## Post-write handover validation

The handover's required-section/trailing-whitespace audit passed with no
output. The smallest package-relevant static test was then run:

```bash
./.venv/bin/python -m pytest kss_bc/tests/python/test_package_layout.py -q
```

Outcome: `15 passed in 0.24s`. No file was staged or committed. The final
working tree has the handover as an untracked file and retains all unrelated
untracked user paths listed under “Repository state.”

## Ownership release

The implementation thread has stopped substantive development and has no
remaining exclusive file claim. Ownership of `kss_bc/**` and the KSS-specific
review records is released to the fresh Codex thread, subject to the protected
paths, live-job restrictions, and unresolved KB5/KB6 gates recorded above.
