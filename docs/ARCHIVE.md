# Historical material

The September 18, 2026 cleanup removes accumulated development output from the
public-facing checkout. Maintained implementation, regression tests, reusable
benchmark harnesses, scientific contracts, licenses, and provenance remain.

## Preserved records

831 historical files were copied byte-for-byte into the owner's ignored
local archive and SHA-256 verified before removal. The archive retains original
relative paths, alongside a machine-readable file inventory. It includes old
CI and platform receipts, rename-equivalence output, migration records, dated
progress notes, and superseded reports. The archive is not a runtime or test
dependency.

The local archive and inventory are in
`.local/public-cleanup-20260918/archive/` and
`.local/public-cleanup-20260918/archive-manifest.json`. Previous versions of the
edited guides and the uncommitted checkpoint are preserved separately under
`previous-guides/` in that directory.

Committed historical files can also be inspected at the review-purged source
`ffca8b5cfc0ff8c495923c00d93ca292528e57d9`. For example:

```bash
git show ffca8b5cfc0ff8c495923c00d93ca292528e57d9:fevc/docs/EVIDENCE_INDEX.md
```

Historical links in maintained documentation point to that recorded source.
They describe its results and limitations, including failures; removing an old
report from the active tree does not change its outcome or qualify new code.

## Deleted reviews

At the owner's explicit request, all GPT Pro request packets, responses,
adjudications, and qualification reviews were deleted from the checkout.
They were not copied into the cleanup archive. The former tests that required
retaining review packets and rename-era records have been retired; current
public-identity, license, estimator, and packaging checks remain.

The owner subsequently authorized removal from Git history. The local history
and snapshot references have been rewritten to exclude both review directories;
old reflogs were expired and unreachable objects pruned. The pre-purge source
`fbf8dcd6187351d09cb3de735c148ef608ec0219` maps to
`ffca8b5cfc0ff8c495923c00d93ca292528e57d9`. Scientific receipt identifiers remain
unchanged because they describe their original tested artifacts. GitHub-managed
pull-request refs require server-side removal before a complete remote purge
can be claimed. Audit the exact history and references before public visibility.

The [commit map](HISTORY_MAP.txt) resolves original source identifiers to their
review-purged equivalents. It contains commit hashes only, not review content.
Use it to locate preserved scientific records without changing their original
tested-source identifiers.

On September 21, 2026, the owner accepted leaving the historical GitHub PR
references in place for public publication. Those references can still expose
the former reviews. The cleaned current tree and rewritten main history remain
review-free; no complete server-side purge is claimed.
