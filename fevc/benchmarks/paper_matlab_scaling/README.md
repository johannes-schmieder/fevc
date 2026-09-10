# Paper Stata--MATLAB scaling benchmark

This harness measures command-boundary runtime for the current
`fevc` Stata implementation and the KSS Matlab
`LeaveOutTwoWay` implementation.  It is a descriptive P=200 comparison, not
an equality test for corrected estimates: the implementations retain their own
random-number streams, PCG tolerances, and correction formulas.

## Frozen matrix

- rows: 7,680; 30,720; 122,880; 491,520; 1,966,080;
- graph structures: strong degree 2, strong degree 3, strong degree 6, and a
  weak/local degree-3 ring;
- workers: rows divided by degree; firms: workers divided by 40;
- one stored row and one deletion unit per unique worker--firm match;
- 200 JLA probes, no controls, uniform stored-row targets;
- three deterministic seeds and both Stata-first and MATLAB-first execution
  orders, for six same-host jobs in every structure--size cell;
- four Stata processors, four MATLAB pool workers, four SGE slots, and 14 GiB
  per slot.

`generate_input.do` creates the literal CSV once inside each job.  Both
applications consume those bytes, and validation binds the input hash, task
hash, source bundle, source commit, KSS Matlab source identity, process
tree, application receipts, and `qacct` receipt.  The timed boundary is the
estimator command only; import, MATLAB pool startup, and MEX compilation are
reported separately.

## Workflow

1. Qualify and commit the exact estimator source.
2. Build the immutable bundle with
   `../prep_bnd1_matlab/build_bundle.py`; its allowlist includes this harness.
3. Create the 120 task files with `build_manifest.py` using the bundle hash and
   source commit.
4. Run the dense oracle, then small and largest-case smoke jobs on the SCC.
5. Submit one scalar `run_pair.sge` job per task.  Never use an array or reuse a
   MATLAB process across repetitions.
6. Save `qacct -j JOB_ID` beside each completed task and run
   `validate_scc_job.py`.
7. Run `analyze.py` only after all 120 validations exist.  It emits job-level
   evidence, 20 cell summaries, and a matrix receipt.

The harness does not submit, resubmit, or overwrite evidence automatically.
Timeouts and other failures remain typed job artifacts and cannot silently
disappear from a paper summary.
