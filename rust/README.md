# Native backend

## Current 0.6.0 binaries

All five 0.6.0 plugins are built from `2082f209` with unmodified common
CMG `e5613e81` and the explicit 128-level limit. They include repeat-measurement
component standard errors and normal/q=1 intervals on the supported routes.
Mac arm64 and Rosetta thin/universal/installed, SCC Linux full/native/installed,
and all 18 private Windows groups pass. Stata 15.1 also passes the full suite
and installed-native tests on Intel/Rosetta; focused Stata 18/19 tests pass.
Windows qualification uses Stata 19 and one/two workers. Native Intel hardware
and Stata 16/17 runtime execution are not claimed.
The [adoption manifest](../native/repeat-measurement-20261115/manifest.json) binds all exact bytes,
corresponding source and qualification evidence. Publication/HTTP install
checks follow separately. Historical checkpoints below retain their scope.

## 0.5.2 Windows full-CMG binaries

All five 0.5.2 plugins share build source `ec5e1e0e` and the unmodified common
CMG revision `e5613e81`. Native Mac arm64/Rosetta, SCC Linux and all 18 private
Windows groups pass, including the registered complete-command comparison.
The [adoption manifest](../native/windows-cmg-20261107/manifest.json) binds
exact binaries, corresponding source and evidence. Windows scope is one/two
workers; Mac x86-64 runtime tests use Rosetta. Published installation checks
are recorded separately. Historical evidence below retains its original scope.

## November 13 implementation checkpoint (historical)

`vckss-core::residual_noise::repeat` adds a second standard error on the
leave-out residual pass, for match and for observation units. Units that
share a worker-firm cell receive the sample variance of their outcomes (an
eligible stayer's observations; on observation units also a mover's match of
two or more observations); the remaining units receive a two-step fitted
model (a scale on the ranks of leverage and mass, times a quadratic in the
standardised worker and firm effects of the fit, limited to three standard
deviations); the covariance is the high-rank form with these variances plus
a closed-form term for the pairs of units of one cell, reported without
truncation. The estimated offset is held fixed, so no control term is
formed; on observation units with jointly estimated controls the offset pass
supplies the worker-firm geometry. The method is a second flag on the prepared
attachment (`repeat_measurement`, with `leave_out_residual` also set), so
routing, admission, probe accounting and the pseudo-noise solves are shared.
The public ABI reports it as variance source 4 behind additive capability
bit 19, with fit code 4 in the V5 result receipt and its own diagnostic
export, `vckss_rust_engine_repeat_measurement_diagnostic_v1`. The
registered variance-source enumeration is unchanged. At that checkpoint no distributed binary contained it; source-local native
regressions passed. The 0.6.0 adoption above supersedes that binary status.

With the `q1` reference the same pass also computes the leading direction of
each of the four reported targets and the `q = 1` decomposition along it
(`generic_jla::leading_direction`: thick-restart Lanczos in the `S` inner
product, one solve per step; `residual_noise::finish_repeat_q1_target`). The
high-rank covariance and the result transport are unchanged; the directions
leave through `vckss_rust_engine_repeat_measurement_leading_v1` (872 bytes,
four targets of 208 bytes). Their solves are planned at the cap of the search
and receipted as used.

## November 10 implementation checkpoint (historical)

`vckss-core::residual_noise` adds sampling standard errors for the four
components from squared residuals and pseudo-noise, attached to generic JLA
for observation units and for fixed-offset (or control-free) match units. A
probe is one pseudo-noise solve, four per draw, plus three influence solves.
The public ABI reports it as variance source 3 behind additive capability
bit 18; the registered variance-source enumeration is unchanged and the
method is a flag on the prepared attachment. It follows the automatic solver
route, both resolved executors, numerical replay and every centering mode.
Under joint nuisance with controls the engine adds one target pass with the
worker-firm solver for the fixed-offset unit geometry; a mover-only request
without controls selects the generic engine when the attachment is present.
Its memory forecast is a dedicated lifetime tally, and
`vckss_rust_engine_leaveout_residual_diagnostic_v1` exports the working-variance
fit, offset-pass solves, effective ranks and model-based covariance. At that checkpoint no distributed binary contained it; the 0.6.0 payload
now includes the machinery, while its former public variance model is removed. See
`fevc/docs/standard_errors/standard_errors.tex`.

## October 10 pooled component inference binaries

All five payloads are rebuilt from `002205f2` and adopted for repository
installation under explicit owner authorization. They include pooled mover–stayer
structured component inference, mixed capability bit 17/unit receipt V2, and
bounded diagonal residual refinement. The [manifest](../native/pooled-component-20261010/manifest.json)
binds exact bytes, corresponding source and original platform receipts.
Clean Mac arm64/Rosetta thin/universal and SCC Linux full/installed qualification
pass. Windows hosted build, unit tests and PE/export audit pass; the owner
requests publication now and will test Windows later. Its
[runtime qualification remains pending](../native/pooled-component-20261010/windows-manual-adoption.json).
Authentication is left unchanged. Package installation receipts under
`../native/pooled-component-20261010/evidence/packaging/` record all 61 installed files and five binary hashes;
runtime checks apply only to the recorded platform. No tag or release is created.

## Historical CMG repair qualification

