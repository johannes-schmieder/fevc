# Control-basis repair: local checkpoint

The combined repair preserves the uncentered model and makes the original
50,000-observation AKM example certifiable. It retains semantic ordering,
Neumaier compensation, rank/residual tolerances, deletion semantics and the
`1e-8` downstream forward-error ceiling. API 23 identifies the changed Mata
runtime; public syntax, native layouts and result-matrix schemas are unchanged.
The owner’s manual do-file and the earlier investigation are preserved.

The [complete derivation](CONTROL_BASIS_CERTIFICATION.md) describes weighted
products, compensated accumulation, absolute underflow and outward rounding,
followed by the full basis envelope. The selector still computes the maximum
over all rows, but certifies only the first eligible row and its predecessors.
The adversarial inverse witness remains rejected with public
`AMBIGUOUS_CONTROL_BASIS`; both backends retain its measured inverse-residual
cause. Actual singularity keeps its singularity classification.

## Arithmetic and regression evidence

- Shared exact-input Decimal cross-product endpoints cover cancellation,
  signed terms, exact integer weights including the binary64 physical-total
  boundary, zero products, subnormals, extreme scales and overflow rejection.
  Mata also preserves a low term across 385 terms and multiple accumulator
  lanes. Integer cross-product oracles cover every control count from 1 to 32.
- A separate 160-digit score/span oracle checks both anchors and score
  intervals. Selection tests distinguish irrelevant late boundaries, uncertain
  earlier/selected rows, equality, and the effect of the global maximum.
- An independent 100-digit check of all 50,000 original AKM control rows selects
  anchors 29929 and 14. The computed canonical matrix differs by at most
  `8.61e-16`; its recorded forward envelope is `3.52e-13`. Both score passes
  lie within the certificate.
- The self-contained automated AKM regression has no `reghdfe` dependency.
  It requires all 50,000 observations, finite results, original-system
  residuals, data/RNG/sort restoration, explicit Mata/Rust and automatic routing,
  and equivalence to the centered reference under the registered policy.
- The existing anchor adversaries retain their public status assertions and
  now check the inverse cause explicitly. Existing exact/JLA, frequency-copy,
  mover/stayer, observation/match, transformation, full-residual, rank and native
  lifecycle suites provide the downstream regression coverage.

## Gate accounting

Final command outcomes and artifact identities are recorded in
[`control_basis_repair_v1_result.json`](control_basis_repair_v1_result.json).
The source and binary manifests, exact commands and detailed logs are under
`../../.local/diagnostics/control-basis-repair-20260910/`.

The unmodified Python/integrated runner has a **pre-existing failure**:
`test_manual_referee_files_are_complete` excludes the owner’s untracked
`simple_AKM.do` from its exact directory inventory. It failed before this repair
(797 other tests passed) and afterward (798 other tests passed). The manual file
and test were left unchanged. The aggregate runner is therefore not reported as
green. Its remaining integrated Stata gates passed separately, including
quick/full suites, clean installation, CMG checks, synthetic and paired B1/CMG
smokes, and retained-sample audits. The final full Python run has **796 passed
and three failed**: the original inventory failure plus two failures introduced
by concurrent owner documentation edits. The latter are the immutable relocation
inventory's README hashes and the generated Rust/Mata parity matrix's terminology.
They are recorded separately from the original failure. This repair changes none
of those assertions or files.

The final Rust workspace has **538 passing tests**, no failures and one ignored
test. Workspace formatting, strict Clippy, standalone backend tests, C transport
and ABI checks passed. The final native receipt is
`native-directed/receipt.txt`, classified `LOCAL_CHECKPOINT_DIRTY_TREE`; it
qualifies thin arm64, Rosetta x86_64 and the universal artifact, including cleanup
and clean installation. Supplemental arithmetic/adversarial tests also passed
under Rosetta; the new automated AKM regression passed with the final plugin.

