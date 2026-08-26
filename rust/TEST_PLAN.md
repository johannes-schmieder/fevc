# Rust backend test plan

This is the active native test plan. Dated implementation and CI snapshots are
kept under [`progress/`](progress/) and must not be treated as current
instructions. The authoritative milestone order is
[`../vckss/PLAN.md`](../vckss/PLAN.md).

## Evidence rules

- Bind every claim to the exact tested source SHA and profile.
- A licensed Stata receipt does not by itself prove Rust formatting, Clippy, or
  workspace tests; inspect the workflow jobs as well.
- A green quick profile is not plugin qualification.
- Stata batch process status is not authoritative by itself; require the
  profile receipt and explicit PASS markers.
- Preserve failures and exact diagnostics. Apply the registered
  [`development acceptance policy`](../vckss/docs/development_acceptance_v1.json):
  do not change the requested estimator merely to obtain green output, but do
  not treat harmless bitwise/ULP or legacy fixed-roundoff differences as
  promotion blockers when corrected-result equivalence and hard correctness
  pass.
- Receipt-only `[skip ci]` commits are bookkeeping, not the tested source.

## Routine source gates

Run from a clean checkout:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python vckss/cmg/tools/assemble.py --all --check
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

When Stata is available, also run:

```bash
./ci/run_stata_ci.sh quick
./ci/run_stata_ci.sh full
```

The integrated package command is:

```bash
./.venv/bin/python vckss/tools/run_checks.py
```

## Plugin qualification

A meaningful change to Rust estimator code, the plugin ABI, the Ado native
boundary, result posting, packaging, or native route tests requires:

```bash
./ci/run_stata_ci.sh plugin-build
```

The comprehensive macOS profile runs locked Rust and C gates, SPI
authentication, ABI/layout checks, thin and universal builds, binary audits,
native arm64 lifecycle and clean-install tests, and Rosetta x86_64 coverage
when available. The exact profile and source SHA must be present in
`.ci/stata/results/<sha>.json` with `status == "success"`.

This evidence qualifies only the recorded macOS candidate and routes. It does
not qualify native Intel hardware, Linux, Windows, production scale, or public
release.

## Completed focused milestone: planned auto-exact

Commit `6954da6e190680a65ac271b71a33ece8d0fcfab1` closes these gates locally:

1. Direct V4 exact result export reconciles through
   `_vckss_rust_reconcile_exact_v7` with the exact limit, algorithm/engine
   selection reasons, plan complexity, memory, residual, accounting, and zero
   pre-RNG counter facts intact.
2. The public Rust option predicate admits only the intended planned
   `algorithm(auto)` tuple and preserves explicit backend/RNG consent.
3. `_vckss_rust_generic_planned` recognizes the exact result family before its
   compressed/generic switch and constructs the exact preparation, graph, and
   capability contexts.
4. `_vckss_rust_post_exact_v7` posts exact results, V7 plan fields, and
   `e(sample)` and releases the native handle exactly once.
5. A public command test covers
   `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)` selecting exact.
6. The test verifies requested/selected algorithm and engine, result family,
   exact identities, no estimator RNG consumption, caller RNG/data/sort
   restoration, clean install, and typed corrupt/inconsistent receipt failure.
7. Quick, full, and plugin-build profiles pass at the exact final source SHA.

The focused direct test retains `EXACT_V7_RECONCILE_FAIL` and the returned
receipt list on any future reconciliation failure. The clean source-local
`plugin-build` receipt is green for thin arm64, thin x86_64/Rosetta, and
universal execution under both architectures, including clean installs.

## Completed alpha milestone: exact stayer hybrid

Commit `c199bf017719f09f00fbe7e476e93448747d8a83` closes the native exact
`stayers(both)` gap on macOS:

1. A versioned native augmentation lifecycle constructs the frozen
   mixed-deletion stayer correction without changing the mover headline.
2. The public exact poster reconciles preparation, graph, capability,
   augmentation, result, accounting, memory, and V7 plan receipts before
   posting.
3. Independent dense and Mata differential oracles cover pooled targets,
   physical frequency splits, controls, weights, deletion IDs, and exact
   zero-RNG behavior.
4. Success, typed failure, corrupt receipt, resource failure, and UserBreak
   retain exactly-once release and idle-registry restoration.
5. The exact-SHA macOS `plugin-build` passes the public hybrid suite for thin
   and universal arm64 and x86-64/Rosetta candidates, plus clean installation.

## Active alpha qualification

