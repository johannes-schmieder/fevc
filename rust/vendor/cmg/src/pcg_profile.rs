//! Opt-in phase profiling for the exact planned-PCG arithmetic path.

use std::time::Instant;

use crate::components::ComponentWorkspace;
use crate::graph::compensated_sum;
use crate::{
    CmgError, CmgPreconditioner, CmgWorkspace, Laplacian, ParallelCmgPlan, ParallelExecutor,
    PcgOptions,
};

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

/// Phase-level timing from one exact planned-PCG solve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PcgPhaseProfile {
    total_nanoseconds: u128,
    setup: PcgPhaseSample,
    preconditioner: PcgPhaseSample,
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

    /// Return ordinary finest-level matrix-vector timing outside residual replacement.
    #[must_use]
    pub const fn matvec(&self) -> PcgPhaseSample {
        self.matvec
    }

    /// Return compensated dot-product timing.
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

/// Numerical result and phase profile from one exact planned-PCG solve.
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

struct ProfileWorkspace {
    projected_rhs: Vec<f64>,
    solution: Vec<f64>,
    residual: Vec<f64>,
    preconditioned: Vec<f64>,
    direction: Vec<f64>,
    matrix_direction: Vec<f64>,
    component: ComponentWorkspace,
    cmg: CmgWorkspace,
}

impl ProfileWorkspace {
    fn new(preconditioner: &CmgPreconditioner) -> Self {
        let dimension = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        Self {
            projected_rhs: vec![0.0; dimension],
            solution: vec![0.0; dimension],
            residual: vec![0.0; dimension],
            preconditioned: vec![0.0; dimension],
            direction: vec![0.0; dimension],
            matrix_direction: vec![0.0; dimension],
            component: preconditioner.finest_components().workspace(),
            cmg: preconditioner.workspace(),
        }
    }
}

