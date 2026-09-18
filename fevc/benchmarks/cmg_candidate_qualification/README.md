# CMG candidate qualification

This source-bound gate compares the integrated CMG routing candidate against
VCkss comparison checkpoint `427063bd3ba982d044f6f5b949cf8910ef67ec2d`.
The candidate must descend from integration checkpoint
`170e34bf060291fec1506c13ab2c60428b3f574a`; its exact tested commit is the
clean repository `HEAD` recorded by `build_run.py`. The embedded CMG identities
are candidate `92a12f2d572ca56b30a035220953f9dd4bced999` (direct descendant of
upstream performance commit `d9fef06`) and comparison
`761a0f022f20d1114d9f20589b60563eab6fcb84`.

The frozen matrix is:

```text
4 graph families x 1,966,080 stored rows x 1/8/16 cores
  x 6 AB/BA-balanced repetitions = 72 paired SCC tasks
72 tasks x 2 fresh VCkss processes = 144 estimator calls
```

The graph grid uses comparative-input receipt V6. Its weak family is the
versioned `shallow_hub_tree_leaf_panel_vector_v1` topology. Five worker panels
share one leaf and use five spokes around one of 40 branch hubs in a depth-two
tree with one root and 39 grandchildren per branch. A stride-five pattern
exposes every one of the 1,601 hubs even at the smallest registered size; seven
patterns include the root. At 1,966,080 rows it has 655,360 workers, 131,072
leaf firms, 132,673 total firms, and 788,032 canonical edges. A standalone CMG
probe records one first-level full contraction, zero retained operators, zero
plan bytes, and candidate planned execution. These dimensions clear both
frozen connected-vector floors: 350,000 edges and 131,072 vertices.

Qualification generation `20260828T0434Z-cmgq-b168c98` demonstrated that the
earlier pure ring could exhaust unchanged same-route complete-residual
refinement in both the candidate and comparison checkpoint. Pilot
generation `20260828T0601Z-cmgqpilot-87866ef` then showed that adding only 32
diameter chords was insufficient: task 55 still failed the unchanged complete
residual gate. Pilot `20260828T0619Z-cmgqpilot-0deed11` was cancelled before
completion when an audit found that one active benchmark README still described
that superseded chord topology, even though the generator and executable
contracts had changed. An internally inconsistent immutable source cannot
support a result. No result from any of these rejected generations is accepted.
Pilot `20260828T0638Z-cmgqpilot-0e3dd26` then proved that the two-block repair
passed its eight-core residual, application, and wrapper gates but built a full
CMG hierarchy plan in both frozen sources (`cmg_plan_bytes=155395972`, three
planned batches each). It was therefore not a candidate-only connected-vector
cell and array `7343616` was cancelled. Its timings are also rejected.
Pilot `20260828T0653Z-cmgqpilot-26ed9fb` showed why reweighting one antipodal
matching was insufficient: task `7343638.55` failed the unchanged complete
residual gate at `1.9831122449681996e-5`. Array `7343638` was cancelled and no
timing is accepted. Pilot `20260828T0705Z-cmgqpilot-0b62c1a` then showed that
eight distinct offsets passed the eight-core residual gate
(`9.93260293194e-6`) but still did not identify the candidate route: both
sources were serial with zero planned batches, while both retained the same
864,260-byte plan. Array `7343654` was cancelled and its provisional timings
are rejected. Pilot `20260828T0731Z-cmgqpilot-3a23602` then tested the
operator-free hub-ring repair after exact-source local qualification and
licensed CI run `33151689922`. Preparation job `7343690` passed, but comparison
task `7343701.61` exhausted three refinements with complete residual
`1.7926322539575545e-4`. Array `7343701` was cancelled and no timing is
accepted. Diagnosis established that CMG's vector route also requires at least
131,072 vertices, so earlier low-firm-count operator-free designs could never
discriminate the candidate. The six-hub leaf-panel topology replaces the
rejected hub-ring and clears that floor with substantially more balanced hub
degrees. Exact source `c0d9345e3a818f48534a2a6d90cb762f0d06629d`
passed full local qualification and licensed CI run `33153647170`; preparation
job `7343745` passed, but comparison task `7343749.55` failed after three
refinements with reduced residual `5.3625967261654e-5` and complete residual
`2.3704048296633303e-5`. Array `7343749` was cancelled and no timing is
accepted. The replacement shallow hub tree retains the routing properties but
reduces hub concentration; its final all-sizes mapping passed the actual local
one-core 200-probe estimator screen at maximum complete residual
`6.50839354464e-6`. No scientific gate or CMG routing threshold has been
relaxed.

Exact source `0df7a481166735ffea3bc3f9cb4d194684910702` passed complete local
qualification, licensed CI run `33156008043`, preparation job `7344203`, and
the largest-case 1/8/16-core pilot array `7343969`. Full array `7344263`
produced 72 passing scientific and wrapper results, but SCC omitted the
original accounting records for tasks 13 and 14. Retry array `7344638`
accepted only those two tasks after their source, bundle, manifest row,
binaries, and literal input matched exactly; both retry accounting records have
`failed=0` and `exit_status=0`. The completed generation was still rejected:
the 8/16-core paired geometric-mean command-time ratio was `1.003057` against a
`1.00` limit, and the connected-vector eight-core median ratio was `1.000382`
when strict improvement was required. No result from that generation is
promoted. The next candidate parallelizes independent Schur-RHS construction
while preserving each column's arithmetic order, cancellation behavior, and
the public receipt schema; it must pass under a new immutable source identity.

