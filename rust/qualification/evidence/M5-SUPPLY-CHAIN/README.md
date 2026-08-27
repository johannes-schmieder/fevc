# M5 alpha Rust supply-chain evidence

The exact-source directory under this tree preserves the normalized RustSec,
license-inventory, and CycloneDX 1.5 outputs for the private
`0.4.0-alpha.1` full-CMG candidate. The gate audits all three committed Rust
lockfiles, includes the vendored CMG dependency, and records the pinned
qualification tools. Raw advisory clones, temporary source archives, raw
audit JSON, and unnormalized SBOMs are deliberately not retained.

The receipt excludes human license/provenance approval, Windows, and a public
release claim. Reproduce from a clean `main` checkout with:

```bash
rust/tools/run_supply_chain_checks.sh \
  --receipt RECEIPT_PATH --sbom-dir SBOM_DIRECTORY
```
