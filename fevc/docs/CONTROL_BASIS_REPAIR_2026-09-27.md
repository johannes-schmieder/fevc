# Control-basis repair qualification — September 27, 2026

The owner authorized investigating and repairing a control-basis certification
refusal in an application with thirteen age/year controls. The accepted
installed dependency remains unchanged pending Linux qualification. Nothing
has been pushed, released or adopted downstream.

## Implementation

- `cc3b3efc381197c9381290802b27e715bfe8d376` adds a direct final-basis
  residual certificate in Mata and Rust. The exact residual identity and
  floating-point enclosure are in [the certificate contract](CONTROL_BASIS_CERTIFICATION.md).
  The model, sample, anchor decisions, computed basis, rank/residual gates and
  downstream `1e-8` ceiling are unchanged.
- `6fc08fee39f9e4c9e751a9f9e44af50f7601e650` preserves signed zero in
  the symmetric Jacobi rotation direction. A 21-dimensional synthetic
  repeated-eigenvalue matrix reproduces the former failure; its independent
  analytic spectrum and twenty fixed coordinate permutations pass with the
  same sweep limit and residual threshold.
- The subsequent test-only correction replaces an obsolete blanket refusal
  for the eight-row anchor witness with an independent literal-deletion
  oracle. Its 120-digit Decimal results are stable at 240 digits and agree
  with a separate 100-digit mpmath implementation. Both exact qualified Mac
  architectures pass the revised regression. Harder-case withholding remains.

## Completed checks

The final source checks pass 831 Python tests, CMG assembly consistency and
all Rust workspace/backend targets, formatting and strict Clippy. The earlier
integrated source run at `cc3b3efc` also passed Stata quick/full, installation,
helper migration and harness checks. After the native changes, the complete
Stata full suite passes in a frozen local tree using the rebuilt development
arm64 plugin, including the corrected anchor regression.

The Mac native profile built and tested thin arm64, thin x86_64 and universal
candidates, arm64 and Rosetta routes, and isolated installations. Its exact
receipt is retained as `LOCAL_CHECKPOINT_DIRTY_TREE`: it started clean at
`6fc08fee`, and the anchor test was edited before it ended. This classification
has not been changed. All 225 files in its source manifest match `6fc08fee`;
production, build code and the profile's executed tests are unchanged. The new
anchor test was separately run against both exact qualified thin artifacts.
The full local suite used a separate development build of the same production
source; its distinct binary hash is retained, not relabeled as the profile's
artifact. These checks do not qualify native Intel hardware or Windows.

The staged downstream application passes all twelve numerical-oracle, two
historical-integration and five comparison/report tests. Each receipt confirms
unchanged sources during the run and identifies the exact candidate dependency.
Application data and results remain outside this public-source repository.

## Linux failure and remaining gate

SCC job `7760783` first exposed the signed-zero eigensolver defect. The repaired
source was tested in job `7760983`, which passed the new public exact/JLA
age/year-control panel, including its independent Mata exact oracle. Its full
suite then stopped at the old eight-row blanket-refusal expectation. Scheduler
accounting records `failed=0`, `exit_status=1`, 788 seconds and `maxvmem=5.767G`;
this was an application assertion, not a scheduler success.

Independent diagnosis shows the newly accepted values satisfy the existing
registered deterministic gate: the largest measured scaled error is about
`1.2e-9`, below `1e-8`. The updated test now checks accuracy directly and passes
locally. The original failed receipts remain failures. No further SCC job has
been submitted after this second qualification failure. A successful Linux
full suite and isolated install are still required before adoption for SCC.

## Evidence and payload handling

Synthetic logs, receipts, source manifests, candidate binaries, scheduler
accounting and the compatibility review are retained locally under
`.local/control-certificate-20260927/`. Its `summary.json` enumerates completed
and pending claims. The Mac qualifier automatically staged its candidates in
the package directory; their bytes were verified against the exported artifacts
and the three previous tracked plugin payloads were restored. All rebuilt
candidates remain separately available for qualification and authorized adoption.
Historical qualification receipts and the installed downstream payload remain
unchanged. This record is development evidence, not a public release decision.
