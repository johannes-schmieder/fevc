# Pooled component inference binary adoption — October 10, 2026

All five binaries are rebuilt from `002205f2804a7fc8e54a7a4ad1d48194bc04c6d0`
and adopted for the repository installation package under explicit owner
authorization. They include pooled mover–stayer structured component inference,
mixed capability bit 17, unit receipt V2, and bounded diagonal residual refinement.

The [manifest](manifest.json) binds binary hashes, build source, platform evidence
and the [verified corresponding source](corresponding-source.json). Mac arm64
and Rosetta x86-64 thin/universal qualification passes, including 16 pooled cases
and two clean installs. Linux full and installed qualification passes in SCC job
8015131, including eight pooled cases. Source checks pass 894 Python tests,
CMG assembly, license audit, and the hosted source/Rust matrix.

Windows passes hosted build, Rust unit tests and PE/import/export checks.
The owner explicitly requests publication now and will run Windows later.
[Windows adoption](windows-manual-adoption.json) therefore records runtime
qualification as pending. Authentication is left unchanged. Earlier Windows
runtime receipts do not qualify this new binary.

The package uses the existing deterministic inventory renderer with an explicit
manual-test adoption record. It does not claim the qualified `complete` builder
gate, change that gate, or turn build-only evidence into runtime qualification.
The staged catalog contains 61 installed files and all five plugins. Installation
receipts under `evidence/packaging/` bind actual bytes and apply runtime claims
only to their recorded platform. No tag or native release archive is created.

Original platform receipts are copied unchanged; their historical exclusions
and authentication status describe the original test runs. Sanitized detailed
transcripts remain in the local diagnostic collection; compact validation
records preserve their digests. This engineering evidence does not establish
general statistical coverage, native Intel hardware behavior, or controlled
large-data performance.
