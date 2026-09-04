# Fixed-offset collapsed-match q=0 campaign smoke checkpoint

## Decision and scope

The registered generator-to-validator pipeline and its exact-source SCC Linux
smoke pass. The tested implementation source is
`983ed376b1e0d6ae75825650d416c29cb5fd9c9d`; the frozen registration is
[`match_inference_q0_campaign_v1.json`](match_inference_q0_campaign_v1.json),
SHA-256
`d7254ff52c144122a97a32cbe4cc5aa4633b860cb18448f398a33145c6634325`.
This is pipeline and representative-compute-node evidence only. It authorizes
the already registered bounded q=0 development profile, but it is not coverage
evidence, public promotion, or authority to begin grouped q=1.

The implementation commit changed only these paths:

```text
fevc/CHANGELOG.md
fevc/PLAN.md
fevc/docs/INFERENCE.md
fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md
fevc/docs/README.md
fevc/docs/match_inference_q0_campaign_v1.json
fevc/tests/python/test_match_inference_q0_campaign.py
fevc/tools/run_match_inference_q0_campaign.py
rust/README.md
rust/TEST_PLAN.md
rust/crates/vckss-core/examples/match_inference_q0_development.rs
rust/stata_backend/scc/run_match_inference_q0_aggregate.sge
rust/stata_backend/scc/run_match_inference_q0_build.sge
rust/stata_backend/scc/run_match_inference_q0_task.sge
rust/stata_backend/scc/submit_match_inference_q0_campaign.sh
```

No public parser, Stata return, plugin ABI, estimator, production component
point path, solver implementation, or automatic route changed. The campaign
calls the existing internal grouped q=0 attachment. Its inference remains
conditional on the estimated full-sample fixed control offset, permits
unrestricted dependence within a declared match only through the variance of
the collapsed scalar aggregate, assumes independence across declared matches,
and uses a named structured model for aggregate-match variances. It is neither
joint-nuisance nor unrestricted-KSS inference.

## Frozen campaign construction

The source-bound Rust generator covers 14 registered cells and four component
targets. It distinguishes regression mass, target mass, and the number of
independent matches; generates independent, common-shock, serial-correlation,
and equal-aggregate-variance block shapes; includes equal and highly unequal
match sizes; and varies controls within matches before fixed-offset removal.
The correct aggregate variance is exactly affine in the primary model's
normalized match-mass midrank. The leverage-only fit is retained as a
sensitivity model.

Outcome-free, target-specific preflight checks verify diffuse, one-mode, and
deliberately multi-mode designs before manifest creation. The registered
matrix also includes weak/null signal and mild and severe omitted-driver
cells. Counter-style semantic seeds, task keys, source and file hashes,
expected rows, and the complete target-attempt inventory are frozen in the
manifest. Tasks write new-only atomic payloads and receipts. Aggregation
rejects missing, duplicate, partial, malformed, source-mixed, registration-
mixed, or binary-mixed evidence and classifies every attempted replication.

## Local pipeline and source gates

The complete tiny profile ran all 14 tasks and produced all 56 expected target
rows. Its manifest SHA-256 is
`49b49e652c7eae0601ceb45c0417a36476a7ee77c77fcb5202cc30f403586cc0`;
the aggregate and summaries hashes are respectively
`fc401a81514e370475961e51340599031dd9feef04f9af8e25785dcd4cec2886`
and
`e8458e0fa71e5d4d26c6168b6cb4c4358837e23fc9e48115e86b0e4a5944f07b`.
The aggregate receipt status is `COMPLETE`, with no scientific gate failures.
The null-signal target attempts were retained and classified with the intended
typed `JLA_CONSTRAINT_FAILED:component_inference_psd` failure; no attempt was
silently dropped.

This tiny run was made on the source-manifest-bound prospective tree before
the implementation commit. Its receipt records the dirty source state and is
therefore deliberately non-evidentiary. It validates the entire local
preflight, manifest, task, aggregate, receipt, and failure-classification
path; it is not reused as scientific evidence.

The implementation passed:

```text
./.venv/bin/python -m pytest -q fevc/tests/python/test_match_inference_q0_campaign.py
    17 passed; only known sandbox cleanup warnings
cargo test --manifest-path rust/Cargo.toml -p vckss-core --example match_inference_q0_development --locked
    3 passed
./.venv/bin/python -m pytest -q
    541 passed; only known sandbox cleanup warnings
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
./.venv/bin/python fevc/tools/run_checks.py
    FEVC LOCAL QUALIFICATION PASS, including licensed Stata quick/full and clean-install checks
bash -n rust/stata_backend/scc/run_match_inference_q0_build.sge rust/stata_backend/scc/run_match_inference_q0_task.sge rust/stata_backend/scc/run_match_inference_q0_aggregate.sge rust/stata_backend/scc/submit_match_inference_q0_campaign.sh
    passed
git diff --check
    passed
```

