//! Opt-in phase profiling of the one certified PCG recurrence.

use std::time::Instant;

use crate::pcg::{Pcg, PcgPhase, PcgRecorder};
use crate::preconditioner::with_meter;
use crate::{CmgError, CmgPreconditioner, Laplacian, PcgOptions, PcgWorkspace, SolveContext};

/// Accumulated wall-clock time and invocation count for one PCG phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PcgPhaseSample {
    nanoseconds: u128,
    calls: usize,
}

impl PcgPhaseSample {
    /// Return accumulated wall-clock nanoseconds.
    #[must_use]
    pub const fn nanoseconds(&self) -> u128 {
        self.nanoseconds
    }

    /// Return the number of measured invocations.
    #[must_use]
    pub const fn calls(&self) -> usize {
        self.calls
    }

    fn add(&mut self, elapsed: u128) {
        self.nanoseconds = self.nanoseconds.saturating_add(elapsed);
        self.calls = self.calls.saturating_add(1);
    }
}

/// Phase-level timing from one profiled PCG solve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PcgPhaseProfile {
    total_nanoseconds: u128,
    setup: PcgPhaseSample,
    preconditioner: PcgPhaseSample,
    #[cfg(feature = "cycle-profiling")]
    cycle: crate::CmgApplyProfile,
    #[cfg(not(feature = "cycle-profiling"))]
    cycle: (),
    matvec: PcgPhaseSample,
    dot_products: PcgPhaseSample,
    vector_updates: PcgPhaseSample,
    centering: PcgPhaseSample,
    norms: PcgPhaseSample,
    residual_recompute: PcgPhaseSample,
    certification: PcgPhaseSample,
}

impl PcgPhaseProfile {
    /// Return complete profiled solve wall-clock nanoseconds.
    #[must_use]
    pub const fn total_nanoseconds(&self) -> u128 {
        self.total_nanoseconds
    }

    /// Return setup, projection, and workspace-initialization timing.
    #[must_use]
    pub const fn setup(&self) -> PcgPhaseSample {
        self.setup
    }

    /// Return stationary CMG application timing.
    #[must_use]
    pub const fn preconditioner(&self) -> PcgPhaseSample {
        self.preconditioner
    }

    /// Return exclusive phase attribution within the measured CMG applications.
    #[must_use]
    #[cfg(feature = "cycle-profiling")]
    pub fn cycle(&self) -> &crate::CmgApplyProfile {
        &self.cycle
    }

    /// Return ordinary finest-level matrix-vector timing outside residual replacement.
    #[must_use]
    pub const fn matvec(&self) -> PcgPhaseSample {
        self.matvec
    }

    /// Return compensated dot-product timing outside experimental centering fusion.
    #[must_use]
    pub const fn dot_products(&self) -> PcgPhaseSample {
        self.dot_products
    }

    /// Return fused solution/residual and search-direction update timing.
    #[must_use]
    pub const fn vector_updates(&self) -> PcgPhaseSample {
        self.vector_updates
    }

    /// Return finest-component centering timing.
    /// With `experimental-components`, includes the paired preconditioned-vector
    /// centering and residual dot product, measured together even on the parallel fallback.
    #[must_use]
    pub const fn centering(&self) -> PcgPhaseSample {
        self.centering
    }

    /// Return Euclidean norm and tolerance-calculation timing.
    #[must_use]
    pub const fn norms(&self) -> PcgPhaseSample {
        self.norms
    }

    /// Return fresh residual matvec, subtraction, and projected norm timing.
    #[must_use]
    pub const fn residual_recompute(&self) -> PcgPhaseSample {
        self.residual_recompute
    }

    /// Return original-system residual verification and result construction timing.
    #[must_use]
    pub const fn certification(&self) -> PcgPhaseSample {
        self.certification
    }

    /// Return the sum of explicitly attributed phase nanoseconds.
    #[must_use]
    pub fn attributed_nanoseconds(&self) -> u128 {
        [
            self.setup,
            self.preconditioner,
            self.matvec,
            self.dot_products,
            self.vector_updates,
            self.centering,
            self.norms,
            self.residual_recompute,
            self.certification,
        ]
        .into_iter()
        .map(|sample| sample.nanoseconds)
        .fold(0_u128, u128::saturating_add)
    }

    /// Return wall-clock time not enclosed by an explicit phase timer.
    #[must_use]
    pub fn unattributed_nanoseconds(&self) -> u128 {
        self.total_nanoseconds
            .saturating_sub(self.attributed_nanoseconds())
    }
}

