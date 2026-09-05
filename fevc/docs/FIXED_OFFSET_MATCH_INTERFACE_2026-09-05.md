# Fixed-offset match interface: engineering and RC review

## Outcome and permitted scope

The explicit public fixed-offset match q0/q1 interface is implemented at
`53f22a109effee87467b4ef0602b21d0b8ec1ca9`. This is a bounded engineering
qualification, not a new coverage experiment or approval to release.
The owner decision is `fixed_offset_match_interface_v1.json`. The earlier
paused checklist and existing-output review retain their historical status;
this record supplies the subsequent integration checkpoint.

The supported request explicitly names Rust, JLA, generic engine, Counter-V1,
match deletion, fixed-offset nuisance handling, movers, diagonal or CMG,
structured-common or structured-leverage variance, and highrank/q1 reference.
Integer frequency weights are regression mass, not independent copies.
Target weights retain stored-row semantics; distinct declared match IDs are
not merged merely because worker and firm coordinates agree.

Point defaults remain match deletion, joint nuisance and the combined
mover/eligible-stayer population. That default point route does not acquire
component inference. No joint-controls or second-stage correction is added.

## Interface and diagnostic contract

An additive native match augmentation entrypoint selects the existing grouped
kernel. The old observation entrypoint retains observation-only semantics.
The statistical result V4 remains 208 bytes; the separate unit receipt V1 is
64 bytes. Native generation, struct size and capacity, deletion mode,
independent-unit count, omitted-uncertainty flag and finite/range diagnostics
are reconciled before Stata posts inference. Unsupported public tuples fail
in capability preflight; structural, solver and joint-PSD failures remain
whole-request failures. Target-local q1 withholding is preserved.

Help, display, `estat diagnostics` and `e()` returns identify the declared
matches, effective count by regression mass, largest regression-mass share,
largest match leverage and smallest maker denominator. Effective count is
`(sum F_g)^2/sum F_g^2`, not a degrees-of-freedom adjustment or validity test.
The required label is:

**Fixed-offset approximate match inference, ignoring nuisance-control
estimation uncertainty.**

With genuinely fixed offsets, collapsing permits unrestricted dependence
within matches. Independence across declared matches, the named structured
aggregate-variance model and the q-specific concentration conditions remain
assumptions. Same-sample estimated controls can induce cross-match dependence;
few controls alone do not establish that the omitted uncertainty is small.
Severe variance misspecification and multi-mode q1 targets remain outside the
confirmed claim. Numerical availability is not statistical validation.

## Scientific source compatibility

The new public source is `53f22a109effee87467b4ef0602b21d0b8ec1ca9`; its
development baseline is `a88cd7f`. The diagnostic-only commit is
`804491dfe7c9e36cd311955fbd6e00bbb42b3d99`. The complete estimator-core tree is
byte-identical between a88cd7f and the new source. Core production source and
vendored CMG, Cargo dependency/lock identities and the native build script
also match the repaired match-q1 confirmation source
`4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`. Core production source matches the
independent match-q0 source `bb580fe69085d1f98c9151cea2de038aec0a8ba6`.
No covariance, recenter, curvature, critical-value simulation, variance fit,
RNG domain, solver, collapse formula or scientific threshold changed.
The earlier q0-example test-only change remains covered by
`RC_INFERENCE_SOURCE_COMPATIBILITY_2026-09-05.md`.

The accepted input designs, semantic seeds, exclusions, fold rules, archived
source bundles/binaries, outputs and acceptance thresholds remain those of
the original confirmations. The result and registration file identities
are recorded in the accompanying JSON. Their coverage claims are reused
only for those registered fixed-offset q0 and eligible one-mode q1 regimes.
They do not measure the new public boundary, representative-scale runtime,
or arbitrary user designs. Changed native/ado boundaries require the new
qualification below: old native receipts are not reused as new-binary proof.

The corrected observation confirmation remains **FAIL**, at source
`73fa75805c8cef6d4d1a6ad843da5894ccedc956`: the one-mode leverage-only k=16
firm SE ratio is 1.101204 against 1.10. The existing-output review does not
change that outcome. Its small cutoff excess is only 0.072 MCSE, but this is
not evidence that the broader SE calibration shortfall is absent. The public
observation-q1 metadata/display now states the unresolved failure explicitly.

## Local validation

- Python: `./.venv/bin/python -m pytest -q`, 703 passed; Python 3.13.0,
  pytest 7.4.4. Integrated rerun used an isolated pytest base directory.
- CMG: `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check`, PASS;
  integrated CMG mathematical, namespace, hierarchy and API-5 checks PASS.
- Rust 1.85.1: workspace `cargo test --workspace --all-targets --locked`,
  strict Clippy and formatting PASS; standalone Stata backend all-target
  tests, strict Clippy and formatting PASS. The native qualifier repeats the
  standalone gates and compiles the C ABI, interruption and error tests.
- `./.venv/bin/python fevc/tools/run_checks.py`: PASS, including public
  identity/history/provenance, parity, temporary deterministic package check,
  quick/full Stata suites, clean installation, B1/CMG and preparation smokes.
