# Approximate inference completion, 8 September 2026

The owner-approved implementation and paper scope is complete. This is a
source-local Mac software checkpoint and an approximate-inference paper
candidate. Point estimation remains the main contribution. No new coverage
confirmation, release or submission was performed.

## Implementation

The structured Rust observation and fixed-offset mover-match routes now use
the direct residual Gram: one half the centered sample covariance of
`Z'[(g-Pg)^2]`, with denominator `R-1`. The default is 2,048 Gaussian Gram
probes. The separate `inferencegramprobes(#)` option accepts integers from
512 through 2,147,483,647, subject to the existing resource/count gates.
Unsupported explicit use is rejected before estimator randomness. The method
is returned as `e(inference_gram_method)=direct_residual_covariance`.

Point probes remain 200, covariance probes 1,000, and the spectral and
critical-value budgets and RNG domains remain unchanged. V4 augmentation
entrypoints carry the explicit Gram count while V1–V3 preserve legacy
policies and result V5 preserves its layout. Requested, planned, executed
and returned counts agree. No ridge, clipping, automatic escalation or exact
fallback was introduced. The q0/q1 formulas, positivity floor, rank gates,
point correction and individual/joint reporting contracts are unchanged.

## Fixed evidence and exclusions

| Check | Result |
|---|---|
| Native saved-outcome replay | 70 calls; all 280 target attempts accounted for; 277 computed intervals, three typed exclusions, no shared fitting failures |
| Archived direct-2048 comparison | All 40 primary calls agree at `1e-8 * max(1,abs(archived))`, with exact discrete/status comparison; 30 diagnostic calls retain their point/sample and status checks |
| Public/native parity | Observation q0/q1 and match q0/q1 pass with the final qualified Mac binary at `1e-7 * max(1,abs(native))` |
| Veneto | One point-only call and q0/q1 at six preselected inference seeds; 13 successful calls, 48 intervals, unchanged points and sample, agreement with archived direct-2048 outputs |
| Scaling | 27 complete calls at 15,360/61,440/245,760 rows; 69 of 72 intervals, with the smallest-size q1 covariance interval withheld in all three repetitions; no timeouts |
| Package source | 798 Python tests pass; CMG assembly check and integrated local Stata qualification pass |
| Rust/native | 525 workspace tests pass; one separately registered long diagnostic remains ignored; pinned formatting, strict Clippy, standalone backend, C/ABI and Mac install checks pass |
| Paper | Complete paper checks pass (134 Python tests; 5 historical completed-run tests skipped); 60 PDF pages visually inspected, including 24 main-text pages |

The native exclusions are all covariance targets with status 1
(`NonpositiveVariance`), at `observation-dominant_common_null-16-0019`,
`match_q1-weak_signal-20-0009`, and `match_q1-weak_signal-20-0019`.
They are retained outcomes, not dropped replications. The independent audit
also reaggregates every old controls draw and verifies its input hashes.

The public example keeps point seed 8675309 and uses inference seeds
8675309–8675314, varying Gram, covariance, spectral and critical-value draws.
Wald-SE coefficients of variation are 5.17%, 4.55%, 6.47% and 1.04% for
worker, firm, covariance and total targets. q1 width CVs are 4.41%, 6.47%,
7.11% and 1.05%. The largest q1 endpoint range is 12.18% of the median
interval width; at most 0.234% of unit variances reach the positivity floor.
This check holds point randomization fixed and does not measure coverage.

At 245,760 rows, median complete command times are 20.54s
(point-only), 397.33s (q0) and 216.33s (q1).
The inference medians use 1.05 and 1.06 GiB peak process-tree RSS.
The three repetitions use each fixed dataset; they are not coverage draws.
Historical million-row timeouts remain unchanged and were not rerun.

## Scientific conclusion and practical guidance

The archived 1,600-call direct-2048 assessment retains three failed screens:
the match one-mode firm interval loses 3.5 percentage points of coverage
relative to its baseline (93.5% versus 97.0%); observation-controls firm
SD/RMS-SE is 1.163; and observation dominant-mode firm SD/RMS-SE is 1.120.
No threshold was weakened. Exact-Gram diagnosis resolves the first and third
cases but leaves observation-controls firm SD/RMS-SE at 1.151. The direct
2,048-probe RMS SEs in those three cases are approximately 9.3%, 1.0% and
5.9% below the corresponding exact-Gram values. Exact Gram remains diagnostic.

