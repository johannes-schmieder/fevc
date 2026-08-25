# M4 Rust supply-chain evidence

The source-bound evidence directory
`2888b838ac744ec72c90373ff54402ab025d7e9b/` was generated from a clean
archive of that exact commit with:

```bash
rust/tools/run_supply_chain_checks.sh \
  --receipt RECEIPT_PATH --sbom-dir SBOM_DIRECTORY
```

The gate pins `cargo-audit 0.22.2` and `cargo-cyclonedx 0.5.9`, audits all
three committed lockfiles against one RustSec snapshot, and rejects every
vulnerability or warning. It emits deterministic CycloneDX 1.5 JSON for the
core, plugin, Stata backend, and fuzz workspaces, plus a lockfile/audit
summary, component-license inventory, and SHA-256 manifest. Registry
components require Cargo package URLs and SHA-256 hashes; git dependencies,
unreviewed licenses, and machine-local paths fail the gate.

The one registered metadata bridge is `version_check 0.9.5`'s published
legacy `MIT/Apache-2.0` spelling, interpreted as `MIT OR Apache-2.0`. The raw
advisory checkout, exact-source archive, audit JSON, unnormalized SBOMs, and
tool build products were temporary and deleted. This automated inventory does
not claim final human license/provenance approval or public-release readiness.