Focused adversarial tests also cover preflight payload tampering, malformed and
partial inventories, mixed binary/source evidence, duplicate rows, scientific
gate failure, new-only output, and the platform-specific binary rule: local
tasks must use the preflight binary, while SCC tasks must all use the Linux
binary named by the exact-source build receipt.

## Exact-source SCC Linux smoke

The clean committed source produced a one-task smoke manifest at
`5a56df0aebf74f04e1b306cdb39e5192b69fecf7594b071932179f0142c8f212`.
It records an empty worktree diff, source-manifest SHA-256
`cb201a65b204f9cf84adeefeb2ea71b6f612a5545debd60aa66fde36c3ca1593`,
one `controls_varying_fixedoffset` task, two replications, and eight target
attempts. Its local preflight receipt is
`505d184a89feec918b16f240848d9ad69e96498d554fd8d87a64b74cd147bb94`;
the preflight payload hash is
`e655c20ca74f284cfaa2029d7048362359b7edc5fb89b3057629c60e95ee1a36`.

The exact Git archive SHA-256 is
`94bc3ee818cd2eb28c5ebcee069fe0c6008f67dca717534314fc2f2b5bc4fa56`.
It was validated and unpacked under:

```text
/projectnb/welfgr/vckss/runs/20260904T154057Z-match-q0-smoke-983ed37
```

The real launcher submitted the required one-core dependency chain:

| Stage | SGE job | Dependency | Slots | Limit | Memory/core | Result |
|---|---:|---:|---:|---:|---:|---|
| build | 7442122 | none | 1 | 30 min | 4 GiB | pass |
| task 1 | 7442123 | 7442122 | 1 | 30 min | 4 GiB | pass |
| aggregate | 7442124 | 7442123 | 1 | 15 min | 4 GiB | `COMPLETE` |

All three `qacct` records report queue `econ`, host `scc-gr4.scc.bu.edu`, one
slot, `failed=0`, and `exit_status=0`. Build/task/aggregate wall times were
35/1/1 seconds and CPU times were 40.416/1.471/0.543 seconds. Reported maximum
virtual memory was 1.727 GiB for the build and 106.949 MiB for the task. SGE
reported `0.000` for the one-second aggregate validator, so that value is not
used as a measured zero-memory claim. Frozen accounting hashes are:

```text
build       56fbcb39c622b67e7307e5c30125659af4b8aea3cf110f7e7dc99bc61b7b4eb1
task        5024fba33eaa1a80f2a9b4f8148c172dc71eabf240a21819ded3320188f88a78
aggregate   eb617f74a6f06e815976748be857e6ac21029e6ff6cc660e559a3ac85c2dd797
```

The SCC build used Rust/Cargo 1.85.1 and Python 3.13.8. Its Linux binary hash
is `8219958cb47443c61a129f62554d16c16eb03b33f384dea7f64a660acf246b76`;
the local macOS preflight binary hash is
`9319bbca46b4532bb14724fc035694100b2b3f9bd04fc029a2f01de1e2a2acfb`.
The difference is expected across platforms. The task and aggregate both
reconciled to the Linux build receipt, whose SHA-256 is
`e869ed6031e109bcc5a394080bb2a205b2ba8356b8cf60afc04de780d5a5ea9d`.

The task emitted exactly one payload and one receipt with eight rows. The
aggregate reproduced exactly those eight rows and emitted four target
summaries plus its header. The task/aggregate payload SHA-256 is
`c667b9f194b8bc5a50a3993a1d3569b235a3f2cbeb0d581c5cc51f74e7edfd2c`;
the summaries hash is
`56bad05db036f047196a53b5082e0438b26fe85cccab13f27cf491cc39ccab40`;
the final aggregate receipt hash is
`22c6d2a939eb9696b2e5c77dcdd5bd08896c5f5c0fe1c1e96a4892a3fa042310`.
It reports `pipeline_complete_non_evidentiary`, no process or scientific
failures, and no omitted attempt. Because this is the fixed-offset controls
diagnostic with only two replications, its coverage rows are explicitly
ineligible and no coverage or calibration conclusion is drawn.

The initial read-only quota probe used an invalid user-form `pquota`
invocation and stopped before deployment or submission. The corrected
project-form probe for `welfgr` passed, after which the single smoke chain was
submitted. No repair, resubmission, task expansion, or large array was needed.
No binary was copied into the repository or distributed.

## Boundary and handoff

This checkpoint verifies the complete harness and one representative clean-
source Linux compute path. It does not establish q=0 coverage, standard-error
calibration, or robustness to variance-model misspecification. It does not
account for uncertainty in `gamma_hat`; no delta method, influence function,
cross-fitting correction, or joint-nuisance procedure is present. It does not
cover eligible stayers, public match-inference routing, automatic q selection,
grouped q=1, general q greater than one, release, publication, push, or binary
distribution.

The next bounded step is the already frozen 14-cell, 400-replication-per-cell
development profile: 280 tasks and 22,400 classified target attempts. Its
output may diagnose the construction but must not be tuned, relabeled, or
treated as confirmation evidence. Grouped q=1 remains fail-closed until the
q=0 development result is complete, reviewed, and accepted.
