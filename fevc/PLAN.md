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
  preceding identities. See [the new candidate record](../native/mean-component-20261008/checkpoint.json).
- The [completed assessment](docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
  accounts for all 48,000 primary calls and 192,000 target rows. **All 22 failed
  primary screen entries are exact-Mata availability failures**, affecting
  highrank k12/k20 and q1 k12 in both arms. The eight native cells have 100%
  interval availability and pass the broad descriptive screens. Stress and
  fixed-outcome numerical profiles are complete; material numerical
  sensitivity and historical calibration failures remain limitations. There
  is no general coverage claim and no changed cutoff or discarded attempt.
- Windows remains blocked: the first source-build smoke failed at
  `build_toolchain` with return code 601; the second failed at `build_native`
  with a null return code. Both attempts completed cleanup and left the guarded
  machine stopped. The prepared hosted-build alternative passed 81 offline
  tests and independent review. The owner has approved the GitHub-hosted build
  and source-first sequence; preparation is underway. Windows remains
  unqualified, and the Mac/Linux candidates remain unadopted.
- The companion technical note in
  `technical-memos/centering/mean_centered_component_inference.tex` is complete,
  with a compiled and visually reviewed 13-page PDF, independent moment checks,
  and the assessment and numerical limits. Unrelated paper edits remain preserved.

## Approved continuation

The owner approved pushing the reviewed source before all five payloads qualify,
using GitHub for the Windows build, then conducting a bounded private Windows
retry on the exact hosted artifact. This replaces the earlier
qualification-before-push order. Source-first preparation is underway; Windows
qualification and new payload adoption remain incomplete. No tag or hosted
release has been made. The immutable
candidate checkpoint preserves the preceding blocked state and failed attempts.

Complete the source review and push, obtain and verify the hosted artifact,
then run the applicable private Windows gate. Finish five-payload
source/provenance, compatibility and HTTP fresh/replacement installation checks
before adoption. Preserve every failed attempt and stop after the authorized
bounded repair/retest if another operational failure occurs. Complete the
package/paper review, commit only intended records and source, and push each
`main` only within the approved sequence. The outgoing four-ancestor audit found
no blocker within its recorded scope; it does not replace review of new changes.

## Preserved previous evidence

`native/prerelease-20261008/manifest.json` retains its original Mac/Linux
point/Mean-projection claims at `24754269` (packaging `623b156d`, follow-up
`7fb65c41`). Its Windows payload remains unqualified. Earlier receipts and
registrations are immutable, including the observation-q1 SE-ratio failure at
1.101204 against 1.10 and the invalidated development pilot aggregate.

Current contracts live in [CENTERING.md](docs/CENTERING.md),
[INFERENCE.md](docs/INFERENCE.md) and [DECISIONS.md](docs/DECISIONS.md).
Engineering gates follow [development_acceptance_v1.json](docs/development_acceptance_v1.json).
