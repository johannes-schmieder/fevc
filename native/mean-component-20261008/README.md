# Mean component inference: qualification checkpoint

Source implementation and the registered assessment are complete. Four new Mac/Linux candidates pass source-bound qualification and installed Mean checks. Windows remains blocked after its bounded source-build repair/retest; the prepared hosted-build alternative awaits the owner’s sequencing approval. The shipped plugin files are unchanged. This is not a complete five-payload adoption or a release.

`checkpoint.json` binds the completed evidence and candidate hashes. Historical receipts are unchanged. Byte-preserving source, platform, harness, and assessment records are under `evidence/`; corresponding Git source archives are alongside this file. The original raw per-task results and verbose sanitized logs are retained in the task’s ignored local evidence store. Manifest and result hashes permit reproduction from the committed harness.

The assessment has no engineering failures, but exact Mata has 22 failed availability screens and 38,090 unavailable target rows. Native primary cells have full availability and pass the broad descriptive screens. Numerical and stress diagnostics expose additional limitations. Read the assessment report in `fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md`; none of these records supplies a general coverage guarantee.

The integrated checker retains its development-tree scope. Clean Rust checks bind implementation commit `63757839`; clean Mac qualification and native sampling bind `b9f80ce9`. Their four-file Windows-only difference and the earlier development source limitations are explicitly reviewed.
