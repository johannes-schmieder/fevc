# Rust backend qualification test plan

This plan uses the handover's P1--P11 phase identifiers. Run commands from the
repository root against an otherwise clean checkout, record the source commit,
tool versions, operating system/architecture, seeds, tolerances, and failures,
and preserve console output plus machine-readable receipts at the stated
artifact locations. A planned artifact path is not evidence until it exists.

Status meanings:

- **Pass:** executed at `06afb8d` with the stated result.
- **Red:** executed or source-reproduced failure in the current tree.
- **Blocked:** the named implementation, fixture, SDK, license, or hardware is
  absent, so the qualification command cannot yet succeed.
- **Pending:** the command has not been executed at this source commit.

The local macOS arm64 locked root workspace passes all 87 tests and strict
Clippy on Rust 1.81 and stable; the Rust 1.81 non-mutating formatting check also
passes. Terminal output was not archived as a durable receipt. The separate
standalone crate was not built because no authentic Stata SDK was supplied.
Stata, Rust--Mata differential, cross-platform, sanitizer, performance, and
scale evidence is pending and must not be inferred from source inspection or
workflow YAML.

## P1: clean build

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P1-CORE-181 | Core compile and unit tests | All compiled `vckss-core` unit fixtures | 67 tests pass, 0 fail | macOS arm64; Rust 1.81.0 | `RUSTC=/Users/johannes/.rustup/toolchains/1.81.0-aarch64-apple-darwin/bin/rustc RUSTDOC=/Users/johannes/.rustup/toolchains/1.81.0-aarch64-apple-darwin/bin/rustdoc /Users/johannes/.rustup/toolchains/1.81.0-aarch64-apple-darwin/bin/cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked --target-dir /private/tmp/vckss-rust-181-target` | **Pass:** 67 core tests within the 87/87 workspace run on 2026-08-21 at `06afb8d` | Not yet recorded; local terminal output only |
| P1-CORE-STABLE | Core compile and unit tests | All compiled `vckss-core` unit fixtures | Same logical result as P1-CORE-181 | macOS arm64; stable 1.97.1 | `RUSTC=/Users/johannes/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc RUSTDOC=/Users/johannes/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustdoc /Users/johannes/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked --target-dir /private/tmp/vckss-rust-stable-target` | **Pass:** 67 core tests within the 87/87 workspace run on 2026-08-21 at `06afb8d` | Not yet recorded; local terminal output only |
| P1-CORE-LOCKED | Portable declared-toolchain core gate | All compiled `vckss-core` unit fixtures and root lockfile | 67 tests pass without changing `Cargo.lock` | Rust 1.81 and stable; all CI OSes | `cargo +1.81.0 test --manifest-path rust/Cargo.toml -p vckss-core --locked` | **Pass locally:** covered by both locked 87-test workspace runs; cross-platform execution remains pending | Not yet recorded; local terminal output only |
| P1-FMT | Formatting | Entire Rust workspace | No diff and exit 0; CI does not rewrite source | Rust 1.81 and stable; all CI OSes | `PATH=/Users/johannes/.rustup/toolchains/1.81.0-aarch64-apple-darwin/bin:$PATH cargo fmt --manifest-path rust/Cargo.toml --all -- --check` | **Pass locally:** Rust 1.81 on macOS arm64 on 2026-08-21 at `06afb8d`; CI/platform evidence pending | Local terminal output only |
| P1-WORKSPACE | Full workspace compile/tests | Core, plugin library, all unit and integration targets | 87 tests pass, 0 fail for every root-workspace target | Rust 1.81 and stable; Windows, Linux, macOS | `cargo +1.81.0 test --manifest-path rust/Cargo.toml --workspace --all-targets --locked` | **Pass locally:** 87/87 on both Rust 1.81 and stable on macOS arm64 at `06afb8d`; CI OS matrix pending | Not yet recorded; local terminal output only |
| P1-LINT-DOC | Lints and documentation | Full workspace module graph | Clippy has no warnings and docs build without errors | Rust 1.81 and stable; all CI OSes | `bash -lc 'cargo +1.81.0 clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings && cargo +1.81.0 doc --manifest-path rust/Cargo.toml --workspace --no-deps'` | **Partial:** strict Clippy passes locally on both Rust 1.81 and stable; documentation and CI-platform legs are pending | Planned: `rust/qualification/evidence/P1-LINT-DOC.txt` |
| P1-STANDALONE-LOCK | Standalone dependency lock | `rust/stata_backend/Cargo.lock` | Locked metadata resolves `vckss-core` at workspace version `0.1.0-dev` without changing the lock | Rust 1.81 and stable | `cargo +1.81.0 metadata --manifest-path rust/stata_backend/Cargo.toml --locked --format-version 1` | **Pass by source-bound lock inspection:** the lock records `vckss-core` at `0.1.0-dev`; portable command output is not archived | Planned: `rust/qualification/evidence/P1-STANDALONE-LOCK.txt` |
| P1-STANDALONE | Standalone Stata crate | Rust library, C shim, authentic official Stata SDK sources | Tests, Clippy, and release build exit 0 | Rust 1.81 and stable; Windows, Linux, macOS | `VCKSS_STATA_SDK_DIR=/path/to/authentic-sdk cargo +1.81.0 test --manifest-path rust/stata_backend/Cargo.toml --locked --all-targets` | **Blocked, fail-closed:** no authentic SDK directory was supplied; the build now rejects missing/incomplete `VCKSS_STATA_SDK_DIR`, and no SDK-backed build claim is made | Planned: `rust/qualification/evidence/P1-STANDALONE.txt` |

