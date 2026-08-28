# Accepted SCC CMG candidate qualification

This compact packet preserves the accepted 72-task paired qualification for
candidate source `35db5825c7ee3c20418d58cb005170e6f2d7e459` against comparison
checkpoint `427063bd3ba982d044f6f5b949cf8910ef67ec2d`. The immutable SCC run is
`20260828T171700Z-cmgq-35db582`; preparation job `7349610` and array job
`7349704` completed with 72 wrapper passes, 72 validation passes, and 72
accounting records having `failed=0` and `exit_status=0`.

The aggregate promotion receipt passes every registered gate:

- 8/16-core paired geometric-mean command-time ratio: `0.927446510790391`;
- maximum graph/core median command-time ratio: `1.0046661374735186`;
- one-core command-time geometric-mean ratio: `1.0070740387593988`;
- one-core estimator-phase peak-RSS geometric-mean ratio:
  `0.9999049962448338`;
- connected vector-only eight-core median ratio: `0.9255738270636651`;
- connected vector-only sixteen-core median ratio: `0.8398686783326478`.

The 72 pairs span 22 hosts and eight reported CPU models. Within-task paired
ratios are primary; heterogeneous-host absolute scaling is not inferred from
this qualification.

## Compatibility review

Active repository source `e1514185445cab36f930c90a44c5a4e23326e027` carries
this qualification forward under `development_acceptance_v1.json`. Relative
to the tested source, its changes are confined to the source-local CI receipt,
test-selection and evidence-reuse policy, agent/testing guidance, and benchmark
protocol documentation. No estimator, solver, Ado/native boundary, dependency,
build script, benchmark generator, qualification harness, scientific input,
timing path, binary, or promotion threshold relevant to this claim changed.
Focused policy and qualification-contract tests passed (`26 passed`).

The carried-forward claims are candidate scientific qualification, route and
memory safety for the tested matrix, and the paired promotion result above.
They are invalidated by a later change that can affect estimator/solver
behavior, the native boundary or built binary, qualification inputs, timing
measurement, validation semantics, or promotion thresholds. Historical
receipts retain their exact tested source identity.

## Packet contents

- `collection/`: acceptance receipt, 72 paired rows, and 12 graph/core cells;
- `validations/`: the 72 source-, application-, scientific-, memory-, and
  accounting-bound task validations;
- `qacct/`: the 72 scheduler accounting records;
- `preparation/`: preparation capability, binary, wrapper, and accounting
  receipts;
- `submissions/`: preparation and array resource/submission receipts; and
- `run_identity.json`: immutable source, bundle, manifest, thread, and memory
  identity.

Large task outputs and binaries remain in the immutable SCC run directory and
are not duplicated here.
