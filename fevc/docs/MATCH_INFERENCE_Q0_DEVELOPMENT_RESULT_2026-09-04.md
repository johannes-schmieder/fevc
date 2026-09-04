# Fixed-offset collapsed-match q=0 development result

## Decision and scope

The registered grouped match-deletion `q=0` development campaign passes. Its
exact execution source is
`c26a7ee44cbcbececc42baf9706d793a7abb4e6f`; the original registration and
pre-result amendment are
[`match_inference_q0_campaign_v1.json`](match_inference_q0_campaign_v1.json)
and
[`match_inference_q0_campaign_v1_amendment1.json`](match_inference_q0_campaign_v1_amendment1.json).
Their SHA-256 values are respectively
`d7254ff52c144122a97a32cbe4cc5aa4633b860cb18448f398a33145c6634325`
and
`9503074ad77a44f31e67f090e11f04a71ad5bffba32e99878eb38773851ab52b`.
The machine-readable result is
[`match_inference_q0_campaign_v1_result.json`](match_inference_q0_campaign_v1_result.json).

This is accepted development evidence for the internal foundation. It is
sufficient to begin a separately derived and registered grouped-scalar `q=1`
local slice. It is not an independent `q=0` confirmation, public promotion,
automatic route, package release, or authority to distribute a native binary.
No `q=1` code or public match-inference option was added in this result slice.

The inferential interpretation remains narrow:

> Match-cluster inference conditional on the full-sample fixed nuisance-control
> offset, allowing unrestricted within-match dependence and using a structured
> model for match-aggregate variances.

Distinct declared matches are assumed independent, including distinct matches
belonging to the same worker. The calculation does not account for uncertainty
in the estimated nuisance-control coefficients. It is neither joint-nuisance
nor unrestricted-KSS inference.

## Prospective amendment and prerequisites

The original V1 registration remains unchanged. The first development-profile
preflight at source `7d41347` stopped before manifest creation because the
outcome-free one-mode firm target had leading share `0.6594`, below the frozen
`0.75` regime gate. No outcome, coverage, standard error, result manifest, or
SCC task existed. Amendment 1 changed only that diagnostic target's mass
multiplier and the non-evidentiary tiny profile's probe resolution. It did not
change a development DGP, seed, dimension, replication, threshold, task, or
expected output.

The amended complete local tiny path produced all 56 expected rows from 14
tasks and returned `pipeline_complete_non_evidentiary`. Its manifest,
aggregate, and summaries hashes are:

```text
manifest    253a5584a7e354d26bdeb2c2c822f01f86848cd200dc8e7f23df9869fd0605bf
aggregate   358a7b9e14ea8543228f4b72ec481817f99aa4cbc7128d513a13aaec3e733cdf
summaries   642866e202f38b49ffed902ca819516701303c1d78a171f8c007782a1938f7c1
```

Because that run used the prospective dirty amendment tree, it is retained
only as a complete pipeline check. The subsequently committed source produced
a clean one-task SCC smoke under:

```text
/projectnb/welfgr/vckss/runs/20260904T164029Z-match-q0-amended-smoke-c26a7ee
```

Jobs `7444613`/`7444614`/`7444615` built, ran, and aggregated the eight
expected rows. Every stage used one slot and had `failed=0` and
`exit_status=0`. The manifest hash was
`3fc88e68df595f277d7bf3dbc93ce925d1969a9a6a34a01adab3b11bd9b2e9a7`;
the Linux binary hash was
`800bf00265b2056b8294fc08d3c08b0773444db5fe37a8ce67cb10149fe648ec`;
and the aggregate receipt reported `pipeline_complete_non_evidentiary` with
all eight attempts present. No smoke repair or resubmission was needed.

The clean development preflight then passed all 56 outcome-free target checks.
For the amended one-mode fixture, the worker, firm, and total leading shares
were `0.9345`, `0.7673`, and `0.9683`; their remainder shares were `0.1344`,
`0.0309`, and `0.0858`. The covariance target's remainder share was `0.9533`,
so it remains deliberately outside the one-mode claim. The multi-mode fixture
had remainder shares `0.7396`--`1.0000`. These registered geometry checks are
fixture gates, not public concentration cutoffs.

## Exact-source development execution

The clean development manifest has SHA-256
`b716cd3900f90edd44b1ba97827f88f2ae2bd3e3f0855e7823b29924ae2cfda8`.
It binds source `c26a7ee`, an empty worktree diff, both registrations, 14 cells,
400 replications per cell, four targets, 20 replications per shard, 280 unique
tasks, and 22,400 expected target attempts. The exact Git archive has SHA-256
`778e4cd214d7c11c5535ac31e14e2a104332d41533a3fa837b377148ce99703a`.
It was deployed read-only under:

```text
/projectnb/welfgr/vckss/runs/20260904T165450Z-match-q0-development-c26a7ee
```

The real dependency launcher submitted:

| Stage | SGE job | Tasks | Result |
|---|---:|---:|---|
| exact-source build | 7444695 | 1 | pass |
| development array | 7444696 | 280 | pass |
| aggregate validator | 7444697 | 1 | `PASS` |

