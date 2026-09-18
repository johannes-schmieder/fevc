# Current checkpoint — 2026-09-18

Prepare a clean public-facing checkout: concise user documentation, one-step
installation with native binaries, maintained source and regression tests,
and historical output outside the active tree. GPT Pro review packets,
responses, and adjudications are removed at the owner's request.

## Preserved implementation work

The uncommitted optimization and symmetric deletion/stayer work remains in
place. The preserved optimization campaigns launched September 16: 400k array `7589985`,
1.6m array `7589987`, and private Veneto array `7589989`, with success-gated
aggregators `7589986`, `7589988`, and `7589990`. These are last recorded
launches, not a claim about their present scheduler status.

The frozen candidate is `b14496bc`; the extension manifest is `9c4bc90a`.
The working tree includes additional experiments and is not interchangeable
with the qualified candidate. Detailed handoffs remain in ignored
`.local/pipeline-optimization-20260915/` and the existing optimization
experiment directories. The complete previous plan was preserved under
`.local/public-cleanup-20260918/previous-guides/fevc/PLAN.md`.

## Native qualification — owner request of September 18

The owner authorized purging review history, creating qualified platform
binaries, and verifying installers. Use the current preserved implementation
as the candidate; freeze its exact source before platform qualification.
Results and run manifests belong in ignored `.local/public-release-20260918/`.
Mac arm64, Rosetta x86-64, universal, and Linux x86-64 qualification passed
for source `6a8ddc9dce6eb6c40ffeb9e4f34787de36340fda`; Linux job `7637232`
finished with scheduler failure and exit status both zero. The four collected
binaries match their qualification hashes. Windows collection still needs the
prepared controller extension and owner approval. Remote history replacement
also awaits explicit force-push approval; GitHub PR refs need support cleanup.
Do not infer success from historical binaries or change scientific acceptance
criteria. Repository visibility and public distribution are not changed by
this preparation.

## Publication prerequisites

- Assemble matching Mac, Linux, and Windows binaries with the installable
  Stata source. Finish Windows qualification and final package installation
  checks; historical platform receipts do not qualify a changed runtime.
- Verify both advertised installers, including `github` branch compatibility.
- Audit the exact tree and reachable refs before making the repository public.
  The local review-history purge is complete; GitHub PR refs need server cleanup.
- Public visibility, pushes, tags, and binary distribution remain separate
  owner actions; this cleanup performs none of them.

See [installation status](../INSTALLATION.md),
[native packaging](docs/RC_BINARY_PAYLOAD.md), and
[the archive note](../docs/ARCHIVE.md).
Candidate promotion and evidence reuse follow
[`development_acceptance_v1.json`](docs/development_acceptance_v1.json).
