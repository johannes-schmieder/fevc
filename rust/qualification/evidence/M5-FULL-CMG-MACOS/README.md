# M5 full-CMG macOS qualification

The exact-source directory under this tree preserves the private
`0.4.0-alpha.1` normal-plugin qualification after the SCC launcher repair.
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
