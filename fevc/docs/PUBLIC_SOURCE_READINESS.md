# Public-source preparation — 2026-09-10

This is a local cleanup and audit of the working tree based on commit
`929a1d77a2fec6e9e1a78550a84d1a65b2ad51fd`. It does not change repository
visibility or authorize a tag, release archive or native binary distribution.

## Repository cleanup

The active plan is reduced from 573 to 57 lines, and the documentation index
from 447 to 61 lines. Completed checkpoints now have a
[historical evidence index](EVIDENCE_INDEX.md); the obsolete root Rust plan is a
pointer to its original Git version. `finalize_fevc.md` is retained because the
residual-moment experiment binds it as an input. Accepted reports, reviews,
source manifests, migration records and benchmark evidence remain unchanged.

Generated `output/` and `tmp/` are ignored. The cleaner now preserves `.local/`,
generated research outputs and installed qualified plugins, avoids symlink
traversal, retains unignored children, and makes Rust build-cache removal
explicit. Useful loose logs and disposable Stata run files are copied and
hash-verified in local diagnostics before removal. In total, 333 disposable
files (21,677,456 bytes) were removed; 133 useful run/log files were archived.
The owner's Windows,
platform and five-way scaling work remains in place.

The tracked tree before cleanup contained 2,343 files and 36,961,748 bytes,
with no tracked `.log` or `.smcl` files. Most larger tracked files are retained
scientific or migration evidence, rather than disposable logs.

## Public-source inspection

Gitleaks 8.30.1 was downloaded from its
[official release](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1)
and verified against
the published SHA-256 checksum. The scan used its default rules, full redaction,
and ignored inline allow comments. The 15 local refs reach 1,512 commits.
The `git --log-opts=--all` scan reported 1,433 commits with scanned changes and
zero credential matches. A direct scan of all 8,181 reachable file blobs,
including archives through depth two, also found zero matches.
A separate export of 2,421 tracked and nonignored untracked working files was scanned
with archive inspection through depth two: zero matches.

The reachable path inventory includes historical benchmark PDFs/TeX and two
manual benchmark `.dta` outputs; their variable inventories describe benchmark
runs and timings. Twenty archived review ZIP inventories were inspected; the
retained PPML numerical theory is part of the frozen CMG review/provenance
record. Three additional generated figure/table paths occur only outside main
in local development refs. These records were preserved, not republished or
removed from history. A supplemental screen of all 8,181 reachable blobs
(199,558,644 bytes) found no Stata serial-number or private-key signatures.
Its 35 licensee-label matches were all versions of the qualifier code that
removes those labels from logs.

The scan covers locally reachable refs and the inspected working tree. It
cannot establish the absence of every private datum, nor cover remote-only or
unreachable objects. Before an eventual visibility change, refresh the relevant
refs and repeat the scan against the exact tree and references to be exposed.
No history rewrite or visibility operation was performed.

## Validation and evidence boundary

All 800 Python tests pass (82.36 seconds), including focused cleanup, parity,
public-identity, immutable-history, legacy-name and acceptance-policy checks.
CMG assembly verification, the license audit, portable-artifact determinism and
local navigation links pass. Pytest again emitted pre-existing warnings while
removing older protected temporary test directories after its passing result;
there were no test failures. The current [backend ledger](RUST_MATA_PARITY.md)
now names the RC target and distinguishes source-specific platform evidence
from current release qualification.

The 2,409 files outside the cleanup edits match the pre-cleanup snapshot,
including unfinished owner work and historical evidence. All 156 entries in
the API-24 runtime/native source manifest match their
validated hashes, and all installed plugin hashes are unchanged. Estimator
code, native ABI, build inputs and numerical acceptance policies are unchanged
by this cleanup. The API-24 local checkpoint's claims remain
limited to its recorded source and binary identities. Existing Windows harness
changes are separate unfinished work. This documentation/tooling cleanup does
not rerun Stata, rebuild plugins or promote historical evidence to a new
all-platform qualification.

Local audit logs, exact ref inventory, before/after manifests, archived loose
logs and redacted scanner reports are under
`.local/diagnostics/repository-cleanup-20260910/` and remain ignored.
The [current plan](../PLAN.md) identifies the outstanding Windows/private-payload
work. Public source visibility and complete native release qualification are
separate decisions; the latter still needs the platform and final-artifact
checks in [RC binary preparation](RC_BINARY_PAYLOAD.md).
