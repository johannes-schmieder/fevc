# Optional memory budgets and allocation forecasts

This development change implements the owner's request to remove the implicit
4-GiB budget, default to warning, and improve memory accuracy using deterministic
preparation. It changes resource policy, accounting and receipts; it does not
change the estimator, sample, semantic random draws or numerical tolerances.

## Public contract

| Options | Behavior |
| --- | --- |
| No `memory_gib()` | Forecast and continue. No memory-based batch, concurrency or route adjustment. |
| `memory_gib(#)` | Explicit budget; automatic batches use it, warning is the default. |
| `memory_gib(#) memorycheck(error)` | Explicit forecast admission gate. |
| `memory_gib(#) memorycheck(off)` | Budget guides automatic batches; suppress warnings and forecast rejection. |
| `memorycheck()` without a budget | Does not create a budget. |

Explicit batches are preserved. With an advisory budget that cannot fit even
width one, automatic planning chooses width one and continues. Normal probe,
processor, structural and implementation limits remain. Invalid options,
overflow, allocation failures and numerical failures remain errors.

`e(memory_budget_supplied)` distinguishes absence from a number. An omitted
budget posts missing `e(memory_gib)` and `e(batch_memory_budget_bytes)`.
Native zero-valued compatibility fields are accompanied by an explicit
versioned presence/policy receipt; they are never replaced with a large limit.
Legacy Rust/ABI callers retain their previous strict numeric semantics.
Versioned callers charge retained boolean masks at their actual one-byte
element size; historical packed-mask receipt arithmetic is retained only for
legacy callers. Allocation-layout and one-byte budget-boundary tests cover
that distinction.

## Forecast timing and scope

Rust preparation measures requested heap payload while ingesting and preparing
the actual input. The receipt includes overlapping C input and mask export
storage. Before ingestion, strict admission checks unavoidable input copies;
a further strict check can reject after deterministic preparation. This is a
measured preparation peak, not a promise that preparation cannot exceed a
budget before the check. Preparation does not consume estimator randomness.

Solver construction then supplies realized CMG hierarchy, terminal and
workspace sizes. Final phase forecasts use the actual solver and selected
batches before estimator random draws. Full-CMG accounting no longer prices
an embedded-CMG terminal at its 6,144-vertex cap, counts the second solver a
second time, or treats every worker as an auxiliary vertex. Temporary graph
copies and target-column assembly are charged at their actual lifetimes.
Embedded CMG also uses its constructed setup receipt for new-policy calls.

`e(memory_forecast_bytes)` is expected peak direct command allocations.
`e(memory_admission_forecast_bytes)` includes conditional refinement workspace;
`e(memory_conditional_reserve_bytes)` is their difference. Strict and warning
checks use admission bytes. A preparation warning can precede the solve; the
final forecast warning is printed with the completed command.

These are not predictions of total process RSS. Stata's original dataset,
allocator retention, runtime libraries and thread stacks have different
lifetimes and residency behavior. Mata retains a model of working data but
no longer adds the historical fixed 96-MiB runtime and 64-MiB preservation
allowances to public allocation forecasts. Direct standalone diagnostic calls
retain their previous source-bound models.

## Accuracy evidence and limitations

Baseline source: `f3098bc1369992fccbc1d276aac5fc65ceb3f404`. Fresh local
Stata/MP 19 processes on macOS, deterministic degree-three fixture, 200 probes,
seed 8675309. At 30,720 rows and four cores the old forecast was 1,340,521,894
bytes, versus 117,620,736 bytes of process peak RSS. An explicit 0.5-GiB budget
was falsely rejected. The revised command completes that same strict-budget
case; its expected direct-allocation forecast is 35,584,940 bytes. The scopes
of allocation bytes and process RSS must not be compared as an accuracy ratio.

The independent System-forwarding allocator regression tests entire core
execution, including constructed hierarchy and batched solves:

| Rows | Cores | Worker degree | Observed owned peak | Expected forecast | Excess |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 30,720 | 1 | 3 | 34,889,149 | 34,947,675 | 0.17% |
| 61,440 | 4 | 3 | 70,547,356 | 71,354,907 | 1.14% |
| 245,760 | 4 | 3 | 294,845,596 | 300,886,040 | 2.05% |
| 61,440 | 4 | 4 | 78,149,000 | 80,214,982 | 2.64% |

These cases pass the registered maximum excess of max(5%, 1 MiB). The two
larger row counts were held out from initial degree-three tuning. An earlier
degree-four attempt failed that accuracy gate when the conditional refinement
bound was treated as the expected peak; the failed measurement is retained.
The threshold was not changed. No global multiplicative correction was fitted.

The precision result is specific to these full-CMG fixtures. Generic JLA,
exact, projections, controls/weights, stayer augmentation and component
inference have behavioral or allocation-coverage regressions; that is not a
5% accuracy qualification for every regime. In particular, component inference
and stayer augmentation retain conservative bounds. A universal RSS predictor
has not been calibrated: the prospective RSS accuracy target remains unmet,
and no cross-platform RSS accuracy claim follows from this change.

Five alternating baseline/candidate pairs at 122,880 rows and four cores
passed the registered runtime gate. Median command time was 3.072 versus
3.066 seconds (allowed additional time: 0.09216 seconds). Corrected estimates
were identical in all ten fresh processes. This comparison preceded the final retained-mask byte-count correction;
the numerical and core solver sources were unchanged by that correction.
The measured candidate forecast was 154,045,057 bytes versus the baseline's 3,550,141,913 bytes. Process RSS was
recorded independently; it was not treated as owned-allocation ground truth.

The allocator regression's first debug run exposed a test-harness counter
bug: late worker-thread destruction could debit a counter reset for the next
fixture. Continuous allocation accounting fixes the harness; both debug and
release allocation campaigns then pass. Production preparation metering
already uses continuous accounting.

## Validation record

Local integrated qualification passes: 798 Python tests, CMG assembly and
core gates, and both Stata quick and full suites. The Rust workspace passed
506 tests (one existing ignored test); after the final mask accounting fix,
all 83 plugin tests pass, including the allocation-layout and strict-boundary
regressions. Independent allocation campaigns pass in debug and release.

The final source manifest and all three installed local Mac candidates match
the successful native receipt. Native arm64 and Rosetta x86_64 checks cover
exact, generic, compressed/full-CMG, projection, stayer and inference routes,
corrupt receipts, lifecycle and clean installation. C shim, ABI, formatting
and strict Clippy checks pass. The focused memory-policy regression also
passes on both final architectures. The previously rejected 0.5-GiB case now
completes with expected/admission forecasts of 35,584,940/35,978,156 bytes.
No new Linux, Windows or native-Intel qualification was run.


The prospective criteria are in `memory_forecast_v1.json`. Diagnostic logs,
including failed attempts, are retained under
`.local/diagnostics/memory-investigation-20260910/` and
`.local/diagnostics/memory-implementation-20260910/` in the working checkout.
The [durable result record](memory_forecast_v1_result.json) identifies completed
gates, exact native source and artifact hashes, and remaining qualification limits. This is source-local development, not a
release or reuse of historical exact-artifact qualification.