The SCC build used Rust/Cargo 1.85.1 and Python 3.13.8. Its Linux executable
hash is
`82cafe91a14a10dc7c71ec0458920a09e2469f201ec43930eb90e057b1e51bda`.
The platform-specific macOS preflight binary hash is
`d5eb894e90e2b2b8a872e7d182af958ebbd0e0368c69407958ef3cfc133ce5af`.
The difference is expected; the aggregate verified every task against the
Linux exact-source build receipt.

The separately built clean SCC smoke binary has a different hash even though
it used the same source archive. Cross-host or cross-run byte reproducibility
was not a registered gate. The smoke and development DAGs each bind every task
and aggregate to their own exact-source build receipt; no binary is carried
between them or distributed.

All 280 array records have unique task IDs 1--280, one slot,
`failed=0`, and `exit_status=0`. Their wall times were 8--33 seconds and their
maximum reported virtual memory was 107.305 MiB across 53 hosts and 19 queues.
The build used 35 seconds and 1,710.08 MiB; aggregation used 9 seconds and
331.82 MiB. The frozen `qacct` hashes are:

```text
build       285dbd53889c8d142fd9b998b0860c741de6b02311e51ba2e231eb952cd61edb
tasks       90dad135c75a72aa6cbf2bd65b24eb8107c9f3b0e9e3461855b67e31e4e5b236
aggregate   1062ce37210a41bfd73d1983dc0080713402416eeb47970424f93bbbb46c184b
```

The run contains exactly 280 task payloads, 280 task receipts, and 280 task
logs; all 280 logs have the task PASS marker. Stable tree hashes for the task
files and logs are
`e9f3c403d65e5a1ee2972c6b3867e41487cb109a6203986b0fddbfd6aa23635f`
and
`a09a7a97d0daf23b621e610f95c13a7de01ce1ed341137fc3badaf2dbf894ffd`.
The final aggregate, summaries, and receipt hashes are:

```text
aggregate   1442cb363a8e66a75fc17f74f689147cf5fd8c8bdc09bd59580ce7d8ecd02781
summaries   e11f6aba73548e8b4b05a10a8bf2d509c663f14e08b9a95765c782b9f91e37a8
receipt     ee55a6d41976237f25cfead5ed155e66040d7762695a21f060fe7fa0b66c1de5
```

No task was retried, repaired, excluded, relabeled, or resubmitted after
outcomes were observed. No unrelated user job was altered.

## Changed-surface review

Execution source `c26a7ee` differs from the immediately preceding evidence
source `7d41347` in exactly these paths:

```text
fevc/PLAN.md
fevc/docs/INFERENCE.md
fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md
fevc/docs/README.md
fevc/docs/match_inference_q0_campaign_v1_amendment1.json
fevc/tests/python/test_match_inference_q0_campaign.py
fevc/tools/run_match_inference_q0_campaign.py
rust/README.md
rust/TEST_PLAN.md
rust/crates/vckss-core/examples/match_inference_q0_development.rs
```

The only numerical fixture change is the registered outcome-free one-mode
target mass in the development example. The other executable changes bind and
validate the amendment. The estimator attachment, match covariance formula,
fixed-offset convention, solvers, Counter-V1 implementation, build system,
plugin ABI, public parser, Stata results, input outcome DGPs, semantic seeds,
replications, and acceptance thresholds are unchanged. The campaign therefore
tests exactly the amended registered development fixture without carrying
scientific results across a changed production path.

This result-recording change adds the machine-readable result and this report,
then updates only `fevc/CHANGELOG.md`, `fevc/PLAN.md`, `fevc/docs/DECISIONS.md`,
`fevc/docs/INFERENCE.md`, `fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md`,
`fevc/docs/README.md`, `rust/README.md`, and `rust/TEST_PLAN.md`. It changes no
executable source.

## Scientific result

The aggregate has 22,400 unique expected keys, no missing or extra key, 22,236
successful target attempts, and 164 typed failures. All frozen scientific
gates pass.

| Registered group | Rows | Success | Coverage | Empirical/estimated SE | Result |
|---|---:|---:|---:|---:|---|
| correct model | 24 | 0.9975--1.0000 | 0.9325--0.9775 | 0.9324--1.0437 | pass |
| mild omitted driver | 4 | 1.0000 | 0.9325--0.9525 | 1.0147--1.0533 | pass |
| severe omitted driver | 4 | 0.9850 | 0.8096--0.9365 | 1.0634--1.4519 | limitation visible |

The largest absolute correct-model bias was 1.68 Monte Carlo standard errors,
well inside the frozen four-MCSE gate. The correct-model cells include equal
and highly unequal match sizes, independent physical-row errors, common match
shocks, serial within-match correlation, different within-match covariance
shapes with the same scalar aggregate variance, and the leverage-only
sensitivity fit. The separate CMG solver diagnostic completed every attempt;
its targetwise SE ratios were `0.9742`--`1.0544`.

Severe omission does not masquerade as robustness. Worker and firm coverage
fell to `0.9086` and `0.9061`; total coverage fell to `0.8096`, with a total
empirical-to-estimated SE ratio of `1.4519`. This satisfies the registered
visibility gate and confirms that inference depends on the structured model
for aggregate-match variances. It does not establish protection against an
omitted aggregate-variance driver.

