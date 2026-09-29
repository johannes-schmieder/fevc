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

The planned complete-command screen uses one deterministic synthetic panel
with 8,195 workers, 257 firms and eight observations per worker, seed 1731,
64 Counter-V1 probes, observation deletion and explicit generic Rust JLA.
The cells are diagonal with 0/8/32 controls at one native thread, diagonal
with eight controls at four threads, and CMG with eight controls at four
threads. Each fresh process performs one warm-up and five measured commands
per cell. Run baseline/candidate/candidate/baseline with no concurrent test or
build jobs; compare median complete-command time, all four targets, sample,
route, full residual and caller-state invariants. This is a development screen,
not scale or comparator qualification. Numerical tolerances stay omitted and
therefore retain their registered defaults.

## Validation

The regression exercises empty, short, exact and partial 4,096-element chunks,
unaligned columns and the usize boundary. Large model tests cover 0/1/32
controls, multi-column scalar parity, exact callback counts, cancellation in
each changed batched phase, successful workspace reuse and the observable
completed output prefix at cancellation.

Source gates and exact-source Mac plugin qualification are pending. Native
qualification must retain its exact SHA. Distributed binary adoption, Linux,
Windows, native Intel hardware and public release are outside this experiment.