- Public match Stata regression: point invariance, integer/literal-copy
  weighting, target mass, fixed-offset controls, match-ID partitioning,
  diagonal/CMG agreement, row/batch invariance, fixed folds, RNG/data/lifecycle
  restoration and corrupted-receipt rejection PASS. It uses 800 stored rows,
  400 matches, frequency two, 256 point probes, 512 inference probes,
  default seed 8675309 and explicit solve tolerance 1e-11. Point invariance
  and row/batch comparisons are bitwise; literal-copy comparison is 1e-10
  and solver comparison 1e-8. The numerical/spectral gates were not relaxed.

The initial tiny native fixture lacked cross-fit support; its enlarged but
nearly noiseless variant lacked positive training responses. The final
boundary fixture uses a stronger deterministic residual. An early public
fixture also failed the existing spectral residual gate; a separated-mode
target fixture was used for integration testing while preserving that failure
log. These are diagnostic test development, not confirmation or new coverage
evidence. Integration uncovered and repaired an obsolete observation-only
parser guard, a Stata option-default syntax error, and ereturn's move of a
temporary matrix before its scalars were read. The final regression protects
the posting order. No failed scientific receipt was edited.

An initial native-qualifier launch failed while downloading public Stata SPI
headers under the network sandbox, before compilation or numerical testing.
It is preserved separately; the clean-source permitted retry supplies the
qualification record. Temporary artifact checks and local candidate binaries
are engineering outputs, not release staging or binary distribution.

## Exact-source native qualification and retained packet

`./ci/run_ci_profile.sh plugin-build` PASSES on the clean source above, clean
at both start and end. Stata/MP 19 (bundle 19.0.115) ran the explicit match
test and isolated `net install` on native arm64 and Rosetta x86_64. The
qualifier also checks both thin binaries and their signed universal package,
export inventory, architecture/deployment floors and dependency policy. This
is local macOS qualification, not Linux, Windows or native-Intel evidence.

The 142-file source manifest has SHA-256
`145b5faa4c4e95c74114439f5fa4784a9a57a49953125dd910fe74b665dccba1`.
The exact tested local binary SHA-256 values are:

| Artifact | SHA-256 |
| --- | --- |
| arm64 | `67637b88bb8ff34d500263b40ea17ac06b26994a6efcf4d03de63677aa1f854f` |
| x86_64 | `6860224a14442812bc80089dd3ef7abea78a5ad0fa93e24e53146a5c71de63b3` |
| universal | `4a9860430572d0516a2218aa0fae40b34940a6997e186ce356349b1707a5d6e7` |

The source-bound text receipt, CI receipt, source manifest and checksum packet
are under `rust/qualification/evidence/FIXED-OFFSET-MATCH-INTERFACE-MACOS/`
`53f22a109effee87467b4ef0602b21d0b8ec1ca9/`. No binary or licensed startup
banner is committed. Full local engineering outputs, tested binaries and
preserved development diagnostics are ignored under
`.local/diagnostics/fixed-offset-match-interface/53f22a109effee87467b4ef0602b21d0b8ec1ca9/`.

The accompanying `fixed_offset_match_interface_v1_result.json` enumerates
the changed interface paths relative to 804491d, gate results and retained
scientific record identities. The four diagnostic-only paths in 804491d are
the ratio-review report/result, its Python tool and its regression test.
They do not enter the estimator or native build.

The follow-up checkpoint commit contains only this report/result, the
source-bound text packet, PLAN/index and generated parity status updates.
It carries qualification by **content identity**: all 142 native-manifest
entries, package source, build inputs and tested binary hashes revalidate
unchanged. It does not relabel the tested source as the later documentation
SHA. The original campaign records and acceptance policy remain unchanged.
Any later executable/build/input/threshold change requires a new impact
review; this receipt does not prequalify it.

## Owner-facing RC checklist

- Passing independent match q0 and repaired eligible match q1 scientific
  evidence is available and retained under unchanged assumptions.
- Public explicit tuple, diagnostics, fail-closed boundary and local
  integration are implemented and tested.
- Corrected observation q1 still has an unresolved calibration failure.
  Any release must retain its warning or make a separate prospective scope
  restriction; no silent waiver or automatic rerun is warranted.
- Same-sample control uncertainty, joint-control match inference, combined
  mover/stayer component inference, automatic q selection, q>1 and second
  stage corrections remain deferred.
- This boundary has no new Linux, Windows, native-Intel or representative-
  scale qualification. Mac Rosetta is not native-Intel evidence. Existing
  scaling records do not establish a performance claim for the new route.
- Final version/tag, exact distributable artifact, source correspondence,
  notices/licensing/human review and binary distribution remain owner gates.
  No push, tag, publication or release staging is authorized or performed.

The next bounded step is owner review of this scoped candidate and the
observation-route labeling/restriction decision, not another estimator or
controls research campaign. A future release can use the qualified local
Mac artifacts only after the separate exact-artifact and human approvals.
