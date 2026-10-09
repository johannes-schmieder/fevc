# CMG default replay and refinement refresh — October 9, 2026

All five payloads are rebuilt from `eec825d6` and adopted locally after their
registered platform qualification and HTTP fresh/replacement installation gates.
[The manifest](manifest.json) binds actual sources, exact bytes, corresponding
source and bounded receipts. Commit `a8617882` fixes the replay gate and stayer
reporting without changing native bytes; [its compatibility review](evidence/source/compatibility-A.json)
retains the preceding payload identities.

Mac thin arm64, x86-64 under Rosetta and universal qualification passes, including
four installed defect cases. Linux SCC job `7981653` passes full and installed
qualification. Windows hosted build `37938556177` and all Rust/source jobs pass;
private smoke `win-20261009T135537Z-6d364439` and full
`win-20261009T140221Z-834b89b4` pass with cleanup/source restoration and stopped
state verified. All five actual build sources are `eec825d6`.

Local HTTP fresh/replacement checks verify 61 installed files and all five
plugin hashes per case, the new regressions, installed Mean component checks,
and public Veneto automatic-batch projection on Mac arm64. The separate four
Mac installs also cover the public Veneto default call. The source archive
contains 1,528 exported exact Git files, with no row-level fixtures.

[The checkpoint](checkpoint.json) preserves the earlier rejected push and every
failed development attempt. Historical adoption records are unchanged. Public
HTTP checks await publication authorization. No tag or release is created.
Native Intel hardware, scale performance and statistical coverage are not claimed.
