# Model-operator interrupt chunks — September 29, 2026

## Scope

The actual callback already runs every 4,096 scalar iterations. This experiment
moves its modulus/branch outside the inner loops in scalar and batched model
actions, including dense worker/control and firm/control products, RHS
reduction and worker reconstruction. The production diagonal queue calls the
batched action at width one, so both action implementations are covered.

Arithmetic traversal and accumulation order remain unchanged. A small range
iterator splits work at the original flattened checkpoint indices; a column
starting within a chunk does not add a callback or postpone its next one.
Local-index loops retain their original entry checks and phases. No allocation,
data layout, RNG, solver routing, tolerance, ABI or residual gate is changed.
Setup, control-basis certification, PCG recurrences and unrelated interrupt
sites remain outside this experiment.

## Development measurements

Baseline source is `0155d6e619b2605af508a74fee9de95cfe5b8edf`. Its native build
and production source are unchanged from qualified source
`70516881340adf12959870e0afb833514031ec69`; only harness search-path setup and
documentation changed. The existing qualified arm64 candidate is therefore
the native baseline, preserving its original source identity and receipt.
Its SHA-256 is
`2487f96adb32932bd297f8c54e5182b03ccc838c7775024587a80b89c455250e`.

The ignored `.local/interrupt-overhead/` directory retains diagnostic harnesses,
logs, executable identities and raw comparisons. Kernel measurements use Rust
1.85.1 with the production release settings, 65,539 workers, 8,197 firms, four
cells per worker, controls 0/8/32, and widths one/four. A non-inlined checker
behind a black-boxed trait object loads an atomic flag and counts callbacks.
Seven timed samples of 64 applications follow one warm-up sample. Initial
measurements show lower time with identical output bits and callback counts;
they do not establish complete-command performance.


Initial development kernel medians (seven samples; milliseconds per
application; pre-commit builds retained by executable hash):

| Action | Controls | Width | Baseline ms | Candidate ms | Time reduction |
| --- | ---: | ---: | ---: | ---: | ---: |
| scalar | 0 | 1 | 1.420 | 1.021 | 28.1% |
| batch | 0 | 1 | 1.607 | 1.409 | 12.3% |
| scalar | 8 | 1 | 2.396 | 1.911 | 20.2% |
| batch | 8 | 1 | 2.620 | 2.441 | 6.8% |
| scalar | 32 | 1 | 4.970 | 4.479 | 9.9% |
| batch | 32 | 1 | 5.700 | 5.250 | 7.9% |
| batch | 8 | 4 | 10.264 | 9.546 | 7.0% |

The complete-command screen uses one deterministic synthetic panel on an
Apple M2 Ultra (24 logical CPUs, 192 GiB), Stata/MP 19.0.115, with 4,099
workers, 257 firms and 32,792 observations. Each worker has four firms and
eight observations; all observations remain in the estimation sample. The
fixture and Counter-V1 probe seed are 1731, with 16 probes, observation
deletion, mover targets, joint nuisance estimation, explicit generic Rust JLA,
`batch(4)`, and omitted tolerances. Stata processors stay at four; native
threads are one or four as stated per cell.

For reproduction, `key=_n`, `worker=ceil(key/8)`, `slot=mod(key-1,8)`, and
`firm=mod((worker-1)*37+floor(slot/2)*31,257)+1`. Set Stata's RNG to `mt64`
and seed 1731, generate double columns `x1` through `x32` with `rnormal()` in
that order, then generate
`y=sin(worker/11)-.2*cos(firm/7)+.01*slot+.2*x1-.1*x2+rnormal()`.
Sort by worker, firm and key. Select no controls, `x1-x8` or `x1-x32` per cell;
weights are one. Every timed call includes preparation, fitting, corrections,
certification and ado posting; only the initial per-cell warm-up is excluded.

The five cells are diagonal with 0/8/32 controls at one native thread,
diagonal with eight controls at four threads, and planned generic CMG with
eight controls at four threads. Explicit `batch(4)` selects planned generic
CMG; this cell must not be described as the separate `CMG_FULL_V2` route.
Each fresh process runs one warm-up and five measured complete commands per
cell, in baseline/candidate/candidate/baseline process order. This yields ten
measured observations per binary/cell. No test or build runs overlap the
comparison. Input signatures, the entire sample, route, complete residual,
caller data, RNG and sort state, and native lifecycle are checked. All four
point estimates use the registered deterministic `1e-8 * scale` equivalence
gate; bitwise output and residual identity are additional diagnostics.

