# Fixed-offset match inference: release-candidate finalization

## Current checkpoint

RC preparation source `4d5470f870f350121da7a1c8bf1a625e66e04a4c`
sets `0.5.0-rc.1`, adds a complete five-binary payload constructor and a
guarded Windows build/install smoke, and aligns the Windows C/Rust CRT.
It passes 726 Python tests, integrated Stata quick/full/install checks,
exact-source macOS arm64/Rosetta/universal qualification, SCC Linux job
`7468587` (including public match q0/q1 and isolated installation), and a
fresh dependency/SBOM gate. Windows smoke failed with `STATA_DRIVER_FAILED`;
the machine is stopped and cleanup is complete. Its receipt does not identify
the compiler/Stata failure stage. Sanitized diagnostic and tested-binary
collection needs the separately requested bounded runner extension before
another Windows attempt.
See [RC binary checkpoint](docs/RC_BINARY_CHECKPOINT_2026-09-05.md).
No complete all-platform archive or public release is claimed yet.

The explicit fixed-offset, mover-only match q0/q1 interface is implemented.
Source `53f22a109effee87467b4ef0602b21d0b8ec1ca9` passes public Stata,
clean-source macOS arm64/Rosetta native qualification and isolated installation.
The statistical core and joint-nuisance/combined-population point defaults
are unchanged. The source-bound implementation, scientific compatibility and
artifact identities are in
[`FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md`](docs/FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md)
and its JSON result. The preceding documentation checkpoint is `0adc143`.

The completed documentation cleanup removes obsolete internal-only and
confirmation-pending claims without changing execution or old evidence.
It passes 714 Python tests, CMG checks, integrated Stata quick/full suites,
and clean installation with the updated help. Eleven documentation regressions
protect the supported scope, warnings, examples and package inventory.
[`RC_FINALIZATION.md`](docs/RC_FINALIZATION.md) records the remaining
candidate decisions and verification. On 2026-09-05 the owner authorized
`0.5.0-rc.1` preparation with the complete Mac, Linux and Windows binary
payload, private Linux/Windows tests and exact-artifact checks. Push, tag and
public distribution remain separate owner decisions.

## Accepted scientific scope

- Match q0: independent full confirmation PASS at
  `bb580fe69085d1f98c9151cea2de038aec0a8ba6`; 140,000 target attempts.
  See [match-q0 result](docs/RC_MATCH_Q0_CONFIRMATION_2026-09-05.md).
- Eligible one-mode match q1: repaired independent confirmation PASS at
  `4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`; 140,000 target attempts.
  See [match-q1 result](docs/INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md).
- Corrected observation confirmation: FAIL at
  `73fa75805c8cef6d4d1a6ad843da5894ccedc956`; one firm q1 SE ratio of
  1.101204 exceeds 1.10. All 200,000 attempts were audited. See the
  [confirmation](docs/RC_OBSERVATION_CONFIRMATION_2026-09-05.md) and
  [existing-output diagnosis](docs/RC_OBSERVATION_RATIO_REVIEW_2026-09-05.md).
  The small cutoff excess does not erase the broader calibration limitation.
- The [prospective owner decision](docs/fixed_offset_match_interface_v1.json)
  permits match integration while preserving the observation FAIL; it does
  not approve public release or waive a scientific gate.
- The fixed-offset approximation omits nuisance-control estimation uncertainty.
  Independent matches, a named structured aggregate-variance model and the
  target's q-specific concentration assumptions remain necessary. Few
  controls do not guarantee negligible omitted uncertainty. The
  [paired controls diagnosis](docs/FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md)
  and [deterministic diagnosis](docs/FIXED_OFFSET_DIAGNOSIS_2026-09-04.md)
  remain limitations, not a second-stage correction.

## Remaining bounded work

1. Prepare and qualify the full native payload for macOS arm64/x86_64,
   Linux x86_64 and Windows x86_64; retain the observation-q1 warning and
   current scientific scope. Start with bounded platform smoke gates.
2. Bind the final selected artifact to its source, inventory, notices and
   isolated installation checks. The approved Windows runner currently returns
   only a receipt; bounded tested-binary collection needs separate runner approval.
   The human package-boundary/provenance review is already complete; final
   exact-artifact approval remains distinct.
3. Obtain explicit authorization before any push, tag, publication or native
   binary distribution.

Do not reopen joint-control match inference, combined mover/stayer component
inference, a second-stage correction, automatic q selection, q>1, or a new
Monte Carlo/performance campaign during this milestone. The new public match
boundary is locally qualified on Mac arm64/Rosetta only; old Linux or scaling
evidence does not automatically qualify it.

## Verification and evidence policy

Candidate promotion follows
[`development_acceptance_v1.json`](docs/development_acceptance_v1.json).
Choose gates by affected behavior and record compatible evidence reuse.
Documentation-only changes do not require rebuilding native binaries or
rerunning scientific campaigns. Help/catalog prose can change the portable
package hash while leaving native build inputs and runnable examples intact.

Minimum source gates:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

When local Stata/MP is available, use
`./.venv/bin/python fevc/tools/run_checks.py`. Details and impact-selected
native gates are in [TESTING.md](TESTING.md). Public workflows remain hosted,
source-only and read-only; licensed Stata remains local/private.

## Historical evidence

The former chronological PLAN entries are preserved in Git at `0adc143`.
Their development instructions are superseded, not current work. Original
registrations, failed and passing campaigns, source manifests, reviews and
receipts remain immutable under [docs](docs/README.md) and
`rust/qualification/evidence/`. Durable scientific and engineering contracts
live in the active documentation, not duplicated here.
