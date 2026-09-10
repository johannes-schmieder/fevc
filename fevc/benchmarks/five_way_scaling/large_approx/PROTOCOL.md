# Twenty-times-larger approximate comparison

Owner request: repeat the approximate variance/runtime/memory comparison at
about twenty times 3,840 observations. Main fixture: 76,800 rows, 25,600 workers,
640 firms, three distinct firm matches per worker. Keep the prior strong_d3
graph formula, three independent fixed PCG64 DGP seeds, latent variances .25
and .0625, noise SD 2, and once-centered outcome. One new fixed draw at the new
dimensions; no outcome search, replication of old rows or disjoint copies.

Run only approximate paths: 280 projections, two active cores, three caller
seeds 202609091/202609193/202609299, rotated role order. Fifteen calls total.
Julia retains its fixed internal thread-local RNG streams; these calls do not
measure three independent Julia projection realizations. No comparator
estimation formulas or tolerances change. Copied size admission guards alone
admit the new N. Retain all four targets, common population-N normalization,
original primary timing phase, 100 ms process-tree RSS and sample checks.

Before submission, independently compute an exact reference by eliminating
workers and solving the grounded firm Schur system. Block target norms over
observations, avoiding a full coefficient inverse or N-by-N matrix. Test this
reference against the prior dense oracle on small fixtures and the previous
3,840-row draw. Reference worker/firm corrections must still be at least
25%/20% of plug-in values; residual gates are 1e-8. Reference calculation is
outside every comparator's timing. Known latent moments are context, not an
oracle for the realized sample estimator.

Reuse the same prepared SCC environments and pinned comparator versions from
20260909T112050Z-f3098bc. Freeze all input/code hashes and task identities.
The prior 3,840-row run already validated the real modules/launcher at two
cores. The first five-role 76,800-row repetition serves as the resource smoke;
its wrapper must pass before the remaining two execute in the same job.
Reserve four slots and 8 GiB/slot (32 GiB), two active cores, and one hour:
prior peak virtual memory was 15.659G, mainly runtime overhead. No whole-node
isolation or general size/core scaling claim is implied.

Accept execution only with all 15 statuses, finite targets, sample/seed/probe
metadata, normalizations, phase RSS, input/code hashes and successful scheduler
accounting. Report approximation gaps descriptively, not as an exact-tolerance
pass/fail gate. Preserve prior evidence and any failed attempts. Deliver tables
of median values and observed ranges, runtime and memory, and note setup/JIT
overhead, unequal accuracy at equal projection counts, and the Julia RNG caveat.