/// Run the exact production planned-PCG arithmetic while accumulating phase timing.
///
/// This diagnostic is compiled only by the opt-in `profiling` feature. It uses
/// the same operation order, recursive CMG schedule, residual replacement, and
/// original-system certification as `solve_pcg_with_plan`. Callers should still
/// compare its result with the ordinary solver before interpreting timings.
pub fn profile_pcg_with_plan(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    plan: &ParallelCmgPlan,
    rhs: &[f64],
    options: PcgOptions,
    executor: &ParallelExecutor,
) -> Result<ProfiledPcgResult, CmgError> {
    let overall_start = Instant::now();
    let options = options.validate()?;
    let dimension = graph.vertex_count();
    if !preconditioner.matches_graph(graph) {
        return Err(CmgError::InvalidHierarchy {
            context: "profiled PCG graph differs from the preconditioner's finest graph",
        });
    }
    if rhs.len() != dimension {
        return Err(CmgError::dimension(
            "profile_pcg_with_plan rhs",
            dimension,
            rhs.len(),
        ));
    }
    plan.validate(preconditioner)?;

    let mut profile = PcgPhaseProfile::default();
    let mut workspace = ProfileWorkspace::new(preconditioner);
    let components = preconditioner.finest_components();

    let rhs_projection_norm = measure(&mut profile.setup, || {
        workspace.projected_rhs.copy_from_slice(rhs);
        let projection = components.project_rhs_in_place_with_workspace(
            &mut workspace.projected_rhs,
            options.validation,
            &mut workspace.component,
        )?;
        workspace.solution.fill(0.0);
        workspace.residual.copy_from_slice(&workspace.projected_rhs);
        workspace.preconditioned.fill(0.0);
        workspace.direction.fill(0.0);
        workspace.matrix_direction.fill(0.0);
        Ok::<f64, CmgError>(projection)
    })?;

    let initial_residual_norm = measure(&mut profile.norms, || euclidean_norm(rhs));
    let projected_initial_norm = measure(&mut profile.norms, || {
        euclidean_norm(&workspace.projected_rhs)
    });
    let operator_bound = graph.operator_norm_bound();
    let initial_tolerance = allowed_residual(options, initial_residual_norm, operator_bound, 0.0);
    if initial_residual_norm <= initial_tolerance {
        return Ok(finish_result(
            workspace.solution,
            0,
            initial_residual_norm,
            initial_residual_norm,
            initial_tolerance,
            operator_bound,
            0,
            rhs_projection_norm,
            profile,
            overall_start,
        ));
    }
    if projected_initial_norm == 0.0 {
        return Err(CmgError::ResidualVerificationFailed {
            iteration: 0,
            residual_norm: initial_residual_norm,
            tolerance: initial_tolerance,
        });
    }

    measure(&mut profile.preconditioner, || {
        plan.apply_compatible_into_prevalidated(
            preconditioner,
            &workspace.residual,
            &mut workspace.preconditioned,
            &mut workspace.cmg,
            options.validation,
            executor,
        )
    })?;
    measure(&mut profile.centering, || {
        components
            .center_in_place_with_workspace(&mut workspace.preconditioned, &mut workspace.component)
    })?;
    let mut rho = measure(&mut profile.dot_products, || {
        dot(&workspace.residual, &workspace.preconditioned)
    });
    validate_positive_pcg(0, "r^T M r", rho)?;
    measure(&mut profile.vector_updates, || {
        workspace
            .direction
            .copy_from_slice(&workspace.preconditioned);
    });

    let mut restarts = 0_usize;
    let mut last_tolerance = initial_tolerance;

    for iteration in 1..=options.max_iterations {
        measure(&mut profile.matvec, || {
            plan.finest_matvec_into(
                graph,
                &workspace.direction,
                &mut workspace.matrix_direction,
                executor,
            )
        })?;
        let direction_curvature = measure(&mut profile.dot_products, || {
            dot(&workspace.direction, &workspace.matrix_direction)
        });
        validate_positive_pcg(iteration, "p^T A p", direction_curvature)?;
        let alpha = rho / direction_curvature;
        validate_finite_pcg(iteration, "alpha", alpha)?;

        measure(&mut profile.vector_updates, || {
            for (((solution, residual), direction), matrix_direction) in workspace
                .solution
                .iter_mut()
                .zip(&mut workspace.residual)
                .zip(&workspace.direction)
                .zip(&workspace.matrix_direction)
            {
                *solution += alpha * *direction;
                *residual -= alpha * *matrix_direction;
            }
        });
        measure(&mut profile.centering, || {
            components
                .center_in_place_with_workspace(&mut workspace.solution, &mut workspace.component)
        })?;

        let solution_norm = measure(&mut profile.norms, || euclidean_norm(&workspace.solution));
        last_tolerance = allowed_residual(
            options,
            initial_residual_norm,
            operator_bound,
            solution_norm,
        );
        let recursive_residual_norm =
            measure(&mut profile.norms, || euclidean_norm(&workspace.residual));
        let candidate = recursive_residual_norm <= last_tolerance;
        let scheduled_recompute = iteration % options.residual_recompute_interval == 0;
        let mut restarted = false;

        if candidate || scheduled_recompute {
            let projected_fresh_norm = measure(&mut profile.residual_recompute, || {
                recompute_residual_with_plan(
                    plan,
                    executor,
                    graph,
                    &workspace.projected_rhs,
                    &workspace.solution,
                    &mut workspace.matrix_direction,
                )
            })?;
            measure(&mut profile.vector_updates, || {
                workspace
                    .residual
                    .copy_from_slice(&workspace.matrix_direction);
            });
            restarted = true;
            restarts += 1;
            if projected_fresh_norm <= last_tolerance {
                let original_norm = measure(&mut profile.certification, || {
                    original_residual_norm(
                        rhs,
                        &workspace.projected_rhs,
                        &workspace.matrix_direction,
                    )
                });
                if original_norm <= last_tolerance {
                    return Ok(finish_result(
                        workspace.solution,
                        iteration,
                        initial_residual_norm,
                        original_norm,
                        last_tolerance,
                        operator_bound,
                        restarts,
                        rhs_projection_norm,
                        profile,
                        overall_start,
                    ));
                }
                if iteration == options.max_iterations {
                    return Err(CmgError::ResidualVerificationFailed {
                        iteration,
                        residual_norm: original_norm,
                        tolerance: last_tolerance,
                    });
                }
            }
        }

        if iteration == options.max_iterations {
            break;
        }

        measure(&mut profile.centering, || {
            components
                .center_in_place_with_workspace(&mut workspace.residual, &mut workspace.component)
        })?;
        measure(&mut profile.preconditioner, || {
            plan.apply_compatible_into_prevalidated(
                preconditioner,
                &workspace.residual,
                &mut workspace.preconditioned,
                &mut workspace.cmg,
                options.validation,
                executor,
            )
        })?;
        measure(&mut profile.centering, || {
            components.center_in_place_with_workspace(
                &mut workspace.preconditioned,
                &mut workspace.component,
            )
        })?;
        let new_rho = measure(&mut profile.dot_products, || {
            dot(&workspace.residual, &workspace.preconditioned)
        });
        validate_positive_pcg(iteration, "new r^T M r", new_rho)?;

        if restarted {
            measure(&mut profile.vector_updates, || {
                workspace
                    .direction
                    .copy_from_slice(&workspace.preconditioned);
            });
        } else {
            let beta = new_rho / rho;
            validate_finite_pcg(iteration, "beta", beta)?;
            measure(&mut profile.vector_updates, || {
                for (direction, preconditioned) in workspace
                    .direction
                    .iter_mut()
                    .zip(&workspace.preconditioned)
                {
                    *direction = *preconditioned + beta * *direction;
                }
            });
        }
        rho = new_rho;
    }

    measure(&mut profile.residual_recompute, || {
        recompute_residual_with_plan(
            plan,
            executor,
            graph,
            &workspace.projected_rhs,
            &workspace.solution,
            &mut workspace.matrix_direction,
        )
    })?;
    let residual_norm = measure(&mut profile.certification, || {
        original_residual_norm(rhs, &workspace.projected_rhs, &workspace.matrix_direction)
    });
    Err(CmgError::MaximumIterations {
        iterations: options.max_iterations,
        residual_norm,
        tolerance: last_tolerance,
    })
}