## P2: source-bound mathematical audit

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P2-SOURCE-MAP | Estimator-critical source map | Current Mata routines and production Rust call graph | Every graph, operator, solve, probe, correction, normalization, and failure stage maps to code and a fixture; discrepancies are explicit | Source audit | `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test mata_source_map -- --nocapture` | **Partial:** the no-control core engine was source-reviewed before `06afb8d`, but this full source-map target and durable audit artifact do not exist | Planned: `rust/qualification/evidence/P2-SOURCE-MAP.json` |
| P2-QUOTIENT-ORACLE | Full-firm quotient and complete residual | Uneven weighted three-firm problem with relabeling that changes numeric-last firm | Exact and diagonal predictions match an independently assembled constrained system; complete residual passes | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib full_zero_sum_routes_are_stable_when_the_numeric_last_firm_changes -- --exact` | **Pass within both 87-test workspace runs** | Not yet recorded; local terminal output only |
| P2-MATA-GRAPH | Graph/source differential | Minimal disconnected, articulation, bridge, duplicate, and cascade fixtures | Rust and current Mata retain identical rows/maps and report identical fixed-point decisions | Licensed Stata 18+ on each target OS | `stata-mp -b do rust/tests/stata/test_graph_differential.do` | **Blocked:** harness and fixtures do not exist | Planned: `rust/qualification/evidence/P2-MATA-GRAPH/` |

## P3: end-to-end numerical engine

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P3-ENGINE-FINITE | `run_jla_no_controls` core engine | Small explicit no-control, match-deletion two-way problems | Plugin, correction, corrected, `NumericalMcse`, detailed numerical arrays, accounting, routing, probes, and complete residual receipts pass | macOS arm64; Rust 1.81 and stable | `cargo test --manifest-path rust/Cargo.toml -p vckss-core engine::tests -- --nocapture` | **Pass:** 13 engine tests within both 87/87 workspace runs at `06afb8d`; no separate filtered run | Not yet recorded; local full-suite terminal output only |
| P3-SESSION-E2E | Staged numerical session | Existing dense no-control fixture | Prepare, solve, inspect result, and release succeed through the Rust plugin module graph | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test session_engine -- --nocapture` | **Pass locally:** 1/1 within both 87-test workspace runs at `06afb8d`; this is Rust-side evidence, not Stata qualification | Not yet recorded; local full-suite terminal output only |
| P3-RESULT-SURFACE | Numerical result contract | Core engine result and canonical engine ABI | Plugin, correction, corrected, `NumericalMcse`, detail arrays, and route/probe/residual/accounting/topology receipt are validated and exported | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test engine_ffi -- --nocapture` | **Pass locally:** canonical ABI result/receipt export is covered by 3/3 engine-ABI tests within both workspace runs; no C/Stata consumer has run | Not yet recorded; local full-suite terminal output only |

## P4: PCG and CMG

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P4-BATCH-PCG | Scalar and batched PCG | Existing mixed RHS fixtures, including zero RHS | Batched actions match scalar actions; every RHS has an independent receipt and complete residual | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib batch::tests -- --nocapture` | **Pass within both 87-test workspace runs** | Not yet recorded; local terminal output only |
| P4-ROUTER | Exact/diagonal/CMG routing | Small, boundary, setup-failure, route-freezing, and relabeling fixtures | `F-1` admission is exact; route is frozen before RNG; fallback occurs only on setup failure; iterative failure does not reroute | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib solver::tests -- --nocapture` | **Pass within both 87-test workspace runs** | Not yet recorded; local terminal output only |
| P4-CMG-RUST | Hybrid graph, hierarchy, V-cycle, CMG-PCG | Degree 2/3/4+, density-four, tie-rich and structural-twin fixtures | Exact hybrid algebra, bounded hierarchy, symmetric positive application, converged certified solve, and relabeling equivariance | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib cmg:: -- --nocapture` | **Pass within both 87-test workspace runs** | Not yet recorded; local terminal output only |
| P4-CMG-MATA | Current API-7 CMG differential | Densities 3/4/5, high degree, skewed weights, repeated RHSs | Rust and Mata receipts compare at declared tolerances for action, hierarchy, iterations, residuals, setup/application time, and memory | Licensed Stata 18+; all target OSes | `stata-mp -b do rust/tests/stata/test_cmg_differential.do` | **Blocked:** differential harness does not exist | Planned: `rust/qualification/evidence/P4-CMG-MATA/` |

