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
- All five qualified payloads are adopted and published in `66d0278b`. Mac
  thin arm64, thin x86-64 under Rosetta and universal builds retain full qualification at
  `b9f80ce9` and installed Mean checks across all four aliases. Linux retains
  full and staged/installed Mean qualification at `63757839` (SCC job
  `7969972`). Their original bytes and corresponding-source archives are
  retained under an affected-route compatibility review limited to unchanged
  public routes. They do not contain the lower-level serial exact Corrected
  repair in `240ad74d`. The [adoption manifest](../native/mean-component-20261008/manifest.json)
  binds their original identities and the new Windows candidate.
- The [completed assessment](docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
  accounts for all 48,000 primary calls and 192,000 target rows. **All 22 failed
  primary screen entries are exact-Mata availability failures**, affecting
  highrank k12/k20 and q1 k12 in both arms. The eight native cells have 100%
  interval availability and pass the broad descriptive screens. Stress and
  fixed-outcome numerical profiles are complete; material numerical
  sensitivity and historical calibration failures remain limitations. There
  is no general coverage claim and no changed cutoff or discarded attempt.
- Reviewed source `240ad74d` is pushed. Its hosted Windows build, Rust and
  source checks pass. The identical Windows artifact passes private smoke and
  full qualification; full run `win-20261009T023703Z-209dc9ed` includes installed
  Mean component and Corrected exact checks. Cleanup/source restoration pass
  and the machine is stopped. The [Windows summary](../native/mean-component-20261008/evidence/windows/qualification-240ad74d.json)
  records four of the 15 authorized infrastructure repair cycles and retains
  all prior failed attempts.
- Local forced-Windows-serial execution reproduces the first Corrected cell's
  `CORRECTION_NONFINITE` failure (native status 71, `exact_stayer_hybrid`). The
  repair at `240ad74d` materializes the scalar total before checked Corrected
  addition, preserving final reconstruction and numerical gates. Focused
  checks pass: 890 Python tests, CMG assembly, 23 existing exact-core tests,
  15 exact FFI tests, a 36-cell serial/parallel accounting regression, strict
  core Clippy and formatting, and the unchanged official 12 Corrected and
  12 Mean-component cells under serial emulation. The new regression fails
  when the three-line repair is removed. Final independent source review
  passes, followed by actual Windows qualification of the rebuilt artifact.
- [Local staging and HTTP fresh/replacement checks](../native/mean-component-20261008/evidence/packaging/local-install-240ad74d.json)
  pass at `240ad74d` on Mac arm64: both cases verify all 61 files and five
  plugin hashes and run eight installed Mean-component cells. These receipts
  retain the help bytes at that source. The adopted package is published at
  `66d0278b`. Its four public `net`/`github` fresh/replacement cases have status
  PASS on Mac arm64 under Stata 19, with all 61 files, five plugin hashes
  and eight Mean-component cells per case. The [public-install summary](../native/mean-component-20261008/evidence/packaging/public-install-66d0278b.json)
  and [exact binding](../native/mean-component-20261008/evidence/packaging/public-install-binding-66d0278b.json)
  bind the tested source and final help bytes; this does not qualify other
  platform runtime or native Intel hardware. Source checks `37877625153` and
  all seven Rust-backend CI jobs in run `37877625154` also pass at `66d0278b`.
- The companion technical note in
  `technical-memos/centering/mean_centered_component_inference.tex` is complete,
  with a compiled and visually reviewed 13-page PDF, independent moment checks,
  and the assessment and numerical limits. The scoped five-file paper follow-up
  is committed and pushed at `9c6b27540485ec17aa1511f7dce502d67c5bbd93`;
  built-in compilation, PDF export and visual review pass. Scientific content
  and the 12 companion files remain unchanged; unrelated paper edits are
  preserved.

## Completed implementation and publication

The owner approved the source-first GitHub build/private Windows sequence and
up to 15 infrastructure repair cycles. Four were used before successful final
qualification; the serial Corrected repair was a separate project source
change. The later authority superseded the earlier single-retry stopping
instruction. Source and paper pushes are complete. All five native payloads,
source archives and reviewed local installation evidence are published at
`66d0278b`.
No tag, native release archive or hosted release has been made.

The adopted package and documentation were reviewed, committed and pushed at
`66d0278b`. All four public installation cases pass, with exact source and
installed-file bindings in the records above. This follow-up records those completed checks without
changing any installed file. Preserve all prior failures and original
Mac/Linux source identities and reuse limits. No scientific or platform scope
is expanded by the public Mac arm64 installation check.

## Preserved previous evidence

The immutable candidate checkpoint retains the initial Windows source-build
failures: `build_toolchain` return code 601 and `build_native` null return code,
both with cleanup and the machine stopped. The hosted-build alternative passed
81 offline tests and independent review. The later prebuilt smoke
`win-20261008T235353Z-856fd74e` failed artifact collection with zero accepted
artifacts despite positive controller Stata/project-test flags; cleanup and
shutdown passed. Subsequent repairs and the passing smoke do not relabel these
attempts. At `f8d74504`, smoke `win-20261009T014353Z-e95b12a1` passed,
but full run `win-20261009T015013Z-79411bd4` failed at `centering_exact` with
return code 498. That failure remains unchanged after the repaired-source
Windows retest passed.

`native/prerelease-20261008/manifest.json` retains its original Mac/Linux
point/Mean-projection claims at `24754269` (packaging `623b156d`, follow-up
`7fb65c41`). Its Windows payload remains unqualified. Earlier receipts and
registrations are immutable, including the observation-q1 SE-ratio failure at
1.101204 against 1.10 and the invalidated development pilot aggregate.

Current contracts live in [CENTERING.md](docs/CENTERING.md),
[INFERENCE.md](docs/INFERENCE.md) and [DECISIONS.md](docs/DECISIONS.md).
Engineering gates follow [development_acceptance_v1.json](docs/development_acceptance_v1.json).
