// SPDX-License-Identifier: GPL-3.0-only

//! VCkss-private fused independent-PCG experiment.
//!
//! This file is injected into an exact archive of CMG commit
//! `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10` by the VCkss spike builder.  It
//! deliberately remains outside the public CMG and VCkss APIs while the
//! end-to-end performance hypothesis is tested.  The Krylov recurrences remain
//! mathematically independent by column; only sparse-operator and hierarchy
//! traversals are fused.

use rayon::prelude::*;
use std::time::Instant;

use crate::{
    CmgError, Components, GroundedLdl, Laplacian, ParallelExecutor, ParallelPcgSolver, PcgOptions,
    TerminalReason,
};

const REDUCTION_CHUNK_VERTICES: usize = 16_384;

/// One column's certified fused-PCG diagnostics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VckssFusedPcgColumnReport {
    iterations: usize,
    restarts: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    tolerance: f64,
    rhs_projection_norm: f64,
}

impl VckssFusedPcgColumnReport {
    /// Return completed PCG iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Return explicit residual-replacement restarts.
    #[must_use]
    pub const fn restarts(&self) -> usize {
        self.restarts
    }

    /// Return the original submitted RHS norm.
    #[must_use]
    pub const fn initial_residual_norm(&self) -> f64 {
        self.initial_residual_norm
    }

    /// Return the freshly certified residual norm against the submitted RHS.
    #[must_use]
    pub const fn residual_norm(&self) -> f64 {
        self.residual_norm
    }

    /// Return the final absolute residual threshold.
    #[must_use]
    pub const fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// Return the norm of the component-nullspace projection.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.rhs_projection_norm
    }
}

/// Fused result with solutions retained in contiguous column-major order.
#[derive(Debug, Clone, PartialEq)]
pub struct VckssFusedPcgBatchResult {
    solutions: Vec<f64>,
    reports: Vec<VckssFusedPcgColumnReport>,
}

impl VckssFusedPcgBatchResult {
    /// Return the contiguous column-major solution block.
    #[must_use]
    pub fn solutions(&self) -> &[f64] {
        &self.solutions
    }

    /// Return per-column diagnostics in input order.
    #[must_use]
    pub fn reports(&self) -> &[VckssFusedPcgColumnReport] {
        &self.reports
    }
}

impl ParallelPcgSolver {
    /// Run an ordered VCkss-private column map on the solver-owned pool.
    ///
    /// This helper is injected only into the archived performance spike. It
    /// keeps Rayon out of the VCkss crate, preserves indexed input order, and
    /// lets the caller poll Stata UserBreak between bounded column chunks.
    pub fn vckss_map_ordered<Input, Output, Operation>(
        &self,
        input: Vec<Input>,
        operation: Operation,
    ) -> Vec<Output>
    where
        Input: Send,
        Output: Send,
        Operation: Fn(Input) -> Output + Send + Sync,
    {
        self.executor()
            .install(|| input.into_par_iter().map(operation).collect())
    }
}

#[derive(Debug)]
struct FusedCsr {
    row_offsets: Vec<usize>,
    columns: Vec<u32>,
    weights: Vec<f64>,
}

#[derive(Debug)]
struct FusedCsrF32 {
    row_offsets: Vec<usize>,
    columns: Vec<u32>,
    weights: Vec<f32>,
}

impl FusedCsrF32 {
    fn from_f64(csr: &FusedCsr) -> Result<Self, CmgError> {
        let mut weights = Vec::with_capacity(csr.weights.len());
        for &weight in &csr.weights {
            let converted = weight as f32;
            if !converted.is_finite() || converted <= 0.0 {
                return Err(CmgError::InvalidHierarchy {
                    context: "VCkss mixed fused CSR weight is not positive finite f32",
                });
            }
            weights.push(converted);
        }
        Ok(Self {
            row_offsets: csr.row_offsets.clone(),
            columns: csr.columns.clone(),
            weights,
        })
    }

    fn byte_len(&self) -> usize {
        self.row_offsets
            .len()
            .saturating_mul(core::mem::size_of::<usize>())
            .saturating_add(
                self.columns
                    .len()
                    .saturating_mul(core::mem::size_of::<u32>()),
            )
            .saturating_add(
                self.weights
                    .len()
                    .saturating_mul(core::mem::size_of::<f32>()),
            )
    }