The 164 failures were retained and classified:

- 4 target attempts in one unequal-mass replication and 24 severe-omission
  attempts were withheld with
  `JLA_CONSTRAINT_FAILED:structured_variance_positivity`;
- 36 weak-signal and 100 null-signal target attempts were withheld with
  `JLA_CONSTRAINT_FAILED:component_inference_psd`.

Weak and null cells have no coverage gate. Their respective success rates were
`0.9775` and `0.9375`; the intended result is typed withholding without
regularization, target dropping, or success-conditioned concealment.

All 400 varying-control replications succeeded after fixed-offset removal, and
all successful rows explicitly mark nuisance uncertainty as conditioned away.
That cell is not coverage-eligible. Its repeated-estimation coverage combines
variation in `gamma_hat` that the conditional procedure deliberately omits and
must not be cited as coverage evidence.

## Numerical and diagnostic result

Each successful replication reports 400 independent match clusters. Effective
match count ranged from `342.34` to `400`; the largest match-mass share ranged
from `0.0025` to `0.00477`. Across successful attempts, largest match leverage
was `0.1192`--`0.2609`, the smallest delete-match maker denominator was
`0.7404`--`0.8814`, and maximum match influence share was
`0.00880`--`0.40517`.

The largest complete-system residual was `9.96e-12`, largest trace Monte Carlo
error was `2.99e-6`, and largest point-correction identity error was
`1.73e-18`. No covariance needed PSD cleanup. Successful smallest covariance
eigenvalues ranged from `1.74e-13` to `5.16e-5`; unsupported weak/null draws
were withheld before they could become posted results. Variance-floor share
ranged from zero to `0.3925`, boundary share from `0.0075` to `0.0375`, and
minimum fitted-design reciprocal condition number from `2.44e-5` to `0.9998`.
The full receipt preserves target-specific support, boundary, sensitivity,
solver, spectral, and covariance diagnostics.

The realized one-mode diagnostic had mean leading/remainder shares of
`0.8679/0.2009` for worker, `0.8646/0.1938` for firm, and
`0.9135/0.2950` for total. Its covariance target had
`0.4854/0.9463`, deliberately demonstrating that a design label cannot place
every target inside a one-mode regime. In the multi-mode diagnostic, mean
remainder shares were `0.8280`, `0.8291`, `0.6425`, and `0.8397` for worker,
firm, covariance, and total. These diagnostics constrain the next q1 fixture;
they do not select q automatically or create a post-hoc cutoff.

## Validation record

Before SCC execution, source `c26a7ee` passed:

```text
./.venv/bin/python -m pytest -q fevc/tests/python/test_match_inference_q0_campaign.py
    18 passed
./.venv/bin/python -m pytest -q
    542 passed; only known permission-protected pytest temporary-cleanup warnings
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
    passed
PATH=<pinned Rust 1.85.1>/bin:$PATH cargo fmt --manifest-path rust/Cargo.toml --all -- --check
    passed
PATH=<pinned Rust 1.85.1>/bin:$PATH cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
    passed
PATH=<pinned Rust 1.85.1>/bin:$PATH cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
    passed
PATH=<pinned Rust 1.85.1>/bin:$PATH cargo test --manifest-path rust/stata_backend/Cargo.toml --all-targets --locked
    passed
```

The result-recording change affects documentation and evidence only. Its
focused Python suite passed 18 tests in 6.13 seconds; the full suite passed 542
tests in 37.15 seconds. The CMG generated-source check, campaign SGE shell
syntax, JSON parse, and `git diff --check` also passed. Both pytest runs emitted
only the known permission-protected temporary-cleanup warnings. A separate
local check reconciled the result record to the exact manifest, receipt,
summary, and aggregate hashes. Rust, licensed Stata, and plugin qualification
were not repeated because neither the campaign nor this result record changes
the grouped estimator, public Stata surface, plugin ABI, build system, RNG,
solver, or production numerical path. The internal grouped implementation's
earlier exact-source macOS arm64/Rosetta licensed-Stata qualification remains
recorded in
[`MATCH_INFERENCE_Q0_LOCAL_CHECKPOINT_2026-09-04.md`](MATCH_INFERENCE_Q0_LOCAL_CHECKPOINT_2026-09-04.md).

## Boundary and next slice

The development result supports the collapsed-match `q=0` foundation under
its fixed-offset and structured-variance assumptions. It does not authorize a
public option or an unconditional interval claim. Grouped `q=0` still needs a
separately registered independent confirmation before eventual promotion.

The next internal slice is grouped scalar `q=1`. Start with a written
generalized-mode and raw leave-match recentering derivation, independent
original-row and collapsed-scalar dense oracles, a direct rank-one remainder
identity, and tiny local failure/numerical tests. Register any campaign before
outcomes are generated. Do not inherit a q1 claim for the covariance target in
the one-mode fixture or for any deliberately multi-mode target, and do not
expose q1 through the public parser until its own confirmation and promotion
decision.
