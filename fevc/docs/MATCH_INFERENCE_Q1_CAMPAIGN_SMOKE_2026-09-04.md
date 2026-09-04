# Fixed-offset collapsed-match q1 campaign smoke checkpoint

## Decision and scope

The separately registered q1 generator-to-validator pipeline and its
exact-source SCC Linux smoke pass. The tested implementation source is
`0694e4c3fda624f289e30ef9da7842f1363bd844`; the frozen registration is
[`match_inference_q1_campaign_v1.json`](match_inference_q1_campaign_v1.json),
SHA-256
`c12047cd6c40702f2d86b18c1ee18cdc118014a8286502f655b88e124c046127`.
This is pipeline and representative-compute-node evidence only. It authorizes
the already registered bounded q1 development profile, but it is not coverage
evidence, independent confirmation, or public promotion.

The implementation commit changed only these paths:

```text
fevc/CHANGELOG.md
fevc/PLAN.md
fevc/docs/README.md
fevc/docs/match_inference_q1_campaign_v1.json
fevc/tests/python/test_match_inference_q1_campaign.py
fevc/tools/run_match_inference_q1_campaign.py
rust/crates/vckss-core/examples/match_inference_q1_development.rs
rust/stata_backend/scc/deploy_match_inference_q1_campaign.sh
rust/stata_backend/scc/run_match_inference_q1_aggregate.sge
rust/stata_backend/scc/run_match_inference_q1_build.sge
rust/stata_backend/scc/run_match_inference_q1_task.sge
rust/stata_backend/scc/submit_match_inference_q1_campaign.sh
```

No public parser, Stata return, plugin ABI, production estimator, component
point path, solver implementation, or automatic q route changed. The campaign
calls the existing internal grouped q0/q1 attachment. Inference remains
conditional on the estimated full-sample fixed nuisance-control offset,
permits unrestricted dependence within a declared match only through the
variance of its collapsed scalar aggregate, assumes independence across
declared matches, and uses a named structured model for aggregate-match
variances. It is neither joint-nuisance nor unrestricted-KSS inference.

## Frozen campaign construction

The source-bound Rust generator covers 14 registered cells and four component
targets. It includes a diffuse q0 comparator; target-specific one-mode q1
worker, firm, and total cases; a deliberately multi-mode covariance target;
weak/null signal; equal and highly unequal match mass; four within-match
dependence constructions; correct, mild, and severe structured-model cases;
both supported internal solver routes; and controls varying within matches
before fixed-offset removal.

Outcome-free preflight uses 4,096 trace probes to certify the intended geometry
before manifest creation. Production tasks retain the registered 128-probe
diagnostic and do not turn concentration into an automatic cutoff. The q1
path uses the raw whole-match variance product for leading-square recentering,
the structured fit only for covariance and studentization, and 4,000
Counter-V1 critical-value simulations per target-replication. The covariance
target is never included in the one-mode q1 coverage claim. Counter semantic
seeds, target eligibility, task keys, source and file hashes, expected rows,
thresholds, and complete attempt inventory are frozen.

## Local pipeline and source gates

The complete tiny profile ran from the clean committed source and produced all
56 expected target rows. Its source-manifest SHA-256 is
`1da4fe81f7cdc341904a0db3e674b875a7b0f6bf05967893dda671f5805387e8`;
its manifest SHA-256 is
`a636b7b7ee5c9d6966f34957690e90b71ee90f0715d6ea9bd644e8b59708e4cf`.
The local macOS binary hash is
`2fd54af00e61cb5ca51a452d76abf643070f37ed57d1f2c568a4a1a0f1e2dbb7`.
The preflight receipt and payload hashes are respectively
`d02e99d3a04f40d60259d4e75fe89247840dc13931656821bf6ed48e36edf8c1`
and
`045a7d3caf845d94dd92ddf47981934a4fa3467671bd1544f451b049fe6c0dfc`.

The aggregate receipt status is `COMPLETE`, with no scientific gate failures.
There were 48 successful rows. All eight weak/null rows were retained and
classified: the four weak-signal targets failed closed with
`JLA_CONSTRAINT_FAILED:component_inference_psd`, and the four null-signal
targets failed closed with
`JLA_CONSTRAINT_FAILED:component_inference_q1`. No attempt was silently
dropped. The aggregate payload, summaries, and final receipt hashes are:

```text
aggregate  b6d3aab8ab9e3498b16e8b328a0c3747477bd48150d6df8406859c9fbbdedb51
summaries  d52c0016bb3dafda4c3097778a850179964d4bb8dab7866a99b3acb6c57deb4a
receipt    e2e34af0d7b11ce41a97d214fb74ec6f0f051c81e3f6a47af2e0662f982f42a1
```

An initial local task loop supplied named flags to positional CLI arguments.
The parser rejected all 14 commands before any task output was created. The
same frozen manifest and binary then completed through the documented
positional form; no source, registration, seed, threshold, or task changed.

The implementation passed:

```text
./.venv/bin/python -m pytest -q fevc/tests/python/test_match_inference_q1_campaign.py
    20 passed; only known sandbox cleanup warnings
PATH=<pinned Rust 1.85.1>/bin:$PATH cargo test --manifest-path rust/Cargo.toml -p vckss-core --example match_inference_q1_development --locked
    2 passed
./.venv/bin/python -m pytest -q
    559 passed; only known sandbox cleanup warnings
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
bash -n rust/stata_backend/scc/deploy_match_inference_q1_campaign.sh rust/stata_backend/scc/run_match_inference_q1_build.sge rust/stata_backend/scc/run_match_inference_q1_task.sge rust/stata_backend/scc/run_match_inference_q1_aggregate.sge rust/stata_backend/scc/submit_match_inference_q1_campaign.sh
    passed
git diff --check
    passed
```

