# Native backend

## Current CMG repair qualification

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

The pooled mover–stayer component development route supports generic JLA with
fixed nuisance offsets, original mover deletion units and independent physical
stayer observations. It uses a joint residual-moment fit with separate variance
coefficients by type. Public callers need the additive mixed-component capability
and unit receipt V2. See the [pooled contract](../fevc/docs/POOLED_COMPONENT_INFERENCE.md)
and [current checkpoint](../fevc/PLAN.md) for assessment and native distribution
status; historical qualification below retains its original scope.

The core supports None, Mean and Corrected in exact and JLA point execution.
Mean changes only the retained working-outcome correction factor. Corrected
exact shares one coefficient-space system; Corrected JLA uses the existing
full/two-half leverage pools. Its numerical covariance is Mean's with the
increment fixed. Centering API 1 configures a prepared generation before
solve independently of numerical API 2 and without changing old ABI layouts.
Mean projection uses the same retained frequency-weighted working-outcome
mean in its covariance proxy. Mean highrank and q1 component inference
use that observed mean as fixed in realized influence and q1 calculations;
residual-moment fitting and Gaussian error probes are unchanged. This omits
mean-estimation uncertainty and does not claim conditional validity given
the estimated mean. Corrected projection/component inference remains
unsupported. Mean projection requires additive projection-centering API 1;
Mean component inference requires additive component-centering API 1.
Existing combined exact-Mata requests support Mean; native combined
component/projection requests remain unsupported for Mean and None. Explicit
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

The code is GPL-3.0-only, subject to [source provenance](SOURCE_PROVENANCE.md)
and the included third-party notices.
