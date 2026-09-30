# Canonical MCSE stored results — September 30, 2026

`e(mcse)` is the canonical numerical SE vector for the selected mode,
all main point probes by default. `mcse(off)` disables the additional work;
`mcse(conditional)` remains a developer option. The fourth row of `e(results)`
is named `mcse` and equals `e(mcse)`. The point-estimate rows, `e(b)`, `e(kss)`,
projection and sampling-inference results are preserved.

```stata
fevc log_wage, worker(worker_id) firm(firm_id)
matrix list e(mcse)
estat diagnostics
```

| Return | Meaning |
|---|---|
| `e(mcse)` | Four numerical SEs: worker variance, firm variance, raw worker–firm covariance, total variance |
| `e(mcse_mode)` | `all` (default), `off`, or developer `conditional` |
| `e(mcse_method)` | `crossfit_if_v1`, `exact`, `none`, or developer `conditional_target_v1` |
| `e(mcse_status)` | Typed numerical diagnostic status |
| `e(mcse_available)` | One when the selected SE vector is usable; otherwise zero |
| `e(mcse_cov_raw)` | Raw four-by-four numerical covariance, including the total contrast |
| `e(mcse_cov)` | Usable four-by-four numerical covariance |

Covariance row/column order matches `e(mcse)`. Total variance is exactly
worker variance + firm variance + 2 × covariance; all cross terms are retained.
These covariances describe numerical uncertainty, not econometric `e(V)`.
MCSE covers the four main point estimates, including when supported projection
or inference is requested; it does not cover those additional outputs.

All-probe statuses `ok_local`, `ok_local_psd_adjusted` and `exact_zero` are
usable. Exact all mode has method `exact`, zero SEs/covariance and availability
one. Off has method `none`, status `off`, missing SEs/covariance and availability
zero. A withheld all-probe diagnostic has missing SEs/usable covariance and
availability zero; finite raw covariance remains available for a material
non-PSD result. An older native capability retains the point backend and
reports `unavailable_capability` for an omitted option. There is no conditional
substitution. Explicit unsupported all mode retains its pre-RNG failure.

Developer conditional mode returns only its SE vector; its full covariance
is missing rather than inferred from its four SEs. Its calculation is preserved
as `e(mcse_conditional)` with `e(mcse_conditional_available)` when mode is all
or conditional. Exact conditional mode has zero SEs and availability one.

Compatibility `e(numerical_mcse)` and `e(numerical_mcse_available)` now alias
the selected canonical vector and availability. `e(numerical_mcse_all)` and
the old primitive three-by-three `e(numerical_mccov_*)` attachments retain
their all-probe/developer meanings. Callers that require the conditional vector
must use its explicit developer name. This owner-authorized naming follow-up
supersedes only the return-name descriptions in the original
[all-probe specification](ALL_PROBE_MCSE.md) and earlier
[default-interface record](MCSE_DEFAULT_INTERFACE_2026-09-30.md); the derivation,
calculation and scientific gates remain intact. Exporters save the selected SEs, mode,
method, status, availability and raw covariance together. Updated CSV writers
use `mcse_cov_raw_ij`, for one-based target indices `i,j=1,...,4`; they preserve
missing diagnostics and export exact zeros. The alpha writer retains its
existing SE column names and appends metadata/raw-covariance fields.

This follow-up starts from clean `main` at
`e3fa34e76c7e8f5bf96e0622eacb61cb24a6106d`. Changes affect Ado result posting,
display/postestimation, exporters, help/documentation and regressions. Mata/Rust
estimator code, numerical formulas, gates, seeds, probe streams, native ABI,
build inputs and all five binary bytes are unchanged. Existing source-bound
calibration, timing and platform receipts remain immutable, with their original
scope. This naming change makes no fresh calibration, performance or platform
qualification claim and does not require a binary rebuild.