All five CMG repair payloads are rebuilt from `eec825d6` and published at
`13a98492` after registered platform qualification and explicit owner approval.
The [manifest](../native/cmg-defects-20261009/manifest.json) binds exact bytes,
original platform receipts and the common B corresponding-source archive.
Mac arm64/Rosetta thin/universal, SCC Linux and hosted/private Windows gates pass.
Local HTTP fresh/replacement checks and all four public `net`/`github`
fresh/replacement cases pass. Each public case verifies 61 installed files,
five hashes, both new regressions, eight Mean-component cells and public Veneto
automatic-batch projection on Mac arm64. The [public receipt](../native/cmg-defects-20261009/evidence/packaging/public-install-13a98492.json)
and [exact binding](../native/cmg-defects-20261009/evidence/packaging/public-install-binding-13a98492.json)
record this scope separately from other platform qualification. Earlier dated
receipts retain their original identities and limitations. No tag or release
is created.

This directory contains the Rust backend for `fevc`. Users of the complete
binary distribution will not need Rust or a compiler. For installation, see
[the package instructions](../INSTALLATION.md).

## Source layout

- `crates/vckss-core/`: estimator, graph, exact/JLA algorithms, solvers,
  planning, memory accounting, and inference.
- `crates/vckss-plugin/`: native lifecycle and versioned ABI.
- `stata_backend/`: Stata C shim, platform builds, and installation checks.
- `vendor/cmg/`: CMG source with its upstream license and provenance.
- `experiments/`: independent research adapters and development harnesses.

Private internal identifiers retain their established names and ABI meanings.
`fevc` is the public package and command.

## Outcome centering

The pooled mover–stayer structured component route (generic JLA with fixed
nuisance offsets, original mover deletion units, independent physical stayer
observations and a joint residual-moment fit with separate variance
coefficients by type) was removed from the `fevc` command in 0.6.0; the code
stays, unreachable from the command, and the
[pooled contract](../fevc/docs/POOLED_COMPONENT_INFERENCE.md) is its design
record. The repeat-measurement standard errors still need the additive
mixed-component capability and unit receipt V2. Historical qualification
below retains its original scope.

The core supports None, Mean and Corrected in exact and JLA point execution.
Mean changes only the retained working-outcome correction factor. Corrected
exact shares one coefficient-space system; Corrected JLA uses the existing
full/two-half leverage pools. Its numerical covariance is Mean's with the
increment fixed. Centering API 1 configures a prepared generation before
solve independently of numerical API 2 and without changing old ABI layouts.
Mean projection uses the same retained frequency-weighted working-outcome
mean in its covariance proxy. The repeat-measurement attachment centres its
pseudo-noise as the outcome is, so Mean needs no fixed-mean approximation,
and accepts Corrected (the standard error of the mean-centred estimate with
the increment fixed). The structured attachments, unreachable from the
command since 0.6.0, held the observed mean fixed, and the native boundary
rejects Corrected for them and for projection. Mean projection requires
additive projection-centering API 1; Mean component standard errors require
additive component-centering API 1. The command does not combine component
standard errors and projection in one call. Explicit
None needs neither attachment-centering capability for its separately
supported requests. Existing supported tuples and ABI
request/result layouts remain.

At the preceding Mean checkpoint, five qualified payloads were published at `66d0278b`. Mac builds at `b9f80ce9` retain
full arm64/Rosetta thin/universal and four-alias installed Mean checks; Linux
at `63757839` retains full and staged/installed Mean checks (SCC job `7969972`).
Windows at `240ad74d` passes hosted checks and private smoke/full qualification,
including installed Mean and Corrected exact checks. The original Mac/Linux
bytes and source archives are retained under a review limited to unchanged
public routes; their lower-level serial exact Corrected path does not contain
the repair in `240ad74d`. The [adoption manifest](../native/mean-component-20261008/manifest.json)
binds actual sources and route limitations, preserving prior failed attempts.
Local HTTP fresh/replacement checks pass at `240ad74d`. The published
`66d0278b` package has public `net`/`github` fresh/replacement status
PASS on Mac arm64: four cases, all 61 final files and
five plugin hashes, and eight Mean cells per case. See the
[public-install summary](../native/mean-component-20261008/evidence/packaging/public-install-66d0278b.json)
and [exact binding](../native/mean-component-20261008/evidence/packaging/public-install-binding-66d0278b.json).
This does not extend other platform or native Intel scope. No tag is implied.

The [completed assessment](../fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
retains 22 exact-Mata availability-screen failures. All eight native primary
cells pass broad descriptive screens, but material numerical sensitivity
remains; there is no general coverage guarantee. Earlier receipts retain their
original scope and do not qualify different payloads.
See [the centering contract](../fevc/docs/CENTERING.md) and
[the Stata boundary](stata_backend/README.md) for lifecycle and restrictions.

## Development

Use the pinned toolchain and locked dependencies. See [TEST_PLAN.md](TEST_PLAN.md)
for affected-surface gates and [the native build guide](stata_backend/README.md)
for platform requirements. The current checkpoint is maintained in
[fevc/PLAN.md](../fevc/PLAN.md).

The core provides exact, compressed JLA, and generic JLA result families.
Backend selection, capability checks, memory admission, RNG planning, original
system residual certification, and caller-state restoration follow the
[numerical contract](../fevc/docs/NUMERICAL_ARCHITECTURE.md) and
[development acceptance policy](../fevc/docs/development_acceptance_v1.json).
See also [RNG_CONTRACT.md](RNG_CONTRACT.md),
[memory planning](../fevc/docs/MEMORY.md), and
[inference](../fevc/docs/INFERENCE.md).

Current feature and platform status is in the
[capability ledger](../fevc/docs/RUST_MATA_PARITY.md). Historical results qualify
only their recorded sources; they do not qualify the current worktree.

The code is MIT, subject to [source provenance](SOURCE_PROVENANCE.md)
and the included third-party notices.