fn measure<Output, Operation>(sample: &mut PcgPhaseSample, operation: Operation) -> Output
where
    Operation: FnOnce() -> Output,
{
    let start = Instant::now();
    let output = operation();
    sample.add(start.elapsed().as_nanos());
    output
}

#[allow(clippy::too_many_arguments)]
fn finish_result(
    solution: Vec<f64>,
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    tolerance: f64,
    operator_bound: f64,
    restarts: usize,
    rhs_projection_norm: f64,
    mut profile: PcgPhaseProfile,
    overall_start: Instant,
) -> ProfiledPcgResult {
    let certification_start = Instant::now();
    let solution_norm = euclidean_norm(&solution);
    let denominator = initial_residual_norm + operator_bound * solution_norm;
    let relative_residual = if initial_residual_norm > 0.0 {
        residual_norm / initial_residual_norm
    } else {
        residual_norm
    };
    let backward_error = if denominator > 0.0 {
        residual_norm / denominator
    } else {
        0.0
    };
    profile
        .certification
        .add(certification_start.elapsed().as_nanos());
    profile.total_nanoseconds = overall_start.elapsed().as_nanos();
    ProfiledPcgResult {
        solution,
        iterations,
        initial_residual_norm,
        residual_norm,
        relative_residual,
        backward_error,
        tolerance,
        restarts,
        rhs_projection_norm,
        profile,
    }
}

fn allowed_residual(
    options: PcgOptions,
    rhs_norm: f64,
    operator_bound: f64,
    solution_norm: f64,
) -> f64 {
    options.absolute_tolerance
        + options.relative_tolerance * (rhs_norm + operator_bound * solution_norm)
}

fn recompute_residual_with_plan(
    plan: &ParallelCmgPlan,
    executor: &ParallelExecutor,
    graph: &Laplacian,
    rhs: &[f64],
    solution: &[f64],
    residual: &mut [f64],
) -> Result<f64, CmgError> {
    plan.finest_matvec_into(graph, solution, residual, executor)?;
    for (value, rhs_value) in residual.iter_mut().zip(rhs) {
        *value = *rhs_value - *value;
    }
    Ok(euclidean_norm(residual))
}

fn original_residual_norm(
    original_rhs: &[f64],
    projected_rhs: &[f64],
    projected_residual: &[f64],
) -> f64 {
    let values = || {
        original_rhs
            .iter()
            .zip(projected_rhs)
            .zip(projected_residual)
            .map(|((&original, &projected), &residual)| residual + (original - projected))
    };
    let scale = values().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        0.0
    } else {
        scale
            * compensated_sum(values().map(|value| {
                let scaled = value / scale;
                scaled * scaled
            }))
            .sqrt()
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    compensated_sum(left.iter().zip(right).map(|(x, y)| x * y))
}

fn euclidean_norm(values: &[f64]) -> f64 {
    let scale = values.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        0.0
    } else {
        scale
            * compensated_sum(values.iter().map(|value| {
                let scaled = *value / scale;
                scaled * scaled
            }))
            .sqrt()
    }
}

fn validate_positive_pcg(
    iteration: usize,
    quantity: &'static str,
    value: f64,
) -> Result<(), CmgError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(CmgError::PcgBreakdown {
            iteration,
            quantity,
            value,
        })
    }
}

fn validate_finite_pcg(
    iteration: usize,
    quantity: &'static str,
    value: f64,
) -> Result<(), CmgError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CmgError::PcgBreakdown {
            iteration,
            quantity,
            value,
        })
    }
}
