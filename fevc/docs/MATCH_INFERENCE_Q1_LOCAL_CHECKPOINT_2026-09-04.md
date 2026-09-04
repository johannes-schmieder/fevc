# Fixed-offset collapsed-match q=1 local checkpoint

## Source identity and scope

The accepted grouped q0 prerequisite is
`817f6b3ea18639b6e269d9bfcc97296777827af7`. The q1 construction was frozen
before implementation in
[`match_inference_q1_development_v1.json`](match_inference_q1_development_v1.json)
at `44bd437a3958a91120a7e15eaf1565952adfedae`. While constructing the
independent dense covariance oracle, before any successful grouped q1 result,
the population-versus-realized influence notation was corrected by the
immutable pre-result
[`match_inference_q1_development_v1_amendment1.json`](match_inference_q1_development_v1_amendment1.json)
at `4b572de1e63f76409756a7ee3d864f4b6cccbbc8`.

The internal implementation source is
`0b2a2d90ed6b2757b6f79c0868bcd3e5a713ca3e`. This checkpoint establishes the
smallest local grouped q1 foundation only. It creates no parser option, public
capability, plugin ABI field, Stata return, automatic route, concentration
cutoff, coverage claim, or authority to run a q1 campaign.

The implementation commit changed:

```text
fevc/CHANGELOG.md
fevc/PLAN.md
fevc/docs/FAILURES_AND_RETURNS.md
fevc/docs/INFERENCE.md
fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md
rust/README.md
rust/TEST_PLAN.md
rust/crates/vckss-core/src/component_inference.rs
rust/crates/vckss-core/src/generic_jla.rs
rust/crates/vckss-core/tests/generic_jla.rs
rust/crates/vckss-core/tests/grouped_component_inference.rs
```

The statistical estimator, q0 attachment, aggregate-match variance fits,
observation q1 recentering and covariance, Andrews--Mikusheva critical-radius
and ellipse-image code, solvers, public Counter-V1 address domains, build
system, plugin protocol, and Stata surface did not change. The grouped
internal q1 route now consumes its registered critical-value Counter domain;
its accounting is tested for batch invariance.

## Construction established locally

Conditional on the full-sample fixed control offset, the inferential rows are
the declared match aggregates

```text
y_g_c = sum_i f_i y_i_star / sqrt(F_g),
x_g_c = sqrt(F_g) x_g.
```

For each reported target, the implementation extracts one leading grouped
generalized mode, recenters its realized square with the raw whole-match
leave-out product, subtracts the corresponding rank-one target kernel, and
forms the remainder directly. The positive structured aggregate-variance fit
is used only for covariance and studentization. The q1 point decomposition is
checked against the unchanged match-deletion component point estimate, and
the existing Andrews--Mikusheva/KSS ellipse-image construction is reused
without modification.

The independent frequency-expanded physical-block and separately accumulated
collapsed-scalar dense oracles agree for all four reported targets on:

- the leading eigenvalue, normalized mode, and realized score;
- the raw whole-match leading recenter, including within-match cross-products;
- the rank-one-subtracted target and direct remainder leave-out kernels;
- compression of the physical remainder kernel to the scalar match kernel;
- the q1 point decomposition; and
- population leading/remainder covariance under independent, common-shock,
  serial, and equal-aggregate-variance/different-shape within-match covariance
  blocks.

The fixture includes unequal match sizes, positive nonunit frequency weights,
heterogeneous target mass, arbitrary within-match outcomes, controls varying
within matches before fixed-offset removal, and distinct deletion IDs at one
worker--firm coordinate. A separate assertion shows that an incorrect
observationwise recenter differs from the whole-match contraction.

The internal diagonal and CMG paths retain the match count and mass,
leverage, maker, influence, leading and remainder spectrum, maximum mode
weight, remainder influence, structured-model support/boundary/floor,
sensitivity, solver residual, numerical Monte Carlo, covariance PSD, and
remainder-identity diagnostics. They set
`nuisance_uncertainty_conditioned_away=true`. Null signal, wrong nuisance mode,
cross-coordinate deletion IDs, and delete-match identification failures remain
typed and atomic.

## Validation record

The prospective implementation content passed:

- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test grouped_component_inference physical_block_and_collapsed_scalar_q1_oracles_agree -- --nocapture` — 1 passed.
- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test grouped_component_inference` — 3 passed.
- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test generic_jla internal_fixedoffset_match_q1 -- --nocapture` — 3 passed.
- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test generic_jla grouped_wrong_nuisance_fails_before_inference_rng -- --nocapture` — 1 passed.
- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test generic_jla grouped_inference_preserves_cross_coordinate_and_delete_match_failures -- --nocapture` — 1 passed.
- `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test generic_jla` — 41 passed.
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` — passed with
  pinned Rust 1.85.1.
- `cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked` — passed, including 199 core unit tests, 41 generic-JLA tests, and all 3 grouped-oracle tests.
- `cargo test --manifest-path rust/stata_backend/Cargo.toml --all-targets --locked` — passed (6 unit and 5 build-boundary tests).
- `./.venv/bin/python -m pytest -q` — 542 passed. Pytest emitted cleanup
  warnings when removing temporary deliberately rejected symlink fixtures;
  there was no test failure.
- `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check` — passed.
- `./.venv/bin/python fevc/tools/run_checks.py` — the identity, history,
  license/provenance, parity, artifact, generated-CMG, Python, licensed-Stata
  quick/full, clean-install, benchmark, and separation checks passed, ending
  `FEVC LOCAL QUALIFICATION PASS`.
- `git diff --check` — passed.

After committing the implementation, its exact SHA reran the focused dense
q1 oracle (1 passed), the three internal q1 attachment tests (3 passed), and
Rust formatting (passed). The broader gates above tested byte-identical source
content immediately before commit.

No SCC task or campaign was launched. No native plugin-build qualification was
repeated: the change is unreachable from the plugin and Stata interfaces,
changes neither ABI nor build configuration, and the full Rust workspace,
standalone Stata-backend boundary suite, and licensed Stata source suite cover
the unchanged public surface. No binary was distributed.

## Claim and limitations

This checkpoint establishes an internal numerical foundation for suggestive
match-cluster uncertainty conditional on the estimated full-sample fixed
nuisance-control offset. It allows unrestricted dependence within each
declared match only through the scalar aggregate-match variance and assumes
independence across declared matches, including matches belonging to the same
worker. The structured-common and leverage-only fits are models for those
aggregate variances; omitted variance drivers can invalidate the covariance.
Cross-fitting is regularization, not independent nuisance estimation.

There is no delta-method, influence-function, cross-fitted, joint-nuisance, or
other correction for uncertainty in the estimated control coefficients. The
slice remains mover-only and internal. It does not establish q1 coverage,
representative scale, Linux/SCC execution, confirmation, eligible-stayer or
mixed-population support, dependence across matches, public routing,
automatic q selection, q greater than one, release, publication, push, or
binary distribution. Numerical completion does not place a target in the
one-mode regime; leading and remainder diagnostics remain target-specific,
and deliberately multi-mode targets remain outside any future coverage claim.

The next bounded slice is separate q1 campaign registration followed by the
complete tiny generator-to-validator-to-receipt path, including deliberate
failure inputs. Only after that local pipeline passes should one representative
compute-node task run through the real SCC launcher. A larger development
campaign must wait for both smokes and must preserve a frozen generator,
manifest, seeds, estimands, regime checks, thresholds, attempt accounting, and
expected inventory.
