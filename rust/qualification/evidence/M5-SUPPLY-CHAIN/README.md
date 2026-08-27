# M5 alpha Rust supply-chain evidence

The exact-source directories under this tree preserve the normalized RustSec,
license-inventory, and CycloneDX 1.5 outputs for the private
`0.4.0-alpha.1` full-CMG candidate. Directory
`fddc50f842c584ab194514df00467539639b8ea7/` is the final-source rerun; the
earlier directories remain source-bound milestone evidence. The gate audits
all three committed Rust
lockfiles, includes the vendored CMG dependency, and records the pinned
qualification tools. Raw advisory clones, temporary source archives, raw
audit JSON, and unnormalized SBOMs are deliberately not retained.

The receipt excludes human license/provenance approval, Windows, and a public
release claim. Reproduce from a clean `main` checkout with:

```bash
rust/tools/run_supply_chain_checks.sh \
  --receipt RECEIPT_PATH --sbom-dir SBOM_DIRECTORY
```
