//! Certified quotient-space preconditioned conjugate gradients.

use crate::components::ComponentWorkspace;
use crate::graph::compensated_sum;
use crate::{CmgError, CmgPreconditioner, CmgWorkspace, Laplacian, PcgOptions};
#[cfg(feature = "parallel")]
use crate::{ParallelCmgPlan, ParallelExecutor, ParallelOptions};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Reusable vectors for repeated PCG solves with one preconditioner.
#[derive(Debug, Clone)]
pub struct PcgWorkspace {
    projected_rhs: Vec<f64>,
    solution: Vec<f64>,
    residual: Vec<f64>,
    preconditioned: Vec<f64>,
    direction: Vec<f64>,
    matrix_direction: Vec<f64>,
    component: ComponentWorkspace,
    cmg: CmgWorkspace,
}

impl PcgWorkspace {
    /// Allocate a solver workspace for a fixed preconditioner.
    #[must_use]
    pub fn new(preconditioner: &CmgPreconditioner) -> Self {
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

    /// Return the system dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.solution.len()
    }

    /// Return the number of heap bytes reserved by the principal work arrays.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.dimension()
            .saturating_mul(8)
            .saturating_mul(6)
            .saturating_add(self.component.byte_len())
            .saturating_add(self.cmg.byte_len())
    }

    fn validate(&self, dimension: usize) -> Result<(), CmgError> {
        for (context, actual) in [
            ("PcgWorkspace projected rhs", self.projected_rhs.len()),
            ("PcgWorkspace solution", self.solution.len()),
            ("PcgWorkspace residual", self.residual.len()),
            ("PcgWorkspace preconditioned", self.preconditioned.len()),
            ("PcgWorkspace direction", self.direction.len()),
            ("PcgWorkspace matrix direction", self.matrix_direction.len()),
        ] {
            if actual != dimension {
                return Err(CmgError::dimension(context, dimension, actual));
            }
        }
        Ok(())
    }
}

/// A successfully certified PCG solution and its diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct PcgResult {
    solution: Vec<f64>,
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    relative_residual: f64,
    backward_error: f64,
    tolerance: f64,
    restarts: usize,
    rhs_projection_norm: f64,
}

impl PcgResult {
    /// Return the certified solution.
    #[must_use]
    pub fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Consume the result and return its solution vector.
    #[must_use]
    pub fn into_solution(self) -> Vec<f64> {
        self.solution
    }

    /// Return completed PCG iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Return the initial Euclidean residual norm for the submitted RHS.
    #[must_use]
    pub const fn initial_residual_norm(&self) -> f64 {
        self.initial_residual_norm
    }

    /// Return the freshly recomputed residual norm against the submitted RHS.
    #[must_use]
    pub const fn residual_norm(&self) -> f64 {
        self.residual_norm
    }

    /// Return `||r|| / ||b||`, with zero denominator handled explicitly.
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

    /// Return the norm of accepted component-nullspace roundoff removed from
    /// the submitted RHS before iteration.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.rhs_projection_norm
    }
}

/// Solve a compatible graph-Laplacian system with a newly allocated workspace.
pub fn solve_pcg(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    options: PcgOptions,
) -> Result<PcgResult, CmgError> {
    let mut workspace = PcgWorkspace::new(preconditioner);
    solve_pcg_with_workspace(graph, preconditioner, rhs, options, &mut workspace)
}