## P5: RNG and reproducibility

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P5-COUNTER-VECTORS | `VCKSS-COUNTER-V1` | Frozen Philox block/word/sum/domain/batch vectors | Exact bitwise match and typed invalid-count failure | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib rng::tests -- --nocapture` | **Pass within both 87-test workspace runs on macOS arm64** | Not yet recorded; local terminal output only |
| P5-END-TO-END-INVARIANCE | Complete Counter-V1 core estimator | Row order and leverage/target batch widths | Same logical atoms and numerical outputs for the registered counter contract | macOS arm64; Rust 1.81 and stable | `cargo test --manifest-path rust/Cargo.toml -p vckss-core engine::tests::batch_width_and_row_order_do_not_change_logical_result -- --exact` | **Pass within both 87-test workspace runs; dedicated filtered output not archived** | Not yet recorded; local full-suite terminal output only |
| P5-THREAD-PLATFORM-INVARIANCE | Complete Counter-V1 estimator | Threads 1/2/4/8/16/32 and all target operating systems | Same registered deterministic outputs for fixed counter contract | Available thread counts; all target OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test rng_invariance -- --nocapture` | **Blocked:** dedicated thread/platform target and cross-platform evidence are absent | Planned: `rust/qualification/evidence/P5-THREAD-PLATFORM-INVARIANCE/` |
| P5-MATA-CONTRACT | RNG compatibility decision | Registered Stata runtime contracts and counter V1 | Counter V1 remains explicitly distinct; any compatible mode has a separate name and vectors | Licensed Stata 18 and later | `stata-mp -b do rust/tests/stata/test_rng_contracts.do` | **Blocked:** no Stata-compatible Rust mode or harness | Planned: `rust/qualification/evidence/P5-MATA-CONTRACT/` |

## P6: native plugin lifecycle

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P6-CONTEXT | Generation-safe context | Existing lifecycle, stale-handle, failure, panic, and abandoned-context fixtures | Correct state transitions; idempotent release; stale handles fail; panic remains contained | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test context_registry -- --nocapture` | **Pass locally:** 6/6 within both 87-test workspace runs at `06afb8d` | Not yet recorded; local full-suite terminal output only |
| P6-PREP-FFI | Canonical engine preparation ABI | Owned columns plus ABI mismatch, short-structure, stale-handle, null-pointer, and capacity fixtures | Typed status, stable error text, no invalid dereference or output write, exact receipt | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test engine_ffi -- --nocapture` | **Pass locally:** preparation and capacity preflights are covered by the 3/3 canonical engine-ABI tests; former `ffi_session` exports are not part of `vckss-plugin` | Not yet recorded; local full-suite terminal output only |
| P6-RETAINED | Retained marked-row mask | Existing disconnected and reversed-row fixtures | Exact mask aligned to original marked-row order and exported through the canonical engine ABI | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test session_retained -- --nocapture` | **Pass locally:** 2/2 retained-session tests plus canonical ABI mask export coverage within both workspace runs | Not yet recorded; local full-suite terminal output only |
| P6-ENGINE-ABI | Complete numerical C ABI | Finite engine fixture plus null/short/oversized/stale/retry cases | Prepare, mask, solve, result, receipt, snapshot, release, and failed-context inspection all pass | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test engine_ffi -- --nocapture` | **Pass locally:** 3/3 on both toolchains at `06afb8d`; C header/shim and licensed Stata consumption remain blocked | Not yet recorded; local full-suite terminal output only |

