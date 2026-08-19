# PREP-RHS-1 qualification evidence

This directory binds the performance candidate
`95d3950c4cfea669c2a244ad98f6fed03ae8ca19` to baseline
`73ea94a9bfb69a2839ef66f9d9b79c219016abbe`.

`local/` contains the archive-isolated same-machine comparison produced by
`benchmarks/prep_rhs1/run_local.py`: one cold and three warm P200 runs per
role, complete logs, raw CSV receipts, and the comparison summary. The five
tracked file hashes are recorded inside `local/summary.json`.

`scc_summary.json` covers 67 validated baseline runs, 68 validated candidate
runs, and 67 paired experiment IDs across 23 scenarios. It records every job,
source commit, bundle hash, task and input hash, scheduler host, scientific and
structural fields, performance fields, and SHA-256 hashes for 14 raw artifacts
per run. The complete raw artifacts remain at:

- candidate: `/projectnb/welfgr/varcomp-kss/runs/20260819T081752Z-prep-rhs1`
- baseline: `/projectnb/welfgr/varcomp-kss/runs/20260819T081752Z-prep-rhs1-baseline`

Candidate bundle SHA-256:
`751eff760a639592a4065693e99ba5fea7319399380a5c3a2c272cdb4f82b3d9`.
Baseline bundle SHA-256:
`e387cbf04b0d9aeb41ab73483501352cc7f131a3168f45f9151499f71cc3e1a3`.

Every SCC job passed qacct, wrapper, Stata application, dimensional,
scientific-identity, resource, and complete-RHS-residual validation. Paired
structural fields match exactly and the maximum paired scientific relative
difference is `1.0206906706043713e-14`, below the registered `2e-13` roundoff
gate. SCC timing is explicitly `HOST_CONFOUNDED`: only 9 of 67 pairs landed on
the same host, and jobs also differed in contemporaneous load. Use SCC for
scale, memory, topology, and correctness evidence; use the local archive run
for causal timing.
