# Symmetric stayer population options — 2026-09-12

Local implementation above `fbf8dcd6187351d09cb3de735c148ef608ec0219`, preserving
the pre-existing uncommitted observation/match solver work. No commit, push,
installed PLUS replacement, public release, paper rerun or private-data run.

## Public contract and compatibility

Both deletion modes default to `stayers(both)`. Explicit `stayers(movers)` means
original movers only in both modes, assessed in the frozen complete-case input.

Observation/both retains its previous default sample and ordinary physical-
observation corrections. Observation/movers now excludes original one-firm
workers before the existing observation graph selector. It does not substitute
the match selector or reclassify graph-dropped movers as stayers. This is an
intentional compatibility change for explicit observation/movers calls: omit
the option or request both to preserve their former retained population.

Match/both retains the pooled mover-match/stayer-observation fit, targets and
mixed corrections; match/movers retains its prior meaning. Symmetric population
options do not mean identical deletion units, graph fixed points or correction
formulas. They do not add a full-CMG solver to the match/stayer hybrid.

The public population choice is separated from the frozen native augmentation
flag. No Rust kernel or ABI was changed for this option repair. Both observation
populations use the ordinary observation engine. Public `e(stayers)`, option
presence and inference-population labels are reconciled independently; raw
native receipts preserve their internal meaning. `e(N_complete)` counts the
original complete cases; `e(N_graph_input)` and `e(N_stayer_option_dropped)`
explain population selection before graph pruning. Display and `estat sample`
do not describe observation/both as a mixed correction.

Existing observation inference follows this same population selection. The
default observation sample/formulas are unchanged. This is not evidence for a
new variance model, better coverage, or match/stayer hybrid component inference.

## Implementation and regression coverage

The public parser, preflight and population poster share the same option rule.
Two installed Ado helpers keep population selection/posting outside the main
program, whose compiled size is close to Stata's limit. The selector reuses
existing dense-map storage and avoids remapping when there are no stayers.
Package manifests, isolated-install gates and the SCC bundle inventory include
both helpers. Shared native state remains reset on success and typed failure.

`test_stayer_option_symmetry.do` uses 48 mover rows and six stayer rows. It
checks omitted versus explicit both, explicit movers versus manual filtering,
all four corrected targets, `e(sample)` and pre-graph counts, Mata exact/JLA,
Rust exact/generic/parallel full CMG and automatic routing. It also checks
controls, literal frequency copies, stored-row target weights, unchanged match
semantics, projection coefficient/covariance equivalence, display, all-stayer
withholding, successful reuse, and caller data/order/RNG/sort-RNG restoration.
Point/projection comparisons use the existing `1e-8` equivalence gate; no
residual, convergence, identification or memory gate was relaxed.

Existing component-inference tests now compare explicit both with the default
instead of expecting the obsolete option rejection. Existing public exact tests
expect the new mover-only population label only for explicit movers calls.

## Validation and attempted checks

Evidence directory: `.local/stayer-options-20260912/` in the software repository.
The complete 35 MiB diagnostic folder was moved intact from its original
`/private/tmp/fevc-stayer-options.AgzUEe/` path after all processes finished;
frozen receipts retain their original execution paths.

- Expanded focused Stata regression: PASS (`symmetry-expanded.log`).
- Python/package tests: 805 passed, repeated after final edits
  (`python-accepted.log`, 76.85 seconds).
- CMG generated-source checks and shell syntax checks: PASS.
- Fresh macOS native/isolated-install qualification: PASS
  (`native-receipt.txt`, `native-accepted.log`, `native-artifacts-accepted/`).
  Pinned formatting, strict Clippy, native Rust/C boundary tests, arm64 and
  Rosetta thin/universal tests, and both isolated-install modes pass. All four
  native artifact/architecture combinations pass the new option regression.
- Full Stata suite: PASS (`stata-full-accepted.log`, exact marker
  `FEVC TEST SUITE PASS: full`).

The native source manifest contains 188 files, SHA256
`cdf9efc0e77a9afeb350045580b259aa7ba83ebfc160dc735b0033a475e84632`.
All 188 rehash unchanged after the run. Subsequent report/index/parity and
project-note changes do not alter these native/package/test inputs. Exact
candidate hashes and sanitized transcripts are retained in the evidence folder;
the qualifier stages ignored source-local developer plugins, not installed PLUS.

Development attempts are preserved, not promoted into benchmark evidence:
the baseline rejected observation/both; initial inline code exceeded Stata's
compiled-program limit; helper/metadata assertions exposed missing public
option-presence posting and private-program visibility. These were fixed by
separate installed helpers. Package tests found missing/unsorted additive
manifest entries, now repaired. Broad Stata tests found obsolete population-
label and observation/both rejection assertions, replaced by positive semantic
checks. Initial native launches stopped for an absent/nonempty evidence
directory and sandbox-blocked official-header download; the launched first
qualification stopped on the obsolete inference rejection test. Fresh evidence
directories retain those attempts separately from the final run.

Native qualification records the dirty source manifest, toolchain, tested
arm64/Rosetta thin/universal binaries and isolated installations. The separate
display/sample file `fevc/fevc_estat.ado`, not listed in the pre-existing native
source inventory, was held unchanged during testing at SHA256
`2ae2ad120b73517b02cf2f0146d72b27ae4527418e2e8c7282b2636908c169b8`.
No Linux/Windows qualification, representative-scale speedup or private Veneto
performance claim is made by these local checks.