The focused adversarial suite also rejects bad target-specific preflight
geometry, tampered preflight, malformed/duplicate/partial/missing inventories,
overlapping task ranges, invalid q1 covariance, mixed source or binary
receipts, dirty smoke manifests, and attempts to coverage-gate the multi-mode
covariance target. It verifies schedule-invariant seeds, explicit q0/q1
selection, atomic writes, and complete typed failure accounting.

`fevc/tools/run_checks.py` and a fresh native/plugin qualification were not
rerun. This slice adds only internal example, campaign, validator, and SCC
tooling and changes no Stata, plugin ABI, build-system, production estimator,
solver, RNG primitive, or existing public route. The immediately preceding
exact-source q1 local foundation already passed the integrated licensed-Stata
gate, so these external gates would not qualify an affected surface here.

## Exact-source SCC Linux smoke

The clean committed source produced a one-task smoke manifest at
`2ce0cbc1d70b61b0b49f5d72513b12a130a4b7baabd141781aab61c657b7b232`.
It records the same source manifest and an empty worktree diff, one
`controls_varying_fixedoffset` task, two replications, and eight target
attempts. Its local preflight receipt is
`9c53d8d36aaa28d54e9222f6dcef06e357a08f5fbcd14de661318333ad0e034c`;
the preflight payload hash is
`0c5eeae25c0aea71b284a52e0b8189bc73127bca22874e44a3b6af9969e76144`.

The exact source-bundle SHA-256 is
`635a8f9619bbb119e9d39259e41b6aa86b3c5ec18529e3cea7cdfb80651f0996`.
It was validated, unpacked, and made read-only under:

```text
/projectnb/welfgr/vckss/runs/20260904T184014Z-q1-smoke
```

The real launcher submitted the required one-core dependency chain:

| Stage | SGE job | Dependency | Host | Slots | Limit | Memory/core | Result |
|---|---:|---:|---|---:|---:|---:|---|
| build | 7445238 | none | scc-gd4 | 1 | 30 min | 4 GiB | pass |
| task 1 | 7445239 | 7445238 | scc-gr4 | 1 | 30 min | 4 GiB | pass |
| aggregate | 7445240 | 7445239 | scc-gd4 | 1 | 15 min | 4 GiB | `COMPLETE` |

All three `qacct` records report queue `econ`, one slot, `failed=0`, and
`exit_status=0`. Build/task/aggregate wall times were 42/2/1 seconds and CPU
times were 43.280/1.713/0.610 seconds. Maximum virtual memory was 2.735 GiB,
110.285 MiB, and 84.941 MiB respectively. Frozen accounting hashes are:

```text
build       5fa0c9c780c840a607560de6e55230641d6578e5888a57c2208f7f56d25157d1
task        68f475127061f28172e45ac78f21e6a83ad09a4ed059d44fcf2ff87dd88d876b
aggregate   447c794c18e7faa4cfe19a5415b81a309333a1d2425a93ec6e9372153bece222
```

The SCC build used Rust/Cargo 1.85.1 and Python 3.13.8. Its Linux binary hash
is `53014ca1b59badc9c8799ac57f290f4f2059b59e72c05c43bed4f7b4971e7739`;
the platform difference from the local binary is expected. The task and
aggregate both reconcile to build receipt
`d7d0b3f1f754bef7712d7514d97e533aac2d5b6bbf290ca51d814b50205b5413`.

The task emitted exactly one payload and one receipt with eight successful
rows. The aggregate reproduced exactly those rows and emitted four target
summaries plus its header. Every row reports that nuisance uncertainty is
conditioned away, and every row is coverage-ineligible because this is a
two-replication controls diagnostic. The maximum q1 point-identity error was
`5.551115123125783e-17`; the maximum direct rank-one remainder-identity error
was `3.073652443674746e-13`. Exact hashes are:

```text
task/aggregate payload  295f4333d7882991d2808b4eec88c17d01992b6212f84495388e3c452826391c
task receipt            bf27991d16ac5d2cce4447e04e106a6839169c36338a9f0fc8d76d95c237234d
summaries               78103579f9db7dd3f6c56fbfb8bb833be9b7c9c58048dd916c3e9a5b568c22a6
aggregate receipt       a8894f8479902720d7ac4f24b336f349e9063e1762934fd6b9e479fb133bdd37
submission ledger       e6831dcdeb50ebf259415ceb8aa5bc1849339588fd302c6584cce94f403d2ba1
```

The aggregate decision is `pipeline_complete_non_evidentiary`, with no
process or scientific failures and no omitted attempt. Scheduler accounting
appeared several minutes after each completed log marker; the dependency
chain was left intact and no job was repaired, resubmitted, or expanded. No
binary was copied into the repository or distributed.

## Boundary and handoff

This checkpoint verifies the complete q1 harness and one representative clean-
source Linux compute path. It does not establish q1 coverage, standard-error
calibration, or robustness to structured aggregate-variance misspecification.
It does not account for uncertainty in the full-sample nuisance-control
estimate. It does not authorize a q1 claim for the multi-mode covariance
target, eligible stayers, dependence across different matches, public routing,
automatic q selection, q greater than one, confirmation, release, publication,
push, or binary distribution.

The next bounded step is the already frozen 14-cell,
400-replication-per-cell q1 development profile: 280 tasks and 22,400
classified target attempts. Its output may diagnose the construction but must
not be tuned, relabeled, or treated as confirmation evidence. Public match
inference remains fail-closed until a later independent confirmation and
separate promotion decision.
