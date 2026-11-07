# CMG consolidation — November 6, 2026

The owner authorizes one common CMG main revision, new 0.5.1 plugins and
release packages after qualification, and removal of abandoned CMG branches.
The replacement candidate is `e5613e81fb3b03c78de98658b372646fa802aabb`;
it is not frozen on main. The vendor payload is byte-identical to upstream.
FEVC owns its pool and input-ordered dynamically claimed RHS queue and uses
public contexts, explicit 128-level configuration, and checked caller-buffer PCG.
Root, plugin and fuzz lockfiles use CMG 0.2.0.

Exact replacement build `94d94999be54e9a2309497d5fbe9899e2822cd51` passes
full Mac arm64/Rosetta native and installation qualification, hosted Rust/MSRV
checks, and private Windows smoke/full qualification. The shared full-CMG
public route remains Mac/Unix only; Windows boundary coverage verifies its
existing compressed/exact route, residuals and caller-state restoration.
SCC Linux smoke 8488089 passes; full native job 8488094 is in progress.

The [four frozen comparisons](../native/cmg-consolidation-20261106/evidence/performance/README.md)
retain the earlier timing failures and development-binary pass. The exact new
qualified Mac arm64 binary passes all four cells (+0.6% to +3.3%) and all
scientific gates under the unchanged max(5%, 1 ms) allowance. Every study
retains all 96 observations; no adverse measurements were repeated or excluded.

Next: collect Linux qualification, assemble the five exact source-bound plugins,
corresponding source and adoption evidence, and validate the final packages.
Cross-consumer release remains gated on common-revision qualification and the
owner's decision on fereg's inconclusive performance evidence. No new plugins,
main update or release has been published.
