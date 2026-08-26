// SPDX-License-Identifier: GPL-3.0-only

// This file is included at the end of exact standalone CMG commit dbefbc5's
// `pcg.rs` only inside the VCkss private performance build. It deliberately
// reuses the official scalar PCG state, preconditioner, recurrence, residual
// replacement, and final certification. The experiment changes only vector
// traffic: on a connected graph it defers solution null-space centering until
// a fresh-residual checkpoint and fuses the update with the recursive-residual
// norm and quotient-space solution mean.

/// Reusable scalar PCG workspace for the VCkss deterministic pass-fusion spike.
#[derive(Debug)]
pub struct VckssPassFusedPcgWorkspace {
    projected_rhs: Vec<f64>,
    solution: Vec<f64>,
    residual: Vec<f64>,
    preconditioned: Vec<f64>,
    direction: Vec<f64>,
    matrix_direction: Vec<f64>,
    component: ComponentWorkspace,
    cmg: CmgWorkspace,
}

impl VckssPassFusedPcgWorkspace {
    /// Allocate one workspace for an immutable preconditioner.
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

    /// Return retained principal heap bytes.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.solution
            .len()
            .saturating_mul(8)
            .saturating_mul(6)
            .saturating_add(self.component.byte_len())
            .saturating_add(self.cmg.byte_len())
    }

    fn validate(&self, dimension: usize) -> Result<(), CmgError> {
        for (context, actual) in [
            ("VCkss pass-fused projected rhs", self.projected_rhs.len()),
            ("VCkss pass-fused solution", self.solution.len()),
            ("VCkss pass-fused residual", self.residual.len()),
            ("VCkss pass-fused preconditioned", self.preconditioned.len()),
            ("VCkss pass-fused direction", self.direction.len()),
            (
                "VCkss pass-fused matrix direction",
                self.matrix_direction.len(),
            ),
        ] {
            if actual != dimension {
                return Err(CmgError::dimension(context, dimension, actual));
            }
        }
        Ok(())
    }
}

/// Solve one connected Laplacian RHS with deterministic fused vector passes.
///
/// This remains mathematically an independent scalar PCG. It retains the
/// official preconditioner, recurrence, compensated reductions, explicit
/// residual replacement, and certification against the submitted RHS.
pub fn vckss_solve_pcg_pass_fused_with_workspace(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    options: PcgOptions,
    workspace: &mut VckssPassFusedPcgWorkspace,
) -> Result<PcgResult, CmgError> {
    let options = options.validate()?;
    let dimension = graph.vertex_count();
    if !preconditioner.matches_graph(graph) {
        return Err(CmgError::InvalidHierarchy {
            context: "VCkss pass-fused PCG graph differs from the hierarchy",
        });
    }
    if preconditioner.finest_components().count() != 1 {
        return Err(CmgError::InvalidHierarchy {
            context: "VCkss pass-fused PCG requires one connected component",
        });
    }
    if rhs.len() != dimension {
        return Err(CmgError::dimension(
            "VCkss pass-fused PCG rhs",
            dimension,
            rhs.len(),
        ));
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

        let (solution_norm, recursive_residual_norm) = vckss_update_and_quotient_norms(
            &mut workspace.solution,
            &mut workspace.residual,
            &workspace.direction,
            &workspace.matrix_direction,
            alpha,
        );
        last_tolerance = allowed_residual(
            options,
            initial_residual_norm,
            operator_bound,
            solution_norm,
        );
        let candidate = recursive_residual_norm <= last_tolerance;
        let scheduled_recompute = iteration % options.residual_recompute_interval == 0;
        let mut restarted = false;

        if candidate || scheduled_recompute {
            // Fix the unique quotient representative before every explicit
            // residual replacement and before any solution can be returned.
            components.center_in_place_with_workspace(
                &mut workspace.solution,
                &mut workspace.component,
            )?;
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

    components.center_in_place_with_workspace(&mut workspace.solution, &mut workspace.component)?;
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

#[inline]
fn vckss_compensated_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let next = *sum + value;
    *correction += if sum.abs() >= value.abs() {
        (*sum - next) + value
    } else {
        (value - next) + *sum
    };
    *sum = next;
}

fn vckss_update_and_quotient_norms(
    solution: &mut [f64],
    residual: &mut [f64],
    direction: &[f64],
    matrix_direction: &[f64],
    alpha: f64,
) -> (f64, f64) {
    let mut solution_sum = 0.0;
    let mut solution_correction = 0.0;
    let mut residual_squared = 0.0;
    let mut residual_correction = 0.0;
    for (((solution_value, residual_value), direction_value), matrix_value) in solution
        .iter_mut()
        .zip(residual.iter_mut())
        .zip(direction)
        .zip(matrix_direction)
    {
        *solution_value += alpha * *direction_value;
        *residual_value -= alpha * *matrix_value;
        vckss_compensated_add(&mut solution_sum, &mut solution_correction, *solution_value);
        vckss_compensated_add(
            &mut residual_squared,
            &mut residual_correction,
            *residual_value * *residual_value,
        );
    }
    let mean = (solution_sum + solution_correction) / solution.len().max(1) as f64;
    let solution_squared = compensated_sum(solution.iter().map(|value| {
        let centered = *value - mean;
        centered * centered
    }));
    (
        solution_squared.sqrt(),
        (residual_squared + residual_correction).sqrt(),
    )
}
