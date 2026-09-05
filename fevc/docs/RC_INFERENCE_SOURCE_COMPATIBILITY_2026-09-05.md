# RC inference source compatibility review — 2026-09-05

## Sources and permitted reuse

The accepted repaired match-q1 confirmation source is
`4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`. The current frozen observation
confirmation source is `73fa75805c8cef6d4d1a6ad843da5894ccedc956`.
The full match-q0 confirmation separately tests
`bb580fe69085d1f98c9151cea2de038aec0a8ba6`.
This review carries only unchanged scientific/implementation claims to
73fa758; it does not qualify a new public match interface or a new distributed
native binary.

The changed paths from the accepted q1 confirmation source to 73fa758 are:

```text
fevc/PLAN.md
fevc/docs/FIXED_OFFSET_DIAGNOSIS_2026-09-04.md
fevc/docs/FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md
fevc/docs/INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md
fevc/docs/README.md
fevc/docs/fixed_offset_diagnostic_v1.json
fevc/docs/fixed_offset_diagnostic_v1_result.json
fevc/docs/fixed_offset_paired_v1.json
fevc/docs/fixed_offset_paired_v1_result.json
fevc/docs/inference_repair_match_confirmation_v1_result.json
fevc/docs/rc_match_q0_v1.json
fevc/docs/rc_observation_inference_v1.json
fevc/tests/python/test_fixed_offset_diagnostic.py
fevc/tests/python/test_fixed_offset_paired.py
fevc/tests/python/test_rc_match_q0.py
fevc/tests/python/test_rc_observation_inference.py
fevc/tests/python/test_rc_qacct.py
fevc/tools/check_rc_qacct.py
fevc/tools/diagnose_fixed_offset.py
fevc/tools/run_fixed_offset_paired.py
fevc/tools/run_rc_match_q0.py
fevc/tools/run_rc_observation_inference.py
rust/crates/vckss-core/examples/fixed_offset_paired.rs
rust/crates/vckss-core/examples/rc_match_q0.rs
rust/crates/vckss-core/examples/rc_observation_inference.rs
rust/stata_backend/scc/deploy_rc_match_q0.sh
rust/stata_backend/scc/deploy_rc_observation_inference.sh
rust/stata_backend/scc/run_rc_match_q0_aggregate.sge
rust/stata_backend/scc/run_rc_match_q0_build.sge
rust/stata_backend/scc/run_rc_match_q0_task.sge
rust/stata_backend/scc/run_rc_observation_inference_aggregate.sge
rust/stata_backend/scc/run_rc_observation_inference_build.sge
rust/stata_backend/scc/run_rc_observation_inference_task.sge
rust/stata_backend/scc/submit_rc_match_q0.sh
rust/stata_backend/scc/submit_rc_observation_inference.sh
```

These paths add independent diagnostic/confirmation examples, their Python
validators, tests, source-bound SCC wrappers, registrations, results, and
planning documents. No production solver, estimator, component covariance,
q1 recenter, curvature, critical-value generator, RNG implementation, public
ado, native ABI, C shim, Cargo manifest/lockfile, native build script, or
qualification script changed. The new SCC example wrappers do not build or
alter a Stata plugin.

## Verified identities and gates

All 138 files in the native qualification source manifest from
`31dd37f2954c02d223ad81175dd4ded7b5840b8d` rehash unchanged at 73fa758.
The manifest SHA-256 is
`d5f9d246537c6b983507cd3dd1bf89bae2c11d629e9b616d63bef4a49cb33e5e`.
The exact check is:

```bash
shasum -a 256 -c rust/qualification/evidence/INFERENCE-REPAIR-MACOS/31dd37f2954c02d223ad81175dd4ded7b5840b8d/source-manifest.sha256
```

The repaired match-q1 campaign's ten frozen files also rehash unchanged.
Its registration SHA-256 remains
`9d32eabd8f07152f0b6bb64552ee33c2dd31ba6b355e329e3bd92bc4e1f469d8`.
The accepted input designs, semantic seeds, target exclusions, fold rules,
source bundle, binary, original acceptance thresholds and 140,000-attempt
result retain their exact archived identities in
[`inference_repair_match_confirmation_v1_result.json`](inference_repair_match_confirmation_v1_result.json).
No old receipt or scientific result was edited.

Proportional new-source checks include the complete Python suite (682 passed),
the current CMG assembler check, both new examples' release builds, six
independent observation-example oracle tests, tiny-pipeline schedule/shard
invariance, and the source-bound real-launcher SCC smokes. The complete match
q0 confirmation is a separate new scientific measurement, not a carry-forward.
The observation confirmation remains a separate gate and cannot be replaced
by this review.

The working additions after 73fa758 consist of new result and compatibility
records, PLAN/index updates, and the test-only correction below. They do not
change these production identities. Any later executable/native/public-boundary
change must be reviewed against its own exact source.

### Post-confirmation test-only correction

The first `./ci/run_rust_quick.sh` sweep passed format and strict Clippy, but
found three stale q1 assertions in `rc_match_q0.rs`'s `cfg(test)` module:
an old seed vector, a removed one-mode cell name in the fixed-fold check,
and q1's covariance exclusion in the q0 eligibility test. Those assertions
were copied when creating the new q0 example. They are not part of its release
executable. The correction changes only that test module: it checks the
independently implemented Python seed vectors, a real diffuse q0 cell's fixed
folds, and the original all-four-target q0 eligibility with diagnostic
exclusions.

`fevc/tests/python/test_rc_match_q0.py` now reconstructs and verifies all 15
registered files from the exact bb580fe commit in a temporary test tree. It
also checks that the current, unregistered test revision cannot create a new
confirmation manifest, and that removing the single `cfg(test)` module leaves
byte-identical source. That non-test source SHA-256 is
`504b135a7321e2b2f325ac380a53cb94c774cac5d9f1551a9ec32dd21ac78a7d`.
The archived registration, original source/binary, raw outputs, scientific
gates and decisions are unchanged. Existing output reaggregation does not
use the test module. Reproducing a new outcome run still requires the original
registered source bundle; no updated campaign is registered or submitted.

After this correction, the focused three Rust and 21 Python q0 tests pass,
the complete Python suite passes 684 tests in 74.28 seconds, and the complete
Rust workspace/all-target format, strict Clippy and test sweep passes. The
CMG assembler remains current. A focused pytest run also emitted cleanup
warnings for unrelated old pytest temporary directories; an isolated fresh
base directory avoided those warnings in the full run. No old temporary
directory was manually removed. No estimator or native change warrants a
new Stata/plugin or SCC qualification run for this test-only maintenance.

## Claims and limits

The repaired eligible match-q1 scientific result can be reused: all 18 primary
worker/firm/total rows passed, availability was 99.96--100%, coverage was
94.24--96.76%, and empirical-SD/mean-SE ratios were 0.9675--1.0297.
Its multi-mode covariance, estimated-controls, weak/null and severe-model
limitations remain unchanged. The current q0 claim comes from its new full
confirmation, not from the q1 campaign's single q0 comparator.

Existing exact-source macOS arm64/Rosetta qualification remains valid evidence
for the unchanged existing native/public boundary. It does not qualify
Windows, native Intel hardware, a newly built binary identity, a future match
interface, or large-data inference performance. A new match ABI/ado/return
surface requires focused Rust/C/Stata tests and exact-clean-source native and
installation qualification even if its numerical kernel is unchanged.
A statistical, RNG, solver, covariance, generator, input, or acceptance change
would invalidate the corresponding scientific carry-forward and require a
new prospective decision. No public release, tag, push or distribution is
implied.
