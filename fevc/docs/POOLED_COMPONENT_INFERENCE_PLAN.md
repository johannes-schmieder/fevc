# Pooled component inference — accepted October 9, 2026

Status: source implementation and local assessment/validation complete, with
recorded large-diagonal numerical failures and completed CMG diagnostics.
External platform qualification and five-payload adoption remain pending. The owner
approved implementation of the
conversational plan, including scalable Rust/JLA, exact Mata, fixed nuisance
offsets, a bounded assessment, and qualification of all five native payloads.
Publication, source pushes, tags and releases are not authorized by this plan.

## Statistical contract

Freeze workers' original deletion-unit classification and the existing retained
sample. Movers have multiple original units; eligible stayers have one original
unit and at least two physical observations attached to a retained mover firm.
Mover units permit unrestricted internal dependence. Stayer physical observations
and different mover units are mutually independent, with unequal variances.
Do not reclassify dropped movers or change sample selection using synthetic IDs.

Use the combined fit and pooled four targets. Under fixed nuisance offsets,
collapse mover unit g to sqrt(F_g) times its working-outcome mean and retain
physical stayer observations. Literal frequency copies remain independent for
stayers. Compute Mean centering before this transformation and hold it fixed
inside inference; Gaussian error probes remain uncentered. Mean and fixed-offset
estimation uncertainty remain omitted. Corrected inference is out of scope.

## Implementation

- Extend explicit structured match inference to default/explicit stayers(both).
  Rust retains generic JLA, Counter-V1 and explicit diagonal/CMG. Mata uses
  explicit exact with guarded Stata RNG. Both require nuisance(fixedoffset).
- Keep the existing target-specific Mata route and all old route meanings.
  New exact Mata also accepts structured mover-only match inference.
- Fit one joint residual-moment system with separate type coefficients:
  21 mover and 15 stayer candidate terms for structured_common, three per type
  for structured_leverage, with within-type normalized midranks. Preserve
  outcome-free redundancy reduction, identification/support and positivity gates.
  Do not drop cross-type projection terms or silently pool deficient strata.
- Reuse matrix-free native point, influence, covariance, spectral and q1
  machinery; certify the mixed point and remainder identities. Exact Mata uses
  coefficient-space Gram contractions and exact solves, with common Gaussian
  covariance simulations. No production observation-by-observation matrices.
- Add native mixed capability and a versioned unit receipt without changing old
  layouts. Reconcile mover/stayer/total independent counts and omitted uncertainty.
  Old plugins fail before preparation/RNG. Preserve caller state and memory policy.
- Keep individual availability separate from joint covariance admissibility.
  Structured q1 never posts a substitute Gaussian covariance or q0 fallback.

## Validation and completion

First independent physical-row and transformed oracles, then focused public
regressions: unequal blocks, short/long stayers, literal frequency copies,
target weights, fixed controls, centering, original parallel units, zero-stayer
limit, sample exclusions, rank failures, q1 identities, resource failures,
batch/order/thread invariance, stale plugins and caller-state restoration.
Use the registered development tolerances, not point MCSE for inference outputs.

New assessment: two backends x two reference distributions x stayer target-mass
shares .25/.75, 1,000 outcomes each and paired Mean/fixed-population-mean arms:
16,000 calls and 64,000 target rows. All four highrank targets are primary;
q1 worker/firm/total are primary, covariance is a predeclared multi-mode
diagnostic. Use attached histories of 2/4/8 observations. Verify geometry and
variance-model support before repeated outcomes. Separate 100-outcome stress
profiles cover null/weak signals, multimode, mild/severe misspecification,
concentrated mass, short histories and estimated offsets. Freeze a new manifest
and registration before assessment; preserve every failure. Broad screens are
descriptive, not general coverage qualification or permission for retuning.

Run separate fixed-outcome numerical sensitivity and three-size complete-command
Rust timing/RSS checks. Run source, assembly, integrated Stata, Rust and native
ABI checks, then qualify the five exact native payloads and installed routes.
Prepare adoption/source manifests without publishing. Scientific failures remain
limitations; engineering failures prevent completion. All prior evidence is
immutable. No source/native/platform/coverage completion is claimed until tested.
