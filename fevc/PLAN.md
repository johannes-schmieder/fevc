# Mean-centered component inference — October 8, 2026

The owner accepted all five recommendations in
[the implementation plan](docs/MEAN_COMPONENT_INFERENCE_PLAN.md): highrank and
q1 on existing supported routes, the Mean default with a fixed-observed-mean
approximation, independent tests and a bounded sampling assessment, all-platform
native qualification/adoption, and reviewed package/paper commits and pushes.
Work stays on `main`; tags and hosted releases remain separate decisions.

## Current checkpoint

- Source `63757839` implements Mean component inference and discloses omitted
  mean-estimation uncertainty. Gaussian error probes remain uncentered.
  Corrected inference and existing unsupported tuples remain withheld.
  Native `component_centering_api=1` uses readiness bit 16 through the existing
  capability interface, with no ABI layout or export-inventory change.
- Independent exact/native fixed-c oracles, q1 imperfect-solve and fault tests,
  caller-state/capability regressions and pinned clean Rust source gates pass.
  The integrated checker passed on the earlier staged development tree; its
  receipt is not relabelled as a clean-source integrated run. The final 32
  harness tests pass at `b9f80ce9`, with identical harness bytes at `63757839`.
- The shared None/Mean q1 certificate repair accounts for both accepted solves'
  signed normal-equation residuals at unchanged tolerances. Public raw
  remainder discrepancies, estimator values and solver gates remain intact.
  The preserved failing draw and independent dense checks pass after repair.
- Mac thin arm64, thin x86-64 under Rosetta and universal candidates pass full
  clean-source qualification at `b9f80ce9` and separate exact-byte installed
  Mean checks across all four architecture aliases. Linux passes full and
  staged/installed Mean qualification at `63757839` (SCC job `7969972`). These
  candidates are preserved and **not adopted**; shipped payloads retain their
  preceding identities. Reuse after the Windows serial-path repair is
  limited to the unchanged public routes and requires a recorded compatibility
  review. The retained binaries do not contain that lower-level repair and must
  not be described as builds of the repaired source. See
  [the candidate checkpoint](../native/mean-component-20261008/checkpoint.json).
- The [completed assessment](docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
  accounts for all 48,000 primary calls and 192,000 target rows. **All 22 failed
  primary screen entries are exact-Mata availability failures**, affecting
  highrank k12/k20 and q1 k12 in both arms. The eight native cells have 100%
  interval availability and pass the broad descriptive screens. Stress and
  fixed-outcome numerical profiles are complete; material numerical
  sensitivity and historical calibration failures remain limitations. There
  is no general coverage claim and no changed cutoff or discarded attempt.
- Source `f8d74504` is pushed. Its GitHub Windows build (`37859073572`), Rust
  checks (`37859048705`) and source checks (`37859048711`) pass. After four
  infrastructure repair cycles, the private Windows smoke
  `win-20261009T014353Z-e95b12a1` passes on the exact hosted artifact. The full
  run `win-20261009T015013Z-79411bd4` fails at `centering_exact` with return code
  498. Cleanup and source restoration pass; Windows remains unqualified.
- Local forced-Windows-serial execution reproduces the first Corrected cell's
  `CORRECTION_NONFINITE` failure (native status 71, `exact_stayer_hybrid`). The
  candidate repair materializes the scalar total before checked Corrected
  addition, preserving final reconstruction and numerical gates. Focused
  checks pass: 890 Python tests, CMG assembly, 23 existing exact-core tests,
  15 exact FFI tests, a 36-cell serial/parallel accounting regression, strict
  core Clippy and formatting, and the unchanged official 12 Corrected and
  12 Mean-component cells under serial emulation. The new regression fails
  when the three-line repair is removed. Final independent source review
  passes; a rebuilt Windows artifact and runtime qualification are still
  required before adoption.
- The companion technical note in
  `technical-memos/centering/mean_centered_component_inference.tex` is complete,
  with a compiled and visually reviewed 13-page PDF, independent moment checks,
  and the assessment and numerical limits. The reviewed paper changes are
  pushed; unrelated paper edits remain preserved.

## Approved continuation

The owner approved the source-first GitHub build and private Windows sequence,
then authorized up to 15 repair cycles. Four infrastructure cycles have been
consumed. This later authority supersedes the earlier single-retry stopping
instruction. Source and paper pushes are complete through the checkpoint above;
the source repair has passed independent review and focused checks. Windows
qualification and all five new payload adoptions remain incomplete. No tag or
hosted release has been made.

Commit and push the reviewed source repair, then build
and verify its exact hosted Windows artifact and run the applicable private
smoke and full gates within the remaining authorized cycles. Preserve every
failed attempt. Record the scope-based compatibility review for the retained
Mac/Linux candidates, including their original source and payload identities
and the serial repair they do not contain. Complete five-payload provenance,
source archives, staged installation and final public HTTP fresh/replacement
checks against the actual adopted package bytes. The final 16-path active-docs
update remains held until qualification and local installation checks pass.
Commit only intended records
and changes, and push within the approved sequence. The outgoing four-ancestor
audit found no blocker within its recorded scope; it does not replace review
of new changes.

## Preserved previous evidence

The immutable candidate checkpoint retains the initial Windows source-build
failures: `build_toolchain` return code 601 and `build_native` null return code,
both with cleanup and the machine stopped. The hosted-build alternative passed
81 offline tests and independent review. The later prebuilt smoke
`win-20261008T235353Z-856fd74e` failed artifact collection with zero accepted
artifacts despite positive controller Stata/project-test flags; cleanup and
shutdown passed. Subsequent repairs and the passing smoke do not relabel these
attempts or the later full-run Corrected failure.

`native/prerelease-20261008/manifest.json` retains its original Mac/Linux
point/Mean-projection claims at `24754269` (packaging `623b156d`, follow-up
`7fb65c41`). Its Windows payload remains unqualified. Earlier receipts and
registrations are immutable, including the observation-q1 SE-ratio failure at
1.101204 against 1.10 and the invalidated development pilot aggregate.

Current contracts live in [CENTERING.md](docs/CENTERING.md),
[INFERENCE.md](docs/INFERENCE.md) and [DECISIONS.md](docs/DECISIONS.md).
Engineering gates follow [development_acceptance_v1.json](docs/development_acceptance_v1.json).
