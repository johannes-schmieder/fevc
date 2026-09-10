# Audit repairs and 256-lane control preparation

This source-local follow-up addresses the owner's request to repair the three
audit discrepancies, then validate the 256-lane Mata optimization. It preserves
the original [API 23 repair report](CONTROL_BASIS_REPAIR_2026-09-10.md), its
failed audit records and its measured 13.2% overhead. The earlier investigation
is retained under `.local/diagnostics/control-basis-limits-20260910/`; new raw
logs and diagnostic drivers are under
`.local/diagnostics/control-basis-256-20260910/`. No commit, release, remote
campaign or binary distribution is part of this checkpoint.

## Audit discrepancies

All three failures were reproduced before editing and repaired without
relaxing an audit:

1. The exact manual inventory now includes the owner's `simple_AKM.do`.
   The manual README identifies its external `reghdfe` dependency and points
   to the independent automated AKM regression. The owner example's bytes
   remain unchanged from this task's starting snapshot.
2. Only the appended terminology notes were removed from the two frozen
   historical READMEs. Their recorded hashes are restored: history
   `7a2477adbc1ca265464611a7acda907a5a955c50241186d86f6587721e3ad142`
   and migration
   `d28f1c79ee2447e29b8a9210234d7332a20ad484174e1a816c3dada4fdc9949c`.
   The immutable inventory was not edited. Current terminology is already
   specified in active source-provenance guidance.
3. The parity JSON now uses the same current comparator label as its generated
   Markdown. The normal renderer regenerated the document; qualification
   statuses and historical evidence were not changed.

The three affected Python modules pass all 14 tests. The complete Python gate
passes 799 tests. These audit defects are separate from the numerical repair.

## Implementation and numerical contract

Only Mata's canonical-control weighted-product block grows from 64 to 256
lanes. The two weighted multiplications, compensated value and magnitude sums,
unrounded lane merging and final rounding retain the same arithmetic design.
For `L=min(256,n)` and `m=ceil(n/L)`, `m+2L+1 <= 4n`; consequently the existing
second-order summation coefficient still bounds both stages. The full
[derivation](CONTROL_BASIS_CERTIFICATION.md) covers product error, underflow,
scaling, whitening, score intervals, decisive-prefix selection, projectors and
span reconstruction. No uncertainty floor, rank gate, inverse-residual gate,
forward-error ceiling, sample or estimator is changed.

The larger block requires resource accounting before allocation. The core now
checks its conservative control-phase slot count before forming the Gram;
the generic resource model also charges that phase while prepared solver
state is live. Four persistent accumulators require 8 MiB at 32 controls,
plus bounded temporary storage. The formula and its limits are documented in
the numerical contract. Strict explicit budgets fail closed, and advisory,
off and omitted-budget policies retain their meanings.

Interruption validation exposed two existing wrapper defects. JLA labeled a
real control-product interrupt as a runtime failure and left results posted;
the ordinary exact bridge discarded its captured return code and produced a
blank validation status. Both exact and JLA now preserve UserBreak code 1,
clear results, and finish ordinary caller-state cleanup. Other captured Mata
runtime failures pass through the established runtime-failure branch.

The Mata identity advances to API 24, `vckss-api24-control-lanes256`, and the
resource runtime to API 12, `vckss-resource-api12-control-scratch`. Loader,
installation and stale-runtime tests advance together. Public syntax, result
matrix layouts, native ABI, native source and installed binaries are unchanged.

## Focused validation

- Arithmetic: independently generated Decimal intervals for cancellation,
  signed products, extreme scales, zero products, integer frequencies,
  subnormals and overflow; exact integer matrix oracles for every control count
  1 through 32 at 63, 64, 65, 255, 256, 257, 511, 512, 513 and 769 rows.
  Deep cancellation includes a 1,537-term example and cancellation within a
  single lane across three blocks. The independent high-precision score,
  anchor and span checks remain in place.