    fn matvec(
        &self,
        input: &[f32],
        output: &mut [f32],
        columns: usize,
        active: &[bool],
        executor: &ParallelExecutor,
    ) {
        debug_assert_eq!(input.len(), output.len());
        debug_assert_eq!(input.len(), (self.row_offsets.len() - 1) * columns);
        let all_active = active[..columns].iter().all(|value| *value);
        executor.install(|| {
            output
                .par_chunks_mut(columns)
                .enumerate()
                .for_each(|(row, output_row)| {
                    output_row.fill(0.0);
                    let input_row = &input[row * columns..(row + 1) * columns];
                    for edge_index in self.row_offsets[row]..self.row_offsets[row + 1] {
                        let neighbor = self.columns[edge_index] as usize;
                        let neighbor_row = &input[neighbor * columns..(neighbor + 1) * columns];
                        let weight = self.weights[edge_index];
                        if all_active && columns == 16 {
                            for column in 0..16 {
                                output_row[column] +=
                                    weight * (input_row[column] - neighbor_row[column]);
                            }
                        } else if all_active {
                            for column in 0..columns {
                                output_row[column] +=
                                    weight * (input_row[column] - neighbor_row[column]);
                            }
                        } else {
                            for column in 0..columns {
                                if active[column] {
                                    output_row[column] +=
                                        weight * (input_row[column] - neighbor_row[column]);
                                }
                            }
                        }
                    }
                });
        });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FusedPrecision {
    F64,
    MixedF32,
}

impl FusedCsr {
    fn from_laplacian(graph: &Laplacian) -> Result<Self, CmgError> {
        let mut counts = vec![0_usize; graph.vertex_count()];
        for edge in graph.edges() {
            counts[edge.u()] += 1;
            counts[edge.v()] += 1;
        }
        let mut row_offsets = Vec::with_capacity(graph.vertex_count() + 1);
        row_offsets.push(0);
        for count in counts {
            let next = row_offsets
                .last()
                .copied()
                .unwrap_or(0_usize)
                .checked_add(count)
                .ok_or(CmgError::InvalidHierarchy {
                    context: "VCkss fused CSR row offsets overflowed",
                })?;
            row_offsets.push(next);
        }
        let directed = row_offsets.last().copied().unwrap_or(0);
        let mut columns = vec![0_u32; directed];
        let mut weights = vec![0.0; directed];
        let mut next = row_offsets[..graph.vertex_count()].to_vec();
        for edge in graph.edges() {
            let left = next[edge.u()];
            columns[left] = u32::try_from(edge.v()).map_err(|_| CmgError::VertexIndexTooWide {
                vertex: edge.v(),
                maximum: u32::MAX as usize,
            })?;
            weights[left] = edge.weight();
            next[edge.u()] += 1;

            let right = next[edge.v()];
            columns[right] = u32::try_from(edge.u()).map_err(|_| CmgError::VertexIndexTooWide {
                vertex: edge.u(),
                maximum: u32::MAX as usize,
            })?;
            weights[right] = edge.weight();
            next[edge.v()] += 1;
        }
        Ok(Self {
            row_offsets,
            columns,
            weights,
        })
    }

    fn byte_len(&self) -> usize {
        self.row_offsets
            .len()
            .saturating_mul(core::mem::size_of::<usize>())
            .saturating_add(
                self.columns
                    .len()
                    .saturating_mul(core::mem::size_of::<u32>()),
            )
            .saturating_add(
                self.weights
                    .len()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
    }

    fn matvec(
        &self,
        input: &[f64],
        output: &mut [f64],
        columns: usize,
        active: &[bool],
        executor: &ParallelExecutor,
    ) {
        debug_assert_eq!(input.len(), output.len());
        debug_assert_eq!(input.len(), (self.row_offsets.len() - 1) * columns);
        let all_active = active[..columns].iter().all(|value| *value);
        executor.install(|| {
            output
                .par_chunks_mut(columns)
                .enumerate()
                .for_each(|(row, output_row)| {
                    output_row.fill(0.0);
                    let input_row = &input[row * columns..(row + 1) * columns];
                    for edge_index in self.row_offsets[row]..self.row_offsets[row + 1] {
                        let neighbor = self.columns[edge_index] as usize;
                        let neighbor_row = &input[neighbor * columns..(neighbor + 1) * columns];
                        let weight = self.weights[edge_index];
                        if all_active && columns == 16 {
                            for column in 0..16 {
                                output_row[column] +=
                                    weight * (input_row[column] - neighbor_row[column]);
                            }
                        } else if all_active {
                            for column in 0..columns {
                                output_row[column] +=
                                    weight * (input_row[column] - neighbor_row[column]);
                            }
                        } else {
                            for column in 0..columns {
                                if active[column] {
                                    output_row[column] +=
                                        weight * (input_row[column] - neighbor_row[column]);
                                }
                            }
                        }
                    }
                });
        });
    }
}

#[derive(Debug)]
struct FusedLevel {
    dimension: usize,
    inverse_diagonal_f64: Option<Vec<f64>>,
    inverse_diagonal_f32: Option<Vec<f32>>,
    csr_f64: Option<FusedCsr>,
    csr_f32: Option<FusedCsrF32>,
    aggregation_labels: Option<Vec<usize>>,
    coarse_dimension: usize,
    component_sizes: Vec<usize>,
    terminal_reason: Option<TerminalReason>,
}

/// Prepared immutable sparse operators for fused independent PCGs.
#[derive(Debug)]
pub struct VckssFusedPcgSolver {
    levels: Vec<FusedLevel>,
    structural_bytes: usize,
    precision: FusedPrecision,
}

impl VckssFusedPcgSolver {
    /// Prepare row-oriented operators from an already built full-CMG hierarchy.
    pub fn build(solver: &ParallelPcgSolver) -> Result<Self, CmgError> {
        Self::build_with_precision(solver, FusedPrecision::F64)
    }

    /// Prepare a private mixed-precision hierarchy while retaining f64 PCG.
    pub fn build_mixed(solver: &ParallelPcgSolver) -> Result<Self, CmgError> {
        Self::build_with_precision(solver, FusedPrecision::MixedF32)
    }

    fn build_with_precision(
        solver: &ParallelPcgSolver,
        precision: FusedPrecision,
    ) -> Result<Self, CmgError> {
        let hierarchy = solver.preconditioner().hierarchy();
        let mut levels = Vec::with_capacity(hierarchy.levels().len());
        let mut structural_bytes = 0_usize;
        for (level_index, level) in hierarchy.levels().iter().enumerate() {
            let graph = level.graph();
            let base_csr = FusedCsr::from_laplacian(graph)?;
            let (csr_f64, csr_f32, inverse_diagonal_f64, inverse_diagonal_f32) = match precision {
                FusedPrecision::F64 => (
                    Some(base_csr),
                    None,
                    Some(level.inverse_diagonal().to_vec()),
                    None,
                ),
                FusedPrecision::MixedF32 => {
                    let mixed_csr = FusedCsrF32::from_f64(&base_csr)?;
                    let mut inverse = Vec::with_capacity(level.inverse_diagonal().len());
                    for &value in level.inverse_diagonal() {
                        let converted = value as f32;
                        if !converted.is_finite() || converted < 0.0 {
                            return Err(CmgError::InvalidHierarchy {
                                context: "VCkss mixed inverse diagonal is not finite f32",
                            });
                        }
                        inverse.push(converted);
                    }
                    (
                        (level_index == 0).then_some(base_csr),
                        Some(mixed_csr),
                        None,
                        Some(inverse),
                    )
                }
            };
            structural_bytes = structural_bytes
                .saturating_add(csr_f64.as_ref().map_or(0, FusedCsr::byte_len))
                .saturating_add(csr_f32.as_ref().map_or(0, FusedCsrF32::byte_len))
                .saturating_add(
                    inverse_diagonal_f64
                        .as_ref()
                        .map_or(0, |values| values.len() * core::mem::size_of::<f64>()),
                )
                .saturating_add(
                    inverse_diagonal_f32
                        .as_ref()
                        .map_or(0, |values| values.len() * core::mem::size_of::<f32>()),
                );
            let components = Components::from_laplacian(graph);
            let aggregation_labels = level
                .aggregation()
                .map(|aggregation| aggregation.labels().to_vec());
            let coarse_dimension = level
                .aggregation()
                .map_or(0, |aggregation| aggregation.coarse_dimension());
            if components.count() != 1 {
                return Err(CmgError::InvalidHierarchy {
                    context: "VCkss fused spike currently requires a connected hybrid graph",
                });
            }
            levels.push(FusedLevel {
                dimension: graph.vertex_count(),
                inverse_diagonal_f64,
                inverse_diagonal_f32,
                csr_f64,
                csr_f32,
                aggregation_labels,
                coarse_dimension,
                component_sizes: components.sizes().to_vec(),
                terminal_reason: level.terminal_reason(),
            });
        }
        Ok(Self {
            levels,
            structural_bytes,
            precision,
        })
    }

    /// Return retained bytes in the additional private row operators.
    #[must_use]
    pub const fn structural_bytes(&self) -> usize {
        self.structural_bytes
    }

    /// Allocate a reusable workspace for at most `columns` simultaneous RHSs.
    #[must_use]
    pub fn workspace(&self, columns: usize) -> VckssFusedPcgWorkspace {
        VckssFusedPcgWorkspace::new(self, columns)
    }

    /// Solve a contiguous column-major RHS block using independent fused PCGs.
    pub fn solve_batch_with_workspace(
        &self,
        solver: &ParallelPcgSolver,
        right_hand_sides: &[f64],
        columns: usize,
        options: PcgOptions,
        workspace: &mut VckssFusedPcgWorkspace,
    ) -> Result<VckssFusedPcgBatchResult, CmgError> {
        let total_start = Instant::now();
        let mut preconditioner_nanoseconds = 0_u128;
        let mut finest_matvec_nanoseconds = 0_u128;
        let mut reduction_nanoseconds = 0_u128;
        let mut centering_nanoseconds = 0_u128;
        let layout_start = Instant::now();
        let options = options.validate()?;
        let dimension = solver.graph().vertex_count();
        if columns == 0 || right_hand_sides.len() != dimension.saturating_mul(columns) {
            return Err(CmgError::dimension(
                "VCkss fused RHS block",
                dimension.saturating_mul(columns),
                right_hand_sides.len(),
            ));
        }
        if self.levels.first().map(|level| level.dimension) != Some(dimension) {
            return Err(CmgError::InvalidHierarchy {
                context: "VCkss fused operators differ from prepared finest graph",
            });
        }
        workspace.ensure_shape(self, columns);

        let fine = dimension.saturating_mul(columns);
        workspace.projected_column_major[..fine].copy_from_slice(right_hand_sides);
        let components = Components::from_laplacian(solver.graph());
        for column in 0..columns {
            workspace.rhs_projection_norm[column] = components.project_rhs_in_place(
                &mut workspace.projected_column_major[column * dimension..(column + 1) * dimension],
                options.validation,
            )?;
        }
        transpose_columns_to_vertices(
            &workspace.projected_column_major[..fine],
            &mut workspace.projected[..fine],
            dimension,
            columns,
        );
        let mut layout_nanoseconds = layout_start.elapsed().as_nanos();
        workspace.solution[..fine].fill(0.0);
        workspace.residual[..fine].copy_from_slice(&workspace.projected[..fine]);
        workspace.preconditioned[..fine].fill(0.0);
        workspace.direction[..fine].fill(0.0);
        workspace.matrix_direction[..fine].fill(0.0);
        workspace.active[..columns].fill(true);
        workspace.restarted[..columns].fill(false);
        workspace.iterations[..columns].fill(0);
        workspace.restarts[..columns].fill(0);

        let operator_bound = solver.graph().operator_norm_bound();
        for column in 0..columns {
            let original = &right_hand_sides[column * dimension..(column + 1) * dimension];
            workspace.initial_norm[column] = norm(original.iter().copied());
            workspace.tolerances[column] =
                allowed_residual(options, workspace.initial_norm[column], operator_bound, 0.0);
            if workspace.initial_norm[column] <= workspace.tolerances[column] {
                workspace.active[column] = false;
                workspace.final_residual[column] = workspace.initial_norm[column];
            }
        }
        if workspace.active[..columns].iter().any(|active| *active) {
            let phase_start = Instant::now();
            self.apply_preconditioner_from_residual(
                solver.preconditioner().terminal_factor(),
                solver.preconditioner().repeat_counts(),
                fine,
                columns,
                workspace,
                solver.executor(),
            )?;
            preconditioner_nanoseconds += phase_start.elapsed().as_nanos();
            let phase_start = Instant::now();
            center_columns(
                &self.levels[0],
                &mut workspace.preconditioned[..fine],
                columns,
                &workspace.active[..columns],
                solver.executor(),
                &mut workspace.center_sums,
                &mut workspace.center_corrections,
            );
            centering_nanoseconds += phase_start.elapsed().as_nanos();
            workspace.direction[..fine].copy_from_slice(&workspace.preconditioned[..fine]);
            let phase_start = Instant::now();
            dot_columns(
                &workspace.residual[..fine],
                &workspace.preconditioned[..fine],
                dimension,
                columns,
                &workspace.active[..columns],
                &mut workspace.rho,
                solver.executor(),
                &mut workspace.reduction_sums,
                &mut workspace.reduction_corrections,
            );
            reduction_nanoseconds += phase_start.elapsed().as_nanos();
            validate_positive_columns(0, &workspace.rho, &workspace.active[..columns])?;
        }

        for iteration in 1..=options.max_iterations {
            if !workspace.active[..columns].iter().any(|active| *active) {
                break;
            }
            let phase_start = Instant::now();
            self.levels[0]
                .csr_f64
                .as_ref()
                .expect("finest f64 operator is retained")
                .matvec(
                    &workspace.direction[..fine],
                    &mut workspace.matrix_direction[..fine],
                    columns,
                    &workspace.active[..columns],
                    solver.executor(),
                );
            finest_matvec_nanoseconds += phase_start.elapsed().as_nanos();
            let phase_start = Instant::now();
            dot_columns(
                &workspace.direction[..fine],
                &workspace.matrix_direction[..fine],
                dimension,
                columns,
                &workspace.active[..columns],
                &mut workspace.curvature,
                solver.executor(),
                &mut workspace.reduction_sums,
                &mut workspace.reduction_corrections,
            );
            reduction_nanoseconds += phase_start.elapsed().as_nanos();
            validate_positive_columns(
                iteration,
                &workspace.curvature,
                &workspace.active[..columns],
            )?;
            for column in 0..columns {
                if workspace.active[column] {
                    workspace.alpha[column] = workspace.rho[column] / workspace.curvature[column];
                    validate_finite(iteration, "alpha", workspace.alpha[column])?;
                    workspace.iterations[column] = iteration;
                }
            }
            update_solution_residual(
                &mut workspace.solution[..fine],
                &mut workspace.residual[..fine],
                &workspace.direction[..fine],
                &workspace.matrix_direction[..fine],
                columns,
                &workspace.active[..columns],
                &workspace.alpha,
                solver.executor(),
            );
            let phase_start = Instant::now();
            center_columns(
                &self.levels[0],
                &mut workspace.solution[..fine],
                columns,
                &workspace.active[..columns],
                solver.executor(),
                &mut workspace.center_sums,
                &mut workspace.center_corrections,
            );
            centering_nanoseconds += phase_start.elapsed().as_nanos();
            let phase_start = Instant::now();
            norm_columns(
                &workspace.solution[..fine],
                dimension,
                columns,
                &workspace.active[..columns],
                &mut workspace.solution_norm,
                solver.executor(),
                &mut workspace.reduction_sums,
                &mut workspace.reduction_corrections,
            );
            norm_columns(
                &workspace.residual[..fine],
                dimension,
                columns,
                &workspace.active[..columns],
                &mut workspace.recursive_norm,
                solver.executor(),
                &mut workspace.reduction_sums,
                &mut workspace.reduction_corrections,
            );
            reduction_nanoseconds += phase_start.elapsed().as_nanos();
            let scheduled = iteration % options.residual_recompute_interval == 0;
            let mut needs_fresh = scheduled;
            for column in 0..columns {
                if workspace.active[column] {
                    workspace.tolerances[column] = allowed_residual(
                        options,
                        workspace.initial_norm[column],
                        operator_bound,
                        workspace.solution_norm[column],
                    );
                    workspace.candidate[column] =
                        workspace.recursive_norm[column] <= workspace.tolerances[column];
                    needs_fresh |= workspace.candidate[column];
                } else {
                    workspace.candidate[column] = false;
                }
                workspace.restarted[column] = false;
            }
            if needs_fresh {
                let phase_start = Instant::now();
                self.levels[0]
                    .csr_f64
                    .as_ref()
                    .expect("finest f64 operator is retained")
                    .matvec(
                        &workspace.solution[..fine],
                        &mut workspace.fresh_residual[..fine],
                        columns,
                        &workspace.active[..columns],
                        solver.executor(),
                    );
                finest_matvec_nanoseconds += phase_start.elapsed().as_nanos();
                residual_from_projected_rhs(
                    &mut workspace.fresh_residual[..fine],
                    &workspace.projected[..fine],
                    columns,
                    &workspace.active[..columns],
                    solver.executor(),
                );
                let phase_start = Instant::now();
                norm_columns(
                    &workspace.fresh_residual[..fine],
                    dimension,
                    columns,
                    &workspace.active[..columns],
                    &mut workspace.fresh_norm,
                    solver.executor(),
                    &mut workspace.reduction_sums,
                    &mut workspace.reduction_corrections,
                );
                reduction_nanoseconds += phase_start.elapsed().as_nanos();
                for column in 0..columns {
                    if !workspace.active[column] || (!scheduled && !workspace.candidate[column]) {
                        continue;
                    }
                    copy_column_vertex_major(
                        &workspace.fresh_residual[..fine],
                        &mut workspace.residual[..fine],
                        dimension,
                        columns,
                        column,
                    );
                    workspace.restarted[column] = true;
                    workspace.restarts[column] += 1;
                    if workspace.candidate[column]
                        && workspace.fresh_norm[column] <= workspace.tolerances[column]
                    {
                        let original =
                            &right_hand_sides[column * dimension..(column + 1) * dimension];
                        let projected = &workspace.projected_column_major
                            [column * dimension..(column + 1) * dimension];
                        let original_norm = original_residual_norm_vertex_major(
                            original,
                            projected,
                            &workspace.fresh_residual[..fine],
                            dimension,
                            columns,
                            column,
                        );
                        if original_norm <= workspace.tolerances[column] {
                            workspace.final_residual[column] = original_norm;
                            workspace.active[column] = false;
                        }
                    }
                }
            }
            if !workspace.active[..columns].iter().any(|active| *active) {
                break;
            }
            if iteration == options.max_iterations {
                let first = workspace.active[..columns]
                    .iter()
                    .position(|active| *active)
                    .unwrap_or(0);
                return Err(CmgError::MaximumIterations {
                    iterations: options.max_iterations,
                    residual_norm: workspace.recursive_norm[first],
                    tolerance: workspace.tolerances[first],
                });
            }
            let phase_start = Instant::now();
            center_columns(
                &self.levels[0],
                &mut workspace.residual[..fine],
                columns,
                &workspace.active[..columns],
                solver.executor(),
                &mut workspace.center_sums,
                &mut workspace.center_corrections,
            );
            centering_nanoseconds += phase_start.elapsed().as_nanos();
            let phase_start = Instant::now();
            self.apply_preconditioner_from_residual(
                solver.preconditioner().terminal_factor(),
                solver.preconditioner().repeat_counts(),
                fine,
                columns,
                workspace,
                solver.executor(),
            )?;
            preconditioner_nanoseconds += phase_start.elapsed().as_nanos();
            let phase_start = Instant::now();
            center_columns(
                &self.levels[0],
                &mut workspace.preconditioned[..fine],
                columns,
                &workspace.active[..columns],
                solver.executor(),
                &mut workspace.center_sums,
                &mut workspace.center_corrections,
            );
            centering_nanoseconds += phase_start.elapsed().as_nanos();
            let phase_start = Instant::now();
            dot_columns(
                &workspace.residual[..fine],
                &workspace.preconditioned[..fine],
                dimension,
                columns,
                &workspace.active[..columns],
                &mut workspace.new_rho,
                solver.executor(),
                &mut workspace.reduction_sums,
                &mut workspace.reduction_corrections,
            );
            reduction_nanoseconds += phase_start.elapsed().as_nanos();
            validate_positive_columns(iteration, &workspace.new_rho, &workspace.active[..columns])?;
            for column in 0..columns {
                if workspace.active[column] {
                    workspace.beta[column] = if workspace.restarted[column] {
                        0.0
                    } else {
                        workspace.new_rho[column] / workspace.rho[column]
                    };
                    validate_finite(iteration, "beta", workspace.beta[column])?;
                    workspace.rho[column] = workspace.new_rho[column];
                }
            }
            update_direction(
                &mut workspace.direction[..fine],
                &workspace.preconditioned[..fine],
                columns,
                &workspace.active[..columns],
                &workspace.restarted[..columns],
                &workspace.beta,
                solver.executor(),
            );
        }

        let layout_start = Instant::now();
        let mut solutions = vec![0.0; fine];
        transpose_vertices_to_columns(
            &workspace.solution[..fine],
            &mut solutions,
            dimension,
            columns,
        );
        layout_nanoseconds += layout_start.elapsed().as_nanos();
        let reports = (0..columns)
            .map(|column| VckssFusedPcgColumnReport {
                iterations: workspace.iterations[column],
                restarts: workspace.restarts[column],
                initial_residual_norm: workspace.initial_norm[column],
                residual_norm: workspace.final_residual[column],
                tolerance: workspace.tolerances[column],
                rhs_projection_norm: workspace.rhs_projection_norm[column],
            })
            .collect();
        let total_nanoseconds = total_start.elapsed().as_nanos();
        let measured_nanoseconds = preconditioner_nanoseconds
            .saturating_add(finest_matvec_nanoseconds)
            .saturating_add(reduction_nanoseconds)
            .saturating_add(centering_nanoseconds)
            .saturating_add(layout_nanoseconds);
        eprintln!(
            "CMG_FUSED_PROFILE_V1 columns={columns} total_ns={total_nanoseconds} preconditioner_ns={preconditioner_nanoseconds} finest_matvec_ns={finest_matvec_nanoseconds} reductions_ns={reduction_nanoseconds} centering_ns={centering_nanoseconds} layout_ns={layout_nanoseconds} other_ns={}",
            total_nanoseconds.saturating_sub(measured_nanoseconds),
        );
        Ok(VckssFusedPcgBatchResult { solutions, reports })
    }

    fn apply_preconditioner_from_residual(
        &self,
        terminal: Option<&GroundedLdl>,
        repeat_counts: &[usize],
        fine: usize,
        columns: usize,
        workspace: &mut VckssFusedPcgWorkspace,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        match self.precision {
            FusedPrecision::F64 => self.apply_preconditioner(
                terminal,
                repeat_counts,
                &workspace.residual[..fine],
                &mut workspace.preconditioned[..fine],
                columns,
                &workspace.active[..columns],
                &mut workspace.hierarchy,
                executor,
            ),
            FusedPrecision::MixedF32 => {
                executor.install(|| {
                    workspace.mixed_rhs[..fine]
                        .par_iter_mut()
                        .zip(workspace.residual[..fine].par_iter())
                        .for_each(|(target, &source)| *target = source as f32);
                });
                self.apply_level_f32(
                    0,
                    terminal,
                    repeat_counts,
                    &workspace.mixed_rhs[..fine],
                    &mut workspace.mixed_output[..fine],
                    columns,
                    &workspace.active[..columns],
                    &mut workspace.hierarchy_f32,
                    executor,
                    1,
                )?;
                executor.install(|| {
                    workspace.preconditioned[..fine]
                        .par_iter_mut()
                        .zip(workspace.mixed_output[..fine].par_iter())
                        .for_each(|(target, &source)| *target = f64::from(source));
                });
                Ok(())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_preconditioner(
        &self,
        terminal: Option<&GroundedLdl>,
        repeat_counts: &[usize],
        rhs: &[f64],
        output: &mut [f64],
        columns: usize,
        active: &[bool],
        workspace: &mut [FusedLevelWorkspace],
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        self.apply_level(
            0,
            terminal,
            repeat_counts,
            rhs,
            output,
            columns,
            active,
            workspace,
            executor,
            1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_level(
        &self,
        level_index: usize,
        terminal: Option<&GroundedLdl>,
        repeat_counts: &[usize],
        rhs: &[f64],
        output: &mut [f64],
        columns: usize,
        active: &[bool],
        workspace: &mut [FusedLevelWorkspace],
        executor: &ParallelExecutor,
        iterations: usize,
    ) -> Result<(), CmgError> {
        let level = &self.levels[level_index];
        let inverse_diagonal = level
            .inverse_diagonal_f64
            .as_ref()
            .expect("f64 hierarchy has f64 inverse diagonals");
        let csr = level
            .csr_f64
            .as_ref()
            .expect("f64 hierarchy has f64 sparse operators");
        if let Some(terminal_reason) = level.terminal_reason {
            if terminal_reason.is_iterative() {
                assign_scaled(output, rhs, inverse_diagonal, columns, active, executor);
                return Ok(());
            }
            let factor = terminal.ok_or(CmgError::InvalidHierarchy {
                context: "VCkss fused terminal factor is missing",
            })?;
            let local = &mut workspace[level_index];
            for column in 0..columns {
                if !active[column] {
                    continue;
                }
                for vertex in 0..level.dimension {
                    local.terminal_rhs[vertex] = rhs[vertex * columns + column];
                }
                factor.solve_into_compatible(
                    &local.terminal_rhs,
                    &mut local.terminal_solution,
                    &mut local.factor_forward,
                    &mut local.factor_solution,
                )?;
                for vertex in 0..level.dimension {
                    output[vertex * columns + column] = local.terminal_solution[vertex];
                }
            }
            return Ok(());
        }
        let child_iterations = repeat_counts[level_index];
        let level_len = level.dimension.saturating_mul(columns);
        let coarse_len = level.coarse_dimension.saturating_mul(columns);
        let mut local = core::mem::take(&mut workspace[level_index]);
        output.fill(0.0);
        let result = (|| {
            for iteration in 0..iterations {
                if iteration == 0 {
                    assign_scaled(output, rhs, inverse_diagonal, columns, active, executor);
                } else {
                    csr.matvec(
                        output,
                        &mut local.residual[..level_len],
                        columns,
                        active,
                        executor,
                    );
                    jacobi_add(
                        output,
                        rhs,
                        &local.residual[..level_len],
                        inverse_diagonal,
                        columns,
                        active,
                        executor,
                    );
                }
                csr.matvec(
                    output,
                    &mut local.residual[..level_len],
                    columns,
                    active,
                    executor,
                );
                residual_from_projected_rhs(
                    &mut local.residual[..level_len],
                    rhs,
                    columns,
                    active,
                    executor,
                );
                restrict(
                    level
                        .aggregation_labels
                        .as_ref()
                        .expect("checked aggregation"),
                    &local.residual[..level_len],
                    &mut local.coarse_rhs[..coarse_len],
                    level.coarse_dimension,
                    columns,
                    active,
                );
                center_columns(
                    &self.levels[level_index + 1],
                    &mut local.coarse_rhs[..coarse_len],
                    columns,
                    active,
                    executor,
                    &mut local.center_sums,
                    &mut local.center_corrections,
                );
                local.coarse_correction[..coarse_len].fill(0.0);
                self.apply_level(
                    level_index + 1,
                    terminal,
                    repeat_counts,
                    &local.coarse_rhs[..coarse_len],
                    &mut local.coarse_correction[..coarse_len],
                    columns,
                    active,
                    workspace,
                    executor,
                    child_iterations,
                )?;
                prolong_add(
                    level
                        .aggregation_labels
                        .as_ref()
                        .expect("checked aggregation"),
                    &local.coarse_correction[..coarse_len],
                    output,
                    columns,
                    active,
                    executor,
                );
                csr.matvec(
                    output,
                    &mut local.residual[..level_len],
                    columns,
                    active,
                    executor,
                );
                jacobi_add(
                    output,
                    rhs,
                    &local.residual[..level_len],
                    inverse_diagonal,
                    columns,
                    active,
                    executor,
                );
            }
            Ok(())
        })();
        workspace[level_index] = local;
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_level_f32(
        &self,
        level_index: usize,
        terminal: Option<&GroundedLdl>,
        repeat_counts: &[usize],
        rhs: &[f32],
        output: &mut [f32],
        columns: usize,
        active: &[bool],
        workspace: &mut [FusedLevelWorkspaceF32],
        executor: &ParallelExecutor,
        iterations: usize,
    ) -> Result<(), CmgError> {
        let level = &self.levels[level_index];
        let inverse_diagonal = level
            .inverse_diagonal_f32
            .as_ref()
            .expect("mixed hierarchy has f32 inverse diagonals");
        let csr = level
            .csr_f32
            .as_ref()
            .expect("mixed hierarchy has f32 sparse operators");
        if let Some(terminal_reason) = level.terminal_reason {
            if terminal_reason.is_iterative() {
                assign_scaled_f32(output, rhs, inverse_diagonal, columns, active, executor);
                return Ok(());
            }
            let factor = terminal.ok_or(CmgError::InvalidHierarchy {
                context: "VCkss mixed fused terminal factor is missing",
            })?;
            let local = &mut workspace[level_index];
            for column in 0..columns {
                if !active[column] {
                    continue;
                }
                for vertex in 0..level.dimension {
                    local.terminal_rhs[vertex] = f64::from(rhs[vertex * columns + column]);
                }
                factor.solve_into_compatible(
                    &local.terminal_rhs,
                    &mut local.terminal_solution,
                    &mut local.factor_forward,
                    &mut local.factor_solution,
                )?;
                for vertex in 0..level.dimension {
                    output[vertex * columns + column] = local.terminal_solution[vertex] as f32;
                }
            }
            return Ok(());
        }
        let child_iterations = repeat_counts[level_index];
        let level_len = level.dimension.saturating_mul(columns);
        let coarse_len = level.coarse_dimension.saturating_mul(columns);
        let mut local = core::mem::take(&mut workspace[level_index]);
        output.fill(0.0);
        let result = (|| {
            for iteration in 0..iterations {
                if iteration == 0 {
                    assign_scaled_f32(output, rhs, inverse_diagonal, columns, active, executor);
                } else {
                    csr.matvec(
                        output,
                        &mut local.residual[..level_len],
                        columns,
                        active,
                        executor,
                    );
                    jacobi_add_f32(
                        output,
                        rhs,
                        &local.residual[..level_len],
                        inverse_diagonal,
                        columns,
                        active,
                        executor,
                    );
                }
                csr.matvec(
                    output,
                    &mut local.residual[..level_len],
                    columns,
                    active,
                    executor,
                );
                residual_from_projected_rhs_f32(
                    &mut local.residual[..level_len],
                    rhs,
                    columns,
                    active,
                    executor,
                );
                restrict_f32(
                    level
                        .aggregation_labels
                        .as_ref()
                        .expect("checked aggregation"),
                    &local.residual[..level_len],
                    &mut local.coarse_rhs[..coarse_len],
                    level.coarse_dimension,
                    columns,
                    active,
                );
                center_columns_f32(
                    &self.levels[level_index + 1],
                    &mut local.coarse_rhs[..coarse_len],
                    columns,
                    active,
                    executor,
                    &mut local.center_sums,
                    &mut local.center_corrections,
                );
                local.coarse_correction[..coarse_len].fill(0.0);
                self.apply_level_f32(
                    level_index + 1,
                    terminal,
                    repeat_counts,
                    &local.coarse_rhs[..coarse_len],
                    &mut local.coarse_correction[..coarse_len],
                    columns,
                    active,
                    workspace,
                    executor,
                    child_iterations,
                )?;
                prolong_add_f32(
                    level
                        .aggregation_labels
                        .as_ref()
                        .expect("checked aggregation"),
                    &local.coarse_correction[..coarse_len],
                    output,
                    columns,
                    active,
                    executor,
                );
                csr.matvec(
                    output,
                    &mut local.residual[..level_len],
                    columns,
                    active,
                    executor,
                );
                jacobi_add_f32(
                    output,
                    rhs,
                    &local.residual[..level_len],
                    inverse_diagonal,
                    columns,
                    active,
                    executor,
                );
            }
            Ok(())
        })();
        workspace[level_index] = local;
        result
    }
}

#[derive(Debug, Default)]
struct FusedLevelWorkspace {
    residual: Vec<f64>,
    coarse_rhs: Vec<f64>,
    coarse_correction: Vec<f64>,
    center_sums: Vec<f64>,
    center_corrections: Vec<f64>,
    terminal_rhs: Vec<f64>,
    terminal_solution: Vec<f64>,
    factor_forward: Vec<f64>,
    factor_solution: Vec<f64>,
}

#[derive(Debug, Default)]
struct FusedLevelWorkspaceF32 {
    residual: Vec<f32>,
    coarse_rhs: Vec<f32>,
    coarse_correction: Vec<f32>,
    center_sums: Vec<f64>,
    center_corrections: Vec<f64>,
    terminal_rhs: Vec<f64>,
    terminal_solution: Vec<f64>,
    factor_forward: Vec<f64>,
    factor_solution: Vec<f64>,
}

/// Reusable admitted storage for one maximum-width fused RHS block.
#[derive(Debug)]
pub struct VckssFusedPcgWorkspace {
    capacity_columns: usize,
    projected_column_major: Vec<f64>,
    projected: Vec<f64>,
    solution: Vec<f64>,
    residual: Vec<f64>,
    preconditioned: Vec<f64>,
    direction: Vec<f64>,
    matrix_direction: Vec<f64>,
    fresh_residual: Vec<f64>,
    hierarchy: Vec<FusedLevelWorkspace>,
    mixed_rhs: Vec<f32>,
    mixed_output: Vec<f32>,
    hierarchy_f32: Vec<FusedLevelWorkspaceF32>,
    active: Vec<bool>,
    candidate: Vec<bool>,
    restarted: Vec<bool>,
    iterations: Vec<usize>,
    restarts: Vec<usize>,
    initial_norm: Vec<f64>,
    final_residual: Vec<f64>,
    tolerances: Vec<f64>,
    rhs_projection_norm: Vec<f64>,
    solution_norm: Vec<f64>,
    recursive_norm: Vec<f64>,
    fresh_norm: Vec<f64>,
    rho: Vec<f64>,
    new_rho: Vec<f64>,
    curvature: Vec<f64>,
    alpha: Vec<f64>,
    beta: Vec<f64>,
    center_sums: Vec<f64>,
    center_corrections: Vec<f64>,
    reduction_sums: Vec<f64>,
    reduction_corrections: Vec<f64>,
}

impl VckssFusedPcgWorkspace {
    fn new(solver: &VckssFusedPcgSolver, columns: usize) -> Self {
        let fine_dimension = solver.levels[0].dimension;
        let fine_len = fine_dimension.saturating_mul(columns);
        let hierarchy = if solver.precision == FusedPrecision::F64 {
            solver
                .levels
                .iter()
                .enumerate()
                .map(|(index, level)| {
                    let coarse = level.coarse_dimension.saturating_mul(columns);
                    let factor = if index + 1 == solver.levels.len() {
                        level.dimension.saturating_sub(level.component_sizes.len())
                    } else {
                        0
                    };
                    FusedLevelWorkspace {
                        residual: vec![0.0; level.dimension.saturating_mul(columns)],
                        coarse_rhs: vec![0.0; coarse],
                        coarse_correction: vec![0.0; coarse],
                        center_sums: vec![
                            0.0;
                            level
                                .coarse_dimension
                                .div_ceil(REDUCTION_CHUNK_VERTICES)
                                .saturating_mul(columns)
                        ],
                        center_corrections: vec![
                            0.0;
                            level
                                .coarse_dimension
                                .div_ceil(REDUCTION_CHUNK_VERTICES)
                                .saturating_mul(columns)
                        ],
                        terminal_rhs: vec![0.0; if factor > 0 { level.dimension } else { 0 }],
                        terminal_solution: vec![0.0; if factor > 0 { level.dimension } else { 0 }],
                        factor_forward: vec![0.0; factor],
                        factor_solution: vec![0.0; factor],
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let hierarchy_f32 = if solver.precision == FusedPrecision::MixedF32 {
            solver
                .levels
                .iter()
                .enumerate()
                .map(|(index, level)| {
                    let coarse = level.coarse_dimension.saturating_mul(columns);
                    let factor = if index + 1 == solver.levels.len() {
                        level.dimension.saturating_sub(level.component_sizes.len())
                    } else {
                        0
                    };
                    FusedLevelWorkspaceF32 {
                        residual: vec![0.0; level.dimension.saturating_mul(columns)],
                        coarse_rhs: vec![0.0; coarse],
                        coarse_correction: vec![0.0; coarse],
                        center_sums: vec![
                            0.0;
                            level
                                .coarse_dimension
                                .div_ceil(REDUCTION_CHUNK_VERTICES)
                                .saturating_mul(columns)
                        ],
                        center_corrections: vec![
                            0.0;
                            level
                                .coarse_dimension
                                .div_ceil(REDUCTION_CHUNK_VERTICES)
                                .saturating_mul(columns)
                        ],
                        terminal_rhs: vec![0.0; if factor > 0 { level.dimension } else { 0 }],
                        terminal_solution: vec![0.0; if factor > 0 { level.dimension } else { 0 }],
                        factor_forward: vec![0.0; factor],
                        factor_solution: vec![0.0; factor],
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let mixed_len = if solver.precision == FusedPrecision::MixedF32 {
            fine_len
        } else {
            0
        };
        Self {
            capacity_columns: columns,
            projected_column_major: vec![0.0; fine_len],
            projected: vec![0.0; fine_len],
            solution: vec![0.0; fine_len],
            residual: vec![0.0; fine_len],
            preconditioned: vec![0.0; fine_len],
            direction: vec![0.0; fine_len],
            matrix_direction: vec![0.0; fine_len],
            fresh_residual: vec![0.0; fine_len],
            hierarchy,
            mixed_rhs: vec![0.0; mixed_len],
            mixed_output: vec![0.0; mixed_len],
            hierarchy_f32,
            active: vec![false; columns],
            candidate: vec![false; columns],
            restarted: vec![false; columns],
            iterations: vec![0; columns],
            restarts: vec![0; columns],
            initial_norm: vec![0.0; columns],
            final_residual: vec![0.0; columns],
            tolerances: vec![0.0; columns],
            rhs_projection_norm: vec![0.0; columns],
            solution_norm: vec![0.0; columns],
            recursive_norm: vec![0.0; columns],
            fresh_norm: vec![0.0; columns],
            rho: vec![0.0; columns],
            new_rho: vec![0.0; columns],
            curvature: vec![0.0; columns],
            alpha: vec![0.0; columns],
            beta: vec![0.0; columns],
            center_sums: vec![0.0; fine_dimension.div_ceil(REDUCTION_CHUNK_VERTICES) * columns],
            center_corrections: vec![
                0.0;
                fine_dimension.div_ceil(REDUCTION_CHUNK_VERTICES) * columns
            ],
            reduction_sums: vec![0.0; fine_dimension.div_ceil(REDUCTION_CHUNK_VERTICES) * columns],
            reduction_corrections: vec![
                0.0;
                fine_dimension.div_ceil(REDUCTION_CHUNK_VERTICES) * columns
            ],
        }
    }

    fn ensure_shape(&mut self, solver: &VckssFusedPcgSolver, columns: usize) {
        if columns > self.capacity_columns {
            *self = Self::new(solver, columns);
        }
    }

    /// Return principal retained heap bytes.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        let fine_vectors = [
            &self.projected_column_major,
            &self.projected,
            &self.solution,
            &self.residual,
            &self.preconditioned,
            &self.direction,
            &self.matrix_direction,
            &self.fresh_residual,
        ]
        .iter()
        .map(|values| values.len())
        .sum::<usize>();
        let hierarchy_f64 = self
            .hierarchy
            .iter()
            .map(|level| {
                level.residual.len()
                    + level.coarse_rhs.len()
                    + level.coarse_correction.len()
                    + level.center_sums.len()
                    + level.center_corrections.len()
                    + level.terminal_rhs.len()
                    + level.terminal_solution.len()
                    + level.factor_forward.len()
                    + level.factor_solution.len()
            })
            .sum::<usize>();
        let hierarchy_mixed_f32 = self
            .hierarchy_f32
            .iter()
            .map(|level| {
                level.residual.len() + level.coarse_rhs.len() + level.coarse_correction.len()
            })
            .sum::<usize>();
        let hierarchy_mixed_f64 = self
            .hierarchy_f32
            .iter()
            .map(|level| {
                level.center_sums.len()
                    + level.center_corrections.len()
                    + level.terminal_rhs.len()
                    + level.terminal_solution.len()
                    + level.factor_forward.len()
                    + level.factor_solution.len()
            })
            .sum::<usize>();
        let f64_bytes = fine_vectors
            .saturating_add(hierarchy_f64)
            .saturating_add(hierarchy_mixed_f64)
            .saturating_add(self.reduction_sums.len())
            .saturating_add(self.reduction_corrections.len())
            .saturating_mul(core::mem::size_of::<f64>());
        let f32_bytes = self
            .mixed_rhs
            .len()
            .saturating_add(self.mixed_output.len())
            .saturating_add(hierarchy_mixed_f32)
            .saturating_mul(core::mem::size_of::<f32>());
        f64_bytes.saturating_add(f32_bytes)
    }
}

fn assign_scaled(
    output: &mut [f64],
    rhs: &[f64],
    inverse_diagonal: &[f64],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        output
            .par_chunks_mut(columns)
            .zip(rhs.par_chunks(columns))
            .zip(inverse_diagonal.par_iter())
            .for_each(|((output_row, rhs_row), inverse)| {
                for column in 0..columns {
                    output_row[column] = if active[column] {
                        *inverse * rhs_row[column]
                    } else {
                        0.0
                    };
                }
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn jacobi_add(
    output: &mut [f64],
    rhs: &[f64],
    product: &[f64],
    inverse_diagonal: &[f64],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        output
            .par_chunks_mut(columns)
            .zip(rhs.par_chunks(columns))
            .zip(product.par_chunks(columns))
            .zip(inverse_diagonal.par_iter())
            .for_each(|(((output_row, rhs_row), product_row), inverse)| {
                for column in 0..columns {
                    if active[column] {
                        output_row[column] += *inverse * (rhs_row[column] - product_row[column]);
                    }
                }
            });
    });
}

fn residual_from_projected_rhs(
    residual: &mut [f64],
    rhs: &[f64],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        residual
            .par_chunks_mut(columns)
            .zip(rhs.par_chunks(columns))
            .for_each(|(residual_row, rhs_row)| {
                for column in 0..columns {
                    if active[column] {
                        residual_row[column] = rhs_row[column] - residual_row[column];
                    }
                }
            });
    });
}

fn restrict(
    labels: &[usize],
    fine: &[f64],
    coarse: &mut [f64],
    coarse_dimension: usize,
    columns: usize,
    active: &[bool],
) {
    coarse.fill(0.0);
    for (vertex, &label) in labels.iter().enumerate() {
        let fine_row = &fine[vertex * columns..(vertex + 1) * columns];
        let coarse_row = &mut coarse[label * columns..(label + 1) * columns];
        for column in 0..columns {
            if active[column] {
                coarse_row[column] += fine_row[column];
            }
        }
    }
    debug_assert_eq!(coarse.len(), coarse_dimension * columns);
}

fn prolong_add(
    labels: &[usize],
    coarse: &[f64],
    fine: &mut [f64],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        fine.par_chunks_mut(columns)
            .enumerate()
            .for_each(|(vertex, fine_row)| {
                let coarse_row = &coarse[labels[vertex] * columns..(labels[vertex] + 1) * columns];
                for column in 0..columns {
                    if active[column] {
                        fine_row[column] += coarse_row[column];
                    }
                }
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn center_columns(
    level: &FusedLevel,
    values: &mut [f64],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
    sums: &mut Vec<f64>,
    corrections: &mut Vec<f64>,
) {
    debug_assert_eq!(level.component_sizes.len(), 1);
    let chunk_count = level.dimension.div_ceil(REDUCTION_CHUNK_VERTICES);
    let needed = chunk_count.saturating_mul(columns);
    sums.resize(needed, 0.0);
    corrections.resize(needed, 0.0);
    sums.fill(0.0);
    corrections.fill(0.0);
    executor.install(|| {
        sums.par_chunks_mut(columns)
            .zip(corrections.par_chunks_mut(columns))
            .enumerate()
            .for_each(|(chunk, (chunk_sums, chunk_corrections))| {
                let first = chunk * REDUCTION_CHUNK_VERTICES;
                let last = level.dimension.min(first + REDUCTION_CHUNK_VERTICES);
                for vertex in first..last {
                    let row = &values[vertex * columns..(vertex + 1) * columns];
                    for column in 0..columns {
                        if active[column] {
                            neumaier_add(
                                &mut chunk_sums[column],
                                &mut chunk_corrections[column],
                                row[column],
                            );
                        }
                    }
                }
            });
    });
    for column in 0..columns {
        let mut sum = 0.0;
        let mut correction = 0.0;
        for chunk in 0..chunk_count {
            let slot = chunk * columns + column;
            neumaier_add(&mut sum, &mut correction, sums[slot] + corrections[slot]);
        }
        sums[column] = sum + correction;
    }
    let size = level.component_sizes[0] as f64;
    executor.install(|| {
        values.par_chunks_mut(columns).for_each(|row| {
            for column in 0..columns {
                if active[column] {
                    row[column] -= sums[column] / size;
                }
            }
        });
    });
}

fn assign_scaled_f32(
    output: &mut [f32],
    rhs: &[f32],
    inverse_diagonal: &[f32],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        output
            .par_chunks_mut(columns)
            .zip(rhs.par_chunks(columns))
            .zip(inverse_diagonal.par_iter())
            .for_each(|((output_row, rhs_row), inverse)| {
                for column in 0..columns {
                    output_row[column] = if active[column] {
                        *inverse * rhs_row[column]
                    } else {
                        0.0
                    };
                }
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn jacobi_add_f32(
    output: &mut [f32],
    rhs: &[f32],
    product: &[f32],
    inverse_diagonal: &[f32],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        output
            .par_chunks_mut(columns)
            .zip(rhs.par_chunks(columns))
            .zip(product.par_chunks(columns))
            .zip(inverse_diagonal.par_iter())
            .for_each(|(((output_row, rhs_row), product_row), inverse)| {
                for column in 0..columns {
                    if active[column] {
                        output_row[column] += *inverse * (rhs_row[column] - product_row[column]);
                    }
                }
            });
    });
}

fn residual_from_projected_rhs_f32(
    residual: &mut [f32],
    rhs: &[f32],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        residual
            .par_chunks_mut(columns)
            .zip(rhs.par_chunks(columns))
            .for_each(|(residual_row, rhs_row)| {
                for column in 0..columns {
                    if active[column] {
                        residual_row[column] = rhs_row[column] - residual_row[column];
                    }
                }
            });
    });
}

fn restrict_f32(
    labels: &[usize],
    fine: &[f32],
    coarse: &mut [f32],
    coarse_dimension: usize,
    columns: usize,
    active: &[bool],
) {
    coarse.fill(0.0);
    for (vertex, &label) in labels.iter().enumerate() {
        let fine_row = &fine[vertex * columns..(vertex + 1) * columns];
        let coarse_row = &mut coarse[label * columns..(label + 1) * columns];
        for column in 0..columns {
            if active[column] {
                coarse_row[column] += fine_row[column];
            }
        }
    }
    debug_assert_eq!(coarse.len(), coarse_dimension * columns);
}

fn prolong_add_f32(
    labels: &[usize],
    coarse: &[f32],
    fine: &mut [f32],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        fine.par_chunks_mut(columns)
            .enumerate()
            .for_each(|(vertex, fine_row)| {
                let coarse_row = &coarse[labels[vertex] * columns..(labels[vertex] + 1) * columns];
                for column in 0..columns {
                    if active[column] {
                        fine_row[column] += coarse_row[column];
                    }
                }
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn center_columns_f32(
    level: &FusedLevel,
    values: &mut [f32],
    columns: usize,
    active: &[bool],
    executor: &ParallelExecutor,
    sums: &mut Vec<f64>,
    corrections: &mut Vec<f64>,
) {
    debug_assert_eq!(level.component_sizes.len(), 1);
    let chunk_count = level.dimension.div_ceil(REDUCTION_CHUNK_VERTICES);
    let needed = chunk_count.saturating_mul(columns);
    sums.resize(needed, 0.0);
    corrections.resize(needed, 0.0);
    sums.fill(0.0);
    corrections.fill(0.0);
    executor.install(|| {
        sums.par_chunks_mut(columns)
            .zip(corrections.par_chunks_mut(columns))
            .enumerate()
            .for_each(|(chunk, (chunk_sums, chunk_corrections))| {
                let first = chunk * REDUCTION_CHUNK_VERTICES;
                let last = level.dimension.min(first + REDUCTION_CHUNK_VERTICES);
                for vertex in first..last {
                    let row = &values[vertex * columns..(vertex + 1) * columns];
                    for column in 0..columns {
                        if active[column] {
                            neumaier_add(
                                &mut chunk_sums[column],
                                &mut chunk_corrections[column],
                                f64::from(row[column]),
                            );
                        }
                    }
                }
            });
    });
    for column in 0..columns {
        let mut sum = 0.0;
        let mut correction = 0.0;
        for chunk in 0..chunk_count {
            let slot = chunk * columns + column;
            neumaier_add(&mut sum, &mut correction, sums[slot] + corrections[slot]);
        }
        sums[column] = sum + correction;
    }
    let size = level.component_sizes[0] as f64;
    executor.install(|| {
        values.par_chunks_mut(columns).for_each(|row| {
            for column in 0..columns {
                if active[column] {
                    row[column] -= (sums[column] / size) as f32;
                }
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn update_solution_residual(
    solution: &mut [f64],
    residual: &mut [f64],
    direction: &[f64],
    matrix_direction: &[f64],
    columns: usize,
    active: &[bool],
    alpha: &[f64],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        solution
            .par_chunks_mut(columns)
            .zip(residual.par_chunks_mut(columns))
            .zip(direction.par_chunks(columns))
            .zip(matrix_direction.par_chunks(columns))
            .for_each(
                |(((solution_row, residual_row), direction_row), matrix_row)| {
                    for column in 0..columns {
                        if active[column] {
                            solution_row[column] += alpha[column] * direction_row[column];
                            residual_row[column] -= alpha[column] * matrix_row[column];
                        }
                    }
                },
            );
    });
}

#[allow(clippy::too_many_arguments)]
fn update_direction(
    direction: &mut [f64],
    preconditioned: &[f64],
    columns: usize,
    active: &[bool],
    restarted: &[bool],
    beta: &[f64],
    executor: &ParallelExecutor,
) {
    executor.install(|| {
        direction
            .par_chunks_mut(columns)
            .zip(preconditioned.par_chunks(columns))
            .for_each(|(direction_row, preconditioned_row)| {
                for column in 0..columns {
                    if active[column] {
                        direction_row[column] = if restarted[column] {
                            preconditioned_row[column]
                        } else {
                            preconditioned_row[column] + beta[column] * direction_row[column]
                        };
                    }
                }
            });
    });
}

fn dot_columns(
    left: &[f64],
    right: &[f64],
    dimension: usize,
    columns: usize,
    active: &[bool],
    output: &mut [f64],
    executor: &ParallelExecutor,
    sums: &mut Vec<f64>,
    corrections: &mut Vec<f64>,
) {
    let chunk_count = dimension.div_ceil(REDUCTION_CHUNK_VERTICES);
    let needed = chunk_count.saturating_mul(columns);
    sums.resize(needed, 0.0);
    corrections.resize(needed, 0.0);
    sums.fill(0.0);
    corrections.fill(0.0);
    executor.install(|| {
        sums.par_chunks_mut(columns)
            .zip(corrections.par_chunks_mut(columns))
            .enumerate()
            .for_each(|(chunk, (chunk_sums, chunk_corrections))| {
                let first = chunk * REDUCTION_CHUNK_VERTICES;
                let last = dimension.min(first + REDUCTION_CHUNK_VERTICES);
                for vertex in first..last {
                    let begin = vertex * columns;
                    for column in 0..columns {
                        if active[column] {
                            neumaier_add(
                                &mut chunk_sums[column],
                                &mut chunk_corrections[column],
                                left[begin + column] * right[begin + column],
                            );
                        }
                    }
                }
            });
    });
    for column in 0..columns {
        let mut sum = 0.0;
        let mut correction = 0.0;
        for chunk in 0..chunk_count {
            let slot = chunk * columns + column;
            neumaier_add(&mut sum, &mut correction, sums[slot] + corrections[slot]);
        }
        output[column] = sum + correction;
    }
}

fn norm_columns(
    values: &[f64],
    dimension: usize,
    columns: usize,
    active: &[bool],
    output: &mut [f64],
    executor: &ParallelExecutor,
    sums: &mut Vec<f64>,
    corrections: &mut Vec<f64>,
) {
    let chunk_count = dimension.div_ceil(REDUCTION_CHUNK_VERTICES);
    let needed = chunk_count.saturating_mul(columns);
    sums.resize(needed, 0.0);
    corrections.resize(needed, 0.0);
    sums.fill(0.0);
    executor.install(|| {
        sums.par_chunks_mut(columns)
            .enumerate()
            .for_each(|(chunk, chunk_scales)| {
                let first = chunk * REDUCTION_CHUNK_VERTICES;
                let last = dimension.min(first + REDUCTION_CHUNK_VERTICES);
                for vertex in first..last {
                    let begin = vertex * columns;
                    for column in 0..columns {
                        if active[column] {
                            chunk_scales[column] =
                                chunk_scales[column].max(values[begin + column].abs());
                        }
                    }
                }
            });
    });
    output[..columns].fill(0.0);
    for column in 0..columns {
        output[column] = (0..chunk_count)
            .map(|chunk| sums[chunk * columns + column])
            .fold(0.0_f64, f64::max);
    }
    sums.fill(0.0);
    corrections.fill(0.0);
    executor.install(|| {
        sums.par_chunks_mut(columns)
            .zip(corrections.par_chunks_mut(columns))
            .enumerate()
            .for_each(|(chunk, (chunk_sums, chunk_corrections))| {
                let first = chunk * REDUCTION_CHUNK_VERTICES;
                let last = dimension.min(first + REDUCTION_CHUNK_VERTICES);
                for vertex in first..last {
                    let begin = vertex * columns;
                    for column in 0..columns {
                        if active[column] && output[column] > 0.0 {
                            let value = values[begin + column] / output[column];
                            neumaier_add(
                                &mut chunk_sums[column],
                                &mut chunk_corrections[column],
                                value * value,
                            );
                        }
                    }
                }
            });
    });
    for column in 0..columns {
        let scale = output[column];
        if scale > 0.0 {
            let mut sum = 0.0;
            let mut correction = 0.0;
            for chunk in 0..chunk_count {
                let slot = chunk * columns + column;
                neumaier_add(&mut sum, &mut correction, sums[slot] + corrections[slot]);
            }
            output[column] = scale * (sum + correction).sqrt();
        }
    }
}

fn validate_positive_columns(
    iteration: usize,
    values: &[f64],
    active: &[bool],
) -> Result<(), CmgError> {
    for (column, (&value, &is_active)) in values.iter().zip(active).enumerate() {
        if is_active && (!value.is_finite() || value <= 0.0) {
            return Err(CmgError::PcgBreakdown {
                iteration,
                quantity: if column == 0 {
                    "fused r^T M r"
                } else {
                    "fused column r^T M r"
                },
                value,
            });
        }
    }
    Ok(())
}

fn validate_finite(iteration: usize, quantity: &'static str, value: f64) -> Result<(), CmgError> {
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

fn allowed_residual(
    options: PcgOptions,
    rhs_norm: f64,
    operator_bound: f64,
    solution_norm: f64,
) -> f64 {
    options.absolute_tolerance
        + options.relative_tolerance * (rhs_norm + operator_bound * solution_norm)
}

fn original_residual_norm_vertex_major(
    original: &[f64],
    projected: &[f64],
    residual: &[f64],
    dimension: usize,
    columns: usize,
    column: usize,
) -> f64 {
    norm(
        (0..dimension).map(|vertex| {
            residual[vertex * columns + column] + (original[vertex] - projected[vertex])
        }),
    )
}

fn norm(values: impl Iterator<Item = f64> + Clone) -> f64 {
    let scale = values.clone().map(f64::abs).fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    scale
        * compensated_sum(values.map(|value| {
            let scaled = value / scale;
            scaled * scaled
        }))
        .sqrt()
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        neumaier_add(&mut sum, &mut correction, value);
    }
    sum + correction
}

fn neumaier_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let next = *sum + value;
    *correction += if sum.abs() >= value.abs() {
        (*sum - next) + value
    } else {
        (value - next) + *sum
    };
    *sum = next;
}

fn transpose_columns_to_vertices(
    input: &[f64],
    output: &mut [f64],
    dimension: usize,
    columns: usize,
) {
    for column in 0..columns {
        for vertex in 0..dimension {
            output[vertex * columns + column] = input[column * dimension + vertex];
        }
    }
}

fn transpose_vertices_to_columns(
    input: &[f64],
    output: &mut [f64],
    dimension: usize,
    columns: usize,
) {
    for column in 0..columns {
        for vertex in 0..dimension {
            output[column * dimension + vertex] = input[vertex * columns + column];
        }
    }
}

fn copy_column_vertex_major(
    source: &[f64],
    destination: &mut [f64],
    dimension: usize,
    columns: usize,
    column: usize,
) {
    for vertex in 0..dimension {
        destination[vertex * columns + column] = source[vertex * columns + column];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CmgOptions, ParallelOptions};

    #[test]
    fn fused_columns_match_independent_pcg_on_connected_graph() {
        let graph =
            Laplacian::from_edges(8, (0..8).map(|vertex| (vertex, (vertex + 1) % 8, 1.0))).unwrap();
        let solver = ParallelPcgSolver::build(
            &graph,
            CmgOptions::default(),
            ParallelOptions {
                threads: 2,
                ..ParallelOptions::default()
            },
        )
        .unwrap();
        let rhs = vec![
            1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let options = PcgOptions {
            relative_tolerance: 1.0e-12,
            absolute_tolerance: 0.0,
            max_iterations: 100,
            ..PcgOptions::default()
        };
        let independent = solver
            .solve_batch(&[rhs[..8].to_vec(), rhs[8..].to_vec()], options)
            .unwrap();
        let fused = VckssFusedPcgSolver::build(&solver).unwrap();
        let mut workspace = fused.workspace(4);
        let single = fused
            .solve_batch_with_workspace(&solver, &rhs[..8], 1, options, &mut workspace)
            .unwrap();
        for vertex in 0..8 {
            assert!(
                (single.solutions()[vertex] - independent.results()[0].solution()[vertex]).abs()
                    <= 1.0e-10
            );
        }
        let result = fused
            .solve_batch_with_workspace(&solver, &rhs, 2, options, &mut workspace)
            .unwrap();
        for (column, expected) in independent.results().iter().enumerate() {
            for vertex in 0..8 {
                let actual = result.solutions()[column * 8 + vertex];
                assert!((actual - expected.solution()[vertex]).abs() <= 1.0e-10);
            }
            assert!(result.reports()[column].residual_norm() <= 1.0e-10);
        }
        let mixed = VckssFusedPcgSolver::build_mixed(&solver).unwrap();
        let mut mixed_workspace = mixed.workspace(4);
        let mixed_result = mixed
            .solve_batch_with_workspace(&solver, &rhs, 2, options, &mut mixed_workspace)
            .unwrap();
        for (column, expected) in independent.results().iter().enumerate() {
            for vertex in 0..8 {
                let actual = mixed_result.solutions()[column * 8 + vertex];
                assert!((actual - expected.solution()[vertex]).abs() <= 1.0e-8);
            }
            assert!(mixed_result.reports()[column].residual_norm() <= 1.0e-8);
        }

        let iterative = ParallelPcgSolver::build(
            &graph,
            CmgOptions {
                direct_threshold: 1,
                max_levels: 1,
                ..CmgOptions::default()
            },
            ParallelOptions {
                threads: 2,
                ..ParallelOptions::default()
            },
        )
        .unwrap();
        assert!(
            iterative.preconditioner().hierarchy().levels()[0]
                .terminal_reason()
                .is_some_and(TerminalReason::is_iterative)
        );
        let expected = iterative.solve(&rhs[..8], options).unwrap();
        let fused = VckssFusedPcgSolver::build(&iterative).unwrap();
        let mut workspace = fused.workspace(4);
        let actual = fused
            .solve_batch_with_workspace(&iterative, &rhs[..8], 1, options, &mut workspace)
            .unwrap();
        for vertex in 0..8 {
            assert!((actual.solutions()[vertex] - expected.solution()[vertex]).abs() <= 1.0e-10);
        }
        let mixed = VckssFusedPcgSolver::build_mixed(&iterative).unwrap();
        let mut mixed_workspace = mixed.workspace(4);
        let mixed_actual = mixed
            .solve_batch_with_workspace(&iterative, &rhs[..8], 1, options, &mut mixed_workspace)
            .unwrap();
        for vertex in 0..8 {
            assert!(
                (mixed_actual.solutions()[vertex] - expected.solution()[vertex]).abs() <= 1.0e-8
            );
        }
    }
}
