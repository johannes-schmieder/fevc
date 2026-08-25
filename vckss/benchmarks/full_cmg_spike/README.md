# Direct full-CMG architectural spike

This source-bound harness compares, on one registered maintained-MATLAB-
compatible hard problem:

- A: the VCkss performance baseline at `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`;
- C: the private direct hybrid-Laplacian `CMG_FULL_SPIKE_V1` route;
- MATLAB: the unchanged maintained `LeaveOutTwoWay` implementation registered
  by `../matlab_scale/source_contract.json`.

The headline is the existing `paper_matlab_scaling` strong-degree-6 case:
1,966,080 literal match rows, 327,680 workers, 8,192 firms, 200 probes,
uniform stored-row targets, four Stata processors, and four MATLAB pool
workers. Each application runs in a fresh process. One cold pass is followed
by five position-balanced warm passes, and claims use warm medians. The input
CSV is generated once and checksum-bound across all applications.

The Stata comparison fixes explicit `backend(rust) rng(counter_v1)` with
`algorithm(jla) engine(auto)` and requires the frozen plan to select the
compressed family. It keeps the existing estimator, tolerance,
complete-system residual, accounting, sample, data, RNG, and sort gates. A/C
result fields use the registered `2e-12` absolute-or-relative scientific
tolerance. The maintained MATLAB result remains descriptive: it uses its own
RNG, JLA PCG tolerance, and correction formulas, so no cross-language
corrected-estimate equality claim is made.

Run only from a clean checkout after separately building exact-source baseline
and candidate plugins:

```bash
python3 vckss/benchmarks/full_cmg_spike/run_local.py \
  --output-dir /private/tmp/vckss-full-cmg-local \
  --baseline 4124b34f3ca216dcc3aae27e4b31bbac9e011f11 \
  --baseline-plugin /private/tmp/vckss-baseline-4124/plugin/vckss_rust_macos_arm64.plugin \
  --candidate HEAD \
  --candidate-plugin /private/tmp/vckss-full-cmg-candidate/candidate/vckss_rust_macos_arm64.plugin \
  --candidate-build-receipt /private/tmp/vckss-full-cmg-candidate/build-receipt.txt \
  --matlab-root /Users/johannes/Git/varcomp_hdfe/Monte_Carlo/LeaveOutTwoWay \
  --matlab /Applications/MATLAB_R2024b.app/bin/matlab
```

This is a private decision experiment, not release qualification. The runner
does not submit SCC jobs, alter the maintained MATLAB source, install plugins,
or promote raw logs into the repository.

The completed architectural result is recorded in
[`DECISION_REPORT.md`](DECISION_REPORT.md) and the compact machine-readable
[`decision_receipt.json`](decision_receipt.json). Direct full CMG is rejected
for promotion: on the accepted same-node SCC run it is 32.20% faster than the
matched VCkss baseline but 29.99% slower than maintained MATLAB, and the
unchanged A/C scientific gate misses in two covariance fields.

## SCC smoke

After the local A/C smoke, deploy one same-host four-slot A/C/MATLAB run from
a clean commit. The submitter archives the exact A, C, and standalone CMG
commits, authenticates the Stata SPI inputs, and submits one scalar `welfgr`
job under `/projectnb/welfgr/vckss/runs/`. It does not use an array or copy a
row-level input off the execution node. Rust dependency resolution is locked
by the three archived `Cargo.lock` files; their SHA-256 values are included in
the node receipt. The first source-bound smoke, job `7306618`, established
that the shared SCC cache was incomplete, so the job may fetch a locked crate
that is absent rather than failing in Cargo offline mode.

```bash
VCKSS_CMG_ROOT="$GIT_HOME/CMG" \
  vckss/benchmarks/full_cmg_spike/submit_scc_smoke.sh \
  20260825T000000Z-full-cmg-smoke
```

Accept the run only after the application markers, `receipts/node.txt`,
`receipts/wrapper.pass`, process-tree receipt, and post-job `qacct` all pass.

Two failed deployments are retained as evidence: job `7306618` identified an
incomplete SCC Cargo cache, and job `7306623` identified the wrong installed
Linux plugin filename. Accepted job `7306628` ran on `scc-h30` with four
granted slots and ended with `failed=0`, `exit_status=0`. It measured 329.260
seconds for A, 223.232 seconds for C, and 171.733 seconds for maintained
MATLAB R2025b. Because C failed both the parity and MATLAB performance gates,
the registered warm matrix and fixed CZ18 case were deliberately not run.