Each task generates one literal input, atomically claims one 16-core block on
its assigned host, selects the target CPU subset from that block, and runs
comparison and candidate sequentially on those same CPUs and host. A host-local
`flock` prevents two VCkss jobs owned by this user from claiming the same block;
the lock is released automatically when the task exits. This allows both halves
of a 32-core node to be used without a queue, host, or CPU-type restriction even
when SCC exposes the whole host rather than enforcing the submitted binding
hint. Three repetitions use comparison--candidate order and three use
candidate--comparison order. No task requests a queue, host, CPU model or
architecture, exclusive node, or buy-in resource. The 72-task array has no
client-side concurrency throttle. Both SGE scripts begin with `-clear` before
declaring their own resources. SCC's mandatory global JSV subsequently injects
soft `buyin=TRUE` into every batch job; the harness does not request it, and it
does not restrict queue or host eligibility. Every job is initially held,
captures and validates its effective `qstat` specification, records that JSV
injection, and is released only after the hard-resource contract passes.
All SCC-side Python entry points explicitly load and verify
`python3/3.12.4`; they never depend on SCC's default Python 3.6.
Before either plugin is built, the four-slot preparation job loads the pinned
Stata/MP 19 module and records `c(processors_lic)` in a hash-bound capability
receipt. Stata is capped at `min(target,4)` and preparation requires only a
four-processor entitlement. Both comparison and candidate benchmark artifacts
receive the same hash-bound `FEVC-BENCHMARK-THREADS-V1` adapter, allowing their
Rust backends to use the full 1/8/16 target while leaving the public package and
ordinary `c(processors)` behavior unchanged. An insufficient license or either
adapter mismatch produces `wrapper.fail` and nonzero accounting before a
72-task array can be submitted; reserved slots alone do not satisfy the gate.

Every call must pass the public Rust route, `CMG_FULL_V2`, source identity,
requested/used threads, complete original-system residual, target identity,
sample, data/RNG/sort restoration, lifecycle, memory envelope, wrapper,
application, and scheduler-accounting gates. Corrected candidate/comparison
targets must fall inside the registered six-MCSE paired envelope.
Each Rust call also records and validates the documented eight-column native
phase profile (ingest, canonicalize, graph, compress, plan, stayer
augmentation, solve, and native total). The full-CMG early-return route does
not expose the legacy VCkss scalar phase decomposition, so the qualification
does not relabel or fabricate those fields.

Promotion requires all of the following without relaxation:

- candidate/comparison geometric-mean command-time ratio at 8/16 cores at most
  1.00;
- no graph/core cell median ratio above 1.05;
- every observed connected vector-only cell improves at both 8 and 16 cores,
  with at least one such cell at each core count;
- one-core geometric-mean command-time ratio at most 1.03;
- one-core geometric-mean estimator-phase peak-RSS ratio at most 1.05.

`aggregate.py` writes the 72 paired-task rows, 12 graph/core cells, and one
machine-readable acceptance receipt only when every gate passes. A failure
requires diagnosis; a candidate/runtime change that can affect these results
requires a new immutable run/source identity, and the comparison checkpoint is
not promoted as a fallback.

This matrix is an intentional performance benchmark, so its initial run is a
valid reason for large SCC work. It is not a standing requirement to rerun all
72 tasks after every later commit. Under the repository development acceptance
policy, documentation, tests, CI, packaging, provenance, or unrelated
benchmark-workflow changes reuse the accepted qualification when a recorded
compatibility review shows that the candidate and comparison production/build
bytes, binaries, scientific inputs, timing path, validator semantics, and
promotion thresholds relevant to the claim are unchanged. A change confined
to evidence collection or reporting should first revalidate retained receipts;
a bounded harness change should use focused tests or affected-cell pilots. A
full rerun is reserved for changes that can affect the estimator, binary,
qualified route, input, timing measurement, or promotion conclusion, or for an
explicit owner request.

Stage and deploy from a clean checkout:

```bash
./.venv/bin/python fevc/benchmarks/cmg_candidate_qualification/build_run.py \
  --repo "$PWD" --output /private/tmp/RUN_ID \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112
fevc/benchmarks/cmg_candidate_qualification/deploy_scc.sh /private/tmp/RUN_ID
```

On SCC, submit preparation, wait for it to leave the queue, and run
`collect_preparation_qacct.sh RUN PREPARATION_JOB_ID`. Qualification submission
is blocked until that immutable accounting receipt, its four-processor Stata
capability subreceipt, and both benchmark-adapter receipts pass. After all 72
tasks leave the queue and their
accounting is available,
`collect_qacct.sh RUN ARRAY_JOB_ID` validates every task and applies the
promotion gate.

The accepted SCC packet is
`evidence/scc/35db5825c7ee3c20418d58cb005170e6f2d7e459/` (historical path; see [archive access](../../../docs/ARCHIVE.md)).
Preparation job `7349610` and array `7349704` produced 72 wrapper, validation,
and clean-accounting passes. The parallel command-time geometric-mean ratio is
`0.927447`, the worst graph/core median is `1.004666`, one-core time and phase-
RSS ratios are `1.007074` and `0.999905`, and connected vector-only medians are
`0.925574` at eight cores and `0.839869` at sixteen. Candidate `35db582` is
promoted. The later policy-only source carries these claims forward under its
recorded compatibility review without another large qualification run.

The historical SCC rejection packet is
`evidence/scc/9b3b4d5cf210bb788d20eb508be69f8e48e79e60/` (historical path; see [archive access](../../../docs/ARCHIVE.md)).
It proves the fail-fast four-versus-16 processor-license gate and contains no
accepted estimator or performance evidence.
