# Current objective and checkpoint — 2026-09-12

Integrate the approved thread-aware batching and ordered scalar work queue
over degree-four baseline `82be182`. Mixed-degree 2–8 paired SCC checks at
1.6m and 6.4m support adoption; fusion stays experimental. The fixed policy
scales with permitted thread count and retains explicit-budget planning,
input-order results/errors, cancellation and original-system residual gates.
The owner authorized integration, commit and push on September 12.

Rust workspace tests, strict Clippy, formatting and Python tests pass.
Fresh integrated native/package qualification is pending at this source
checkpoint; previous binary receipts do not qualify the new runtime.
See the [integration record](docs/BATCH_QUEUE_2026-09-12.md) for scope,
accepted performance evidence and updated qualification results. The
[degree-four report](docs/DEGREE_FOUR_CMG_2026-09-11.md) remains historical.
No full figure campaign or public binary release is authorized.

## Preserved source-readiness objective

Prepare the repository for public source development: keep current guidance
concise, separate historical evidence, remove disposable local clutter, and
check the current tree and reachable history. The cleanup record and remaining
publication boundary live in [public-source preparation](docs/PUBLIC_SOURCE_READINESS.md).

## Validated implementation

The control-basis repair and 256-lane optimization are committed at
`929a1d77a2fec6e9e1a78550a84d1a65b2ad51fd`. The uncentered 50,000-row AKM
example succeeds through Mata, Rust and automatic routing. Compensated
arithmetic, decisive-prefix certification, identification checks and typed
inverse-residual ambiguity are retained. Runtime identities are Mata API 24
(`vckss-api24-control-lanes256`) and resource API 12
(`vckss-resource-api12-control-scratch`).

The checkpoint passed 799 Python tests, CMG assembly, Stata quick/full and
clean-install checks, focused arithmetic and invariance tests, and local
arm64/Rosetta qualification and interruption checks. On the accepted
100,000-row, 32-control case, median preparation fell from 2.388 to 1.940 seconds
and command time from 20.732 to 20.342 seconds, with about 22 MiB additional
peak RSS charged before allocation. See the
[follow-up report](docs/CONTROL_LANES_256_2026-09-10.md) and
[source/binary record](docs/control_lanes_256_v1_result.json).
The original failures and 13.2% overhead remain in the earlier repair record.
This is a local checkpoint, without release or competitive-performance promotion.

Optional memory budgets and the approximate-inference interface are documented
in the [memory guide](docs/MEMORY.md) and
[inference contract](docs/INDIVIDUAL_INFERENCE_INTERFACE.md). Historical
calibration failures and memory-forecast limits remain applicable.

## Outstanding work preserved

- The [platform follow-up](docs/INFERENCE_PLATFORM_FOLLOWUP_2026-09-08.md)
  records Linux PASS at the older `f3098bc` source and the Windows
  `STATA_DRIVER_FAILED` smoke. The approved Windows collector extension awaits
  authentication to the existing maintenance profile after sign-in timed out.
  Its deployment, Windows qualification, and complete five-binary archive
  installation tests remain pending. The published-text citation checks are
  complete. Later runtime changes need appropriate platform compatibility or
  qualification evidence; old receipts do not establish that automatically.
- The owner's [five-way scaling harness](benchmarks/five_way_scaling/README.md),
  Windows harness changes, and platform evidence remain unfinished work in this
  worktree. Generated local outputs are retained and ignored. Cleanup does not
  submit a new benchmark or remote qualification campaign.
- Public visibility, tags, release archives and native binary distribution
  remain separate owner decisions. Follow [RC binary preparation](docs/RC_BINARY_PAYLOAD.md)
  when that work resumes.

Use [the documentation index](docs/README.md) for active contracts and
[the historical index](docs/EVIDENCE_INDEX.md) for prior checkpoints. Candidate
comparison and evidence reuse follow
[`development_acceptance_v1.json`](docs/development_acceptance_v1.json).
The accumulated plan before cleanup remains in Git at `929a1d7`; the working
copy, including the platform follow-up, is also preserved in local diagnostics.
