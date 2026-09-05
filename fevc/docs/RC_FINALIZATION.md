# Release-candidate finalization

This is the active remaining-work checklist, not a release authorization.
The owner subsequently authorized preparation of `0.5.0-rc.1` with the
complete Mac/Linux/Windows native payload and private Linux/Windows tests
on 2026-09-05. The decision table below records the preceding recommendation;
its source-only/Mac-only options are superseded by that request. Public
distribution and tagging remain unauthorized. The Windows runner's bounded
artifact-collection extension is a separate infrastructure decision.
It supersedes old development instructions in PLAN and historical checklists.
The exact-source [match interface qualification](FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md)
and all earlier scientific receipts remain unchanged.

## What is ready

- Explicit fixed-offset, mover-only match q0 and eligible one-mode q1 are
  integrated in the Stata/Rust interface, with structured-common and
  leverage-only models, diagonal/CMG, frequency mass, target weights and
  declared match IDs. Defaults and point estimates are unchanged.
- Independent match confirmations pass in their registered regimes. Native
  arm64 and Rosetta, public inference and isolated native installation pass
  at `53f22a109effee87467b4ef0602b21d0b8ec1ca9`.
- The required offset-uncertainty warning, match diagnostics, unsupported
  combinations and unresolved observation-q1 calibration failure are visible
  in help, returned metadata and display.
- The owner-selected GPL-3.0-only package boundary and human source/provenance
  review are already complete. They should not be described as an unfinished
  general license decision. Final approval of the actual distribution remains
  separate; see `../../CODE_LICENSE.md`.

## Remaining decisions and packaging work

| Item | Current state | Recommended bounded next action |
| --- | --- | --- |
| Scientific release scope | Match integration approved; corrected observation q1 retains one failed SE-ratio gate | Center this RC on fixed-offset match q0/q1. Retain the observation route's explicit warning and describe its calibration as provisional, unless the owner prefers a separate restriction. Do not turn the failed result into PASS. |
| Installation payload | The portable `fevc.pkg` installs Ado/Mata and native helpers, **not a Rust binary** | Choose source-only with an explicit native-build prerequisite, or source plus separately approved qualified Mac binaries. A portable-only installation cannot run the Rust-only match component-inference request. |
| Platforms | This match interface has native arm64/Rosetta evidence, not new Linux/Windows/native-Intel evidence | Keep the initial native candidate's claim Mac-only. A new platform claim requires its own affected-boundary and clean-install qualification, not a new statistical campaign. |
| Version and changelog | Public source identifiers remain `0.5.0-alpha.1`; no RC version assigned | Confirm whether to use `0.5.0-rc.1`. Then synchronize the public version/date, catalog, help, Python package metadata, tests and release notes. Internal Rust package/API versions are separate identities, not automatic replacements. |
| Exact final artifact | No distributable is staged or approved in this pass | After scope/version/payload approval, freeze a clean source, construct the selected artifact, record SHA-256 and inventory, and install **that artifact** into an empty Stata PLUS directory. Check help, no-native behavior and q0/q1 where the payload includes native support. |
| Notices and final sign-off | Historical human review and automated source audits pass | Verify the chosen payload's actual source correspondence and notices. If binaries are included, bind them to the qualified build/source and review dependency/SBOM evidence and any needed current checks. Obtain exact-artifact approval before distribution. |

My recommendation is a scoped first RC with unchanged point defaults,
fixed-offset match inference as the newly qualified feature, and the existing
observation-q1 warning retained. Source-only is viable for technical testers
who build Rust; a source-plus-Mac-native candidate is more convenient for
testing the feature from a fresh installation. Neither choice implies
Linux/Windows support or authorization to publish binaries.

No estimator rewrite, joint-controls experiment, second-stage correction,
new Monte Carlo campaign or broad performance matrix is needed solely to
finish this scoped RC. Representative-data performance can be a later task;
do not claim new million-observation inference performance without measuring it.

## Documentation-only cleanup and compatible evidence reuse

The cleanup starts at `0adc143` and retains native executable source
`53f22a109effee87467b4ef0602b21d0b8ec1ca9`. Its source identity is the commit
containing the cleanup verification record below, resolvable in Git; the
qualified native receipt continues to name 53f22a1, not the documentation SHA.

Changed active paths are `README.md`, `fevc/PLAN.md`, `fevc/docs/README.md`,
`fevc/docs/DECISIONS.md`, `fevc/docs/INFERENCE.md`,
`fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md`, `fevc/fevc.sthlp`,
`fevc/fevc.pkg`, `fevc/stata.toc`, `rust/README.md`, `rust/TEST_PLAN.md`,
this checklist and `fevc/tests/python/test_match_documentation.py`.
The old chronological PLAN remains available at 0adc143; scientific records
were not deleted or edited.

Only help prose and catalog descriptions differ among the 142 files in the
53f22a1 native source manifest. The installed file inventory, every runnable
help example, all Ado/Mata/Rust/C code, ABI, build inputs, dependency locks,
native binaries, scientific inputs and acceptance rules remain unchanged.
The portable package hash changes because help and catalog text are part of
that package; the old portable hash must not identify the updated package.

Proportional validation uses documentation regressions, the full Python and
CMG source gates, public identity/history/license/parity and deterministic
package checks, plus local Stata installed-help and package checks. Native
behavior and scientific claims carry forward only within the original scope;
this prose change does not justify another native build or scientific run.

## Verification checkpoint

Completed on 2026-09-05:

- `./.venv/bin/python fevc/tools/run_checks.py`: PASS. This includes 714
  Python tests (11 new documentation regressions), deterministic CMG output,
  CMG component checks, public identity/history/license/parity audits, Stata
  quick/full suites, installed-help and clean-install tests, and benchmark/
  preparation smokes. Python 3.13.0, pytest 7.4.4 and local Stata/MP 19 were
  used. The terminal marker is `FEVC LOCAL QUALIFICATION PASS`.
- All 144 local Markdown link targets in changed guidance resolve;
  `git diff --check` passes. All five runnable help examples remain byte-for-
  byte identical to the qualified source, and the package file list is intact.
- Native-manifest review: 139 of 142 files rehash unchanged; the only
  differences are prose in `fevc/fevc.sthlp` and descriptions in `fevc/fevc.pkg`
  and `fevc/stata.toc`. No Ado/Mata/Rust/C, ABI, build or dependency file changes.
  All three exact tested Mac binary hashes and all five registered scientific
  result/registration hashes in `fixed_offset_match_interface_v1_result.json`
  revalidate unchanged.
- The deterministic, in-memory portable-package check passes with 41 files
  and SHA-256
  `523b73e1be14a8d0d67aafd08173bb364d70a3fd08214fb8bc5f19c5e0d99e73`.
  This is a content/determinism check, not a staged or approved release archive.

No new Rust build, broad platform/safety matrix, coverage campaign or
performance qualification was run for this prose-only change. The existing
53f22a1 source-bound native and scientific evidence is carried forward with
the limits above. Old receipts retain their source and status. No push, tag,
publication, native binary distribution or release staging occurred.
