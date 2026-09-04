# Structured observation-inference promotion compatibility review

## Source identity and decision

The preregistered V5 confirmation tested source
`8e3596a40d999c4f9f55b19a0af81a1d5dee9961`. The separate public-boundary
promotion source is
`7608942a09c643fcb78fb52885d3b87333c5429f`.

This review carries the V5 scientific findings to the promotion source. The
promotion makes structured observation-deletion `q=0` and eligible one-mode
`q=1` supported explicit capabilities on their already confirmed tuple. It
does not broaden the tuple, change a statistical calculation, select
inference or `q` automatically, or alter the exact Mata comparator.

The paths changed between the two sources are:

```text
fevc/CHANGELOG.md
fevc/PLAN.md
fevc/README.md
fevc/_fevc_display.ado
fevc/_fevc_rust_component_post.ado
fevc/docs/DECISIONS.md
fevc/docs/ESTIMATOR_CONTRACT.md
fevc/docs/FAILURES_AND_RETURNS.md
fevc/docs/INFERENCE.md
fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md
fevc/docs/README.md
fevc/docs/RUST_MATA_PARITY.md
fevc/docs/rust_mata_parity.json
fevc/docs/structured_inference_confirmation_v5_result.json
fevc/fevc.pkg
fevc/fevc.sthlp
fevc/fevc_estat.ado
fevc/stata.toc
fevc/tests/python/test_package_layout.py
fevc/tests/stata/test_rust_component_inference.do
fevc/tests/stata/test_rust_public_install.do
rust/README.md
rust/RNG_CONTRACT.md
rust/TEST_PLAN.md
rust/crates/vckss-plugin/src/ffi_engine.rs
```

`structured_inference_confirmation_v5_result.json` was added by the intervening
immutable V5 evidence commit `215fa35ee337824f2aa0ae0a7b7b463e97e62b3a`.
The promotion did not edit it. The sole Rust production-source change is a
documentation comment in `ffi_engine.rs`; it changes no executable code.

## Changed-surface audit

| Surface | Changed? | Review |
|---|---:|---|
| Component point estimator | No | Posting tests require bitwise-identical `e(results)` and `e(b)` with and without the inference attachment. |
| Structured covariance | No | No covariance implementation or fitted-variance code changed. |
| `q=1` recentering | No | The raw leave-out leading-square recenter and direct remainder identity are unchanged. |
| Studentization, critical radius, ellipse image | No | No reference-law or confidence-set code changed. |
| Counter-V1 or semantic RNG keys | No | Only the active RNG documentation changed its lifecycle label. |
| Solver or numerical method | No | Solver, tolerance, residual, eigen, and memory code are unchanged. |
| Native ABI semantics | No | No ABI structure, version, request, result, or capability mask changed. Versioned V1 augmentation and V3 result receipts still reconcile in Stata and native tests. |
| Build system or native packaging | No | Cargo manifests, build scripts, C shim, headers, and qualifier are unchanged. |
| Native executable source | No | The only Rust source diff is a doc comment. Fresh binaries were nevertheless rebuilt and qualified from the promotion SHA; no V5 binary-artifact identity is reused. |
| V5 DGPs, inputs, manifest, estimands, or exclusions | No | The registration, task manifest, campaign inputs, and immutable result are unchanged. |
| V5 acceptance thresholds | No | No scientific or execution gate was weakened, relabeled, or rerun. |
| Public interface and lifecycle | Yes | Support metadata, requested/selected reconciliation fields, display, `estat diagnostics`, help, package descriptions, parity ledger, and focused tests now expose the supported explicit boundary. |

The parser continues to accept only the existing
`inferencemodel(structured_common|structured_leverage)` names. The unchanged
capability router requires Rust generic JLA, Counter-V1, observation deletion,
mover-only population, unit frequency, joint nuisance handling, supported
low-dimensional controls, an explicit structured model, an explicit `q=0` or
`q=1` request, and an eligible diagonal or CMG solver. Match deletion,
eligible stayers, general frequency weights, within-match dependence, and
general `q>1` still fail closed. No unrestricted-KSS alias was added.

