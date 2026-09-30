# MCSE default interface — development validation, September 30, 2026

The public option is `mcse(all|off)`, default `all`; `numericalmcse()` remains an
alias and conditional MCSE remains a developer option. Supported projection and
sampling inference can accompany main-point MCSE, with no MCSE for their extra
outputs. The [interface decision](MCSE_DEFAULT_INTERFACE_2026-09-30.md) records
returns, withholding, versioned native transport and memory behavior.

## Source and preserved work

The complete updated issue #7 reviewed source and current `main` base are both
`ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3`. The existing issue implementation,
Rust optimization and optimized Mata work were preserved. This is a dirty-tree
checkpoint, not an exact clean-commit release qualification. No scientific,
residual, RNG or CI-policy gate was relaxed.

M0–M2 retain the independent mathematical/reference oracles and bounded Rust
(four directions) and Mata (eight directions) replay. M3 adds numerical V2
without changing frozen V1/result/RHS layouts or point-work meanings. It keeps
the original point executor and uses separate replay accounting. M4 fresh
calibration and Mac checks have completed; 300-command performance validation also passes. Linux and Windows
qualification are recorded below as they finish.

## Fresh prescribed calibration

The frozen source inventory has SHA-256
`931aea0b1c2763dc75f07926df10f48be06f0082f1a08ff5978253314bf036ce`;
the core inventory identity is
`3ef30ea5f98d99cbbac01c39ef86ee3963ab72d3111df91776bb7bdac42e17c8`.
All 203 inventoried current source files remain identical. The current native
adapter has SHA-256
`23c9bb88eba9468b42f75e15c54954795a6e1f547f9dca886ca92c69c9e8b374`.

Fresh executions used the prescribed fixtures, seeds, R/T cells, replication
counts and acceptance thresholds without tuning. There are 239,616 attempted
fits: 221,184 complete calibration attempts and 18,432 nested attempts, across
72 dense/native cells. No point attempt failed; every diagnostic was usable.
All 540 complete contrast equivalence screens passed. The largest absolute
relative variance discrepancy was 5.337%; the largest uncertainty-inclusive
equivalence bound was 10.585%, within the registered 15% gate. These are
variance calibration screens, not sampling-inference coverage.

All 18 nested cells are `observed_complete`, preserving their prescribed
diagnostic classification. They do not acquire a confirmation gate because
execution was successful. This evidence supports the fixed-data local
all-probe covariance on the tested small dense and native Counter-V1 inputs;
it does not establish a uniform guarantee near singular deletion, broad
production Mata mt64s calibration, representative scale, or MCSE for projection
and inference outputs.

Raw outputs, manifests, audits, execution receipts and their complete hash
inventory are retained under `.local/mcse-default-20260930/calibration-*`.
Earlier receipts and calibration evidence remain immutable.

## Checks and platform qualification

The focused mode tests cover default/all/off/alias/conditional, exclusive option
names, exact zero, old-capability withholding, both deletion and nuisance modes,
weights, target mass, main-result invariance and caller state. Attachment checks
preserve projection coefficients/covariance and highrank/q1 component results,
point work and component widths under diagonal and CMG solvers. Rust FFI tests
also compare literal legacy compressed and generic point/conditional results
bitwise and check interruption, reuse and malformed prepared flags.

The point-only 2.4-million-copy memory fixture retains its original strict gate
under explicit conditional mode. A separate smaller command regression places
a budget between off and all peaks: off succeeds, all fails before RNG, and
point estimates and caller state remain unchanged. Existing point-only full-CMG
fixtures explicitly request conditional mode so their original work-count gates
remain intact; dedicated all-mode tests account for replay separately.

The full local native Stata suite passes. Mac qualification passes thin arm64,
thin x86-64 under Rosetta and universal artifacts on both architectures, with
isolated installs and eight MCSE mode/attachment cases. The receipt is a
`LOCAL_CHECKPOINT_DIRTY_TREE`, source manifest
`97c0d828823a01e175f5c886a02c13fbbc636fa84805627dbbfb1f770908bcd7`.
It uses Stata/MP 19, Rust 1.85.1 and Apple Clang 17, deployment floors 11.0
(arm64) and 10.13 (Intel). Native Intel hardware is not tested.

| Candidate | SHA-256 |
|---|---|
| Mac arm64 | `d9b55a424d7629f1102d8ff696971e5a22ca311f16b2cdb8264caa96ce78a0e0` |
| Mac x86-64 | `b0096a6b573439a3386eff2e83061a7a537b9cc25e35b98c9e12306f30a2e8f7` |
| Mac universal | `b44fe1d257b4c40e6c4d8d3741a7a482acf50956d9fd349ace622ab1a91a76c4` |

## Performance and remaining platform work

A frozen 300-command timing manifest compares all/off/conditional, cold/warm,
three main routes at 1,200/12,000 rows, Mata/Rust, and supported projection and
component attachments. It checks points, conditional MCSE and attachment
results, records complete-command timers and process RSS, and runs serially
after other local compute finishes. Results follow.

The frozen 300-command timing campaign passes. All 100 all-mode commands
report `ok_local`; all off/conditional commands report their intended typed
status. The 14,004 recorded numeric comparisons are identical, including main
points and supported projection/inference results. No other compute was detected
at command endpoints. The result SHA-256 is
`732d1f0b8628d450c6f580d2d08eb48e391ab3c6d51a5ee76b101972cd5f7ddb`.

Medians on the 12,000-row inputs, with 200 leverage/target probes and a literal
point batch of seven, are:

