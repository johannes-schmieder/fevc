# CMG API 4 memory-rich terminal report — 2026-08-15

## Trigger and bounded repair

API 3 admitted a directly factored terminal only through 1,536 hybrid
vertices. Source-bound Stata 19 job `7190153` showed that the natural
all-mover MATLAB-retained graph remained outside that cap: it followed the
API 2 hierarchy path and failed closed as `HIERARCHY_STALLED` in 9.282 command
seconds. The job posted no estimate and scheduler accounting records
`failed=0`, `exit_status=0`, four slots, ten seconds wall, and 210,580 KiB
maximum RSS. Diagonal B1 job `7190152` completed the same 256,472-row sample in
525.619 command seconds with maximum complete residual
`9.99779250872e-11`.

The registered graph has 1,285 firms and 4,063 workers. The clean-room hybrid
can add at most one auxiliary vertex per worker, so 5,348 is a conservative
fine-vertex upper envelope. API 4 uses a 6,144 hard cap. It still requires at
least 512 planned RHSs, at least 16 GiB of declared memory, and a predicted
factor no larger than the memory-derived dense-factor budget. At the hard cap,
one persistent factor is 288 MiB. Actual component factors are rechecked
before allocation; construction scratch has a separate registered cap.

## Red tests and diagnostics

Before the implementation change, the resource-profile assertions for 1,537
and 6,144 vertices failed. API 4 now selects those exact terminals under a 56
GiB envelope and 601 RHSs, retains the recursive 256 threshold at 6,145, and
keeps the 8 GiB and low-RHS cases unchanged. `options_valid()` and the explicit
solver benchmark accept no terminal larger than 6,144.

The forced test adapter now records fine hybrid vertex and edge counts in its
route diagnostics. The SCC aggregate harness exports both counts for a
converged or sufficiently constructed failed hierarchy. This is diagnostic
only and does not alter the operator, estimator, routing, sample, probes, or
RNG state.

## Local calibration and gates

The memory-rich terminal was calibrated locally with Stata 18 on a
2,048-vertex ring, 601 deterministic RHSs, tolerance `1e-10`, and maximum
2,000 iterations. Stata reported eight processors. The exact terminal used
1.264 seconds setup and 4.247 seconds CMG PCG; diagonal PCG used 84.828
seconds. CMG required at most two iterations and its maximum fresh residual
was `4.82479861574e-11`. The setup-inclusive CMG/diagonal gain was 15.39x.
The terminal records one level and 311,312 structural bytes.

The API 4 red test passes after deterministic assembly. The complete shared
CMG and KSS local qualification gates pass. Automatic routing remains
disabled; SCC qualification of the actual all-mover graph is required.

