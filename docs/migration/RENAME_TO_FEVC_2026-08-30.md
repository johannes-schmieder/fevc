# Rename to `fevc` (2026-08-30)

The public Stata package, command, help entry, and repository are renamed from
`vckss` to `fevc` as a hard cut. The release remains `0.5.0-alpha.1`; the rename
does not introduce an alias, compatibility wrapper, tag, or release.

The immutable pre-rename baseline is commit
`a5805145b93961a98068de3452ff793013847942`, tree
`7a3753d0a8550b3de7e8dee32e5671dc35b6d05d`. The rename qualification archives
that revision and the committed candidate, then runs all 16 registered success,
failure, exact, compressed, JLA, CMG, automatic-routing, and Rust cases in fresh
Stata processes. Scientific records are compared exactly in binary64 hex or
text after normalizing only the public command identity, paths, registered
timings, and runtime-dependent memory footprints. The pre- and post-rename
private Rust plugins must be byte-identical.

Private implementation identities intentionally remain stable. This includes
the `vckss__`, `_vckss_`, and `VCKSS_*` namespaces; private Mata and ado runtime
filenames; Rust crate names; the C ABI; plugin filenames; build identifiers; and
receipt schemas owned by the private backend. Existing local runner installation
paths, runner names and labels, and SCC paths also retain `vckss`, because they
are operational identities rather than public package names.

Historical evidence is not rewritten. The history audit binds 2,612 source
files and the pre-rename changelog tail to the baseline Git blobs, including
dated reviews, receipts, qualification artifacts, and the earlier public-name
migration evidence.

Users must update active installations and callers in one step:

```stata
fevc outcome controls, worker(worker_id) firm(firm_id)
help fevc
```

The old `vckss` public command is absent from the new package.
