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

Each task generates one literal input, selects one active CPU subset from 16
bound slots, and runs comparison and candidate sequentially on those same CPUs
and host. Three repetitions use comparison--candidate order and three use
candidate--comparison order. No task requests a queue, host, CPU model or
architecture, exclusive node, or buy-in resource. The 72-task array has no
client-side concurrency throttle. Both SGE scripts begin with `-clear` before
declaring their own resources. SCC's mandatory global JSV subsequently injects
soft `buyin=TRUE` into every batch job; the harness does not request it, and it
does not restrict queue or host eligibility. Every job is initially held,
captures and validates its effective `qstat` specification, records that JSV
injection, and is released only after the hard-resource contract passes.

Every call must pass the public Rust route, `CMG_FULL_V2`, source identity,
requested/used threads, complete original-system residual, target identity,
sample, data/RNG/sort restoration, lifecycle, memory envelope, wrapper,
application, and scheduler-accounting gates. Corrected candidate/comparison
targets must fall inside the registered six-MCSE paired envelope.

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
is blocked until that immutable accounting receipt passes. After all 72 tasks
leave the queue and their accounting is available,
`collect_qacct.sh RUN ARRAY_JOB_ID` validates every task and applies the
promotion gate.
