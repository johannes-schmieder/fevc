# Rust backend qualification test plan

This plan uses the handover's P1--P11 phase identifiers. Run commands from the
repository root against an otherwise clean checkout, record the source commit,
tool versions, operating system/architecture, seeds, tolerances, and failures,
and preserve console output plus machine-readable receipts at the stated
artifact locations. A planned artifact path is not evidence until it exists.

Status meanings:

- **Pass:** executed on 2026-08-21 in the current uncommitted `main` working
  tree based on `fa5fe94`, with the stated result.
- **Red:** executed or source-reproduced failure in the current tree.
- **Blocked:** the named implementation, fixture, plugin input, license, or hardware is
  absent, so the qualification command cannot yet succeed.
- **Pending:** the command has not been executed at this source commit.

Pinned Rust 1.81 and stable 1.97.1 on macOS arm64 each pass all 115 locked
root-workspace tests, 8 standalone tests, strict Clippy for both manifests,
release builds, and formatting. Public SPI 3.0 is hash-pinned. Licensed Stata
18 passes the developer lifecycle, bounded diagnostic, shared atoms, strict
public route, routing matrix, fault/corrupt-receipt cleanup, and isolated local
install on the current native arm64 source. The earlier dirty-tree macOS
receipt predates subsequent source repairs and is not current evidence. A fresh
source-bound arm64/Rosetta qualifier is pending independent review;
Windows/Linux, native Intel, safety, target-scale, broad feature parity, and
release evidence remain incomplete.

## P1: clean build

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P1-CORE-181 | Core compile and unit tests | All compiled `vckss-core` fixtures | Core targets pass | macOS arm64; Rust 1.81.0 | `cargo +1.81.0 test --manifest-path rust/Cargo.toml --workspace --all-targets --locked` | **Pass:** core unit tests plus shared-atom integration within 115/115 | Local terminal output only; not clean-commit evidence |
| P1-CORE-STABLE | Current boundary on stable | Root workspace | Same logical result as P1-CORE-181 | stable; all CI OSes | `cargo +stable test --manifest-path rust/Cargo.toml --workspace --all-targets --locked` | **Pass locally:** 115/115 on stable 1.97.1; CI OS matrix pending | Local terminal output only |
| P1-CORE-LOCKED | Portable declared-toolchain core gate | Root lockfile | Core tests without lock changes | Rust 1.81; macOS arm64 | `cargo +1.81.0 test --manifest-path rust/Cargo.toml -p vckss-core --locked` | **Pass locally:** covered by the locked 115-test workspace run | Local terminal output only |
| P1-FMT | Formatting | Entire Rust workspace | No diff and exit 0 | Rust 1.81; macOS arm64 | `cargo +1.81.0 fmt --manifest-path rust/Cargo.toml --all -- --check` | **Pass locally** in the current working tree; CI/platform evidence pending | Local terminal output only |
| P1-WORKSPACE | Full root workspace | Core, plugin, all unit/integration targets | All tests pass | Rust 1.81; macOS arm64 | `cargo +1.81.0 test --manifest-path rust/Cargo.toml --workspace --all-targets --locked` | **Pass:** 115/115; engine FFI 18/18 | Local terminal output only |
| P1-LINT-DOC | Lints and documentation | Root and standalone manifests | Strict Clippy; docs build | Rust 1.81; macOS arm64 | `cargo +1.81.0 clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings` | **Partial:** strict Clippy and formatting pass for root/standalone; docs and CI platforms pending | Planned durable receipt |
| P1-STANDALONE-LOCK | Standalone dependency lock | `rust/stata_backend/Cargo.lock` | Locked metadata resolves `vckss-core` at workspace version `0.1.0-dev` without changing the lock | Rust 1.81 and stable | `cargo +1.81.0 metadata --manifest-path rust/stata_backend/Cargo.toml --locked --format-version 1` | **Pass by source-bound lock inspection:** the lock records `vckss-core` at `0.1.0-dev`; portable command output is not archived | Planned: `rust/qualification/evidence/P1-STANDALONE-LOCK.txt` |
| P1-STANDALONE | Standalone Stata crate and public SPI 3.0 boundary | Canonical Rust/C/header/ado plus hash-pinned SPI | 8 tests, strict Clippy, and release build pass | Rust 1.81; macOS arm64 | `cargo +1.81.0 test --manifest-path rust/stata_backend/Cargo.toml --locked --all-targets` | **Pass:** 8/8; reviewed local SPI default and optional `VCKSS_STATA_SPI_DIR` are hash-checked | Local output only; cross-platform pending |

