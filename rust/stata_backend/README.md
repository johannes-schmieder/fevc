# Stata plugin boundary

This crate builds the optional `vckss` Rust backend as an ordinary Stata
C plugin. It uses StataCorp's public SPI 3.0 compatibility files,
`stplugin.c` and `stplugin.h`, authenticated against the tracked hash manifest.
It does not require a separate Stata SDK.

The plugin is the native backend for the one public `vckss` command. Omitted
`backend()` and `backend(auto)` prefer a qualified plugin after complete
preflight, while `backend(rust)` remains strict and `backend(mata)` remains
explicit Mata. Native binaries are not shipped by the tracked source manifest.

## Boundary and lifecycle

The versioned boundary provides:

- capability request/receipt V3;
- prepare with retained-mask, graph, control, target, and memory receipts;
- solve/interrupt V4;
- exact, compressed-JLA, and generic-JLA result families;
- detailed execution-plan and numerical receipt V7;
- additive diagnostic performance receipt V1; and
- generation-safe result, release, clear, snapshot, and typed-error handling.

The Stata wrapper must reconcile the complete request, prepared generation,
selected family, execution plan, numerical diagnostics, memory, Counter facts,
and caller-state restoration before posting estimates. Failed or corrupt
receipts are typed failures, not fallback invitations.

The native planner resolves `algorithm(auto)` to exact or JLA and
`engine(auto)` to compressed, generic, or not-applicable. Public auto-exact,
broad effective-option admission, automatic JLA selection, `probeorder()`,
stayer augmentation, macOS arm64/Rosetta, and Linux/SCC are qualified for
their recorded source commits. Performance and alpha-packet work remain; see
[`../../vckss/PLAN.md`](../../vckss/PLAN.md).

## Qualify a local macOS candidate

On Apple Silicon with licensed Stata 18 or newer at the standard path, run from
the repository root:

```bash
rust/stata_backend/qualify_macos.sh \
  --receipt /private/tmp/vckss-macos-candidate-receipt.txt \
  --artifacts-dir /private/tmp/vckss-macos-sanitized-evidence
```

Use `--stata /absolute/path/to/stata-mp` for another installation. The receipt
path must not already exist. The optional artifacts directory must exist and
be empty.

The qualifier:

1. authenticates the pinned SPI sources;
2. runs locked Rust formatting, strict Clippy, tests, and C shim/ABI gates;
3. builds thin arm64 and x86_64 slices and a universal binary from one source
   manifest;
4. audits architectures, deployment floors, install IDs, dependencies,
   signatures, and required exports;
5. runs fresh licensed-Stata plugin lifecycle, shared-atom, exact, compressed,
   generic, routing, fault, corrupt-receipt, and clean-install tests on arm64;
6. repeats architecture-sensitive coverage under Rosetta when available; and
7. writes source, SPI, binary, and sanitized-artifact hashes only after every
   required PASS marker is present.

Raw Stata logs and temporary installs are deleted. Sanitized transcripts begin
at the first batch prompt and omit startup/license banners. A dirty worktree is
labelled as a local checkpoint, not a clean qualification. Rosetta is
compatibility evidence on Apple Silicon, not native Intel qualification.

The CI alias is:

```bash
./ci/run_stata_ci.sh plugin-build
```

Read the exact-SHA receipt under `.ci/stata/results/` and inspect the Rust/C job
steps. See [`../TEST_PLAN.md`](../TEST_PLAN.md) and
[`../../STATA_CI_RUNNER.md`](../../STATA_CI_RUNNER.md).

## Manual macOS build

For iterative development without a candidate receipt:

```bash
rust/stata_backend/fetch_stata_spi.sh
vckss_cargo_181=$(rustup which --toolchain 1.81.0 cargo)
vckss_rustc_181=$(rustup which --toolchain 1.81.0 rustc)
vckss_rust_bin_181=$(dirname -- "${vckss_rustc_181}")
env PATH="${vckss_rust_bin_181}:${PATH}" RUSTC="${vckss_rustc_181}" \
  "${vckss_cargo_181}" test --manifest-path rust/stata_backend/Cargo.toml \
  --locked --all-targets
env PATH="${vckss_rust_bin_181}:${PATH}" RUSTC="${vckss_rustc_181}" \
  "${vckss_cargo_181}" clippy --manifest-path rust/stata_backend/Cargo.toml \
  --locked --all-targets -- -D warnings
env PATH="${vckss_rust_bin_181}:${PATH}" RUSTC="${vckss_rustc_181}" \
  "${vckss_cargo_181}" build --manifest-path rust/stata_backend/Cargo.toml \
  --locked --release
cp rust/stata_backend/target/release/libvckss_stata.dylib \
  vckss/vckss_rust_macos_arm64.plugin
```

Resolving the exact Cargo and `rustc` executables is intentional: some rustup
installations do not expose the selected toolchain's sibling compiler to
Cargo's child process. `VCKSS_STATA_SPI_DIR` may point the SPI fetch/build to a
separate directory. Local SPI files, Cargo output, and plugin binaries are
ignored by Git.

A manual host build is architecture-specific. Do not rename it to the universal
plugin name; only the qualifier constructs and audits a universal candidate.

## Focused Stata tests

For one test during development:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  vckss/tests/stata/test_rust_planned_compressed_post.do \
  /absolute/path/to/checkout/vckss/vckss
```

Other native tests live beside it under `vckss/tests/stata/`. Always
check the explicit terminal PASS marker, the native registry's idle state, and
caller RNG/data/sort restoration. Do not commit raw Stata logs.

## Qualification boundary

The macOS qualifier makes no Linux, Windows, native-Intel,
representative-scale, production, inference, or public-release claim. Linux is
qualified separately on SCC for the alpha; Windows and public distribution
remain deferred pending their own gates and human review.