Development pilots initially used 8,195 workers and 64 probes. Q32 and CMG
commands each took about 51 seconds while validation jobs were active, so the
panel and probe count were reduced before any candidate command timing. Two
harness defects were corrected during those pilots: Mata `printf` needs an
explicit field width, and the explicit-batch CMG route does not return a
`CMG_FULL_V2` receipt. The failed pilot logs remain in the ignored evidence
directory. No numerical threshold or estimator route was changed in response
to a baseline/candidate result.

The source-bound complete-command comparison passes all result and caller-state
gates. Within each cell, all four target estimates and full-system residuals
are bitwise identical between baseline and candidate, including the warm-ups;
120 complete commands were checked. Each cell is faster in the
candidate; even the slowest candidate observation is below the fastest baseline
observation in this screen. Median complete-command times are:

| Route | Controls | Native threads | Baseline s | Candidate s | Time reduction |
| --- | ---: | ---: | ---: | ---: | ---: |
| Diagonal | 0 | 1 | 0.8280 | 0.6080 | 26.6% |
| Diagonal | 8 | 1 | 1.6835 | 1.3150 | 21.9% |
| Diagonal | 32 | 1 | 18.3910 | 17.6260 | 4.2% |
| Diagonal | 8 | 4 | 0.8650 | 0.7665 | 11.4% |
| Planned generic CMG | 8 | 4 | 6.5050 | 6.2270 | 4.3% |

Retain this bounded source change. The observed complete-command gain is
4.2–26.6%, smaller than some kernel gains because the complete command also
performs unchanged work. This does not identify a new bottleneck or qualify
production scale, a KSS Matlab comparison, other hardware or other workloads.
The input signature is `32792:37(51221):2636103682:3086431020`.
The frozen manifest is `.local/interrupt-overhead/command-manifest.json`;
raw times and diagnostics are in `command-summary.json` and the four
`command-<index>-<binary>.log` files beside it. The retained entrypoints are
`command-screen.do` and `run_command_screen.py`; the latter rejects incomplete
or duplicate records, checks the input identity, and applies the numerical
equivalence gate before reporting medians. The harness SHA-256 is
`67179a54b447650005a907235b5b62bda1f91cb9579f582f774d30956fa32182`.


## Validation

The regression exercises empty, short, exact and partial 4,096-element chunks,
unaligned columns and the usize boundary. Large model tests cover 0/1/32
controls, multi-column scalar parity, exact callback counts, cancellation in
each changed batched phase, successful workspace reuse and the observable
completed output prefix at cancellation.

The following gates pass:

- `./.venv/bin/python -m pytest -q`: 835 tests.
- `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check`.
- Rust 1.85.1 workspace formatting, strict all-target Clippy and
  `cargo test --workspace --all-targets --locked`: 692 passed, zero failures,
  one ignored (including nested allocator subprocesses).
- Strict workspace Clippy on installed stable Rust 1.97.1.
- `./.venv/bin/python fevc/tools/run_checks.py`: terminal
  `FEVC LOCAL QUALIFICATION PASS`, including CMG, quick/full Stata suites,
  installation and standalone harness checks.
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools
  ./ci/run_ci_profile.sh plugin-build`: exact-source clean Mac candidate
  qualification for `d9692f6d04c62783c8e7cda0fbce5b7dbc3b07c9`. Includes native
  backend Rust, C shim/ABI gates, arm64 and Rosetta thin/universal runtime
  tests, cancellation/lifecycle and isolated installations.

The new native receipt is
`.ci/stata/results/d9692f6d04c62783c8e7cda0fbce5b7dbc3b07c9.json`, with
`status=success`; its SHA-256 is
`73f7b96d973ef25f4fe88d4f065b0e35542ed216f865450fbfdfd98e4446639a`.
Candidate SHA-256 identities are:

| Artifact | SHA-256 |
| --- | --- |
| arm64 | `b559f407b3f039fd176e18937c3242242d6cdbd6438b55b046ed74ddf6da667d` |
| x86-64 | `54af72b7dddd33cd46c0b23c8b999e2199acdb78487690e562b31d019a9b947f` |
| universal | `3083f1366044c21ded49572bf60c0957f0d5f9c06cb71329adf93cf1b77acd11` |

The candidates and sanitized logs are retained in
`.ci/stata/run/plugin-evidence/`; the preceding qualification was preserved
under `.local/interrupt-overhead/prior-ci-run/`. All five distributed plugin
hashes and their tracked modes are restored. No binary adoption, Linux or
Windows qualification, native Intel hardware qualification, tag or release is
part of this experiment. Later reporting-only commits do not relabel the
exact-source receipt or change its qualified production/build inputs.