- Selection: the existing adversarial anchor suite retains prefix selection,
  boundary uncertainty and inverse-residual cause assertions. Its rejection
  expectations have not been broadened.
- Statistical equivalence: exact and JLA, observation and match deletion,
  integer frequencies versus physical copies, control permutation, sign
  reversal, nonsingular transformations and reversed input order. The new
  600-stored-row fixture represents 1,200 physical observations and crosses
  multiple lane blocks. Tests apply the registered corrected-result policy
  and preserve tighter deterministic row/copy diagnostics. At full supported
  rank, a separate 1,024-row, 32-control canonicalization check covers a mixed
  nonsingular transformation, sign/permutation changes, and frequency weights
  versus their independently expanded physical copies.
- Lifecycle and memory: finite results, complete residuals, inverse residuals,
  caller data/RNG/sort restoration; independent scratch counts across
  `n=1..1025, q=0..32`; strict one-byte-below rejection, exact-limit admission,
  advisory admission, and generic resource charging at batch one. Injected
  UserBreak covers both algorithms and deletion modes. A real SIGINT is sent
  after the first 256-row product block in an instrumented diagnostic copy;
  native idle state, timers and memory globals are checked after cleanup.

The instrumented copy adds only a first-block file marker to synchronize the
real signal. Timing and memory benchmarks use uninstrumented implementations.
Harness-development failures and the original interruption failures remain in
the diagnostic logs; they are not successful validation receipts.

The unmodified integrated entrypoint, `./.venv/bin/python
fevc/tools/run_checks.py`, passes: all 799 Python tests, CMG assembly and
component checks, Stata quick/full suites, clean installation and the remaining
local smoke gates. The full suite includes the original 50,000-observation AKM
fixture through explicit Mata, explicit Rust and automatic routing, together
with its centered reference. The final focused suite passes on arm64 and under
Rosetta; Rosetta also passes the public Rust route and memory-policy checks.
The final real-signal run completes cleanup in 0.074 seconds after SIGINT.

Commands and positive markers are retained in `integrated.log`,
`focused-final3.out`, `rosetta-final.out`, `rosetta-final3.out`, and
`interrupt-final.log` in the diagnostic directory. Additional explicit checks
are `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check`, the legacy-name
audit, and `git diff --check`. Pytest emits existing temporary-directory cleanup
warnings about protected remote-symlink fixtures; no test fails because of
them, and those unrelated fixtures were not altered. Tests and benchmarks use
Stata/MP 19 on macOS 26.6.2; the numerical test seed is 377 and the performance
DGP/JLA seeds are 19471/37. The owner's manual do-file remains unchanged.

## Timing and measured memory

Each timing cell uses three fresh Stata processes per version, with interleaved
and alternating version order. Runs are sequential after integration finishes.
Both implementations are uninstrumented. Direct canonical preparation is timed
first, then the complete estimator command, including its own preparation.
The saved API 23 implementation with 64 compensated lanes is the baseline;
this comparison isolates the follow-up and does not remeasure the original
uncompensated implementation. The 2-control cell is the original AKM data; the
8/32-control cells use accepted paired-sign controls with zero within-match
means. All runs retain their full sample and pass finite-output and complete
residual checks. The corrected outputs match exactly at the recorded binary64
precision, passing the registered common-draw equivalence policy.

| Controls / rows / probes | Preparation, 64 lanes | Preparation, 256 lanes | Command, 64 lanes | Command, 256 lanes |
| --- | ---: | ---: | ---: | ---: |
| 2 / 50,000 / 200 | 0.049 s | 0.047 s | 5.647 s | 5.638 s |
| 8 / 100,000 / 20 | 0.513 s | 0.432 s | 8.676 s | 8.461 s |
| 32 / 100,000 / 20 | 2.388 s | 1.940 s | 20.732 s | 20.342 s |