## P2: source-bound mathematical audit

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P2-SOURCE-MAP | Estimator-critical source map | Current Mata routines and Rust call graph | Every critical stage maps to code/fixture | Source audit | `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test mata_source_map -- --nocapture` | **Partial:** no-control core source review and permanent diagnostic exist; full durable map absent | Planned evidence |
| P2-QUOTIENT-ORACLE | Full-firm quotient and complete residual | Uneven weighted relabeling fixture | Independent constrained-system agreement | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib full_zero_sum_routes_are_stable_when_the_numeric_last_firm_changes -- --exact` | **Pass within 115/115** | Local output only |
| P2-MATA-GRAPH | Graph/source differential | Minimal disconnected, articulation, bridge, duplicate, and cascade fixtures | Rust and current Mata retain identical rows/maps and report identical fixed-point decisions | Licensed Stata 18+ on each target OS | `stata-mp -b do rust/tests/stata/test_graph_differential.do` | **Blocked:** harness and fixtures do not exist | Planned: `rust/qualification/evidence/P2-MATA-GRAPH/` |

## P3: end-to-end numerical engine

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P3-ENGINE-FINITE | Core no-control engine | Explicit match-deletion problems | Complete scientific outputs/receipts | Rust 1.81; macOS arm64 | `cargo +1.81.0 test --manifest-path rust/Cargo.toml -p vckss-core engine::tests` | **Pass within 115/115** | Local output only |
| P3-SESSION-E2E | Staged numerical session | Dense no-control fixture | Prepare, solve, result, release | Rust 1.81; macOS arm64 | `cargo +1.81.0 test --manifest-path rust/Cargo.toml -p vckss-plugin --test session_engine` | **Pass:** Rust test plus licensed Stata lifecycle | Local output only |
| P3-RESULT-SURFACE | Canonical result/receipt ABI | Rust and C/Stata consumers | Scientific fields and receipts exported | Rust 1.81 and licensed Stata 18 arm64 | `cargo +1.81.0 test --manifest-path rust/Cargo.toml -p vckss-plugin --test engine_ffi` | **Pass:** ABI tests and licensed native-arm64 Stata V3/RHS consumption | Local output only; fresh qualifier pending |

## P4: PCG and CMG

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P4-BATCH-PCG | Scalar and batched PCG | Mixed RHS fixtures | Per-RHS certified agreement | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib batch::tests` | **Pass within 115/115** | Cross-platform pending |
| P4-ROUTER | Exact/diagonal/CMG routing | Boundary/failure/relabeling fixtures | Frozen correct route | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib solver::tests` | **Pass within 115/115; public diagonal route passes locally** | Broad Stata route matrix pending |
| P4-CMG-RUST | CMG graph/hierarchy/V-cycle | Existing fixtures | Certified solve/equivariance | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib cmg::` | **Pass within 115/115** | Rust-to-Mata CMG differential pending |
| P4-CMG-MATA | Current API-7 CMG differential | Densities 3/4/5, high degree, skewed weights, repeated RHSs | Rust and Mata receipts compare at declared tolerances for action, hierarchy, iterations, residuals, setup/application time, and memory | Licensed Stata 18+; all target OSes | `stata-mp -b do rust/tests/stata/test_cmg_differential.do` | **Blocked:** differential harness does not exist | Planned: `rust/qualification/evidence/P4-CMG-MATA/` |