The generated feature ledger is
[`../vckss/docs/RUST_MATA_PARITY.md`](../vckss/docs/RUST_MATA_PARITY.md).
Every alpha-required row must be `qualified` on its claimed platform before
tagging the release candidate.

### Active M5 direct full-CMG decision spike

Commit `98a486c` introduces `VckssEnginePerformanceReceiptV1` as an additive
96-byte C ABI surface. It does not extend or replace the frozen V7
science/execution-plan receipt. Rust and C compile-time layout fences require
the exact size and timing offset, while FFI tests reconcile generation,
selected algorithm/engine, applicability, and the saturating phase sum for
exact, compressed, and generic planned solves.

The Stata result bridge validates the receipt before posting and exposes
`e(rust_phase_profile)` with schema `VCKSS-NATIVE-PHASE-PERF-V1` in seconds.
Focused planned V4, public compressed, and public auto-exact tests require
nonnegative phases, a reconciled native total, caller-state restoration, and
an idle registry. A clean exact-SHA `plugin-build` remains required before the
instrumentation is qualified; timings are diagnostics, never routing inputs.

The current private lane adds `CMG_FULL_SPIKE_V1` without changing the public
ABI or advertised capability. Focused Rust tests require fixed-order batch
results, one shared solver/plan/workspace pool, bounded four-thread execution,
no Stata calls from Rayon workers, independent complete-system residuals, and
repeated corrected results within the registered equivalence rule. The
registered local 8,192-firm/200-probe case passes
state and residual gates and is 12.35% faster end-to-end than source `4124b34`.
Its covariance and corrected covariance differ by about `2.22e-12`, which
failed the legacy `2e-12` comparison but easily passes the active development-
equivalence policy. An inner `1e-15` diagnostic produces the same result gap at
higher runtime, so the difference is retained as a nonblocking diagnosis.

Accepted same-host SCC job `7306628` is the original decision gate. The synthetic
headline takes 329.260 seconds for A, 223.232 seconds for C, and 171.733
seconds for maintained MATLAB R2025b. C is 32.20% faster than A but 29.99%
slower than MATLAB. Its two covariance fields fail the historical receipt's
fixed A/C threshold, but under the active policy it is
statistically equivalent; the original route was rejected because it was
slower than MATLAB. Preserve the local and SCC receipts.

The owner-authorized renewed direct-route wave is source-bound at `598a08d`.
Packed/ordered-parallel Counter-V1 generation, direct leverage RHSs,
independent target preparation, parallel moment accumulation, and parallel
recovery/certification reduce one clean macOS headline to 81.145 seconds,
0.473 times the registered MATLAB comparator. The corrected targets remain
bit-identical to the preceding source-bound checkpoint; the maximum complete
residual is `6.97e-6` under the `1e-5` probe gate. This crosses the 2x target
only as a single development run. Do not harden or expose the route until one
cold plus five alternating warm A/C/MATLAB runs and the fixed CZ18 P200 gate
both pass. SCC synthetic job `7311964` measures 156.455 seconds versus
MATLAB's 260.928 seconds. Accepted same-host job `7312041` keeps direct at
151.351 seconds and disables fused f64 after its 239.610-second regression.
Direct is 1.669x MATLAB and misses the 2x threshold by 25.079 seconds; the
official full-CMG repeated solve is the dominant cost. Fixed-CZ18 P20
completes all 61 numerical solves with maximum complete residual `7.993e-6`,
but its post-engine Stata/native result-boundary failure remains under focused
diagnosis and cannot qualify P20 or P200. Do not return to the simplified
hierarchy.

The required new suites cover:

1. Rust-preferred omitted/automatic backend routing and `rng(auto)` with only
   missing-runtime or structurally unsupported preflight fallback to Mata.
2. Strict explicit backend/RNG mismatches and fail-closed stale, corrupt,
   preparation, memory, convergence, numerical, resource, and UserBreak exits.
3. Keep the qualified effective-option admission regression for automatic
   exact/compressed/generic result families, controls, deletion modes, weights,
   targets, deletion IDs, `probeorder()`, route, batch, and wall advisories.
4. Keep the qualified exact `stayers(both)` augmentation regression with
   independent dense and Mata oracles, mover-headline invariance, pooled target
   accounting, and lifecycle stress.
5. Linux x86-64 clean installation and real Stata MP 19 execution on SCC,
   bound to the exact source SHA and successful SGE accounting.
6. Miri, malformed-ABI fuzzing, C-shim sanitizers, dependency/license/SBOM,
   and registered performance/accuracy gates.

### Completed bounded Miri and C-shim sanitizer gate

