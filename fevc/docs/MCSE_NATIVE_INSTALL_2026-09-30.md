# MCSE native publication and installation — 2026-09-30

The MCSE implementation and all five updated plugins are published on `main`.
The public option remains `mcse(all|off)`, default all; conditional MCSE is a
developer option and `numericalmcse()` remains a compatibility alias. Supported
projection/inference results are preserved and receive no additional MCSE.

Source publication `048cc20f` contains the completed implementation, optimized
Mata replay, help/documentation, and qualified Mac/Linux payloads. Test-only
repair `eee457b3` is the Windows build source. Payload commit `4ca6864e` adopts
the exact qualified Windows DLL and native provenance. The
[adoption manifest](../../native/mcse-default-20260930/manifest.json),
[compatibility reviews](../../native/mcse-default-20260930/evidence/compatibility.json),
and [publication receipt](../../native/mcse-default-20260930/publication.json)
bind the sources and artifact hashes. Earlier receipts remain unchanged.

## Windows qualification

[Hosted build 36771390189](https://github.com/johannes-schmieder/fevc/actions/runs/36771390189)
passes Rust 1.85.1 static-CRT compilation, all 14 standalone backend tests, and
the canonical-header PE audit: 142 exports and only system dependencies. All
60 archived files match the build receipt. The DLL SHA-256 is
`1d1140b5031ee360d87c28c1a75f00243709089d63bb1e539f93046d7c211f1b`.

Private AWS run `win-20260930T202520Z-2a82f1e0` passes on Stata/MP 19 with
two processors. Its frozen driver checks a fresh isolated `net install`,
native lifecycle, observation/individual/match inference, pooled deletion,
projection, MCSE default/all/off/conditional and alias behavior, point and
attachment equality, RNG/caller state, installed DLL hash and idle registry.
The controller returns an aggregate source-bound PASS after those assertions;
raw licensed logs and individual project-check JSON are not collected. The
[Windows qualification](../../native/mcse-default-20260930/evidence/windows-qualification.json)
binds the exact hosted bytes to the unchanged committed source plus two
manifest-bound supplemental transfer inputs. The receipt retains its dirty
transfer classification. The instance is stopped, transient transfer objects
are deleted, and the shared lock is released.

## Public installation

At payload commit `4ca6864e`, all advertised command cases pass in isolated
Mac Stata/MP 19 directories:

| Command | Fresh | Replacement | Verified files per case |
| --- | --- | --- | --- |
| `net install fevc` from public `main` | PASS | PASS | 60 |
| `github install johannes-schmieder/fevc` | PASS | PASS | 60 |

Each case verifies all five downloaded binary hashes, installed command/help
paths, native runtime and help examples, decomposition, match q0/q1 inference,
pooled deletion, subsample equivalence, caller data/timers and idle registry.
Replacement restores an intentionally changed help file. The
[four-case receipt](../../native/mcse-default-20260930/evidence/public-install-macos.json)
records every installed hash and sanitized transcript identity. Windows public
HTTP/GitHub installation is not separately claimed: Windows used the exact
artifact in a fresh private isolated installation.

## Gates, retained failure and scope

The repaired source passes 866 Python tests, CMG assembly, local Rust workspace
all-target tests, MSRV formatting and strict Clippy, and the explicit independent
Python full-response/complex-step oracle. All six hosted Rust OS/toolchain jobs
and source CI pass; payload source CI run `36773589269` also passes. Forty-nine
focused package/documentation/Windows-source regressions pass after adoption.
Exact commands and receipt hashes are in the publication record.

First hosted Rust run `36770767638` fails across six jobs because an ordinary
core replay-width regression invokes the Python oracle without a repository
venv. This is a test-harness failure. The narrow thread-local test guard in
`eee457b3` confines full capture/oracle execution to its explicit validation
test, retaining the ordinary regression. The explicit oracle independently
passes; estimator code and CI policy are unchanged. The first successful
Windows build `36770798975` is superseded and its artifact is not adopted.

M0–M4 remain complete within their recorded development scope. The fresh
239,616-attempt calibration and 300-command timing results retain the limits of
the [MCSE development report](MCSE_DEFAULT_VALIDATION_2026-09-30.md). Original
Mac/Linux dirty-source classifications remain explicit; their build/runtime
inputs, scientific inputs, thresholds and adopted bytes are unchanged through
the recorded compatibility reviews. Generic Mata percentage overhead parity,
native Intel hardware, representative scale, broader Mata RNG calibration and
statistical coverage remain open. No release, tag, CI-policy change or large
cluster campaign is included.
