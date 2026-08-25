# Private direct full-CMG spike

This directory builds the deliberately private `CMG_FULL_SPIKE_V1` route.
It links the source-bound standalone CMG checkout directly into the existing
VCkss hybrid-Laplacian batch path. The route is a performance experiment, not
a public backend, ABI, package feature, or qualification claim.

Ordinary VCkss builds remain parseable and buildable with Rust 1.81. The spike
uses Rust 1.85.1 and supplies the standalone crate as an external Rust library
only while the private Cargo feature is enabled. This separation is
intentional: Cargo 1.81 cannot parse CMG's edition-2024 manifest even when an
ordinary optional path dependency is disabled.

The builder requires the standalone checkout to contain commit
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10` and, by default, a clean VCkss
checkout. It builds an exact `git archive` of that commit regardless of the
checkout's current `HEAD`, so unrelated commits and uncommitted work in the
standalone checkout are ignored and left untouched. The receipt records both
the frozen source commit and the checkout head observed at build time. The
builder emits an ad-hoc-signed arm64 plugin and receipt under a caller-selected
temporary work directory. It never installs or ships the candidate.

From the VCkss repository root on Apple Silicon:

```bash
VCKSS_CMG_ROOT="$GIT_HOME/CMG" \
VCKSS_SPIKE_WORK_ROOT=/private/tmp/vckss-full-cmg-build \
  rust/full_cmg_spike/build_macos.sh
```

For a source-local benchmark only, copy the resulting plugin to the ignored
runtime filename:

```bash
cp /private/tmp/vckss-full-cmg-build/candidate/vckss_rust_macos_arm64.plugin \
  vckss/vckss_rust_macos_arm64.plugin
```

Activate the scalar full-CMG route in the Stata process with all three variables:

```bash
VCKSS_PRIVATE_CMG_FULL_V1=1 \
VCKSS_PRIVATE_CMG_THREADS=4 \
VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp ...
```

Add `VCKSS_PRIVATE_CMG_FUSED_V1=1` to select the private fused independent-PCG
executor. Its source lives in `cmg_fused.rs` and is injected into the exact CMG
archive at build time. This is not a patch to the standalone checkout. The
executor keeps a contiguous column-major boundary block, converts admitted
16-RHS sub-blocks to a vertex-interleaved solve layout, traverses shared sparse
operators across the block, and preserves an independent PCG recurrence and
convergence mask for every column. Single-RHS fit solves continue to use the
certified scalar/planned CMG path. The current spike is deliberately limited
to connected hybrid graphs; a disconnected graph fails before estimator RNG.

Add `VCKSS_PRIVATE_CMG_MIXED_V1=1` together with the fused flag to select the
mixed-precision experiment. The finest hybrid operator, every PCG vector and
reduction, solution reconstruction, and residual certification remain `f64`.
Only the copied hierarchy operators, inverse diagonals, and hierarchy vector
traffic use `f32`; conversion buffers and their retained bytes are admitted
before estimator RNG. The mixed route is experimental evidence only and stays
disabled unless it is at least 10% faster than fused `f64`, materially reduces
memory, and passes the unchanged statistical and complete-residual gates.

The private route uses a `1e-10` fit tolerance and MATLAB-like `1e-6` probe
tolerance by default. A registered tolerance ladder may override them with
`VCKSS_PRIVATE_CMG_FIT_TOLERANCE` and
`VCKSS_PRIVATE_CMG_PROBE_TOLERANCE`; both effective tolerances and their
complete-residual gates are emitted in the setup diagnostic.
Probe PCG uses the effective `1e-6` phase tolerance directly; it is not
silently tightened by two orders of magnitude. The independently recomputed
complete original-system residual remains bounded by `1e-5`. The deterministic
fit retains a private two-order inner margin because it has no Monte Carlo
acceptance envelope and accounts for only one RHS.

The private route fails closed unless the frozen public plan selected CMG for
the compressed no-control JLA family and both repeated-solve batch requests
remain automatic. It retains VCkss's independent complete original-system
residual gate. There is no post-RNG fallback.