After qualification, concurrent owner edits changed only `fevc/README.md`,
`fevc/TESTING.md` and `fevc/fevc.sthlp` among the qualifier's 156 manifest entries. These documentation
edits do not change numerical, loader, build, binary, test-input or acceptance
identities. The result JSON records both content identities and this limited
compatibility review. Numerical/native claims carry forward; the changed
documentation is not represented as part of the original exact-source receipt.
The new oracle fixtures and focused drivers have their own content manifest,
because they are outside the qualifier's fixed inventory.

Development attempts remain diagnostic records: an initial SDK fetch failed
under restricted DNS; the next native run correctly rejected source changes
made during performance work. Later source-stable qualification receipts are
kept separately. A passing receipt before the final outward-division guard is
superseded by the final exact-source receipt, not edited or relabeled.

## Performance and scope

Mata uses 64 compensated lanes and an algebraically equivalent TwoSum residual
without comparison-mask matrices. This reduced the initial 100,000-row,
32-control preparation time from about 11.1 seconds to 2.865 seconds in the final
measurement (the old ordinary cross-product implementation takes 0.462 seconds).
Scratch is bounded by `64 x q^2`, independently of sample size. Compensated
certification still costs more than the old ordinary matrix cross product;
complete-command measurements, not the preparation ratio alone, assess that
cost. The final result record contains timings, process maximum RSS and paired
corrected-result comparisons. Earlier timings taken during development are
retained as diagnostics.

Three-repeat medians in seconds on the same local Mac are:

| Complete command | Mata before | Mata repaired | Rust before | Rust repaired |
| --- | ---: | ---: | ---: | ---: |
| 50,000-row AKM, centered reference, 200 probes | 5.883 | 5.910 | 4.127 | 4.111 |
| 50,000-row AKM, original controls, 200 probes | rejects | 5.846 | rejects | 4.092 |
| 100,000 rows, 2 controls, 20 probes | 6.468 | 6.467 | 2.274 | 2.254 |
| 100,000 rows, 8 controls, 20 probes | 8.716 | 9.190 | 3.630 | 3.571 |
| 100,000 rows, 32 paired-sign controls, 20 probes | 18.276 | 20.686 | 19.821 | 20.016 |

AKM control preparation is 0.047 seconds in Mata and 0.00623 seconds in Rust.
Rust preparation for 100,000 rows and 32 dense controls changes from 7.131 to
7.343 seconds. The initial large Mata regression was reduced substantially
within compensated arithmetic. A **13.2% complete-command overhead remains** on
the accepted 32-control Mata design (5.4% at eight controls); this is a measured
cost, not a performance pass or a claim that every regression is eliminated.
The analogous Rust command changes by 1.0%. No existing KSS Matlab competitive
performance claim is extended to this checkpoint.

Process maximum RSS for the accepted 32-control complete commands is 643.2 to
650.5 MB in Mata and 650.0 to 535.1 MB in Rust; these are observed process peaks,
not allocation forecasts. Preparation scratch stays independent of row count.
AKM process-RSS comparisons use different numbers of calls and therefore cannot
isolate the repair's memory effect.

All 180 paired corrected-target comparisons pass the registered policy. With
common draws, baseline versus repaired corrected values differ by at most
`4.34e-19`; the original versus centered AKM differs by at most `1.53e-14`.
Across backends the largest discrepancy uses 37.5% of the registered independent-
draw limit. Full timings, RSS, outputs, seeds, input and driver identities are
retained in the machine-readable record and diagnostic logs.

For the dense 100,000-row sinusoidal 32-control design, the old implementation
withholds at its downstream basis certificate while the repair succeeds.
That failed baseline is not a performance comparator. The paired-sign,
within-match-zero-mean 32-control design supplies an accepted baseline for
comparison. Failed exploratory fixtures remain in the diagnostic directory.
Two final standalone AKM-preparation timing drivers initially stopped at a Mata
struct declaration error. Their failed logs are preserved; corrected drivers
ran to an explicit completion marker. Estimator tests and production code were
unaffected.

This is a local dirty-worktree checkpoint, including preserved owner changes.
It does not authorize or claim a commit, release, public binary distribution,
remote campaign, Linux/Windows qualification or arbitrary-precision refinement.