## Validation record

The promotion source passed these affected-surface gates before commit:

- `./.venv/bin/python -m pytest -q fevc/tests/python/test_package_layout.py fevc/tests/python/test_rust_mata_parity.py` — 29 passed.
- `/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do fevc/tests/stata/test_rust_component_inference.do <repository>/fevc` — `PASS test_rust_component_inference.do`.
- `./.venv/bin/python -m pytest -q` — 524 passed.
- `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check` — passed.
- `./ci/run_rust_quick.sh` — pinned Rust 1.85.1 formatting, strict workspace Clippy with `-D warnings`, and workspace all-target tests passed.
- `./.venv/bin/python fevc/tools/run_checks.py` — all identity, history, license/provenance, parity, deterministic artifact, CMG, Python, licensed Stata quick/full, clean-install, benchmark, and separation gates passed, ending `FEVC LOCAL QUALIFICATION PASS`.
- `git diff --check` — passed.

After commit, `./ci/run_ci_profile.sh plugin-build` rebuilt and tested the
exact promotion source. Its first invocation could not resolve `www.stata.com`
in the restricted network environment and stopped before a build or Stata
return code. The same unmodified clean SHA was rerun with approved network
access and passed. The source-bound receipt records:

- classification `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`;
- native arm64 `PASS_NATIVE` and x86-64 `PASS_ROSETTA`;
- licensed Stata/MP 19 component-inference, routing, lifecycle, clean-install,
  thin, and universal-binary checks;
- C-shim interrupt/error transport and ABI-header compatibility passes;
- Rust format, strict Clippy, and test passes; and
- source-manifest SHA-256
  `66e9458605d43a3a61e8a4aa40c73cbddf4b17a07d10aaaa2ef57814bdeb4a5c`.

The sanitized exact-source packet is under
`rust/qualification/evidence/STRUCTURED-OBSERVATION-INFERENCE-MACOS/7608942a09c643fcb78fb52885d3b87333c5429f/`.
It records candidate hashes but contains no binary or raw Stata log.

## V5 claims carried forward

The following V5 findings apply unchanged to the promotion source:

- all 24 primary correct-model `q=0` rows passed the registered coverage and
  standard-error calibration gates;
- all 15 eligible primary one-mode `q=1` worker, firm, and total rows passed;
- mild misspecification stayed within its frozen degradation bounds;
- severe omitted variance drivers demonstrated invalid inference, including
  the recorded total-target coverage failures, so the structured-model
  warning remains a substantive boundary;
- weak/null designs produced the intended typed covariance or `q=1` failures;
- the deliberately multi-mode worker--firm covariance target remains outside
  the `q=1` coverage claim despite atomic execution testing; and
- component point estimates remain the registered estimator and are
  unaffected by the selected structured variance model.

No new SCC campaign is warranted because every statistical, RNG, numerical,
build, DGP, and acceptance identity relevant to those findings is unchanged.
The focused source gates and fresh exact-SHA native qualification cover the
changed public, return, display, package, and lifecycle surfaces.

## Limitations and next boundary

This review does not carry or create a claim for match deletion, stayers,
nonunit frequency weights, within-match dependence, unrestricted-
heteroskedastic KSS covariance, general `q>1`, automatic `q` selection,
Windows, native Intel hardware, representative scale or performance, public
release, binary distribution, or exact finite-sample pointwise size. The
Rosetta result is x86-64 emulation on Apple Silicon, not native Intel testing.

The next scientific slice is the internal grouped match-deletion `q=0`
foundation. It must begin from a separately specified grouped covariance
model and independent dense block-maker oracle; this review does not authorize
reusing the observation-deletion covariance formula or exposing a match-
deletion inference option.