Focused return regressions pass on Mata and the adopted Mac Rust plugin:
33 probes, batch seven, seed 2026092911, generic/compressed engines, frequency
and target weights, default/explicit/alias all, off, developer conditional,
exact zeros, combined stayers, older capability, caller data and RNG. The
four-target covariance identity uses tolerance 1e-11; the independent negative
identity payload verifies non-PSD withholding and raw-covariance retention.
Generic/compressed public Rust checks retain the frozen private conditional
receipt as their independent reference. Postestimation tests preserve points
and numerical matrices across `estat` calls.

The full Python suite passes 879 tests. Malformed export tests reject missing
metadata, inconsistent availability, negative or missing usable SEs, incomplete,
asymmetric or inconsistent-total covariance, and conditional substitution for
a withheld all-probe result. The alpha Mata smoke exports all four SEs and
16 raw-covariance entries (64 workers, 32 firms, 33 probes, one run, four cores,
seed 8675309). This is an interface smoke, not a calibration or timing result.

Retained development failures include the initial absent canonical return,
obsolete display/export-width and exact availability expectations, a consumed
transport test fixture, the unnormalized hybrid alias, the display label's
collision with the progress-log parser, and normalization of unrequested
private executor receipts. Corrections preserve the numerical references,
thresholds and caller-state checks. Two legacy alpha Rust smokes (with and
without controls) are declined with `RUST_OPTION_UNSUPPORTED` before posting;
their existing advisory/request tuple is not qualified here and was not changed.

`./.venv/bin/python fevc/tools/run_checks.py` completes with
`FEVC LOCAL QUALIFICATION PASS`: identity/provenance, deterministic package
and CMG assembly checks, all Python tests, CMG component gates, Stata quick/full
with the adopted Mac native plugin, clean source installation, both helper
migration layouts, synthetic and paired B1/CMG exports, Separations exact/JLA
exports and retained-sample/bridge smokes. Both standalone quick/full suites
also pass. The final separate mode regression checks combined headline aliases
and exact/conditional/off shapes and names. The source installation exercises
the installed helper and zero-MCSE contract; the package inventory is unchanged.

Exact commands include:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
./.venv/bin/python fevc/tools/run_checks.py
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do fevc/tests/stata/run_all.do quick
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do fevc/tests/stata/run_all.do full
```

The local interpreter is Python 3.13.0 and the licensed runtime is Mac
Stata/MP 19. Logs and immutable smoke specification/source bindings are private
under `.local/mcse-returns-20260930/`. Synthetic exporter smoke source/spec fields
are diagnostic harness arguments; they are not a clean-commit qualification
receipt. The final alpha smoke verifies its actual dirty source hashes and
specification digest before and after execution. An initial manual spec-digest
mismatch was caught, retained separately and rerun with the computed digest.

No statistical recalibration, timing campaign, new Windows/Linux run, binary
build/adoption, CI-policy change, tag or release was performed. The affected
surface is M4 frontend posting/export; M0–M3 mathematics, replay and integration
implementations remain unchanged. Earlier calibration/performance/platform
reports retain their exact sources and qualification limits. Current native
functional checks exercise the existing Mac binary; they do not requalify the
other platforms or make a new overhead claim.

The three changed runtime Ado files and the exporter validator are bound by
these SHA-256 identities:

| Path | SHA-256 |
|---|---|
| `fevc/fevc__numerical.ado` | `aaeb845fb769384b47a029c96ef6003c3939b6e197877b57dc7997c69bff874b` |
| `fevc/fevc__display.ado` | `a2d916e0df783cc348a3fac6b906d93a296ff00671227ae7406e068217f88f6f` |
| `fevc/fevc_estat.ado` | `086353ecbd1b91336e14d413714a6e534be8c6c2dda0a60267f28881b1998df4` |
| `fevc/tools/run_checks.py` | `8638c5ead2e6c3e77f9d29938120770d13ed846218aa05423684b398e0df56e3` |