## P5: RNG and reproducibility

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P5-COUNTER-VECTORS | Counter-V1 | Frozen Philox vectors | Bitwise contract | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib rng::tests` | **Pass within 115/115 and shared-atom Stata test** | Cross-OS receipt pending |
| P5-END-TO-END-INVARIANCE | Counter-V1 estimator | Row/batch permutations | Same registered Rust outputs | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/Cargo.toml -p vckss-core engine::tests::batch_width_and_row_order_do_not_change_logical_result -- --exact` | **Pass within 115/115 and public Stata route** | Thread/platform matrix pending |
| P5-THREAD-PLATFORM-INVARIANCE | Complete Counter-V1 estimator | Threads 1/2/4/8/16/32 and all target operating systems | Same registered deterministic outputs for fixed counter contract | Available thread counts; all target OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-core --test rng_invariance -- --nocapture` | **Blocked:** dedicated thread/platform target and cross-platform evidence are absent | Planned: `rust/qualification/evidence/P5-THREAD-PLATFORM-INVARIANCE/` |
| P5-MATA-CONTRACT | RNG compatibility decision | Registered Stata runtime contracts and counter V1 | Counter V1 remains explicitly distinct; any compatible mode has a separate name and vectors | Licensed Stata 18 and later | `stata-mp -b do rust/tests/stata/test_rng_contracts.do` | **Blocked:** no Stata-compatible Rust mode or harness | Planned: `rust/qualification/evidence/P5-MATA-CONTRACT/` |

## P6: native plugin lifecycle

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P6-CONTEXT | Generation-safe context | Lifecycle/stale/failure/panic fixtures | Correct state and release | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test context_registry` | **Pass:** 6/6 within 115/115 | Safety stress pending |
| P6-PREP-FFI | Canonical engine preparation ABI | ABI/pointer/capacity fixtures | Typed safe boundary | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test engine_ffi` | **Pass:** 18/18; typed capabilities, frozen V1/V2, additive V3/RHS, legacy aliases, and invalid-weight mapping included | Cross-OS ABI pending |
| P6-RETAINED | Retained marked-row mask | Existing disconnected and reversed-row fixtures | Exact mask aligned to original marked-row order and exported through the canonical engine ABI | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test session_retained -- --nocapture` | **Pass locally:** retained-mask and bit-packed-capacity tests plus public Stata authoritative-mask reconciliation on native arm64 | Fresh cross-architecture qualifier pending |
| P6-ENGINE-ABI | Complete canonical ABI | Prepare through release plus failures | Rust/C/Stata lifecycle passes | Rust 1.81 and licensed Stata 18 arm64 | Rust ABI test plus `test_rust_plugin.do` | **Pass:** Rust ABI, C harnesses, compatibility-header fixture, and licensed native-arm64 lifecycle pass | Fresh Rosetta/safety/Linux/Windows evidence pending |

## P7: production Stata command

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P7-DEVELOPER-LIFECYCLE | Built plugin and developer wrapper | Failure, disconnected/permuted, and finite solve fixtures | Probe, cleanup, mask, solve, result, receipt, release | Licensed Stata 18 on Apple Silicon | `stata-mp -b do varcomp_kss/tests/stata/test_rust_plugin.do varcomp_kss` | **Pass:** current native arm64 source, including idempotent release | Fresh Rosetta/Linux/Windows/native Intel evidence pending |
| P7-BACKEND-ROUTING | Public `varcomp_kss` command | Every backend/RNG pairing plus supported and unsupported structures | Mata defaults unchanged; strict Rust consent and preflight; full lifecycle; caller state | Licensed Stata 18+ | `test_backend_routing.do` plus `test_rust_public.do` | **Pass locally on native arm64:** routing trap, successful lifecycle, one-based receipts, reconciliation, Stata-side fault/UserBreak/corrupt-receipt cleanup, resource failure cleanup, and no fallback | Local logs only; fresh source-bound qualifier pending |

## P8: comprehensive safety and differential testing

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P8-INPUT-MATRIX | Input/resource/ABI failures | Empty, missing/nonfinite, invalid frequency/target/ID, overflow, allocation, pointer, length, ABI, and stale-handle cases | Exact typed status for every case; no allocation or output write after rejection | Rust 1.81 and stable; all CI OSes | `cargo test --manifest-path rust/Cargo.toml -p vckss-plugin --test input_failure_matrix -- --nocapture` | **Blocked:** comprehensive target does not exist; current coverage is partial | Planned: `rust/qualification/evidence/P8-INPUT-MATRIX.json` |
| P8-DIFFERENTIAL-MATRIX | Permanent 96-row Mata--Rust diagnostic plus future broad matrix | Weighted no-control match-deletion fixture | Plugin `1e-9`; corrections within 8 combined MCSE | Licensed Stata 18 on Apple Silicon | `stata-mp -b do varcomp_kss/tests/stata/test_rust_mata_diagnostic.do varcomp_kss` | **Partial pass:** permanent bounded diagnostic passes | Not fixed-seed/general parity; broad feature/platform matrix absent |
| P8-SAFETY | Unsafe/FFI, graph, CSR, semantic plans | Fuzz corpora and repeated prepare/solve/release loops | No panic across C, UB, race, leak, stale access, or malformed-input hang | Linux sanitizer runners; Miri-supported targets | `cargo +nightly miri test --manifest-path rust/Cargo.toml -p vckss-plugin` | **Pending:** no Miri, sanitizer, or fuzz evidence; green workspace unit/integration tests do not substitute for these gates | Planned: `rust/qualification/evidence/P8-SAFETY/` |

