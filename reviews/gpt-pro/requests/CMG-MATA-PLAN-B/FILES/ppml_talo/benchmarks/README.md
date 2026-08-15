# Performance benchmarks

`benchmark_akm.do` creates a connected, two-FE, AKM-shaped synthetic panel
with positive exponential outcomes and runs the full observation-level command.
It records wall time, solver residuals/iterations, leverage diagnostics, all
four plug-in/correction/TALO/MCSE vectors, settings, source commit, Stata
version, and a conservative engineering memory estimate. A synthetic year is
retained for future multiway qualification but is not passed to the command
while `absorb()` is gated. The estimate is not a
measurement of resident set size.

For provenance-bound local evidence, use the wrapper rather than calling the
do-file manually. It refuses a dirty package tree and existing output files,
derives the exact Git commit, and captures operating-system resource usage:

```bash
ppml_talo/benchmarks/local_qualify.sh \
  100000 100 100 8 stable mobile \
  /absolute/results/result_100000.csv \
  /absolute/results/result_100000.resources.txt
```

Local smoke benchmark:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do ppml_talo/benchmarks/benchmark_akm.do 100000 40 40 \
  8 /private/tmp/ppmltalo_benchmark_100k.csv stable mobile
```

The provenance-bound 12 August 2026 local `mobile` qualification on Stata 18
(Apple Silicon) used 100+100 probes and batch eight. Ten thousand through ten
million rows completed in `0.618`, `5.207`, `54.283`, `292.465`, and
`629.950` Stata seconds. The ten-million run used `19.656180` decimal GB
measured peak RSS, had maximum recomputed residual `4.912e-09`, and needed at
most 19 iterations. See
[the local qualification report](reports/LOCAL_STATA18_2026-08-12.md) and its
[raw-evidence checksum manifest](reports/LOCAL_STATA18_2026-08-12.sha256).
This is platform-specific qualification evidence, not an SCC result or a
release guarantee.

The provenance-bound 12 August 2026 SCC qualification used Stata/MP 19 on
Linux with the site's four-core license, 100+100 probes, and a four-slot/64-GB
Grid Engine envelope. The 10m task completed in `4,114.027` Stata seconds with
`14.108975` decimal GB measured peak RSS; every ladder task and the same-host
repeat passed its registered validator. See
[the SCC qualification report](reports/SCC_STATA19_2026-08-12.md) and
[raw-evidence checksum manifest](reports/SCC_STATA19_2026-08-12.sha256).

Revalidate the retained raw evidence with
`validate_local_evidence.py`. The validator checks the exact source and fixed
settings, the complete ladder, measured RSS, accounting identities,
repeatability, outcome profiles, and the typed weak-ring non-result. The local
wrapper also reads the CSV return code because Stata on macOS can return a
successful shell status after a do-file-level error.

The SCC qualification ladder is a five-task Grid Engine array: 10k, 100k, 1m,
5m, and 10m observations, each with 100 leverage and 100 trace probes in
batches of eight. Each task receives Stata/MP 19, four licensed processors,
four scheduler slots, a 64 GB allocation, and a 12-hour limit. The
ten-million target is less than 56 GB
measured peak RSS and less than 12 hours. Each task stores its benchmark CSV,
GNU-time resource log, Stata log, and source-bound metadata separately. Retain
the corresponding `qacct` record after each task exits; the in-command memory
estimate and scheduler virtual-memory field are not substitutes for measured
RSS.

Benchmark CSV files are generated evidence. Commit only versioned reports that
record the source commit, cluster module, resource request, exit status, and
the command log. Do not commit transient ladder output or restricted data.
The local and SCC scale harnesses are documented in
[`../TESTING.md`](../TESTING.md). `benchmark_akm.do` writes one result row per
problem size. `sge_scale_ladder.sh` is the prepared 4-core, 64-GB, 12-hour
Grid Engine array for Stata/MP 19. `sge_portability_suite.sh` runs the quick
suite, full suite, and a clean temporary installation on a compute node. Both
scripts require the full source commit and staged archive SHA-256 as
environment variables and refuse to overwrite prior evidence.
`validate_scc_evidence.py` checks those bindings, artifact hashes, dependency
versions, suite statuses, benchmark accounting, numerical gates, GNU-time
RSS, active Stata processor count, and scheduler exit records. The expected
dependency versions and processor count must be supplied from the registered
qualification configuration.

The CSV fields `memory_estimate_gb` and `memory_estimate_gib` are decimal GB
and binary GiB structural engineering estimates. Record operating-system or
Grid Engine maximum RSS separately before qualifying a size. A timeout, solver
failure, or withholding status is a benchmark result and must not be discarded.

`benchmark_akm.do` accepts an outcome profile and a final network profile.
`stable` uses a
score-exact positive worker--firm mean and isolates TALO-engine scaling.
`stochastic_positive` uses a positive exponential draw and stress-tests the
external PPML optimizer as well as the correction. `poisson_zeros` uses a
correctly specified two-way Poisson draw and records its realized zero share.
Every run writes a CSV even when `ppmltalo` fails, including the return code
and typed withholding status.
`mobile` uses heterogeneous deterministic mobility and is the SCC scaling
profile. `weak_ring` preserves the original near-cycle graph as an adversarial
conditioning test; at one million rows it reached the 5,000-iteration leverage
gate and was correctly withheld. The SCC ladder uses `stable mobile`; run
stochastic fit stress and weak-ring stress separately at sizes that converge
under the allocated time.