The older subtractive-512 controls fitter's 400-draw split gives SD/RMS-SE
ratios 1.150, 0.949 and 1.054, with coverage 92.0%, 95.5% and 93.75%, for
the first half, second half and all draws. That historical reaggregation
does not provide 400-draw validation of the current fitter.

Use q0 when contributions are diffuse; q1 treats one leading mode explicitly
and needs a diffuse remainder. The command does not select q automatically.
Match inference uses movers and fixed offsets, assumes independence across
matches (including different matches of one worker), and omits estimated-
offset uncertainty. The structured variance model is an additional assumption.
Severe misspecification, estimated offsets, weak/null and multiple-mode
designs can produce invalid or unavailable intervals. A few inference seeds
chosen in advance can reveal numerical sensitivity; neither stable output
nor higher Gram precision guarantees coverage. Counts below 2,048 are a
lower-precision tradeoff, not recommended for reported inference.

This is the agreed stopping point: offer a documented model-based
approximation with diagnostics and these limitations. Historical confirmation
passes and failures retain their source-specific meaning. The engineering
replay is not a new calibration campaign or a universal KSS coverage claim.

## Source, compatibility and reproducibility

The checkout remains on `main`, based on
`1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`, with the preexisting research
changes preserved. The immutable native replay manifest is
`0dd684c8081ec28e2950998921e852fe6f86d8d2f0b3cb833e8dada617766aae`;
its source bundle is
`dca8bc2a9f063f2b742f7bf022e2a12e0561dd3d9680ebb05d252e835d8dd55a`.
The local Mac qualifier's source manifest is
`9f5a0c80f7ccf9fc86b1b0740019530890da51fe22b86c91feacf6f0b2bb2934`.
All 222 replay and 150 Mac bindings were checked against the current tree.
The separate delivery archive contains the portable package, native
source, CMG, tests, harnesses and contracts; its manifest verifies every byte.
It includes every production-source binding, excluding the original replay
inventory's single `.DS_Store` filesystem-metadata entry. That entry remains
unchanged in the immutable original bundle and is recorded as an exclusion
from the source-only delivery.

The machine-readable [completion result](inference_completion_v1_result.json)
binds source, binaries, checks, paper, raw inventories and the delivery
snapshot. Local artifacts are under
`.local/diagnostics/inference-completion-20260908/` relative to the repository
root: use the result's repository-relative paths. The native
and paper runs froze manifests before execution. The compatibility review
enumerates changed paths, both source identities, unchanged recipes and
inputs, new build/binary identities, checks, reused claims and limitations.
Historical point/projection and FEVC–Matlab results remain measurements of
their original sources; no new-binary benchmark claim is inferred.

Reproduction commands are recorded in the run manifests and the Mac receipt.
The minimum source gates are `./.venv/bin/python -m pytest -q`,
`./.venv/bin/python fevc/cmg/tools/assemble.py --all --check`, and
`./.venv/bin/python fevc/tools/run_checks.py`. Native gates use Rust/Cargo
1.85.1 with locked dependencies. The Mac build uses Apple Clang 21.0.0,
Stata/MP 19 and macOS 26.6.2; Python is 3.13.0. The paper runs use eight
Stata processors on an Apple M2 Ultra, saved double-precision inputs, fixed
seeds/settings and a 600-second whole-process limit. Their command timers
exclude startup/import, and RSS is the absolute process-tree peak sampled
every 0.1 seconds. The complete tiny pipeline includes a deliberate failure;
malformed, partial, duplicate, altered and scientifically failing records
are covered by focused validator tests.

Initial development failures were fixed and retained as diagnostics: a bad
Gram count could poison retained native state, and two assertions still
expected old metadata text. Final source, native, Stata and paper gates pass.
The first successful public/native check used an earlier build; the four
fixtures were repeated with the final qualified binary. No failed historical
scientific receipt was rewritten or converted into a pass.

Windows/Linux qualification of the new plugin, native Intel hardware, the
full private payload, independent human review, the two published-text
citation checks, final release assembly, commits, public binaries, tags,
publication and submission remain separate owner decisions.
