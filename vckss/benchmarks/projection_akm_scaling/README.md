# Staged AKM projection scaling comparison

This harness is the source-bound follow-up to the qualified forced-CMG
`project()` implementation. It does not alter the accepted diagonal comparison
or any paper performance claim. It compares VCkss Rust/generic-JLA/Counter-V1
with forced generic CMG against maintained MATLAB JLA plus `lincom_KSS` on
deterministic AKM-shaped mover graphs.

The registered feasibility grid is:

| Rows | Workers | Firms | Application cores | VCkss maxiter | VCkss/MATLAB role cap | Reserved memory |
|---:|---:|---:|---:|---:|---:|---:|
| 480,000 | 80,000 | 40,000 | 4, 16 | 40,000 | 3 hours | 64 GiB |
| 1,920,000 | 320,000 | 160,000 | 4, 16 | 40,000 | 10 hours | 128 GiB |
| 7,680,000 | 1,280,000 | 640,000 | 4, 16 | 40,000 | 12 hours | 256 GiB |

Every measured job reserves 16 SGE slots. The harness constrains the Rust
solver and MATLAB pool to exactly 4 or 16 application cores and pins the
complete child process tree to that CPU subset. The SCC Stata/MP frontend is
explicitly held to its licensed four processors in both cells; the 16-core
VCkss cell uses 16 Rayon solver threads. The two implementations run
sequentially on the same host with deterministic order rotation. The wrapper
always attempts the second role after a first-role scientific failure or
timeout. A role stopped at its registered cap is recorded as
`RIGHT_CENSORED`; it is not converted into a failure time or used in a ratio.
Maintained MATLAB's exact, reason-coded grounded-fit nonconvergence is also
recorded as `RIGHT_CENSORED` with
`censor_reason=MATLAB_FIT_NONCONVERGENCE`. Its observed failed-command time is
retained diagnostically but is never treated as a completion time, imputed to
the role cap, or used in a ratio. Other scientific failures remain failures.

Each worker contributes six spells and visits three firms. The projection has
an automatic constant and `z1 z2`, targets firm effects, and uses physical
frequency mass. VCkss runs

```stata
backend(rust) algorithm(jla) engine(generic) rng(counter_v1) \
preconditioner(cmg) deletion(observation) batch(16) \
project(z1 z2) projecteffect(firm) projectweight(frequency)
```

The 6,000-row gate retains the original 20,000-iteration solver budget.
Registered feasibility cells use 40,000 iterations without changing the
public tolerance, complete-original-system residual gate, or role cap. A
VCkss result is timed only when its own residual, route, conditioning, PSD,
memory, and receipt gates pass, independently of a censored MATLAB role.
An array stage passes when every task is either a fully paired pass or an
accepted `RIGHT_CENSORED` pair under that rule. The stage receipt reports the
censored-task count; no MATLAB completion time or paired speed ratio is
created for those tasks.

The preparation stage builds the exact-commit Linux plugin with Rust 1.85.1,
builds the maintained MATLAB MEX boundary, checks the two maintained source
hashes, requires at least 100 GiB of free project filesystem space, and
generates all four inputs (including the 6,000-row gate). A 6,000-row exact
Mata/forced-CMG/maintained-MATLAB gate must pass before feasibility work.
Sizes are then released in ascending order, and a larger size is not submitted
unless both core cells at the preceding size pass complete accounting,
application, residual, convergence, conditioning, PSD, memory, coefficient,
SE, and covariance-diagonal gates.

The generic Rust route must report forced-CMG request and selection codes and
zero solver fallback. Its native generic public return currently leaves the
structural hierarchy-level and terminal-vertex counts missing; the harness
records those fields as JSON `null` rather than treating an empty Stata value
as a failed numeric conversion. When either structural count is available it
is range-checked. Complete-system residuals, solver convergence, projection
Gram conditioning, PSD, and memory gates remain mandatory in every cell.

Feasibility uses one pair per size/core cell and is descriptive. Only after all
three feasibility stages pass does `make_topup_manifest.py` admit repetitions
two and three for a cell whose feasibility pair also finished within 10 hours.
Medians and implementation ratios require exactly three passing repetitions;
one-repetition cells remain explicitly indicative. Missing and censored roles
are retained in `cells.csv`, and `aggregate.py` never imputes their time.

The maintained comparator remains external commit
`8b957ffeb10b8465a3584fceb0265cccc48379e1`. Its estimator sources are not
copied into this repository. `leave_out_COMPLETE.m` and `lincom_KSS.m` are
resolved from the external tree and checked against their committed SHA-256
identities. MATLAB retains its incomplete-Cholesky/PCG implementation and its
registered strict grounded-fit convergence gate.

Use `deploy_scc.sh RUN_ID` only from a clean exact commit. Submit `prepare`,
`gate`, and each feasibility size separately with `submit_scc.sh`. After an
array leaves `qstat`, wait for complete `qacct`, run
`collect_stage_qacct.sh`, and inspect the generated stage receipt before
releasing the next size. Create top-up manifests only after all feasibility
pass. Raw data and execution evidence remain under
`/projectnb/welfgr/vckss/runs`; only compact, source-bound receipts belong in
the repository.

## Terminal 480,000-row continuation

The owner-directed continuation at exact source
`30d49fda876597782cca22adf36094993f7a6212` completed preparation job `7373800`
and the 6,000-row gate job `7374823` with clean accounting and passing receipts.
Both tasks in array `7374830` also have `failed=0` and `exit_status=0`, but the
stage is `COMPLETE_NONPASS`: VCkss reached `PCG_MAXITER` after 40,000 model
iterations at reduced residual `4.412307557090175e-10` in both core cells.
Maintained MATLAB independently produced the registered grounded-fit failure
and is recorded as `RIGHT_CENSORED/MATLAB_FIT_NONCONVERGENCE` in both cells.

No VCkss completion time, MATLAB completion time, coefficient/covariance
comparison, or paired speed ratio is accepted from this stage. Observed
failed-role walls remain diagnostic only. The 1,920,000- and 7,680,000-row
stages were not submitted. Compact evidence and its interpretation are indexed
in [`evidence/scc/30d49fda876597782cca22adf36094993f7a6212/README.md`](evidence/scc/30d49fda876597782cca22adf36094993f7a6212/README.md).
