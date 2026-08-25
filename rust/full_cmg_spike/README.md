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

The builder requires a clean standalone CMG checkout at commit
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10` and, by default, a clean VCkss
checkout. It emits an ad-hoc-signed arm64 plugin and a source/build receipt
under a caller-selected temporary work directory. It never installs or ships
the candidate.

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

Activate the route in the Stata process with all three variables:

```bash
VCKSS_PRIVATE_CMG_FULL_V1=1 \
VCKSS_PRIVATE_CMG_THREADS=4 \
VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp ...
```

The private route fails closed unless the frozen public plan selected CMG for
the compressed no-control JLA family and both repeated-solve batch requests
remain automatic. It retains VCkss's independent complete original-system
residual gate. There is no post-RNG fallback.
