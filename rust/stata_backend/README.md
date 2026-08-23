# Stata plugin boundary

This crate builds the experimental `varcomp_kss` Rust backend as an ordinary
Stata C plugin. It does not require a separate Stata SDK. The only external C
inputs are StataCorp's public SPI 3.0 compatibility files, `stplugin.c` and
`stplugin.h`.

The source-local `varcomp_kss` command exposes only the explicitly consented
JLA + match-deletion + diagonal-PCG subset. Its public support mask is 38;
exact, observation deletion, controls, CMG, scale, and automatic Rust routing
remain disabled. Omitted, Mata, and auto backend requests stay on Mata. Public
release remains disabled by the provenance and platform gates in
`../IMPLEMENTATION_STATUS.md` and `../TEST_PLAN.md`.

## Qualify a local macOS candidate

On an Apple Silicon Mac with licensed Stata 18 or newer installed at its
standard path, run from the repository root:

```bash
rust/stata_backend/qualify_macos.sh \
  --receipt /private/tmp/vckss-macos-candidate-receipt.txt \
  --artifacts-dir /private/tmp/vckss-macos-sanitized-evidence
```

Use `--stata /absolute/path/to/stata-mp` for a nonstandard Stata installation.
The receipt path is mandatory and must not already exist.

The optional artifacts directory must exist and be empty. It receives only
sanitized Stata command transcripts, source hashes, and exact candidate
binaries; startup banners and raw logs are still deleted.

The qualifier authenticates the pinned SPI inputs; runs the standalone plugin
crate's Rust 1.81.0 formatting, Clippy, and unit tests; uses Rust 1.81.0 to build
arm64 and x86_64 slices from one source snapshot; ad-hoc signs the thin slices
and a true universal binary; and verifies their architectures, deployment
floors, install IDs, dependencies, signatures, and required exports. It then
runs the plugin lifecycle, bounded Rust--Mata diagnostic, fixed shared-atom
differential, strict public command route, routing matrix, and isolated local
package install against the exact thin arm64 candidate. It separately loads
and exercises the universal candidate. When Rosetta is available, it repeats
the architecture-sensitive cases against the exact thin x86_64 candidate and
the universal x86_64 slice. Every test must emit its explicit PASS marker
because Stata batch exit status is not reliable evidence by itself.

Raw Stata logs and the temporary test installation are deleted on exit, so
license banners are never copied into the repository. When `--artifacts-dir`
is used, each retained transcript begins at Stata's first batch prompt and is
safe to upload as CI evidence. Only after all required
checks pass does the script stage ignored thin and universal plugin candidates
under `varcomp_kss/`. The explicit receipt contains source, SPI, binary, and
artifact hashes plus the verified build and test facts. A run from a dirty
worktree is labeled `LOCAL_CHECKPOINT_DIRTY_TREE`, not a clean qualification.
Rosetta coverage is compatibility testing on Apple Silicon; it is not native
Intel hardware qualification. This script makes no Windows, Linux, scale,
public-release, or production-support claim.

## Manual build on macOS

For development work that does not need the candidate receipt:

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
  varcomp_kss/varcomp_kss_rust_macos_arm64.plugin
```

Resolving the exact executables is intentional. On some rustup installations,
`rustup run` launches Cargo without making that toolchain's sibling `rustc`
available to Cargo's child process.

The fetch helper accepts `VCKSS_STATA_SPI_DIR` when the two SPI files should
live elsewhere. Both the helper and `build.rs` verify them against the tracked
`stata-spi.sha256` manifest. Local SPI files, Cargo outputs, and staged plugin
binaries are intentionally ignored by Git.

A manual host build is architecture-specific. Do not rename it to
`varcomp_kss_rust_macos.plugin`; only the qualifier constructs and verifies a
universal local candidate, which remains developer-only and unpublished.

## Developer integration tests

The qualifier is the preferred way to run the integration tests. To inspect a
single test while developing, invoke its do-file in batch mode and check the
explicit marker:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do \
  varcomp_kss/tests/stata/test_rust_plugin.do \
  /absolute/path/to/checkout/varcomp_kss/varcomp_kss
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do \
  varcomp_kss/tests/stata/test_rust_mata_diagnostic.do \
  /absolute/path/to/checkout/varcomp_kss/varcomp_kss
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do \
  varcomp_kss/tests/stata/test_rust_mata_shared_atoms.do \
  /absolute/path/to/checkout/varcomp_kss/varcomp_kss
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do \
  varcomp_kss/tests/stata/test_rust_public.do \
  /absolute/path/to/checkout/varcomp_kss/varcomp_kss
```

Expected markers are:

```text
VARCOMP_KSS RUST PLUGIN PASS
VARCOMP_KSS RUST MATA DIAGNOSTIC PASS
PASS test_rust_mata_shared_atoms.do
PASS test_rust_public.do
```

The lifecycle test exercises plugin loading, ABI and capability probes,
prepare, retained-mask alignment, solve, result receipts, snapshot, cleanup,
and idempotent release. The bounded diagnostic compares the Rust result with
the existing Mata estimator. It is not fixed-seed parity evidence because Rust
uses the documented `VCKSS-COUNTER-V1` generator while Mata uses Stata's
registered `mt64s` streams. The shared-atom test supplies a fixed independent
Counter-V1 oracle to both implementations and checks retained-sample, result,
residual, and caller-RNG invariants.
The public-route test checks strict routing, public/helper lifecycle equality,
the authoritative returned sample mask, lossless receipts and accounting,
row-order and batch invariance, cleanup after native and Stata-side faults,
corrupt-receipt rejection, structured native errors, and caller state. The
qualifier separately repeats it after `net install` into an isolated PLUS
directory. A second canonical install check proves that the tracked
helper-only manifest has no native artifact and returns typed
`RUST_BACKEND_UNAVAILABLE` for an explicit Rust request.

Do not commit raw Stata logs: the startup banner can contain license-holder
information.