/// Solve using caller-owned workspace suitable for repeated right-hand sides.
///
/// Component-sum defects accepted by `compatibility_tolerance` are projected
/// onto the exact Laplacian range. Final certification is still performed
/// against the submitted, unprojected RHS and the projection norm is reported.
pub fn solve_pcg_with_workspace(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    options: PcgOptions,
    workspace: &mut PcgWorkspace,
) -> Result<PcgResult, CmgError> {
    let options = options.validate()?;
    let dimension = graph.vertex_count();
    if !preconditioner.matches_graph(graph) {
        return Err(CmgError::InvalidHierarchy {
            context: "PCG graph differs from the preconditioner's finest graph",
        });
    }
    if rhs.len() != dimension {
        return Err(CmgError::dimension("solve_pcg rhs", dimension, rhs.len()));
    }
    workspace.validate(dimension)?;

    let components = preconditioner.finest_components();
    workspace.projected_rhs.copy_from_slice(rhs);
    let rhs_projection_norm = components.project_rhs_in_place_with_workspace(
        &mut workspace.projected_rhs,
        options.validation,
        &mut workspace.component,
    )?;
    workspace.solution.fill(0.0);
    workspace.residual.copy_from_slice(&workspace.projected_rhs);
    workspace.preconditioned.fill(0.0);
    workspace.direction.fill(0.0);
    workspace.matrix_direction.fill(0.0);

    let initial_residual_norm = euclidean_norm(rhs);
    let projected_initial_norm = euclidean_norm(&workspace.projected_rhs);
    let operator_bound = graph.operator_norm_bound();
    let initial_tolerance = allowed_residual(options, initial_residual_norm, operator_bound, 0.0);
    if initial_residual_norm <= initial_tolerance {
        return Ok(make_result(
            workspace.solution.clone(),
            0,
            initial_residual_norm,
            initial_residual_norm,
            initial_tolerance,
            operator_bound,
            0,
            rhs_projection_norm,
        ));
    }
    if projected_initial_norm == 0.0 {
        return Err(CmgError::ResidualVerificationFailed {
            iteration: 0,
            residual_norm: initial_residual_norm,
            tolerance: initial_tolerance,
        });
    }

    preconditioner.apply_compatible_into_with_validation(
        &workspace.residual,
        &mut workspace.preconditioned,
        &mut workspace.cmg,
        options.validation,
    )?;
    components
        .center_in_place_with_workspace(&mut workspace.preconditioned, &mut workspace.component)?;
    let mut rho = dot(&workspace.residual, &workspace.preconditioned);
    validate_positive_pcg(0, "r^T M r", rho)?;
    workspace
        .direction
        .copy_from_slice(&workspace.preconditioned);

    let mut restarts = 0_usize;
    let mut last_tolerance = initial_tolerance;

    for iteration in 1..=options.max_iterations {
        graph.matvec_into(&workspace.direction, &mut workspace.matrix_direction)?;
        let direction_curvature = dot(&workspace.direction, &workspace.matrix_direction);
        validate_positive_pcg(iteration, "p^T A p", direction_curvature)?;
        let alpha = rho / direction_curvature;
        validate_finite_pcg(iteration, "alpha", alpha)?;

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
        components
            .center_in_place_with_workspace(&mut workspace.solution, &mut workspace.component)?;

        let solution_norm = euclidean_norm(&workspace.solution);
        last_tolerance = allowed_residual(
            options,
            initial_residual_norm,
            operator_bound,
            solution_norm,
        );
        let recursive_residual_norm = euclidean_norm(&workspace.residual);
        let candidate = recursive_residual_norm <= last_tolerance;
        let scheduled_recompute = iteration % options.residual_recompute_interval == 0;
        let mut restarted = false;

        if candidate || scheduled_recompute {
            let projected_fresh_norm = recompute_residual(
                graph,
                &workspace.projected_rhs,
                &workspace.solution,
                &mut workspace.matrix_direction,
            )?;
            workspace
                .residual
                .copy_from_slice(&workspace.matrix_direction);
            restarted = true;
            restarts += 1;
            if projected_fresh_norm <= last_tolerance {
                let original_norm = original_residual_norm(
                    rhs,
                    &workspace.projected_rhs,
                    &workspace.matrix_direction,
                );
                if original_norm <= last_tolerance {
                    return Ok(make_result(
                        workspace.solution.clone(),
                        iteration,
                        initial_residual_norm,
                        original_norm,
                        last_tolerance,
                        operator_bound,
                        restarts,
                        rhs_projection_norm,
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

        // The public solver projected the submitted RHS once. Remove only the
        // component-nullspace roundoff accumulated by Krylov updates before
        // reusing the compatible stationary core.
        components
            .center_in_place_with_workspace(&mut workspace.residual, &mut workspace.component)?;
        preconditioner.apply_compatible_into_with_validation(
            &workspace.residual,
            &mut workspace.preconditioned,
            &mut workspace.cmg,
            options.validation,
        )?;
        components.center_in_place_with_workspace(
            &mut workspace.preconditioned,
            &mut workspace.component,
        )?;
        let new_rho = dot(&workspace.residual, &workspace.preconditioned);
        validate_positive_pcg(iteration, "new r^T M r", new_rho)?;

        if restarted {
            workspace
                .direction
                .copy_from_slice(&workspace.preconditioned);
        } else {
            let beta = new_rho / rho;
            validate_finite_pcg(iteration, "beta", beta)?;
            for (direction, preconditioned) in workspace
                .direction
                .iter_mut()
                .zip(&workspace.preconditioned)
            {
                *direction = *preconditioned + beta * *direction;
            }
        }
        rho = new_rho;
    }

    recompute_residual(
        graph,
        &workspace.projected_rhs,
        &workspace.solution,
        &mut workspace.matrix_direction,
    )?;
    let residual_norm =
        original_residual_norm(rhs, &workspace.projected_rhs, &workspace.matrix_direction);
    Err(CmgError::MaximumIterations {
        iterations: options.max_iterations,
        residual_norm,
        tolerance: last_tolerance,
    })
}

/// Solve with a prebuilt optional parallel CMG plan and a newly allocated workspace.
#[cfg(feature = "parallel")]
pub fn solve_pcg_with_plan(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    plan: &ParallelCmgPlan,
    rhs: &[f64],
    options: PcgOptions,
    executor: &ParallelExecutor,
) -> Result<PcgResult, CmgError> {
    let mut workspace = PcgWorkspace::new(preconditioner);
    solve_pcg_with_plan_and_workspace(
        graph,
        preconditioner,
        plan,
        rhs,
        options,
        &mut workspace,
        executor,
    )
}

/// Solve with a prebuilt optional parallel CMG plan and caller-owned workspace.
///
/// The submitted right-hand side is projected and the final residual is
/// certified against the original system exactly as in [`solve_pcg_with_workspace`].
#[cfg(feature = "parallel")]
pub fn solve_pcg_with_plan_and_workspace(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    plan: &ParallelCmgPlan,
    rhs: &[f64],
    options: PcgOptions,
    workspace: &mut PcgWorkspace,
    executor: &ParallelExecutor,
) -> Result<PcgResult, CmgError> {
    let options = options.validate()?;
    let dimension = graph.vertex_count();
    if !preconditioner.matches_graph(graph) {
        return Err(CmgError::InvalidHierarchy {
            context: "PCG graph differs from the preconditioner's finest graph",
        });
    }
    if rhs.len() != dimension {
        return Err(CmgError::dimension("solve_pcg rhs", dimension, rhs.len()));
    }
    workspace.validate(dimension)?;
    plan.validate(preconditioner)?;

    let components = preconditioner.finest_components();
    workspace.projected_rhs.copy_from_slice(rhs);
    let rhs_projection_norm = components.project_rhs_in_place_with_workspace(
        &mut workspace.projected_rhs,
        options.validation,
        &mut workspace.component,
    )?;
    workspace.solution.fill(0.0);
    workspace.residual.copy_from_slice(&workspace.projected_rhs);
    workspace.preconditioned.fill(0.0);
    workspace.direction.fill(0.0);
    workspace.matrix_direction.fill(0.0);

    let initial_residual_norm = euclidean_norm_with_executor(rhs, executor);
    let projected_initial_norm = euclidean_norm_with_executor(&workspace.projected_rhs, executor);
    let operator_bound = graph.operator_norm_bound();
    let initial_tolerance = allowed_residual(options, initial_residual_norm, operator_bound, 0.0);
    if initial_residual_norm <= initial_tolerance {
        return Ok(make_result(
            workspace.solution.clone(),
            0,
            initial_residual_norm,
            initial_residual_norm,
            initial_tolerance,
            operator_bound,
            0,
            rhs_projection_norm,
        ));
    }
    if projected_initial_norm == 0.0 {
        return Err(CmgError::ResidualVerificationFailed {
            iteration: 0,
            residual_norm: initial_residual_norm,
            tolerance: initial_tolerance,
        });
    }

    plan.apply_compatible_into_prevalidated(
        preconditioner,
        &workspace.residual,
        &mut workspace.preconditioned,
        &mut workspace.cmg,
        options.validation,
        executor,
    )?;
    components
        .center_in_place_with_workspace(&mut workspace.preconditioned, &mut workspace.component)?;
    let mut rho = dot_with_executor(&workspace.residual, &workspace.preconditioned, executor);
    validate_positive_pcg(0, "r^T M r", rho)?;
    workspace
        .direction
        .copy_from_slice(&workspace.preconditioned);

    let mut restarts = 0_usize;
    let mut last_tolerance = initial_tolerance;

    for iteration in 1..=options.max_iterations {
        plan.finest_matvec_into(
            graph,
            &workspace.direction,
            &mut workspace.matrix_direction,
            executor,
        )?;
        let direction_curvature =
            dot_with_executor(&workspace.direction, &workspace.matrix_direction, executor);
        validate_positive_pcg(iteration, "p^T A p", direction_curvature)?;
        let alpha = rho / direction_curvature;
        validate_finite_pcg(iteration, "alpha", alpha)?;

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
        components
            .center_in_place_with_workspace(&mut workspace.solution, &mut workspace.component)?;

        let solution_norm = euclidean_norm_with_executor(&workspace.solution, executor);
        last_tolerance = allowed_residual(
            options,
            initial_residual_norm,
            operator_bound,
            solution_norm,
        );
        let recursive_residual_norm = euclidean_norm_with_executor(&workspace.residual, executor);
        let candidate = recursive_residual_norm <= last_tolerance;
        let scheduled_recompute = iteration % options.residual_recompute_interval == 0;
        let mut restarted = false;

        if candidate || scheduled_recompute {
            let projected_fresh_norm = recompute_residual_with_plan(
                plan,
                executor,
                graph,
                &workspace.projected_rhs,
                &workspace.solution,
                &mut workspace.matrix_direction,
            )?;
            workspace
                .residual
                .copy_from_slice(&workspace.matrix_direction);
            restarted = true;
            restarts += 1;
            if projected_fresh_norm <= last_tolerance {
                let original_norm = original_residual_norm(
                    rhs,
                    &workspace.projected_rhs,
                    &workspace.matrix_direction,
                );
                if original_norm <= last_tolerance {
                    return Ok(make_result(
                        workspace.solution.clone(),
                        iteration,
                        initial_residual_norm,
                        original_norm,
                        last_tolerance,
                        operator_bound,
                        restarts,
                        rhs_projection_norm,
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

        // The public solver projected the submitted RHS once. Remove only the
        // component-nullspace roundoff accumulated by Krylov updates before
        // reusing the compatible stationary core.
        components
            .center_in_place_with_workspace(&mut workspace.residual, &mut workspace.component)?;
        plan.apply_compatible_into_prevalidated(
            preconditioner,
            &workspace.residual,
            &mut workspace.preconditioned,
            &mut workspace.cmg,
            options.validation,
            executor,
        )?;
        components.center_in_place_with_workspace(
            &mut workspace.preconditioned,
            &mut workspace.component,
        )?;
        let new_rho = dot_with_executor(&workspace.residual, &workspace.preconditioned, executor);
        validate_positive_pcg(iteration, "new r^T M r", new_rho)?;

        if restarted {
            workspace
                .direction
                .copy_from_slice(&workspace.preconditioned);
        } else {
            let beta = new_rho / rho;
            validate_finite_pcg(iteration, "beta", beta)?;
            for (direction, preconditioned) in workspace
                .direction
                .iter_mut()
                .zip(&workspace.preconditioned)
            {
                *direction = *preconditioned + beta * *direction;
            }
        }
        rho = new_rho;
    }

    recompute_residual_with_plan(
        plan,
        executor,
        graph,
        &workspace.projected_rhs,
        &workspace.solution,
        &mut workspace.matrix_direction,
    )?;
    let residual_norm =
        original_residual_norm(rhs, &workspace.projected_rhs, &workspace.matrix_direction);
    Err(CmgError::MaximumIterations {
        iterations: options.max_iterations,
        residual_norm,
        tolerance: last_tolerance,
    })
}

/// Solve multiple right-hand sides sequentially while reusing all work arrays.
pub fn solve_pcg_batch(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: &[Vec<f64>],
    options: PcgOptions,
) -> Result<Vec<PcgResult>, CmgError> {
    let mut workspace = PcgWorkspace::new(preconditioner);
    right_hand_sides
        .iter()
        .map(|rhs| solve_pcg_with_workspace(graph, preconditioner, rhs, options, &mut workspace))
        .collect()
}

/// Solve independent right-hand sides concurrently in a package-owned pool.
///
/// Every RHS uses the unchanged certified serial PCG algorithm and a private
/// reusable workspace. The executor's workspace-memory budget limits the
/// number of simultaneous solves. Results retain input order.
#[cfg(feature = "parallel")]
pub fn solve_pcg_batch_with_executor(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: &[Vec<f64>],
    options: PcgOptions,
    executor: &ParallelExecutor,
) -> Result<Vec<PcgResult>, CmgError> {
    if right_hand_sides.is_empty() {
        return Ok(Vec::new());
    }
    if !preconditioner.matches_graph(graph) {
        return Err(CmgError::InvalidHierarchy {
            context: "parallel PCG graph differs from the preconditioner's finest graph",
        });
    }
    for rhs in right_hand_sides {
        if rhs.len() != graph.vertex_count() {
            return Err(CmgError::dimension(
                "solve_pcg_batch_parallel rhs",
                graph.vertex_count(),
                rhs.len(),
            ));
        }
    }

    let first_workspace = PcgWorkspace::new(preconditioner);
    let concurrency =
        executor.batch_concurrency(first_workspace.byte_len(), right_hand_sides.len())?;
    let mut workspaces = Vec::with_capacity(concurrency);
    workspaces.push(first_workspace);
    workspaces.extend((1..concurrency).map(|_| PcgWorkspace::new(preconditioner)));

    let mut results = Vec::with_capacity(right_hand_sides.len());
    for rhs_chunk in right_hand_sides.chunks(concurrency) {
        let chunk_results: Vec<Result<PcgResult, CmgError>> = executor.install(|| {
            workspaces[..rhs_chunk.len()]
                .par_iter_mut()
                .zip(rhs_chunk.par_iter())
                .map(|(workspace, rhs)| {
                    solve_pcg_with_workspace(graph, preconditioner, rhs, options, workspace)
                })
                .collect()
        });
        for result in chunk_results {
            results.push(result?);
        }
    }
    Ok(results)
}

/// Construct a package-owned pool and solve a batch of independent systems.
#[cfg(feature = "parallel")]
pub fn solve_pcg_batch_parallel(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: &[Vec<f64>],
    options: PcgOptions,
    parallel: ParallelOptions,
) -> Result<Vec<PcgResult>, CmgError> {
    let executor = ParallelExecutor::new(parallel)?;
    solve_pcg_batch_with_executor(graph, preconditioner, right_hand_sides, options, &executor)
}

fn make_result(
    solution: Vec<f64>,
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    tolerance: f64,
    operator_bound: f64,
    restarts: usize,
    rhs_projection_norm: f64,
) -> PcgResult {
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
    PcgResult {
        solution,
        iterations,
        initial_residual_norm,
        residual_norm,
        relative_residual,
        backward_error,
        tolerance,
        restarts,
        rhs_projection_norm,
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

#[cfg(feature = "parallel")]
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
    Ok(euclidean_norm_with_executor(residual, executor))
}

fn recompute_residual(
    graph: &Laplacian,
    rhs: &[f64],
    solution: &[f64],
    residual: &mut [f64],
) -> Result<f64, CmgError> {
    graph.matvec_into(solution, residual)?;
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

#[cfg(feature = "parallel")]
fn dot_with_executor(left: &[f64], right: &[f64], executor: &ParallelExecutor) -> f64 {
    debug_assert_eq!(left.len(), right.len());
    let options = executor.options();
    let parallel_floor = options
        .min_parallel_len
        .max(options.reduction_chunk_size.saturating_mul(8));
    if left.len() < parallel_floor || executor.thread_count() <= 1 {
        return dot(left, right);
    }
    executor.install(|| fixed_chunk_dot(left, right, options.reduction_chunk_size))
}

#[cfg(feature = "parallel")]
fn fixed_chunk_dot(left: &[f64], right: &[f64], chunk_size: usize) -> f64 {
    debug_assert_eq!(left.len(), right.len());
    let chunk_count = left.len().div_ceil(chunk_size);
    if chunk_count == 0 {
        return 0.0;
    }

    fn reduce_range(
        left: &[f64],
        right: &[f64],
        chunk_size: usize,
        first_chunk: usize,
        last_chunk: usize,
    ) -> f64 {
        if last_chunk - first_chunk == 1 {
            let start = first_chunk * chunk_size;
            let end = left.len().min(start + chunk_size);
            return compensated_sum(
                left[start..end]
                    .iter()
                    .zip(&right[start..end])
                    .map(|(left, right)| left * right),
            );
        }
        let middle = first_chunk + (last_chunk - first_chunk) / 2;
        let (left_sum, right_sum) = rayon::join(
            || reduce_range(left, right, chunk_size, first_chunk, middle),
            || reduce_range(left, right, chunk_size, middle, last_chunk),
        );
        compensated_sum([left_sum, right_sum])
    }

    reduce_range(left, right, chunk_size, 0, chunk_count)
}

#[cfg(all(test, feature = "parallel"))]
mod deterministic_parallel_dot_tests {
    use super::{dot, dot_with_executor};
    use crate::{ParallelExecutor, ParallelOptions};

    #[test]
    fn fixed_chunk_dot_is_thread_count_invariant() {
        let left: Vec<f64> = (0..257)
            .map(|index| ((index * 17) % 101) as f64 / 13.0 - 3.0)
            .collect();
        let right: Vec<f64> = (0..257)
            .map(|index| ((index * 31 + 7) % 113) as f64 / 19.0 - 2.0)
            .collect();
        let mut reference = None;
        for threads in [2, 3, 4] {
            let executor = ParallelExecutor::new(ParallelOptions {
                threads,
                min_parallel_len: 1,
                reduction_chunk_size: 16,
                ..ParallelOptions::default()
            })
            .unwrap();
            let value = dot_with_executor(&left, &right, &executor);
            match reference {
                Some(bits) => assert_eq!(bits, value.to_bits()),
                None => reference = Some(value.to_bits()),
            }
        }
        let fixed = f64::from_bits(reference.unwrap());
        let serial = dot(&left, &right);
        assert!((fixed - serial).abs() <= 2.0e-13 * (1.0 + serial.abs()));
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    compensated_sum(left.iter().zip(right).map(|(x, y)| x * y))
}

#[cfg(feature = "parallel")]
fn euclidean_norm_with_executor(values: &[f64], executor: &ParallelExecutor) -> f64 {
    let options = executor.options();
    let parallel_floor = options
        .reduction_chunk_size
        .saturating_mul(executor.thread_count())
        .saturating_mul(2);
    let scale = if executor.should_parallel(values.len()) && values.len() >= parallel_floor {
        executor.install(|| {
            values
                .par_chunks(options.reduction_chunk_size)
                .map(|chunk| {
                    chunk
                        .iter()
                        .map(|value| value.abs())
                        .fold(0.0_f64, f64::max)
                })
                .reduce(|| 0.0_f64, f64::max)
        })
    } else {
        values
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max)
    };
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
