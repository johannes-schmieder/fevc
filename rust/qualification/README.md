# Native qualification

Keep new source-bound receipts and binaries in ignored `.local/` directories.
The platform qualifiers also accept explicit external evidence directories.
Do not commit raw logs, installed plugins, or per-run output.

Each receipt must name the tested source, binary hashes, platform, commands,
and result. A receipt does not qualify later runtime changes automatically.
See [the native test plan](../TEST_PLAN.md) and
[historical material](../../docs/ARCHIVE.md).