## P7: production Stata command

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P7-DEVELOPER-LIFECYCLE | Built plugin and developer wrapper | Small numeric dataset with marked sample | Load, capability, self-test, prepare, retained mask, solve, result, snapshot, and release pass | Licensed Stata 18+; Windows, Linux, macOS | `stata-mp -b do rust/tests/stata/test_developer_lifecycle.do` | **Blocked:** no authentic-SDK standalone build has run, and the C shim/header/ado have not migrated to the canonical engine commands | Planned: `rust/qualification/evidence/P7-DEVELOPER-LIFECYCLE/` |
| P7-BACKEND-ROUTING | Public `varcomp_kss` command | Supported and unsupported exact/JLA fixtures plus missing/wrong plugin | `backend(mata)` is unchanged; `backend(rust)` fails closed when needed; `backend(auto)` chooses before RNG and records reason; all `e()` and caller state pass | Licensed Stata 18+; Windows, Linux, macOS | `stata-mp -b do rust/tests/stata/test_backend_routing.do` | **Blocked:** public `backend()` option and harness do not exist | Planned: `rust/qualification/evidence/P7-BACKEND-ROUTING/` |

## P8: comprehensive safety and differential testing

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P8-INPUT-MATRIX | Input/resource/ABI failures | Empty, missing/nonfinite, invalid frequency/target/ID, overflow, allocation, pointer, length, ABI, and stale-handle cases | Exact typed status for every case; no allocation or output write after rejection | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test input_failure_matrix -- --nocapture` | **Blocked:** comprehensive target does not exist; current coverage is partial | Planned: `rust/qualification/evidence/P8-INPUT-MATRIX.json` |
| P8-DIFFERENTIAL-MATRIX | Rust--Mata estimator differential | Graph shapes, weights, targets, permutations, relabelings, probes 2/10/20/100/200, batches 1/2/4/8/16, available threads | Deterministic stages match exactly; randomized outputs meet predeclared tolerances; every comparison saves receipts | Licensed Stata 18+; all target OSes | `stata-mp -b do rust/tests/stata/test_differential_matrix.do` | **Blocked:** public route, licensed Stata harness, and differential fixtures are absent | Planned: `rust/qualification/evidence/P8-DIFFERENTIAL-MATRIX/` |
| P8-SAFETY | Unsafe/FFI, graph, CSR, semantic plans | Fuzz corpora and repeated prepare/solve/release loops | No panic across C, UB, race, leak, stale access, or malformed-input hang | Linux sanitizer runners; Miri-supported targets | `cargo +nightly miri test --manifest-path rust/Cargo.toml -p vckss-plugin` | **Pending:** no Miri, sanitizer, or fuzz evidence; green workspace unit/integration tests do not substitute for these gates | Planned: `rust/qualification/evidence/P8-SAFETY/` |

## P9: feature parity

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P9-CAPABILITY-GATE | Public support declaration | Current capability JSON | Every unqualified feature remains `false`; flags turn true only with linked qualification receipts | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib capability_json_is_stable_and_valid_shape -- --exact` | **Partial pass within both 87-test workspace runs:** source sets every public flag false, but the existing test asserts only `supports_exact` | Not yet recorded; local terminal output only |
| P9-FEATURE-MATRIX | Exact/JLA, deletion modes, controls, nuisance, weights, targets, normalizations, estimates and receipts | One fixture per documented Mata command combination | Each feature is served with tested parity or remains explicitly unsupported by approved decision | Licensed Stata 18+; all target OSes | `stata-mp -b do rust/tests/stata/test_feature_matrix.do` | **Blocked:** no public Rust route; controls and observation deletion are unsupported | Planned: `rust/qualification/evidence/P9-FEATURE-MATRIX/` |

