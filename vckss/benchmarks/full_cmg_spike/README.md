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
compressed family. It keeps the existing estimator, phase tolerance,
complete-system residual, accounting, sample, data, RNG, and sort gates. A/C
use common Counter-V1 draws; the four corrected targets pass at
`max(1e-8*scale, 0.1*max(reported MCSE))` under the active development policy.
Plug-in, correction, MCSE, iteration, and reduction-order differences remain
diagnostics. The maintained MATLAB result remains descriptive here: it uses
its own RNG, JLA PCG tolerance, and correction formulas, so this runner makes
no cross-language corrected-estimate equality claim.

The Stata command deliberately omits `tolerance()`: the candidate therefore
uses the registered phase defaults (`1e-10` fit and `1e-6` probes), while the
frozen baseline retains its historical omitted-option behavior. An explicit
`tolerance(x)` is reserved for the public contract in which `x` overrides both
phases.

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
historical A/C gate misses in two covariance fields. The latter difference is
statistically equivalent under the subsequent active development policy; the
route remains rejected on end-to-end MATLAB performance.

## SCC smoke

After the local A/C smoke, deploy one same-host four-slot A/C/MATLAB run from
a clean commit. The submitter archives the exact A, C, and standalone CMG
commits, authenticates the Stata SPI inputs, and submits one scalar `welfgr`
job under `/projectnb/welfgr/vckss/runs/`. It does not use an array or copy a
row-level input off the execution node. The standalone CMG checkout may be
dirty or at another `HEAD`; only the archived `dbefbc5` commit enters the run.
Rust dependency resolution is locked by the three archived `Cargo.lock`
files; their SHA-256 values are included in the node receipt. The first
source-bound smoke, job `7306618`, established
that the shared SCC cache was incomplete, so the job may fetch a locked crate
that is absent rather than failing in Cargo offline mode.

```bash
VCKSS_CMG_ROOT="$GIT_HOME/CMG" \
  vckss/benchmarks/full_cmg_spike/submit_scc_smoke.sh \
  20260825T000000Z-full-cmg-smoke
```

Accept the run only after the application markers, `receipts/node.txt`,
`receipts/wrapper.pass`, process-tree receipt, and post-job `qacct` all pass.

The fixed-CZ18 lane begins with one checksum-bound P20 A/C/MATLAB smoke. It
reads the retained DTA only on SCC, stages it only in job-local scratch, runs
all applications on one host with four application threads/workers, and keeps
row-level data out of the repository. Fourteen slots at 4 GiB each reserve the
same 56-GiB whole-job envelope used by the accepted prior CZ18 estimator.

```bash
VCKSS_CMG_ROOT="$GIT_HOME/CMG" \
  vckss/benchmarks/full_cmg_spike/submit_scc_cz18_smoke.sh \
  20260825T000000Z-full-cmg-cz18-p20
```

P20 is an implementation and route smoke only. It cannot satisfy the fixed
CZ18 alpha gate, which requires the subsequently registered P200 comparison.

Two failed deployments are retained as evidence: job `7306618` identified an
incomplete SCC Cargo cache, and job `7306623` identified the wrong installed
Linux plugin filename. Accepted job `7306628` ran on `scc-h30` with four
granted slots and ended with `failed=0`, `exit_status=0`. It measured 329.260
seconds for A, 223.232 seconds for C, and 171.733 seconds for maintained
MATLAB R2025b. The source-bound receipt records the then-active parity failure;
the current policy treats it as nonblocking. Because C still failed the MATLAB
performance gate, the registered warm matrix and fixed CZ18 case were not run.

Subsequent development checkpoints are recorded separately. The
[`TOLERANCE_CHECKPOINT.md`](TOLERANCE_CHECKPOINT.md) ladder establishes the
MATLAB-like `1e-6` probe tolerance. The
[`MIXED_PRECISION_CHECKPOINT.md`](MIXED_PRECISION_CHECKPOINT.md) paired run
preserves but disables the mixed-hierarchy experiment because its 2.01%
end-to-end gain misses the 10% enablement gate and its measured memory rises.
The
[`PROBE_TOLERANCE_DIRECT_CHECKPOINT.md`](PROBE_TOLERANCE_DIRECT_CHECKPOINT.md)
result removes a private 100-fold randomized over-solve. It reaches 112.875
seconds, or 0.657 times registered MATLAB, while passing the unchanged
statistical and complete-system residual gates. It is a substantial checkpoint
but not the required 2× result. The subsequent
[`PACKED_COUNTER_CHECKPOINT.md`](PACKED_COUNTER_CHECKPOINT.md) computes the
four frozen Counter-V1 lanes once per Philox block. It preserves bit-identical
targets and reaches 104.329 seconds, or 0.608 times registered MATLAB; the 2×
gate remains unmet.
The
[`PARALLEL_RESIDUAL_CHECKPOINT.md`](PARALLEL_RESIDUAL_CHECKPOINT.md) reuses the
same isolated CMG pool for bounded ordered recovery and independent residual
certification. It preserves bit-identical targets and reaches 90.102 seconds,
or 0.525 times registered MATLAB; it is 4.236 seconds short of the 2x gate.
The
[`ALPHA_HEADLINE_CHECKPOINT.md`](ALPHA_HEADLINE_CHECKPOINT.md) result adds
ordered parallel probe generation and independent preparation/accumulation.
Its clean source-bound single run reaches 81.145 seconds, or 0.473 times
registered MATLAB, with bit-identical targets and unchanged residual and state
gates. This is the first development crossing of the 2x threshold, but it is
not promotion evidence: alternating warm medians and the fixed CZ18 case are
still required.

Accepted same-host SCC job `7312041` resolves the direct-versus-fused choice.
The independent direct route takes 151.351 seconds against MATLAB's 252.543
seconds (1.669x faster), but still misses the 2x threshold by 25.079 seconds.
The fused-f64 executor regresses to 239.610 seconds and is disabled. Direct and
fused retain bit-identical corrected targets and a `6.971259e-6` maximum
complete residual. See
[`SCC_DIRECT_FUSED_CHECKPOINT.md`](SCC_DIRECT_FUSED_CHECKPOINT.md) and its
machine-readable receipt. The official full-CMG repeated solve is now the
measured dominant bottleneck; do not resume simplified-hierarchy work.
