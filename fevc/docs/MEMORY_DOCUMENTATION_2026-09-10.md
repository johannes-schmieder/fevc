# Memory documentation update, 2026-09-10

The help file, active package and backend documentation, and companion paper
now describe the implemented optional-budget policy. The user guide is
[MEMORY.md](MEMORY.md). This revision changes documentation and the paper's
feature-table generator; it changes no estimator, native build input or
acceptance threshold.

## Documented behavior

- Omitted `memory_gib()` means no assumed budget or memory-based batch,
  concurrency or route adjustment. Ordinary performance choices remain.
- An explicit budget guides automatic planning and defaults to
  `memorycheck(warn)`. `error` requests strict forecast admission; `off`
  suppresses memory warnings and rejection. A policy alone creates no budget,
  and an explicit numeric batch is preserved.
- Native preparation is measured as it runs, and constructed solver storage
  refines the forecast before estimator RNG. Preparation can already have
  allocated memory when a strict check rejects a request.
- Expected direct allocations and the conditional admission reserve are
  separate. Neither is total process RSS. Mata retains formula-based
  forecasts, and inference/stayer bounds remain conservative.

The four tested full-CMG allocation cases were 0.17–2.64% above independently
measured owned peaks. This is bounded development evidence, not a universal
5% accuracy, RSS or cross-platform qualification. The original
[implementation report](MEMORY_FORECAST_2026-09-10.md),
[registration](memory_forecast_v1.json) and
[result](memory_forecast_v1_result.json) remain byte-identical.

## Compatibility review

The baseline is the final September 10 native checkpoint: source HEAD
`f3098bc1369992fccbc1d276aac5fc65ceb3f404` with owner changes, source-manifest
SHA-256 `7d9f441f2e542c2b983c8bcf5e7adec6f23c2740c553a70a2901f7f21987b07b`
and receipt SHA-256
`30c3afcb8edf80ae2ed12afac8e2ec13764bd8b916a5561fec9e5ee7a6166675`.
The comparison is to this documentation revision in the same dirty worktree.
Of the 156 manifest inputs, only `fevc/README.md`, `fevc/TESTING.md`,
`fevc/fevc.sthlp` and `rust/stata_backend/README.md` changed; the other 152
inputs and all three installed macOS plugin hashes match. This retains the
recorded memory implementation checks for the unchanged executable surface.
It does not transfer historical paper performance or coverage evidence to
the new memory implementation.

The companion paper's
[update record](../../../fevc-paper/MEMORY_DOCUMENTATION_2026-09-10.md) and
[source/claims manifest](../../../fevc-paper/replication/manifests/memory_documentation_20260910.json)
record before/after documentation hashes, exact binary identities, copied
evidence, validation commands and delivered artifacts. Historical benchmark
inputs, outputs, receipts, comparator columns and scientific gates are
unchanged by this documentation revision. No new numerical experiment,
native rebuild, commit, publication or release was performed.

## Validation

- `./.venv/bin/python -m pytest -q`: **798 passed**. Pytest emitted a
  temporary-directory cleanup warning after the successful run.
- `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check`: **PASS**.
- Focused policy, documentation, packaging and CMG tests: **70 passed**.
- Companion paper Python tests: **122 passed, 17 skipped**; the skipped
  corruption tests require archived-run fixture environment variables.
- Paper editorial validation and generated feature-overlay check: **PASS**.
  Only the memory-policy row changed; the other 54 rows and all comparator
  columns are unchanged.
- LaTeX build: **PASS**, 58 pages, including 21 main-text pages. Every page
  was visually reviewed, and changed pages were reviewed again after layout
  fixes. The source and delivered PDFs are byte-identical.
- Phone HTML: all 15 embedded figures load, both new sections and navigation
  links are present, and a 390-pixel viewport has no horizontal overflow.

The broad historical `replication/scripts/validate_paper.py` check was also
attempted and **failed** at `analyze_direct_match_scaling.py` with
`current source/binary hash`: the current memory implementation does not match
that archived benchmark identity. The strict historical gate is unchanged.
Passing editorial checks do not replace this source-bound runtime audit.
Licensed Stata and native checks were not repeated for documentation-only
changes; their exact-source memory results are retained under the comparison
above. Diagnostic logs and visual-review artifacts are in the ignored
`.local/diagnostics/memory-documentation-20260910/` directory.