Source `041a89e9fbaa0b4893564b3ca8984bc701e2003b` has a passing Darwin
arm64 receipt under
[`qualification/evidence/M4-SAFETY/`](qualification/evidence/M4-SAFETY/).
The registered gate uses the date-pinned nightly toolchain, strict-provenance
Miri for bounded unsafe FFI/registry/lifecycle cases, and Clang
AddressSanitizer plus UndefinedBehaviorSanitizer for the C shim. macOS leak
detection is explicitly disabled because that sanitizer is unsupported on the
platform; AddressSanitizer and UndefinedBehaviorSanitizer remain enabled.

Miri does not replace native numerical evidence. The full numerical suite
showed a software-interpreter one-ULP floating-point difference, and the dense
exact-stayer spectral solve is impractically slow under interpretation. Those
paths retain native Rust corrected-result equivalence and licensed-Stata
lifecycle gates; the one-ULP difference is diagnostic under the active policy.
The same directory contains a passing bounded malformed-ABI fuzz
receipt for source `a091b37123c16697750af5578c905b7ff172b710`. The
locked `cargo-fuzz 0.12.0` target uses the same date-pinned nightly, runs
nightly Clippy first, preserves only the registered seed corpus, and requires
zero crash/timeout/leak artifacts. Its V1/V2/V3 capability inputs cover
malformed headers and sizes, unknown semantic tuples, null pointers, and
adversarial output capacities.

### Completed SCC Linux qualification

Source `86e0711b1b1d07cff3058461c558838f2233f1d1` has a passing Linux
x86-64 receipt under
[`qualification/evidence/M4-LINUX-SCC/`](qualification/evidence/M4-LINUX-SCC/).
SGE job `7305794` built the ELF candidate from a deterministic source bundle,
ran the locked Rust/C/ABI gates, authenticated Stata MP 19, passed the full
public Stata suite, and repeated the public Rust route from an isolated clean
installation. The candidate, bundle, staged-source, Stata binary, environment,
and required exports are hash/receipt bound; `qacct` records `failed=0` and
`exit_status=0`. This is Linux x86-64 evidence, not representative-scale
benchmarking or a Windows/macOS claim.

### Completed dependency, license-inventory, and SBOM gate

Source `2888b838ac744ec72c90373ff54402ab025d7e9b` has a passing automated
supply-chain receipt under
[`qualification/evidence/M4-SUPPLY-CHAIN/`](qualification/evidence/M4-SUPPLY-CHAIN/).
The source-clean gate pins `cargo-audit 0.22.2` and `cargo-cyclonedx 0.5.9`,
audits the workspace, Stata-backend, and fuzz lockfiles against one RustSec
snapshot, rejects vulnerabilities and warnings, and emits deterministic
CycloneDX 1.5 SBOMs. The normalized evidence requires registry checksums,
Cargo package URLs, a reviewed license allowlist, no unregistered git
dependencies, and no machine-local paths. The registered
`MIT/Apache-2.0 -> MIT OR Apache-2.0` bridge is limited to the spelling
published by `version_check 0.9.5`. Final human license/provenance approval is
still required before any public release.

## Hard acceptance contracts

Differential promotion uses the four corrected targets and the registered
`development_acceptance_v1.json` thresholds. Plug-in/correction path identity,
bitwise/ULP equality, equal iterations, and internal reduction order are
secondary diagnostics. They do not replace the hard contracts below and do
not block a statistically equivalent faster candidate.

All accepted native routes must preserve:

- the retained sample, target population, deletion unit, nuisance convention,
  frequency and target semantics, and four target columns;
- identification, control-basis, deletion-rank, reciprocal/maker, accounting,
  and finite-output gates;
- the complete original-system residual for every RHS at
  `max(1e-11,10*effective_phase_tolerance)`, using the receipted fit or probe
  tolerance as applicable;
- structural pre-RNG algorithm, engine, route, fallback, batch, memory, wall,
  and Counter receipts;
- direct allocation within `memory_gib()`;
- exact request/selection/result-family reconciliation; and
- caller data, `e(sample)`, RNG algorithm/stream/state, sort state, release,
  and idle registry restoration on success, error, and UserBreak.

Point estimates only are implemented. Probe dispersion is numerical Monte
Carlo error, not an econometric standard error, and `e(V)` must not be posted.

## Deferred qualification

Windows Stata/plugin qualification, native Intel hardware qualification, a
command-surviving native cache, public distribution, and final public-release
mathematical/license/provenance approval remain deferred. Representative-scale
RSS/performance, the benchmark report, and a frozen artifact packet remain
alpha gates.