/// Numerical result and phase profile from one profiled PCG solve.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfiledPcgResult {
    solution: Vec<f64>,
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    relative_residual: f64,
    backward_error: f64,
    tolerance: f64,
    restarts: usize,
    rhs_projection_norm: f64,
    profile: PcgPhaseProfile,
}

impl ProfiledPcgResult {
    /// Borrow the profiled solution.
    #[must_use]
    pub fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Return completed PCG iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Return the initial submitted-RHS residual norm.
    #[must_use]
    pub const fn initial_residual_norm(&self) -> f64 {
        self.initial_residual_norm
    }

    /// Return the freshly certified residual norm against the submitted RHS.
    #[must_use]
    pub const fn residual_norm(&self) -> f64 {
        self.residual_norm
    }

    /// Return `||r|| / ||b||`.
    #[must_use]
    pub const fn relative_residual(&self) -> f64 {
        self.relative_residual
    }

    /// Return `||r|| / (||b|| + ||A||_bound ||x||)`.
    #[must_use]
    pub const fn backward_error(&self) -> f64 {
        self.backward_error
    }

    /// Return the absolute residual threshold used for certification.
    #[must_use]
    pub const fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// Return the number of explicit residual-replacement restarts.
    #[must_use]
    pub const fn restarts(&self) -> usize {
        self.restarts
    }

    /// Return the norm of accepted compatibility projection removed from the RHS.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.rhs_projection_norm
    }

    /// Borrow phase-level timing.
    #[must_use]
    pub const fn profile(&self) -> &PcgPhaseProfile {
        &self.profile
    }
}

impl PcgRecorder for PcgPhaseProfile {
    type Stamp = Instant;
    #[cfg(feature = "cycle-profiling")]
    type Cycle = crate::CmgApplyProfile;
    #[cfg(not(feature = "cycle-profiling"))]
    type Cycle = ();

    fn start() -> Instant {
        Instant::now()
    }

    fn finish(&mut self, phase: PcgPhase, start: Instant) {
        let sample = match phase {
            PcgPhase::Setup => &mut self.setup,
            PcgPhase::Preconditioner => &mut self.preconditioner,
            PcgPhase::Matvec => &mut self.matvec,
            PcgPhase::DotProducts => &mut self.dot_products,
            PcgPhase::VectorUpdates => &mut self.vector_updates,
            PcgPhase::Centering => &mut self.centering,
            PcgPhase::Norms => &mut self.norms,
            PcgPhase::ResidualRecompute => &mut self.residual_recompute,
            PcgPhase::Certification => &mut self.certification,
            PcgPhase::Gather | PcgPhase::Solve | PcgPhase::Scatter => return,
        };
        sample.add(start.elapsed().as_nanos());
    }

    fn cycle(&mut self) -> &mut Self::Cycle {
        &mut self.cycle
    }
}

/// Run the production PCG recurrence under `context` while accumulating
/// phase timing.
///
/// This diagnostic is compiled only by the opt-in `profiling` feature. It runs
/// the same recurrence, cycle schedule, residual replacement and
/// original-system certification as [`crate::solve_pcg_into`]; only timers are
/// added. Callers should still compare its result with the ordinary solver
/// before interpreting timings.
pub fn profile_pcg(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    options: PcgOptions,
    context: SolveContext<'_>,
) -> Result<ProfiledPcgResult, CmgError> {
    let overall_start = Instant::now();
    let mut profile = PcgPhaseProfile::default();
    #[cfg(feature = "cycle-profiling")]
    {
        profile.cycle = crate::CmgApplyProfile::new(preconditioner.hierarchy().levels().len());
    }
    let mut workspace = PcgWorkspace::try_new(preconditioner)?;
    let solve = Pcg::new(graph, preconditioner, context);
    let diagnostics = with_meter!(context.cancel, meter => {
        solve.run(rhs, None, options, &mut workspace, meter, &mut profile)
    })?;
    profile.total_nanoseconds = overall_start.elapsed().as_nanos();
    Ok(ProfiledPcgResult {
        solution: workspace.solution().to_vec(),
        iterations: diagnostics.iterations(),
        initial_residual_norm: diagnostics.initial_residual_norm(),
        residual_norm: diagnostics.residual_norm(),
        relative_residual: diagnostics.relative_residual(),
        backward_error: diagnostics.backward_error(),
        tolerance: diagnostics.tolerance(),
        restarts: diagnostics.restarts(),
        rhs_projection_norm: diagnostics.rhs_projection_norm(),
        profile,
    })
}