At 32 controls the measured preparation saving is 18.8%; complete-command
saving is 1.9%. At eight controls the corresponding savings are 15.8% and
2.5%. The two-control difference is negligible. These are three-repeat medians,
not confidence intervals. The 2-control candidate includes one 6.145-second
command versus two at 5.638 seconds. The optimization reduces the preparation
cost but does not eliminate compensation's cost or establish competitive
performance against the KSS Matlab package. The historical 13.2% observation
is preserved and must not be recomputed by mixing medians from different runs.

Whole-process peak RSS is measured in separate fresh processes. In the complete
32-control command its median rises from 593.953 to 615.984 MiB, about 22 MiB.
The two-control medians are 803.781/803.391 MiB and the eight-control medians
442.812/444.609 MiB. The direct dense-product memory checks give:

| Rows / repeated calls | 64-lane peak RSS | 256-lane peak RSS |
| --- | ---: | ---: |
| 100,000 / 1 | 66.203 MiB | 95.859 MiB |
| 100,000 / 5 | 66.359 MiB | 96.422 MiB |
| 200,000 / 1 | 96.156 MiB | 135.828 MiB |
| 400,000 / 1 | 145.703 MiB | 197.438 MiB |

Five calls add only 0.563 MiB over one call for the candidate; there is no
observed accumulation proportional to repeated calls. Input storage grows
with rows. The algorithm's explicit lane arrays remain capped, but total RSS
also includes Stata/runtime allocation behavior: its extra 30–52 MiB in these
direct-kernel runs must not be described as a constant 8-MiB process cost.
The independent slot-count tests, strict/advisory admission checks and generic
phase-charge test validate the allocation model; these RSS observations do
not establish a universal process-memory bound or change the separate memory
qualification limits. The measured speed/memory trade-off supports retaining
256 lanes for this local checkpoint, with the larger workspace explicitly
accounted for.

Raw command times, RSS records, baseline/current source hashes, all corrected
comparisons and diagnostic-run hashes are in `performance/result.json` under
the diagnostic directory. The initial memory driver had a Mata struct-typing
error; the corrected driver declares its variables inside a function. Timing
runs were retained unchanged and only the affected memory runs were repeated.

## Source and native compatibility

The task started on the current `main` worktree at
`ee4afbfb6d30a4c74d03a62c58a0785b96889ae8`, with owner changes already present.
The previous local native receipt is classified `LOCAL_CHECKPOINT_DIRTY_TREE`
at commit `f3098bc1369992fccbc1d276aac5fc65ceb3f404`, bound to source manifest
`51cfd55dcd19baca552c6d4c8b3b41b946541c8984ba56f62c4d5d168e950861`.
Comparing all 156 manifest entries finds seven changed paths: package README,
TESTING, help, `fevc.ado`, `fevc.mata`, `fevc_resource.mata`, and the Stata suite
driver. The first three contain existing owner documentation changes; the
other four cover this follow-up's runtime, accounting, cleanup and tests.
All manifest-bound Rust, C, ABI and build inputs remain byte-identical.

The installed native artifacts still have these hashes:

| Artifact | SHA-256 |
| --- | --- |
| arm64 | `7161126980841ba22f26321d27057ca532f28e61dd606256fbd1e1f5f507b043` |
| x86_64 | `c712786e2d73e2579a81347ac527413885aea6d454de061094fddf247934bbbc` |
| universal | `99b423458f0bb0e24432bb0f613084df10a757181c2921fdadfb8ba76de97836` |

Only unchanged native arithmetic, ABI, build and artifact claims are carried
forward. Earlier whole-source qualification is not relabeled as API 24
qualification. Fresh local Stata integration and Rosetta checks cover the
changed Mata/ado surface. The acceptance policy, numerical oracle inputs,
Stata binary and native artifacts are unchanged. No Rust rebuild, repeated
Rust workspace campaign, Windows/Linux qualification or native Intel hardware
claim is made for this follow-up. Current changed-source and log hashes are
recorded in [the result record](control_lanes_256_v1_result.json).
