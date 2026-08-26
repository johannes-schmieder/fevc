# macOS synthetic-matrix MATLAB authentication failure

## Status

The registered macOS arm64 synthetic A/C/MATLAB matrix at source `787327f`
stopped in the first cold MATLAB cell because MATLAB R2024b was signed out
after the host reboot. This is comparator infrastructure failure, not a VCkss,
scientific, process-tree, or performance result. No partial timing enters a
median or promotion decision.

The maintained-MATLAB source guard passed before the matrix started. The cold
baseline and candidate Stata cells also completed, at 218.015 and 73.672
seconds respectively, but they are preserved only as partial diagnostics
because the registered matrix requires all 18 fresh-process applications.

## Failure boundary

- host: `Mac-Studio.local`, Apple Silicon
- operating system: macOS 26.6.2 build 25G83
- VCkss source and harness: `787327f61eb35c2f61b400ed6d288c131bd0c919`
- baseline source: `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- standalone CMG source: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- MATLAB executable: `/Applications/MATLAB_R2024b.app/bin/matlab`
- case SHA-256:
  `95d676007deb0ad6671a9b76481bbab756d97bc15edfedc8232253840e7f7a37`
- input SHA-256:
  `19744b8418ffff82461527d7426cdc976dce18bf36597555d19bf847dec5afbe`
- maintained-MATLAB source-identity SHA-256:
  `a5c5be72245f9e2269f86a7065650c40c85c4de5705666ac93ea263e965f67a6`
- candidate-build receipt SHA-256:
  `8562fdfab36a86972db60eb3c832de3e084fc7ff331a786c7961764f895c697d`
- MATLAB failure-log SHA-256:
  `e8b5ce8be80f3b275e365610694d033270efca60e68ffd458c948d9c931221a5`
- process-tree receipt SHA-256:
  `ae923dcd04975fa5aa6816d9f0ebc34325f540ca9cbecbd054b2a1c81f69c6fc`
- resource receipt SHA-256:
  `75b4b566c5a2a045c3e474184e6c73bb2ad4bdd92c431836ab0ee116ff07625b`

MATLAB returned code 1 after prompting for a MathWorks Account email address.
The process-tree monitor correctly recorded `FAIL_BEFORE_IDENTITY` rather than
accepting the 15.05-second failed launch as application timing.

## Recovery

Sign MATLAB R2024b back in interactively on the Mac Studio, verify a bounded
`matlab -batch` command exits successfully, and rerun the unchanged registered
matrix in a new output directory. Do not resume this partial directory or
reinterpret its cold Stata cells. The SCC matrix is independent and continues
under MATLAB R2025b.

The compact failure receipt is
[`macos_synthetic_matrix_matlab_auth_failure_2026-08-26.json`](macos_synthetic_matrix_matlab_auth_failure_2026-08-26.json).