| Backend / route | Cold off → all (s) | Extra | Warm off → all (s) | Extra |
|---|---:|---:|---:|---:|
| Rust / generic observation | 0.476 → 0.586 | 23.1% | 0.401 → 0.488 | 22.0% |
| Rust / generic match | 0.477 → 0.559 | 17.3% | 0.400 → 0.461 | 15.3% |
| Rust / compressed match | 0.295 → 0.361 | 22.4% | 0.233 → 0.280 | 19.9% |
| Mata / generic observation | 1.903 → 2.542 | 33.6% | 1.278 → 1.890 | 47.9% |
| Mata / generic match | 1.930 → 2.571 | 33.2% | 1.296 → 1.914 | 47.7% |
| Mata / compressed match | 2.519 → 2.996 | 18.9% | 1.827 → 2.296 | 25.6% |

Off and conditional differ by at most 0.95% on these larger cells, consistent
with the existing cheap conditional reduction remaining in point code.
Projection and component attachment all/off overhead ranges from 0.5% to 6.7%
on their 240/800-row inputs. This does not add MCSE for the extra outputs.
Generic Mata percentage parity with Rust remains unmet; compressed cold
overhead is comparable. No old/new speedup is inferred from different
processor settings or different RNG contracts.

All commands run serially in fresh Stata/MP 19 processes with four processors.
Cold includes loading/preparation; warm follows one all-mode command. There
are three repetitions at 1,200 rows, two at 12,000 and two for attachments.
Timings cover the full `nodisplay` estimator command; CSV writing is outside
the command timer. These are local synthetic timings, not scale guarantees or
statistical confidence intervals. CPU endpoint checks cannot exclude transient
contention. Every timing attempt and its process RSS is retained in
`.local/mcse-default-20260930/performance-v2`.

Cold median RSS for Mata generic observation is 76.8 → 80.4 MiB, generic match
75.0 → 77.9 MiB and compressed match 53.2 → 53.9 MiB. Rust counterparts are
49.6 → 53.3, 51.2 → 54.9 and 45.5 → 47.1 MiB. RSS is a process measure,
separate from direct-allocation forecasts; allocator variation limits small
increment comparisons. Warm RSS includes the all-mode preload for every mode
and therefore cannot isolate memory saved by the toggle.

The first timing preparation was superseded before any measurement to include
typed availability. A resource-metric smoke under the restricted sandbox passed
the Stata assertions but could not read macOS `kern.clockrate`; the same
entrypoint with approved read access passed. Neither is a confirmation failure
or an omitted timing attempt.


Linux job 7802820 stopped before Stata because the dirty-source bundle included
installed plugins, contrary to the qualifier's source-only contract. Its
accounting is `failed=0`, `exit_status=1`. The bundle builder now mirrors
`git archive` export-ignore for plugins; its 11 focused tests pass. The single
four-core retest, job 7803612, passes with `failed=0`, `exit_status=0`,
1,341 seconds wall time and 9.273 GiB reported maximum virtual memory, source bundle
`63c1bc89d470f48adc17114c4f74f07a078b75322122af09f8c3585b857e3e69`.
Linux full Stata/MP 19, native/MCSE/attachment and isolated installation
checks pass. Its immutable 1,305-file source manifest is
`7a2afa4abbeb047f06a79154c5297d391014a9f146391dd5621871139f27bb30`.
Seventeen sanitized logs, wrapper/qualification receipts, scheduler accounting
and the exact candidate are collected and checked. Linux candidate SHA-256 is
`da640fae92e5a24df9e91eeb971c4e62179de3c738bd8e1ccd4163f6d178e3a8`.
The four exact qualified Mac/Linux artifacts are adopted locally in `fevc/`;
the prior five package binaries remain backed up. Windows is unchanged.

The owner selected the existing hosted Windows build and approved private
Windows qualifier. The source enables numerical V2 on existing supported
Windows routes and adds mode/attachment gates to the private driver. Windows
artifact construction, runtime qualification and adoption require the separate
source-publication approval; the private machine remains stopped. No release,
tag, archive, CI-policy change or large cluster campaign was made.

Other diagnostic failures are preserved: initial local qualification startup
path/network failures, the point-only full-CMG count assumption, and the old
point-memory budget assumption. These were harness/fixture issues, not failed
scientific confirmation cells. Final integrated `./.venv/bin/python fevc/tools/run_checks.py` passes: 866
Python tests, CMG assembly/core gates, Stata quick/full, isolated installation,
helper migration and benchmark/preparation validation. Rust workspace/backend,
formatting, strict Clippy and C transport/header checks also pass.

## Current completion and remaining qualification

M0–M4 pass within the recorded local/Counter-V1 dense development scope and
Mac/Linux runtime qualification. An explicitly run Rust full-cache oracle
against independent dense Python complex-step responses passes both deletion
and nuisance modes; it is not inferred from a normally ignored test.
The public help, usage/installation guides, returned-results, memory, inference,
numerical architecture, parity matrix, decisions, changelog and active plan
are updated. Source, manifests, acceptance rules, all attempt inventories and
artifact identities are recorded in
[the current receipt](mcse_default_development_20260930.json).

Windows runtime qualification and the fifth binary remain required to finish
the owner's requested five-platform refresh. Source publication approval is
pending. No public release, representative-scale claim, broad Mata RNG
calibration, native Intel hardware test, or projection/inference-output MCSE
claim is made. Earlier statistical confirmation failures remain unchanged.