## P10: parallelism and performance

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P10-PARTITIONS | Deterministic executor primitives | Existing partition/order/fixed-tree fixtures | Fixed coverage and merge order | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib parallel::tests -- --nocapture` | **Pass within both 87-test workspace runs** | Not yet recorded; local terminal output only |
| P10-INGESTION | Stata SPI scan | Six numeric columns at 10M, 50M, and 100M rows | Rows/s and bytes/s reported separately with peak memory and source-bound receipts | Licensed Stata 18+ on representative Windows, Linux, macOS hosts | `stata-mp -b do rust/benchmarks/stata/bench_ingestion.do` | **Blocked:** built plugin and benchmark harness are absent | Planned: `rust/qualification/evidence/P10-INGESTION/` |
| P10-SCALE | End-to-end preparation and solve | Medium production-like and synthetic cases approaching 30M workers, 1M firms, 20 years; probes 20/100/200; threads 1/4/8/16/32 | Phase timings, RSS, modeled memory, route, residuals, accounting, and thread invariance recorded; no tiny-graph extrapolation substitutes for run evidence | Qualified local/SCC hosts | `cargo bench --manifest-path rust/Cargo.toml --bench rust_backend_scale -- --save-baseline 06afb8d` | **Blocked:** benchmark target and scale fixtures do not exist | Planned: `rust/qualification/evidence/P10-SCALE/` |

## P11: cross-platform artifacts and release preparation

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P11-CI-MATRIX | Workspace and standalone CI | `main` at the qualification commit | Rust 1.81/stable are green on Windows, Linux, and macOS; logs retained | GitHub-hosted Windows, Linux, macOS | `gh run list --repo johannes-schmieder/varcomp_kss --branch main --workflow rust-backend.yml` | **Pending:** local gates are green, but no durable CI matrix receipt is recorded for `06afb8d` | Planned: GitHub Actions run URLs and downloaded logs under `rust/qualification/evidence/P11-CI-MATRIX/` |
| P11-ARTIFACTS | Native plugin binaries and exports | Release builds from the standalone workflow with authorized SDK inputs | Linux x86-64, Windows x86-64, macOS x86-64/arm64 binaries exist; required lifecycle symbols including `stata_call` export; dependencies are allowed | All target architectures | `gh workflow run rust-stata-backend.yml --repo johannes-schmieder/varcomp_kss --ref main` | **Blocked:** authentic SDK provisioning, canonical C shim/header/ado migration, and SDK-backed qualification are incomplete; no current artifacts | Planned: downloaded Actions artifacts, SHA-256 files, symbol and dependency reports |
| P11-MACOS-UNIVERSAL | Universal macOS plugin | Qualified x86-64 and arm64 slices | `lipo` reports both architectures and each slice passes licensed Stata loading | macOS Intel and Apple Silicon | `lipo -info artifacts/varcomp_kss_rust_macos.plugin` | **Blocked:** no qualified slices or universal artifact | Planned: `rust/qualification/evidence/P11-MACOS-UNIVERSAL.txt` |
| P11-STATA-PLATFORMS | Installed production estimator | Clean package install and differential fixture set | Plugin loads and all P7/P8 gates pass in licensed Stata 18 and later on every target | Windows x86-64, Linux x86-64, macOS x86-64 and arm64 | `stata-mp -b do rust/tests/stata/run_all.do` | **Blocked:** plugin, production route, and harness are incomplete | Planned: `rust/qualification/evidence/P11-STATA-PLATFORMS/` |
| P11-RELEASE-PACKET | Release/provenance closure | Source archive, locks, toolchain, license inventory, notices, SBOM, platform matrix, validation and benchmark reports, contracts, binaries, checksums | All artifacts are source-bound and reviewed; final human mathematical and GPL/provenance reviews approve the exact distribution | Release candidate | `./rust/tools/verify_release_packet.sh` | **Blocked:** verifier and release packet do not exist; human reviews are outstanding | Planned: `rust/qualification/release/` |

## Claim gates

- Do not change any public support flag until the corresponding P3--P9 rows
  pass through the production module graph and licensed Stata route.
- Do not claim Mata parity from core unit tests, source-informed provenance, or
  similar iteration counts. P2, P4, P5, P8, and P9 differential evidence is
  required.
- Do not claim cross-platform support from workflow configuration. P11 CI,
  artifact inspection, and licensed Stata runs are required.
- Do not claim scale qualification from small fixtures or forecasts. P10 must
  record actual phase, residual, accounting, wall-time, and memory receipts.
- Do not make a public release until P11 is complete and the required human
  mathematical, licensing, and provenance reviews approve the exact release.
