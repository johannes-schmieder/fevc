# M5 full-CMG macOS qualification

The exact-source directories under this tree preserve the private
`0.4.0-alpha.1` normal-plugin qualification after the SCC launcher repair.
Directory `fddc50f842c584ab194514df00467539639b8ea7/` is the final-source
rerun after the Linux-only `libm` and direct pinned-`cargo-fmt` qualifier fixes; the
earlier directories remain as source-bound milestone evidence.
It covers thin arm64 and x86-64 candidates, the universal plugin, Rosetta,
clean installation, Rust/C/ABI checks, lifecycle, explicit Rust and automatic
`CMG_FULL_V2` routing, memory/refinement, and cancellation. Candidate hashes
are recorded in the receipt; binaries and raw Stata logs are not committed.

This is a private macOS qualification, not a Windows or public-release claim.
Reproduce from clean `main` with:

```bash
rust/stata_backend/qualify_macos.sh \
  --receipt RECEIPT_PATH --artifacts-dir EMPTY_ARTIFACT_DIRECTORY
```
