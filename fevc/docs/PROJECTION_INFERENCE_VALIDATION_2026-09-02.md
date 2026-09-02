# Projection-inference repair validation — 2026-09-02

## Source and scope

The final implementation is a focused local commit based on Git
`c2109aa3fc6be6c7588aae7b9ddc8a16da5a7425`. The exact final commit is recorded
by the source-bound plugin receipt and task handoff. The maintained MATLAB
comparator was the owner-supplied audited `LeaveOutTwoWay` source at `8b957ffe`; its
`codes/lincom_KSS.m` SHA-256 was
`71fb47ce35d26c91dbf97926031359ed0eb31d9916b07c167c577867620ee5a9`.
No comparator source or row-level data was copied into FEVC.

## Estimand

For an independent deletion block (g), the fit excluding (g) is independent
of its disturbance, so

\[
E[y_g\widehat e_{g,-g}'\mid X]=\Sigma_g.
\]

The real covariance contribution is estimated by

\[
\widehat\Sigma_g={1\over2}
 (y_g\widehat e_{g,-g}'+\widehat e_{g,-g}y_g').
\]

Thus, with (H=X'WX) and fixed-effect loading (L), FEVC computes

\[
L'H^{-1}\left(\sum_g X_g'\widehat\Sigma_gX_g\right)H^{-1}L.
\]

The stored-row implementation forms frequency-weighted block scores. Mover
matches permit unrestricted within-block covariance. Each eligible-stayer
physical copy remains an observation block. A deletion ID spanning more than
one worker-firm coordinate is rejected. Projection slopes are invariant to
equivalent two-way location shifts; the automatic intercept is explicitly
normalization-dependent under last-retained-firm-zero grounding.

## Independent evidence

- The seven-edge referee reconstruction gives true slope variance `2/3`,
  expectation `53/84` for the removed centered proxy, bias `-1/28`, and
  expectation `2/3` for the corrected uncentered estimator.
- The independent dense oracle covers exact observation and match deletion,
  positive frequency compression versus literal expansion, joint controls,
  eligible stayers, and normalization shifts.
- A 3,000-replication block experiment used seed `20260902`, 400 independent
  two-row matches, within-match correlation `0.45`, and heteroskedastic standard
  deviations. The true covariance was
  `[[.0104225771,-.0009071942],[-.0009071942,.0003222784]]`; the mean estimate
  was `[[.0103236184,-.0008950485],[-.0008950485,.0003193073]]`. Entrywise
  relative errors were between `0.92%` and `1.34%`. The positive-slope-variance
  rate was `0.999667` (failure rate `0.000333`), and nominal 95% interval
  coverage was `0.9400` with binomial MCSE `0.00434`.
- The mixed positive-frequency, joint-control, eligible-stayer Stata fixture
  compares complete Rust/JLA covariance and interval matrices to exact Mata at
  Counter-V1 seeds `20260901` through `20260908`, probe counts `600` and
  `1200`, batch `8`, and solver tolerance `1e-12`. Coefficients use the
  registered deterministic floor. For every covariance cell, standard error,
  and confidence endpoint, the repeated-seed mean must lie within the current
  `1e-8*max(1,abs(a),abs(b))` floor plus six empirical numerical MCSEs of the
  exact result. Seven of eight 600-probe streams and all eight 1200-probe
  streams passed; the remaining 600-probe stream was withheld by the typed
  projection-covariance PSD gate. Every accepted fixed-effect solve also
  passes the complete-system residual gate.

## MATLAB diagnostic

MATLAB R2024b Update 5 (`24.2.0.2863752`) ran a seven-row same-input
observation-deletion projection through maintained `lincom_KSS`. All seven rows
were retained. The direct FE coefficient and MATLAB coefficient were
`0.14999999999999986` and `0.14999999999999969`. MATLAB's isolated legacy
centered slope covariance was `0.07931547619047617`; the independent uncentered
calculation on that shifted-outcome fixture was `0.4114583333333332`. This is
an intentional covariance difference, not a coefficient or sample mismatch.
Default match population/deletion identity is additionally exercised by the
mixed Stata fixture; MATLAB source remains a behavioral comparator rather than
the covariance oracle.

## Commands and toolchains

The local toolchains were Python `3.13.0`, Stata/MP `18.0`, MATLAB R2024b
Update 5, and pinned Rust `1.85.1` (`rustc 1.85.1 (4eb161250 2025-03-15)`).
The executed gates were:

```text
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
PATH=<rust-1.85.1>/bin:$PATH cargo fmt --manifest-path rust/Cargo.toml --all -- --check
PATH=<rust-1.85.1>/bin:$PATH cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
PATH=<rust-1.85.1>/bin:$PATH cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
PATH=<rust-1.85.1>/bin:$PATH cargo test --manifest-path rust/stata_backend/Cargo.toml --all-targets --locked
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do fevc/tests/stata/test_inference.do fevc
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do fevc/tests/stata/test_rust_projection.do fevc
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do fevc/tests/stata/test_stayers_hybrid.do fevc
./.venv/bin/python fevc/tools/run_checks.py
PATH=<rust-1.85.1>/bin:$PATH ./ci/run_ci_profile.sh plugin-build
git diff --check
```

The source-bound `plugin-build` qualifier ran from a clean local clone of the
final commit. Locked formatting, Clippy, Rust and C gates, SPI/ABI checks, thin
and universal builds, clean-install/lifecycle tests, and the public projection
suite passed for native arm64 and Rosetta x86_64. The resulting candidates are
local ignored artifacts, not public binaries or a release decision.

## Remaining boundaries

Component `inference(highrank|q1)` is intentionally unchanged and remains
exact observation-deletion only. Sparse projection remains an explicit Rust
generic-JLA route with Counter-V1 and explicit diagonal or forced CMG
preconditioning; automatic solver selection and non-generic native projection
families fail closed. No SCC, Windows, Linux, native-Intel-hardware, public
release, or performance claim was made.
