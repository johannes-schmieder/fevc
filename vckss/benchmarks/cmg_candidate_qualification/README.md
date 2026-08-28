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

The graph grid uses comparative-input receipt V4. Its weak family is the
versioned `hub_ring_leaveout_bottleneck_v1` topology. It uses one hub plus one
ring firm per worker; each worker connects to the hub, its own ring firm, and
the next ring firm. At 1,966,080 rows it has 655,360 workers and 655,361 firms.
Every ring firm has two incidences, so the degree-three graph is connected and
leave-out redundant. Its sparse canonical graph exceeds CMG's frozen
350,000-edge connected-vector threshold without a retained hierarchy operator,
while its hub makes the unchanged numerical gate well conditioned.

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
are rejected. The hub-ring topology replaces that non-discriminating design;
no scientific gate or CMG routing threshold has been relaxed.

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
receive the same hash-bound `VCKSS-BENCHMARK-THREADS-V1` adapter, allowing their
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
requires diagnosis and a corrected candidate under a new immutable run/source
identity; the comparison checkpoint is not promoted as a fallback.

Stage and deploy from a clean checkout:

```bash
./.venv/bin/python vckss/benchmarks/cmg_candidate_qualification/build_run.py \
  --repo "$PWD" --output /private/tmp/RUN_ID \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112
vckss/benchmarks/cmg_candidate_qualification/deploy_scc.sh /private/tmp/RUN_ID
```

On SCC, submit preparation, wait for it to leave the queue, and run
`collect_preparation_qacct.sh RUN PREPARATION_JOB_ID`. Qualification submission
is blocked until that immutable accounting receipt, its four-processor Stata
capability subreceipt, and both benchmark-adapter receipts pass. After all 72
tasks leave the queue and their
accounting is available,
`collect_qacct.sh RUN ARRAY_JOB_ID` validates every task and applies the
promotion gate.

The current SCC rejection packet is
[`evidence/scc/9b3b4d5cf210bb788d20eb508be69f8e48e79e60/`](evidence/scc/9b3b4d5cf210bb788d20eb508be69f8e48e79e60/).
It proves the fail-fast four-versus-16 processor-license gate and contains no
accepted estimator or performance evidence.
