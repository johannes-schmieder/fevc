# M4 BU SCC Linux evidence

The source-bound evidence directory
`86e0711b1b1d07cff3058461c558838f2233f1d1/` records the successful BU
SCC qualification of the Linux x86-64 candidate built from that exact commit.
SGE job `7305794` ran on `scc-gd4` with four slots and exited successfully
after 255 seconds.

The qualification receipt binds the source commit, transfer-bundle hash,
source-file manifest, candidate hash and ELF dependencies, Stata binary and
environment, Rust/C/ABI checks, full public Stata suite, and isolated clean
installation. `wrapper.txt` and `qacct.txt` independently record successful
wrapper and scheduler completion. `qualification-source.sha256` is the
complete staged-source manifest; `SANITIZED_EVIDENCE.txt` records the log
sanitization boundary. Raw licensed-Stata logs, startup/license identifiers,
the build cache, source transfer, and candidate binary are retained only in
the access-controlled SCC run directory and are not committed.

This evidence qualifies the recorded Linux x86-64 candidate under Stata MP
19. It does not qualify performance scale, Windows, macOS, public release, or
human mathematical/license/provenance review.