## P9: feature parity

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P9-CAPABILITY-GATE | Typed/JSON capability declaration | Current boundary | Only approved strict-subset flags true | Rust 1.81 and licensed Stata 18 on macOS | Engine ABI test plus `varcomp_kss_rust probe` | **Pass:** support mask 38 (JLA + match + diagonal); every other support flag false | Local source-gate output only |
| P9-FEATURE-MATRIX | Exact/JLA, deletion modes, controls, nuisance, weights, targets, normalizations, estimates and receipts | One fixture per documented Mata command combination | Each feature is served with tested parity or remains explicitly unsupported by approved decision | Licensed Stata 18+; all target OSes | public-route and future feature-matrix tests | **Partial:** strict JLA/match/diagonal subset passes; controls, observation deletion, exact, and CMG remain unsupported | Planned broad feature matrix |

## P10: parallelism and performance

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P10-PARTITIONS | Deterministic executor primitives | Partition/order fixtures | Fixed merge order | Rust 1.81; macOS arm64 | `cargo test --manifest-path rust/crates/vckss-core/Cargo.toml --lib parallel::tests` | **Pass within 115/115** | Thread/platform estimator matrix pending |
| P10-INGESTION | Stata SPI scan | 10M/50M/100M rows | Throughput and peak memory | Target platforms | `stata-mp -b do rust/benchmarks/stata/bench_ingestion.do` | **Blocked:** plugin exists, but benchmark harness and measurements are absent | Planned evidence |
| P10-SCALE | End-to-end preparation and solve | Medium and target-scale fixtures; probes 20/100/200; threads 1/4/8/16/32 | Timing, RSS, route, residual, accounting, and thread evidence | Qualified hosts | `cargo bench --manifest-path rust/Cargo.toml --bench rust_backend_scale -- --save-baseline current-working-tree` | **Blocked:** benchmark target, whole-command memory admission, and scale fixtures do not exist | Planned evidence |

## P11: cross-platform artifacts and release preparation

| Test identifier | Component | Fixture | Expected result | Platforms | Command | Status | Evidence artifact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P11-CI-MATRIX | Root/standalone CI | Current boundary | Rust/toolchain/OS matrix logs | Windows, Linux, macOS | `gh run list --repo johannes-schmieder/varcomp_kss --branch main` | **Pending:** current work is uncommitted; no clean source-bound matrix receipt | Planned logs |
| P11-ARTIFACTS | Native plugin binaries/exports | Release builds | Loader and canonical lifecycle exports | All targets | `rust/stata_backend/qualify_macos.sh --receipt PATH` | **Pending for current source:** exact thin/universal staging and hash checks are implemented but the repaired qualifier has not run | No current macOS/Linux/Windows/release artifact receipt |
| P11-MACOS-UNIVERSAL | Universal macOS plugin | Qualified x86-64 and arm64 slices | `lipo` reports both architectures and each slice passes licensed Stata loading | macOS Intel and Apple Silicon | `rust/stata_backend/qualify_macos.sh --receipt PATH` | **Pending for current source:** fresh native-arm64/Rosetta run awaits independent review; native Intel hardware remains untested | Planned fresh local receipt |
| P11-STATA-PLATFORMS | Installed production estimator | Isolated package install and strict public fixture | Plugin loads and approved P7 subset passes | Windows x86-64, Linux x86-64, macOS x86-64 and arm64 | qualifier clean-install cases | **Partial:** current native-arm64 isolated install passes; fresh Rosetta and all other platforms remain pending or blocked | Local arm64 log only; fresh qualifier pending |
| P11-RELEASE-PACKET | Release/provenance closure | Source archive, locks, toolchain, license inventory, notices, SBOM, platform matrix, validation and benchmark reports, contracts, binaries, checksums | All artifacts are source-bound and reviewed; final human mathematical and GPL/provenance reviews approve the exact distribution | Release candidate | `./rust/tools/verify_release_packet.sh` | **Blocked:** verifier and release packet do not exist; human reviews are outstanding | Planned: `rust/qualification/release/` |

## Claim gates

- Keep support limited to mask 38 until each additional feature passes its
  corresponding P3--P9 production and licensed-Stata gates.
- Do not claim Mata parity from core unit tests, source-informed provenance, or
  similar iteration counts. P2, P4, P5, P8, and P9 differential evidence is
  required.
- Do not claim cross-platform support from workflow configuration. P11 CI,
  artifact inspection, and licensed Stata runs are required.
- Do not claim scale qualification from small fixtures or forecasts. P10 must
  record actual phase, residual, accounting, wall-time, and memory receipts.
- Do not make a public release until P11 is complete and the required human
  mathematical, licensing, and provenance reviews approve the exact release.
