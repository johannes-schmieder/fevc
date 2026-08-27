# M5 full-CMG Linux x86-64 SCC qualification

The exact-source directory under this tree preserves the accepted private
`0.4.0-alpha.1` normal-plugin qualification for source
`992eba0947ca155c534f532750fc202e41ecf978`. SCC job `7330577` uses the
deterministic bundle
`633758a3e784c4c48d35d421cf4627d915e63ad1db4e59f4116c34e3ad5d348e`,
the VCkss-owned Rust 1.85.1 toolchain, four slots, and Stata/MP 19. The compact
packet retains the Linux qualifier and wrapper receipts, sanitized evidence,
source manifest, scheduler accounting, and a checksum manifest. Native
binaries and licensed or raw Stata logs are deliberately excluded.

The following non-accepted attempts remain preserved as engineering evidence
and do not enter the qualification claim:

| Job | Source | Disposition |
| --- | --- | --- |
| `7328518` | `4b6874e` | rejected before build because the pinned SCC toolchain initially lacked `rustfmt` |
| `7328596` | `4b6874e` | rejected before build because the wrapper dropped the Stata module path |
| `7329012` | `bf5e06e` | rejected because the Linux C error-transport fixture omitted `libm` |
| `7330065` | `a1053f9` | rejected because `cargo fmt` resolved the ambient rustup proxy |
| `7330154` | `3385b97` | never started during a scheduler outage; its scheduler record was preserved before cancellation |
| `7330258` | `3385b97` | rejected because setting `RUSTFMT` did not bypass the separate ambient `cargo-fmt` proxy |
| `7330265` | `fddc50f` | rejected because `cargo clippy` resolved the separate ambient `cargo-clippy` proxy |
| `7330293` | `9aba2ed` | rejected because direct `cargo-clippy` requires the literal `clippy` subcommand before Cargo options |

The final qualifier invokes pinned `cargo-fmt`, `rustfmt`, and
`cargo-clippy clippy` directly. Raw failure logs, source/bundle bindings, and `qacct` records remain
under the corresponding access-controlled directories below
`/projectnb/welfgr/vckss/runs/`; compact local copies were retained under
`/private/tmp/vckss-scc-qualifier-failures/` during qualification. Historical
evidence is source-bound and is not rewritten as the implementation advances.
