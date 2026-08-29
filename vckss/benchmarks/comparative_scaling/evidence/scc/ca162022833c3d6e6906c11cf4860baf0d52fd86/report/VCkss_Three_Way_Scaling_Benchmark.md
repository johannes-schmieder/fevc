# VCkss three-way scaling benchmark

Source commit: `ca162022833c3d6e6906c11cf4860baf0d52fd86`  
Source bundle SHA-256: `02d6eb6ffc96e0eb7eea1252605663e3e206c6ce177f3cf7f0e860c2b25f40f9`  
Compact result ledger: `dc2af020cf9b8f3d37b6df58fcc56d7aac1d087358111184f469a59f7f2cc458`
Rust compiler: `rustc 1.85.1 (4eb161250 2025-03-15)`  
Stata: `stata-mp/19`; MATLAB: `matlab/2024b`  
Plugin SHA-256: `dae0e58273f74a7d9649160ba37a99cfe1b337a8b4f05aec015fad7211bf8ec3`  
Maintained MATLAB source: `8b957ffeb10b8465a3584fceb0265cccc48379e1`
Measurement generations: BASE_CARRIED `e1514185445cab36f930c90a44c5a4e23326e027` (228 tasks); AFFECTED_REPLACEMENT `ca162022833c3d6e6906c11cf4860baf0d52fd86` (69 tasks)
All generations use the same receipted estimator binaries; the replacement changes only failed one-core monitoring and the rejected smallest weak input cell under a machine-readable compatibility review.

## Which route should I use?

The registered grid contains 99 complete rankable cells. VCkss--Mata is fastest in 0, VCkss--Rust in 99, and MATLAB in 0.

Across complete cells, the median Rust/MATLAB command-time ratio is 0.207; the median Rust/Mata ratio is 0.065. Use the cell-specific tables and figures rather than these pooled diagnostics for an applied choice.
For target cells 8 and 16, Rust and MATLAB use the full target while Stata/Mata remains capped at four processors. Those Rust/Mata ratios are explicitly capped-Mata comparisons, not equal-core scaling comparisons.
The largest strong-degree-two one-core cell is not ranked. Maintained MATLAB reached the registered 10,800-second limit in all three repetitions; those observations are retained as right-censored lower bounds and the entire cell is excluded from paired summaries.

The qualified Rust route is preferred only for the measured match-JLA tuple when its plugin and memory requirements are acceptable. Mata remains the source-only portable choice. MATLAB comparisons are descriptive because RNG and numerical policies differ.

## Exact invocation contract

```stata
vckss y, worker(worker) firm(firm) deletion(match) probeorder(observation_key) ///
    backend(rust) rng(counter_v1) algorithm(jla) engine(auto) ///
    preconditioner(auto) batch(auto) memory_gib(MEMORY) wallseconds(10800) ///
    probes(200) seed(SEED) maxiter(20000) nodisplay

vckss y, worker(worker) firm(firm) deletion(match) probeorder(observation_key) ///
    backend(mata) rng(stata) algorithm(jla) engine(auto) ///
    preconditioner(auto) batch(auto) memory_gib(MEMORY) wallseconds(10800) ///
    probes(200) seed(SEED) maxiter(20000) nodisplay
```

```matlab
[firm_variance,covariance,worker_variance] = leave_out_KSS( ...
    outcome,worker,firm,[], 'matches','JLA',200,0,[],[],detail_stub);
```

`SEED`, `MEMORY`, the CPU list, and execution order are literal values from `task_manifest_300.tsv`; each application is launched in a fresh `taskset`-restricted process.

## Evidence and limitations

- Four graph families, five target-core counts, and three position-balanced seed repetitions were registered.
- The compact accepted ledger contains 297 complete same-host tasks and 891 successful estimator calls, plus three separately receipted right-censored MATLAB attempts in the excluded largest strong-degree-two one-core cell.
- Rust and MATLAB use 1/2/4/8/16 effective cores; Stata and Mata use 1/2/4/4/4. The SCC allocation remains 16 bound slots for every task.
- Cell summaries use medians and full three-run ranges, not confidence intervals.
- Route time and memory ratios are paired within each same-host task before being summarized across repetitions.
- The prototype accepts any eligible SCC host. Absolute timing and cross-core speedups spanning CPU models are descriptive pending a homogeneous confirmation.
- Fixed row counts imply different worker and firm counts across graph degrees and topologies.
- Findings apply only within the measured grid and do not establish a universal language ranking.

See the PDF for the full protocol, figures, tables, failure evidence, and exact reproduction commands.
