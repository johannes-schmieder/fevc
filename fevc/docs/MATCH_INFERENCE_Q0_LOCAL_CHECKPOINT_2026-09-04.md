# Fixed-offset collapsed-match q=0 local checkpoint

## Source identity and scope

The internal foundation source is
`77177a6497891d8f6e1cab0aca89366f4e4ca4ad`. Its registered development
contract is [`match_inference_q0_development_v1.json`](match_inference_q0_development_v1.json).
This checkpoint implements and validates the local algebraic and numerical
foundation only. It does not create a public parser option, plugin request,
Stata return, coverage claim, or permission to begin grouped `q=1`.

The changed paths in the implementation commit are:

```text
fevc/CHANGELOG.md
fevc/PLAN.md
fevc/docs/DECISIONS.md
fevc/docs/FAILURES_AND_RETURNS.md
fevc/docs/INFERENCE.md
fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md
fevc/docs/README.md
fevc/docs/match_inference_q0_development_v1.json
rust/README.md
rust/TEST_PLAN.md
rust/crates/vckss-core/src/component_inference.rs
rust/crates/vckss-core/src/generic_jla.rs
rust/crates/vckss-core/src/structured_variance.rs
rust/crates/vckss-core/tests/generic_jla.rs
rust/crates/vckss-core/tests/grouped_component_inference.rs
```

## Construction established locally

Under `nuisance(fixedoffset)`, FEVC estimates the full model once, forms
`y_i_star=y_i-z_i'gamma_hat`, and holds that estimated offset fixed. Because
the FE row is constant within a declared match, the original whole-match
block calculation reduces exactly to one row with

```text
F_g   = sum_i f_i
y_g_c = sum_i f_i y_i_star / sqrt(F_g)
x_g_c = sqrt(F_g) x_g.
```

Independent original-physical-row and collapsed-scalar dense oracles agree on
the FE fit, primitive component plug-ins, whole-match deleted fits, adjusted
deleted-residual contractions, the KSS point correction, the three
zero-block-diagonal kernels, and arbitrary matrix-free kernel actions. A
second oracle test verifies Gaussian covariance equality for diagonal,
common-shock, serial-correlation, and differently shaped covariance blocks,
including blocks scaled to the same aggregate variance. Only
`tau_g_sq=f_g'Gamma_g f_g/F_g` enters the collapsed covariance.

The internal generic-JLA attachment uses one Counter-V1 Gaussian draw per
declared match, not per unit of regression mass. It supports positive integer
frequency mass and separately retained target mass, mover-only match deletion,
the existing diagonal and CMG solver routes, aggregate-match oracle variance,
and the registered primary and leverage-only structured variance fits. It
reports independent/effective match counts, match-mass concentration,
leverage and maker diagnostics, target-specific influence and spectrum
diagnostics, structured-fit support/boundary/floor diagnostics, sensitivity,
solver receipts, trace MCSE, covariance PSD, and the explicit flag
`nuisance_uncertainty_conditioned_away=true`.

The attached covariance does not alter the existing component point estimate.
Cross-coordinate deletion identifiers, delete-match identification failure,
weak/null covariance failure, cancellation, and memory inadmissibility remain
typed and atomic. Grouped `q=1`, hybrid stayers, projection composition,
automatic routing, and any public invocation fail before inference draws.

## Validation record

The prospective implementation source passed these local gates before commit:

- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test grouped_component_inference --locked` — 2 passed.
- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test generic_jla --locked` — 38 passed.
- `./.venv/bin/python -m pytest -q` — 524 passed; pytest emitted only sandbox
  cleanup warnings for temporary symlinks.
- `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check` — passed.
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` — passed with
  the pinned Rust 1.85.1 toolchain on `PATH`.
- `cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked` — passed, including 199 core unit tests, 38 generic-JLA integration tests, and both independent grouped-oracle tests.
- `cargo test --manifest-path rust/stata_backend/Cargo.toml --all-targets --locked` — passed (6 unit and 5 build-boundary tests).
- `./.venv/bin/python fevc/tools/run_checks.py --help` — this script does not
  implement a help-only mode, so it ran the complete integrated identity,
  history, license/provenance, parity, artifact, CMG, Python, licensed-Stata
  quick/full, clean-install, benchmark, and separation suite and ended
  `FEVC LOCAL QUALIFICATION PASS`.
- `git diff --check` — passed.

After commit, `./ci/run_ci_profile.sh plugin-build` was bound to the exact
source SHA. The first restricted invocation stopped before building because it
could not resolve `www.stata.com`. The clean, unchanged SHA was rerun with
approved network access and passed. The receipt records:

- classification `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`;
- native arm64 `PASS_NATIVE` and x86-64 `PASS_ROSETTA`;
- licensed Stata/MP 19 lifecycle, routing, public component-inference, thin,
  universal, and clean-install checks;
- Rust format, strict Clippy, tests, C-shim interrupt/error transport, and ABI
  header compatibility passes; and
- source-manifest SHA-256
  `5ec741104e4389579c3fabf6199eaa5717ea9703e7994669cb01ffebe3692f13`.

The sanitized packet is under
`../../rust/qualification/evidence/MATCH-FIXEDOFFSET-Q0-MACOS/77177a6497891d8f6e1cab0aca89366f4e4ca4ad/`.
It contains candidate hashes but no binary or raw Stata log. The exact-source
Stata profile necessarily checks only the existing public plugin surface; the
new grouped route remains reachable only from source-bound Rust tests.

## Claim and limitations

This checkpoint establishes the local implementation foundation for
suggestive sampling uncertainty conditional on the estimated full-sample
control offset. It assumes independent declared matches and permits arbitrary
within-match covariance only through each match's scalar aggregate variance.
The primary and sensitivity regressions are structured models for those
aggregate variances; omitted variance drivers can invalidate the covariance.
Cross-fitting is regularization, not independent nuisance estimation.

No delta-method, influence-function, cross-fitted, or joint-nuisance
correction for `gamma_hat` is present. The checkpoint does not qualify
coverage, misspecification behavior, SCC/Linux execution, representative
scale, native Intel, Windows, eligible stayers, grouped `q=1`, public routing,
automatic selection, release, publication, push, or binary distribution.

The next bounded stage is to implement the registered campaign's complete
generator-to-validator-to-receipt path, pass it locally including deliberate
failure inputs, and then pass one representative SCC compute-node task through
the real launcher. Only after those smokes may the moderate-dimension grouped
`q=0` development campaign run. Grouped `q=1` remains staged until that q=0
evidence is accepted.
