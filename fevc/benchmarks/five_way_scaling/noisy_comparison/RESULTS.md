# Noisy-fixture comparison checkpoint

Completed September 9, 2026 on SCC. Successful job 7505538, 20/20 estimator
calls, `failed=0`, `exit_status=0`. Run directory:
`output/five_way_scaling/20260909T120000-noisy3840-retry`.

The user-facing interpretation, estimates, timings, memory, limitations and
source identities are in that run's `diagnostic/reviewed_report.md`.
`diagnostic/collection_validation.json` records the post-collection checks.

The fixed 3,840-row fixture realizes an 82.8% worker correction and 38.7% firm
correction. FEVC/MATLAB/Julia exact agree with the independent dense oracle.
R exact and PyTwoWay exact have substantial worker-variance discrepancies;
`explain_exact.py` reproduces their relevant source arithmetic to <4e-12.
Their randomized paths apply substantial corrections. Julia's fixed internal
thread RNGs make caller-seed repetitions effectively one projection draw;
the reviewed report explicitly qualifies the auto-report's seed wording.

Initial job 7505259 is preserved separately under
`output/five_way_scaling/20260909T153200Z-noisy3840`:
FEVC's default exact dimension cap stopped its call. One narrow admission
repair, `exact_limit(1400)`, allowed the 1,311-coefficient design; the new
run's fixture, oracle and task manifest were byte-identical. No comparator
estimation code or numerical tolerances were repaired. No size/core sweep
or package promotion is implied by this diagnostic.

Validation: root pytest 798 passed; explicit `test_noisy.py` 3 passed;
CMG generated-source check passed; 38 bundle hashes, all 20 calls and 80
comparisons, source-formula reproductions, scheduler/application receipts
and both plots checked. Unrelated worktree changes were preserved.
