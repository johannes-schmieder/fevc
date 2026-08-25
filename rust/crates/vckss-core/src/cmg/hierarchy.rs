// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic CMG hierarchy, symmetric V-cycle, and full zero-sum quotient
//! preconditioner for the worker-eliminated two-way fixed-effect system.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use super::{HybridGraph, VertexKey, WeightedEdge};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt, INTERRUPT_CHECK_CHUNK};
use crate::krylov::Preconditioner;
use crate::operator::SymmetricOperator;
use crate::problem::CompressedProblem;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AggregationMethod {
    NormalizedHeavyEdge,
    CanonicalPacking,
}

#[derive(Clone, Copy, Debug)]
pub struct CmgOptions {
    pub terminal_vertices: usize,
    pub dense_vertex_cap: usize,
    pub maximum_levels: usize,
    pub aggregate_cap: usize,
    pub minimum_reduction: f64,
    pub jacobi_weight: f64,
    pub pre_sweeps: u32,
    pub post_sweeps: u32,
    pub maximum_edge_complexity: f64,
    pub maximum_vertex_complexity: f64,
    pub memory_limit_bytes: u64,
}

impl Default for CmgOptions {
    fn default() -> Self {
        Self {
            terminal_vertices: 128,
            dense_vertex_cap: 6_144,
            maximum_levels: 96,
            aggregate_cap: 8,
            minimum_reduction: 0.20,
            jacobi_weight: 2.0 / 3.0,
            pre_sweeps: 1,
            post_sweeps: 1,
            maximum_edge_complexity: 12.0,
            maximum_vertex_complexity: 5.0,
            memory_limit_bytes: 2_u64 << 30,
        }
    }
}

impl CmgOptions {
    pub fn validate(self) -> Result<Self> {
        if self.terminal_vertices == 0 || self.terminal_vertices > self.dense_vertex_cap {
            return Err(cmg_setup_error(
                "terminal vertex target must be positive and not exceed the dense cap",
            ));
        }
        if self.dense_vertex_cap > 6_144 {
            return Err(cmg_setup_error(
                "dense terminal cap exceeds the registered 6,144-vertex limit",
            ));
        }
        if self.maximum_levels < 2 || self.aggregate_cap < 2 {
            return Err(cmg_setup_error(
                "CMG requires at least two levels and aggregate capacity at least two",
            ));
        }
        if !self.minimum_reduction.is_finite() || !(0.0..1.0).contains(&self.minimum_reduction) {
            return Err(cmg_setup_error(
                "minimum hierarchy reduction must lie in [0, 1)",
            ));
        }
        if !self.jacobi_weight.is_finite() || self.jacobi_weight <= 0.0 || self.jacobi_weight >= 1.0
        {
            return Err(cmg_setup_error(
                "Jacobi weight must be finite and lie in (0, 1)",
            ));
        }
        if self.pre_sweeps == 0 || self.post_sweeps == 0 {
            return Err(cmg_setup_error(
                "symmetric V-cycle requires positive pre- and post-sweep counts",
            ));
        }
        if self.pre_sweeps != self.post_sweeps {
            return Err(cmg_setup_error(
                "ordinary PCG requires equal pre- and post-sweep counts for a symmetric V-cycle",
            ));
        }
        if !self.maximum_edge_complexity.is_finite()
            || self.maximum_edge_complexity < 1.0
            || !self.maximum_vertex_complexity.is_finite()
            || self.maximum_vertex_complexity < 1.0
        {
            return Err(cmg_setup_error(
                "hierarchy complexity caps must be finite and at least one",
            ));
        }
        if self.memory_limit_bytes == 0 {
            return Err(cmg_setup_error("CMG memory limit must be positive"));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct CmgLevelReceipt {
    pub level: usize,
    pub vertices: usize,
    pub edges: usize,
    pub coarse_vertices: Option<usize>,
    pub reduction: Option<f64>,
    pub method: Option<AggregationMethod>,
}

#[derive(Clone, Debug)]
pub struct CmgReceipt {
    pub levels: usize,
    pub fine_vertices: usize,
    pub fine_edges: usize,
    pub terminal_vertices: usize,
    pub edge_complexity: f64,
    pub vertex_complexity: f64,
    pub structural_bytes: u64,
    pub workspace_bytes: u64,
    /// Firm-plus-auxiliary full RHS and solution vectors retained by the
    /// quotient preconditioner around the hierarchy workspace.
    pub preconditioner_bytes: u64,
    pub dense_factor_bytes: u64,
    pub level: Vec<CmgLevelReceipt>,
}

impl CmgReceipt {
    pub fn batch_workspace_bytes(&self, columns: usize) -> Result<u64> {
        if columns == 0 {
            return Err(resource_error("CMG batch workspace width must be positive"));
        }
        let columns = to_u64(columns, "CMG batch workspace width")?;
        let level_sums = to_u64(self.levels, "CMG hierarchy levels")?
            .checked_mul(columns)
            .and_then(|value| value.checked_mul(8))
            .ok_or_else(|| resource_error("CMG batch column-sum byte forecast overflow"))?;
        self.workspace_bytes
            .checked_add(self.preconditioner_bytes)
            .and_then(|value| value.checked_mul(columns))
            .and_then(|value| value.checked_add(level_sums))
            .ok_or_else(|| resource_error("CMG batch workspace byte forecast overflow"))
    }
}

#[derive(Clone, Debug)]
struct LaplacianGraph {
    key: Vec<VertexKey>,
    edge: Vec<WeightedEdge>,
    degree: Vec<f64>,
}

impl LaplacianGraph {
    fn from_hybrid_with_interrupt(
        graph: &HybridGraph,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("cmg_laplacian_copy")?;
        Self::from_parts_with_interrupt(
            graph.vertex_keys().to_vec(),
            graph.edges().to_vec(),
            interrupt,
        )
    }

    fn from_parts_with_interrupt(
        key: Vec<VertexKey>,
        mut edge: Vec<WeightedEdge>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if key.is_empty() || key.len() > u32::MAX as usize {
            return Err(cmg_setup_error(
                "CMG graph vertex count is empty or exceeds the u32 limit",
            ));
        }
        for (index, item) in edge.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "cmg_laplacian_validate")?;
            let left = usize::try_from(item.u).expect("validated u32 endpoint");
            let right = usize::try_from(item.v).expect("validated u32 endpoint");
            if left >= right || right >= key.len() || !item.weight.is_finite() || item.weight <= 0.0
            {
                return Err(cmg_setup_error("CMG graph contains an invalid edge"));
            }
        }
        // Rust's sort implementation cannot invoke a fallible callback; poll
        // immediately around this deterministic ordering phase.
        interrupt.checkpoint("cmg_laplacian_sort")?;
        edge.sort_unstable_by(|left, right| {
            edge_key_from_keys(&key, *left)
                .cmp(&edge_key_from_keys(&key, *right))
                .then_with(|| right.weight.total_cmp(&left.weight))
        });
        interrupt.checkpoint("cmg_laplacian_sort_complete")?;
        let mut degree = vec![0.0_f64; key.len()];
        for (index, item) in edge.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "cmg_laplacian_degree")?;
            let left = usize::try_from(item.u).expect("validated u32 endpoint");
            let right = usize::try_from(item.v).expect("validated u32 endpoint");
            degree[left] += item.weight;
            degree[right] += item.weight;
        }
        if key.len() > 1 {
            if edge.is_empty() {
                return Err(cmg_setup_error(
                    "CMG graph is disconnected or has a nonpositive degree",
                ));
            }
            for (vertex, &value) in degree.iter().enumerate() {
                checkpoint_chunk(interrupt, vertex, "cmg_laplacian_degree_validate")?;
                if !value.is_finite() || value <= 0.0 {
                    return Err(cmg_setup_error(
                        "CMG graph is disconnected or has a nonpositive degree",
                    ));
                }
            }
        }
        let graph = Self { key, edge, degree };
        if !graph.is_connected_with_interrupt(interrupt)? {
            return Err(cmg_setup_error(
                "CMG hierarchy currently requires one connected component",
            ));
        }
        Ok(graph)
    }

    fn vertices(&self) -> usize {
        self.key.len()
    }

    fn edges(&self) -> usize {
        self.edge.len()
    }

    fn apply_with_interrupt(
        &self,
        input: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if input.len() != self.vertices() || output.len() != self.vertices() {
            return Err(BackendError::invalid(
                "cmg_apply",
                "Laplacian action has incompatible dimensions",
            ));
        }
        output.fill(0.0);
        for chunk in self.edge.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_laplacian")?;
            for item in chunk {
                let left = usize::try_from(item.u).expect("validated endpoint");
                let right = usize::try_from(item.v).expect("validated endpoint");
                let value = item.weight * (input[left] - input[right]);
                output[left] += value;
                output[right] -= value;
            }
        }
        for chunk in output.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_laplacian_validate")?;
            for value in chunk {
                if !value.is_finite() {
                    return Err(BackendError::new(
                        ErrorCode::CmgApplyFailed,
                        "cmg_apply",
                        "Laplacian action produced a nonfinite value",
                    ));
                }
            }
        }
        Ok(())
    }

    fn apply_batch_with_interrupt(
        &self,
        input: &[f64],
        output: &mut [f64],
        columns: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let required = checked_batch_len(self.vertices(), columns)?;
        if columns == 0 || input.len() != required || output.len() != required {
            return Err(BackendError::invalid(
                "cmg_apply",
                "batched Laplacian action has incompatible dimensions",
            ));
        }
        output.fill(0.0);
        let flattened_edges = checked_batch_len(self.edges(), columns)?;
        let mut chunk_begin = 0_usize;
        while chunk_begin < flattened_edges {
            interrupt.checkpoint("cmg_laplacian_batch")?;
            let chunk_end = chunk_begin
                .saturating_add(INTERRUPT_CHECK_CHUNK)
                .min(flattened_edges);
            let mut cursor = chunk_begin;
            while cursor < chunk_end {
                let edge_index = cursor / columns;
                let first_column = cursor - edge_index * columns;
                let last_column = columns.min(first_column + chunk_end - cursor);
                let item = self.edge[edge_index];
                let left = usize::try_from(item.u).expect("validated endpoint");
                let right = usize::try_from(item.v).expect("validated endpoint");
                for column in first_column..last_column {
                    let left_index = left * columns + column;
                    let right_index = right * columns + column;
                    let value = item.weight * (input[left_index] - input[right_index]);
                    output[left_index] += value;
                    output[right_index] -= value;
                }
                cursor += last_column - first_column;
            }
            chunk_begin = chunk_end;
        }
        for chunk in output.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_laplacian_batch_validate")?;
            if chunk.iter().any(|value| !value.is_finite()) {
                return Err(BackendError::new(
                    ErrorCode::CmgApplyFailed,
                    "cmg_apply",
                    "batched Laplacian action produced a nonfinite value",
                ));
            }
        }
        Ok(())
    }

    fn is_connected_with_interrupt(&self, interrupt: &mut dyn InterruptCheck) -> Result<bool> {
        if self.vertices() <= 1 {
            return Ok(true);
        }
        let mut adjacency = vec![Vec::<u32>::new(); self.vertices()];
        for (index, item) in self.edge.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "cmg_connected_edges")?;
            let left = usize::try_from(item.u).expect("endpoint");
            let right = usize::try_from(item.v).expect("endpoint");
            adjacency[left].push(item.v);
            adjacency[right].push(item.u);
        }
        let mut seen = vec![false; self.vertices()];
        let mut queue = VecDeque::from([0_usize]);
        seen[0] = true;
        let mut visits = 0_usize;
        while let Some(vertex) = queue.pop_front() {
            for &neighbor in &adjacency[vertex] {
                checkpoint_chunk(interrupt, visits, "cmg_connected_bfs")?;
                visits = visits.saturating_add(1);
                let neighbor = usize::try_from(neighbor).expect("neighbor");
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        for (vertex, value) in seen.into_iter().enumerate() {
            checkpoint_chunk(interrupt, vertex, "cmg_connected_validate")?;
            if !value {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn structural_bytes(&self) -> Result<u64> {
        let vertices = to_u64(self.vertices(), "hierarchy vertices")?;
        let edges = to_u64(self.edges(), "hierarchy edges")?;
        vertices
            .checked_mul(32)
            .and_then(|value| value.checked_add(edges.checked_mul(24)?))
            .ok_or_else(|| resource_error("hierarchy structural byte forecast overflow"))
    }
}

#[derive(Clone, Debug)]
struct Aggregation {
    assignment: Vec<u32>,
    coarse_vertices: usize,
    method: AggregationMethod,
}

#[derive(Clone, Debug)]
struct CmgLevel {
    graph: LaplacianGraph,
    aggregation: Option<Aggregation>,
}

#[derive(Clone, Debug)]
struct DenseGroundedSolver {
    vertices: usize,
    lower: Vec<f64>,
    factor_bytes: u64,
}

impl DenseGroundedSolver {
    fn factor_with_interrupt(
        graph: &LaplacianGraph,
        memory_limit: u64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let vertices = graph.vertices();
        if vertices == 1 {
            return Ok(Self {
                vertices,
                lower: Vec::new(),
                factor_bytes: 0,
            });
        }
        let reduced = vertices - 1;
        let entries = reduced
            .checked_mul(reduced)
            .ok_or_else(|| resource_error("dense terminal entry count overflow"))?;
        let factor_bytes = to_u64(entries, "dense terminal entries")?
            .checked_mul(8)
            .ok_or_else(|| resource_error("dense terminal byte forecast overflow"))?;
        let setup_bytes = factor_bytes
            .checked_mul(2)
            .ok_or_else(|| resource_error("dense terminal setup byte forecast overflow"))?;
        if setup_bytes > memory_limit {
            return Err(resource_error(
                "dense terminal setup exceeds the admitted CMG memory limit",
            ));
        }
        let mut matrix = vec![0.0_f64; entries];
        for (index, item) in graph.edge.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "cmg_terminal_assemble")?;
            let left = usize::try_from(item.u).expect("endpoint");
            let right = usize::try_from(item.v).expect("endpoint");
            if left < reduced {
                matrix[left * reduced + left] += item.weight;
            }
            if right < reduced {
                matrix[right * reduced + right] += item.weight;
            }
            if left < reduced && right < reduced {
                matrix[left * reduced + right] -= item.weight;
                matrix[right * reduced + left] -= item.weight;
            }
        }
        let mut maximum_diagonal = 0.0_f64;
        for index in 0..reduced {
            checkpoint_chunk(interrupt, index, "cmg_terminal_diagonal")?;
            maximum_diagonal = maximum_diagonal.max(matrix[index * reduced + index]);
        }
        let pivot_floor = maximum_diagonal * 1.0e-14;
        let mut lower = vec![0.0_f64; entries];
        for row in 0..reduced {
            interrupt.checkpoint("cmg_terminal_factor")?;
            for column in 0..=row {
                let mut value = matrix[row * reduced + column];
                for inner in 0..column {
                    checkpoint_chunk(interrupt, inner, "cmg_terminal_factor")?;
                    value -= lower[row * reduced + inner] * lower[column * reduced + inner];
                }
                if row == column {
                    if !value.is_finite() || value <= pivot_floor {
                        return Err(cmg_setup_error(
                            "grounded terminal Cholesky encountered a nonpositive pivot",
                        ));
                    }
                    lower[row * reduced + column] = value.sqrt();
                } else {
                    let pivot = lower[column * reduced + column];
                    let entry = value / pivot;
                    if !entry.is_finite() {
                        return Err(cmg_setup_error(
                            "grounded terminal Cholesky produced a nonfinite entry",
                        ));
                    }
                    lower[row * reduced + column] = entry;
                }
            }
        }
        Ok(Self {
            vertices,
            lower,
            factor_bytes,
        })
    }

    fn solve_with_interrupt(
        &self,
        right_hand_side: &[f64],
        solution: &mut [f64],
        intermediate: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if right_hand_side.len() != self.vertices
            || solution.len() != self.vertices
            || intermediate.len() + usize::from(self.vertices > 0) != self.vertices
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "terminal solve has incompatible dimensions",
            ));
        }
        solution.fill(0.0);
        if self.vertices == 1 {
            return Ok(());
        }
        let reduced = self.vertices - 1;
        intermediate.fill(0.0);
        for row in 0..reduced {
            interrupt.checkpoint("cmg_terminal_forward")?;
            let mut value = right_hand_side[row];
            if row > 0 {
                // The legacy loop checked at inner index zero.  Keep that
                // callback while avoiding a modulo operation for every dense
                // terminal-factor entry (the terminal cap is below one chunk).
                interrupt.checkpoint("cmg_terminal_forward")?;
            }
            for column in 0..row {
                value -= self.lower[row * reduced + column] * intermediate[column];
            }
            intermediate[row] = value / self.lower[row * reduced + row];
        }
        for row in (0..reduced).rev() {
            interrupt.checkpoint("cmg_terminal_backward")?;
            let mut value = intermediate[row];
            if row + 1 < reduced {
                // As above, preserve the inner-index-zero callback exactly.
                interrupt.checkpoint("cmg_terminal_backward")?;
            }
            for column in (row + 1)..reduced {
                value -= self.lower[column * reduced + row] * solution[column];
            }
            solution[row] = value / self.lower[row * reduced + row];
        }
        center_with_interrupt(solution, interrupt)?;
        Ok(())
    }

    fn solve_batch_with_interrupt(
        &self,
        right_hand_side: &[f64],
        solution: &mut [f64],
        intermediate: &mut [f64],
        column_sum: &mut [f64],
        columns: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let required = checked_batch_len(self.vertices, columns)?;
        let reduced = self.vertices.saturating_sub(1);
        if columns == 0
            || right_hand_side.len() != required
            || solution.len() != required
            || intermediate.len() < checked_batch_len(reduced, columns)?
            || column_sum.len() < columns
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "batched terminal solve has incompatible dimensions",
            ));
        }
        solution.fill(0.0);
        if self.vertices == 1 {
            return Ok(());
        }
        let intermediate = &mut intermediate[..reduced * columns];
        intermediate.fill(0.0);
        let mut work = 0_usize;
        let mut next_checkpoint = 0_usize;
        for row in 0..reduced {
            interrupt.checkpoint("cmg_terminal_forward_batch")?;
            let row_begin = row * columns;
            intermediate[row_begin..row_begin + columns]
                .copy_from_slice(&right_hand_side[row_begin..row_begin + columns]);
            for column_index in 0..row {
                let coefficient = self.lower[row * reduced + column_index];
                let source = column_index * columns;
                for column in 0..columns {
                    if work == next_checkpoint {
                        interrupt.checkpoint("cmg_terminal_forward_batch")?;
                        next_checkpoint = next_checkpoint
                            .checked_add(INTERRUPT_CHECK_CHUNK)
                            .unwrap_or(usize::MAX);
                    }
                    intermediate[row_begin + column] -= coefficient * intermediate[source + column];
                    work += 1;
                }
            }
            let diagonal = self.lower[row * reduced + row];
            for value in &mut intermediate[row_begin..row_begin + columns] {
                *value /= diagonal;
            }
        }
        work = 0;
        next_checkpoint = 0;
        for row in (0..reduced).rev() {
            interrupt.checkpoint("cmg_terminal_backward_batch")?;
            let row_begin = row * columns;
            solution[row_begin..row_begin + columns]
                .copy_from_slice(&intermediate[row_begin..row_begin + columns]);
            for column_index in (row + 1)..reduced {
                let coefficient = self.lower[column_index * reduced + row];
                let source = column_index * columns;
                for column in 0..columns {
                    if work == next_checkpoint {
                        interrupt.checkpoint("cmg_terminal_backward_batch")?;
                        next_checkpoint = next_checkpoint
                            .checked_add(INTERRUPT_CHECK_CHUNK)
                            .unwrap_or(usize::MAX);
                    }
                    solution[row_begin + column] -= coefficient * solution[source + column];
                    work += 1;
                }
            }
            let diagonal = self.lower[row * reduced + row];
            for value in &mut solution[row_begin..row_begin + columns] {
                *value /= diagonal;
            }
        }
        center_batch_with_interrupt(solution, self.vertices, columns, column_sum, interrupt)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct LevelWorkspace {
    rhs: Vec<f64>,
    solution: Vec<f64>,
    action: Vec<f64>,
    residual: Vec<f64>,
    coarse_rhs: Vec<f64>,
    coarse_solution: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct CmgWorkspace {
    level: Vec<LevelWorkspace>,
}

impl CmgWorkspace {
    fn new(hierarchy: &CmgHierarchy) -> Self {
        let level = hierarchy
            .level
            .iter()
            .map(|item| {
                let vertices = item.graph.vertices();
                let coarse = item
                    .aggregation
                    .as_ref()
                    .map_or(0, |aggregation| aggregation.coarse_vertices);
                LevelWorkspace {
                    rhs: vec![0.0; vertices],
                    solution: vec![0.0; vertices],
                    action: vec![0.0; vertices],
                    residual: vec![0.0; vertices],
                    coarse_rhs: vec![0.0; coarse],
                    coarse_solution: vec![0.0; coarse],
                }
            })
            .collect();
        Self { level }
    }
}

#[derive(Clone, Debug)]
struct BatchLevelWorkspace {
    rhs: Vec<f64>,
    solution: Vec<f64>,
    action: Vec<f64>,
    residual: Vec<f64>,
    coarse_rhs: Vec<f64>,
    coarse_solution: Vec<f64>,
    column_sum: Vec<f64>,
}

#[derive(Clone, Debug)]
struct CmgBatchWorkspace {
    capacity: usize,
    full_rhs: Vec<f64>,
    full_solution: Vec<f64>,
    level: Vec<BatchLevelWorkspace>,
}

impl CmgBatchWorkspace {
    fn new(hierarchy: &CmgHierarchy, capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(BackendError::invalid(
                "cmg_apply",
                "CMG batch capacity must be positive",
            ));
        }
        let fine = hierarchy.dimension();
        let mut level = Vec::with_capacity(hierarchy.level.len());
        for item in &hierarchy.level {
            let vertices = item.graph.vertices();
            let coarse = item
                .aggregation
                .as_ref()
                .map_or(0, |aggregation| aggregation.coarse_vertices);
            level.push(BatchLevelWorkspace {
                rhs: zeroed_batch_vector(vertices, capacity)?,
                solution: zeroed_batch_vector(vertices, capacity)?,
                action: zeroed_batch_vector(vertices, capacity)?,
                residual: zeroed_batch_vector(vertices, capacity)?,
                coarse_rhs: zeroed_batch_vector(coarse, capacity)?,
                coarse_solution: zeroed_batch_vector(coarse, capacity)?,
                column_sum: zeroed_vector(capacity)?,
            });
        }
        Ok(Self {
            capacity,
            full_rhs: zeroed_batch_vector(fine, capacity)?,
            full_solution: zeroed_batch_vector(fine, capacity)?,
            level,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CmgHierarchy {
    options: CmgOptions,
    level: Vec<CmgLevel>,
    terminal: DenseGroundedSolver,
    receipt: CmgReceipt,
}

impl CmgHierarchy {
    pub fn build(hybrid: &HybridGraph, options: CmgOptions) -> Result<Self> {
        let mut interrupt = NeverInterrupt;
        Self::build_with_interrupt(hybrid, options, &mut interrupt)
    }

    pub fn build_with_interrupt(
        hybrid: &HybridGraph,
        options: CmgOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("cmg_hierarchy_entry")?;
        let options = options.validate()?;
        let fine = LaplacianGraph::from_hybrid_with_interrupt(hybrid, interrupt)?;
        let fine_vertices = fine.vertices();
        let fine_edges = fine.edges();
        let fine_edge_denominator = fine_edges.max(1);
        let mut level = vec![CmgLevel {
            graph: fine,
            aggregation: None,
        }];
        let mut total_vertices = fine_vertices;
        let mut total_edges = fine_edges;

        loop {
            interrupt.checkpoint("cmg_hierarchy_level")?;
            let current = level.last().expect("fine level exists").graph.vertices();
            if current <= options.terminal_vertices {
                break;
            }
            if level.len() >= options.maximum_levels {
                return Err(cmg_setup_error("CMG hierarchy exceeded the level cap"));
            }
            let aggregation =
                aggregate_with_interrupt(&level.last().expect("level").graph, options, interrupt)?;
            let coarse = contract_with_interrupt(
                &level.last().expect("level").graph,
                &aggregation,
                interrupt,
            )?;
            if coarse.vertices() >= current {
                return Err(cmg_setup_error(
                    "CMG hierarchy failed to reduce vertex count",
                ));
            }
            let proposed_vertices = total_vertices
                .checked_add(coarse.vertices())
                .ok_or_else(|| resource_error("hierarchy vertex total overflow"))?;
            let proposed_edges = total_edges
                .checked_add(coarse.edges())
                .ok_or_else(|| resource_error("hierarchy edge total overflow"))?;
            let vertex_complexity = ratio(proposed_vertices, fine_vertices)?;
            let edge_complexity = ratio(proposed_edges, fine_edge_denominator)?;
            if vertex_complexity > options.maximum_vertex_complexity
                || edge_complexity > options.maximum_edge_complexity
            {
                if current <= options.dense_vertex_cap {
                    break;
                }
                return Err(cmg_setup_error(
                    "CMG hierarchy complexity cap was reached above the dense terminal cap",
                ));
            }
            level.last_mut().expect("level").aggregation = Some(aggregation);
            level.push(CmgLevel {
                graph: coarse,
                aggregation: None,
            });
            total_vertices = proposed_vertices;
            total_edges = proposed_edges;
        }

        let terminal_vertices = level.last().expect("terminal level").graph.vertices();
        if terminal_vertices > options.dense_vertex_cap {
            return Err(cmg_setup_error(
                "CMG terminal graph exceeds the dense terminal cap",
            ));
        }
        let structural_bytes = hierarchy_bytes(&level)?;
        let workspace_bytes = workspace_bytes(&level)?;
        let reserved = structural_bytes
            .checked_add(workspace_bytes)
            .ok_or_else(|| resource_error("CMG memory receipt overflow"))?;
        if reserved > options.memory_limit_bytes {
            return Err(resource_error(
                "CMG hierarchy and workspace exceed the admitted memory limit",
            ));
        }
        let terminal = DenseGroundedSolver::factor_with_interrupt(
            &level.last().expect("terminal").graph,
            options.memory_limit_bytes - reserved,
            interrupt,
        )?;
        let total_bytes = reserved
            .checked_add(terminal.factor_bytes)
            .ok_or_else(|| resource_error("CMG total byte receipt overflow"))?;
        if total_bytes > options.memory_limit_bytes {
            return Err(resource_error(
                "CMG terminal factor exceeds the admitted memory limit",
            ));
        }

        let level_receipt = level
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let coarse_vertices = item
                    .aggregation
                    .as_ref()
                    .map(|aggregation| aggregation.coarse_vertices);
                let reduction = coarse_vertices.map(|coarse| {
                    1.0 - usize_to_f64(coarse).expect("u32 graph size")
                        / usize_to_f64(item.graph.vertices()).expect("u32 graph size")
                });
                CmgLevelReceipt {
                    level: index,
                    vertices: item.graph.vertices(),
                    edges: item.graph.edges(),
                    coarse_vertices,
                    reduction,
                    method: item
                        .aggregation
                        .as_ref()
                        .map(|aggregation| aggregation.method),
                }
            })
            .collect();
        let receipt = CmgReceipt {
            levels: level.len(),
            fine_vertices,
            fine_edges,
            terminal_vertices,
            edge_complexity: ratio(total_edges, fine_edge_denominator)?,
            vertex_complexity: ratio(total_vertices, fine_vertices)?,
            structural_bytes,
            workspace_bytes,
            preconditioner_bytes: 0,
            dense_factor_bytes: terminal.factor_bytes,
            level: level_receipt,
        };
        Ok(Self {
            options,
            level,
            terminal,
            receipt,
        })
    }

    #[must_use]
    pub const fn receipt(&self) -> &CmgReceipt {
        &self.receipt
    }

    #[must_use]
    pub fn dimension(&self) -> usize {
        self.level[0].graph.vertices()
    }

    #[must_use]
    pub fn workspace(&self) -> CmgWorkspace {
        CmgWorkspace::new(self)
    }

    pub fn apply(
        &self,
        right_hand_side: &[f64],
        solution: &mut [f64],
        workspace: &mut CmgWorkspace,
    ) -> Result<()> {
        let mut interrupt = NeverInterrupt;
        self.apply_with_interrupt(right_hand_side, solution, workspace, &mut interrupt)
    }

    pub fn apply_with_interrupt(
        &self,
        right_hand_side: &[f64],
        solution: &mut [f64],
        workspace: &mut CmgWorkspace,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if right_hand_side.len() != self.dimension()
            || solution.len() != self.dimension()
            || workspace.level.len() != self.level.len()
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "CMG application has incompatible dimensions or workspace",
            ));
        }
        for chunk in right_hand_side.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_rhs_validate")?;
            for value in chunk {
                if !value.is_finite() {
                    return Err(BackendError::invalid(
                        "cmg_apply",
                        "CMG right-hand side is nonfinite",
                    ));
                }
            }
        }
        self.cycle(
            0,
            right_hand_side,
            solution,
            &mut workspace.level,
            interrupt,
        )?;
        for chunk in solution.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_solution_validate")?;
            for value in chunk {
                if !value.is_finite() {
                    return Err(BackendError::new(
                        ErrorCode::CmgApplyFailed,
                        "cmg_apply",
                        "CMG V-cycle produced a nonfinite value",
                    ));
                }
            }
        }
        Ok(())
    }

    fn apply_batch_with_interrupt(
        &self,
        right_hand_side: &[f64],
        solution: &mut [f64],
        columns: usize,
        workspace: &mut [BatchLevelWorkspace],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let required = checked_batch_len(self.dimension(), columns)?;
        if columns == 0
            || right_hand_side.len() != required
            || solution.len() != required
            || workspace.len() != self.level.len()
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "batched CMG application has incompatible dimensions or workspace",
            ));
        }
        for chunk in right_hand_side.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_rhs_batch_validate")?;
            if chunk.iter().any(|value| !value.is_finite()) {
                return Err(BackendError::invalid(
                    "cmg_apply",
                    "batched CMG right-hand side is nonfinite",
                ));
            }
        }
        self.cycle_batch(0, right_hand_side, solution, columns, workspace, interrupt)?;
        for chunk in solution.chunks(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_solution_batch_validate")?;
            if chunk.iter().any(|value| !value.is_finite()) {
                return Err(BackendError::new(
                    ErrorCode::CmgApplyFailed,
                    "cmg_apply",
                    "batched CMG V-cycle produced a nonfinite value",
                ));
            }
        }
        Ok(())
    }

    fn cycle(
        &self,
        level_index: usize,
        input: &[f64],
        output: &mut [f64],
        workspace: &mut [LevelWorkspace],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("cmg_cycle_level")?;
        let (current, coarser_workspace) = workspace
            .split_first_mut()
            .ok_or_else(|| BackendError::invariant("cmg_apply", "missing CMG workspace level"))?;
        current.rhs.copy_from_slice(input);
        center_with_interrupt(&mut current.rhs, interrupt)?;
        current.solution.fill(0.0);

        if level_index + 1 == self.level.len() {
            self.terminal.solve_with_interrupt(
                &current.rhs,
                &mut current.solution,
                &mut current.action[..current.rhs.len() - 1],
                interrupt,
            )?;
            output.copy_from_slice(&current.solution);
            return Ok(());
        }

        let graph = &self.level[level_index].graph;
        let aggregation = self.level[level_index]
            .aggregation
            .as_ref()
            .ok_or_else(|| BackendError::invariant("cmg_apply", "missing aggregation map"))?;
        smooth_with_interrupt(
            graph,
            &current.rhs,
            &mut current.solution,
            &mut current.action,
            self.options.jacobi_weight,
            self.options.pre_sweeps,
            interrupt,
        )?;
        graph.apply_with_interrupt(&current.solution, &mut current.action, interrupt)?;
        for ((residual, &rhs), &action) in current
            .residual
            .iter_mut()
            .zip(&current.rhs)
            .zip(&current.action)
        {
            *residual = rhs - action;
        }
        current.coarse_rhs.fill(0.0);
        for begin in (0..current.residual.len()).step_by(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_restrict")?;
            let end = begin
                .saturating_add(INTERRUPT_CHECK_CHUNK)
                .min(current.residual.len());
            for vertex in begin..end {
                let aggregate = usize::try_from(aggregation.assignment[vertex])
                    .expect("validated aggregate index");
                current.coarse_rhs[aggregate] += current.residual[vertex];
            }
        }
        center_with_interrupt(&mut current.coarse_rhs, interrupt)?;
        self.cycle(
            level_index + 1,
            &current.coarse_rhs,
            &mut current.coarse_solution,
            coarser_workspace,
            interrupt,
        )?;
        for begin in (0..current.solution.len()).step_by(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_prolong")?;
            let end = begin
                .saturating_add(INTERRUPT_CHECK_CHUNK)
                .min(current.solution.len());
            for vertex in begin..end {
                let aggregate = usize::try_from(aggregation.assignment[vertex])
                    .expect("validated aggregate index");
                current.solution[vertex] += current.coarse_solution[aggregate];
            }
        }
        smooth_with_interrupt(
            graph,
            &current.rhs,
            &mut current.solution,
            &mut current.action,
            self.options.jacobi_weight,
            self.options.post_sweeps,
            interrupt,
        )?;
        center_with_interrupt(&mut current.solution, interrupt)?;
        output.copy_from_slice(&current.solution);
        Ok(())
    }

    fn cycle_batch(
        &self,
        level_index: usize,
        input: &[f64],
        output: &mut [f64],
        columns: usize,
        workspace: &mut [BatchLevelWorkspace],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("cmg_cycle_batch_level")?;
        let graph = &self.level[level_index].graph;
        let vertices = graph.vertices();
        let required = checked_batch_len(vertices, columns)?;
        if input.len() != required || output.len() != required {
            return Err(BackendError::invalid(
                "cmg_apply",
                "batched CMG cycle has incompatible dimensions",
            ));
        }
        let (current, coarser_workspace) = workspace
            .split_first_mut()
            .ok_or_else(|| BackendError::invariant("cmg_apply", "missing CMG batch level"))?;
        let rhs = &mut current.rhs[..required];
        let level_solution = &mut current.solution[..required];
        let action = &mut current.action[..required];
        let residual = &mut current.residual[..required];
        let column_sum = &mut current.column_sum[..columns];
        rhs.copy_from_slice(input);
        center_batch_with_interrupt(rhs, vertices, columns, column_sum, interrupt)?;
        level_solution.fill(0.0);

        if level_index + 1 == self.level.len() {
            self.terminal.solve_batch_with_interrupt(
                rhs,
                level_solution,
                action,
                column_sum,
                columns,
                interrupt,
            )?;
            output.copy_from_slice(level_solution);
            return Ok(());
        }

        let aggregation = self.level[level_index]
            .aggregation
            .as_ref()
            .ok_or_else(|| BackendError::invariant("cmg_apply", "missing aggregation map"))?;
        smooth_batch_with_interrupt(
            graph,
            rhs,
            level_solution,
            action,
            column_sum,
            columns,
            self.options.jacobi_weight,
            self.options.pre_sweeps,
            interrupt,
        )?;
        graph.apply_batch_with_interrupt(level_solution, action, columns, interrupt)?;
        for begin in (0..required).step_by(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_residual_batch")?;
            let end = begin.saturating_add(INTERRUPT_CHECK_CHUNK).min(required);
            for index in begin..end {
                residual[index] = rhs[index] - action[index];
            }
        }
        let coarse_vertices = aggregation.coarse_vertices;
        let coarse_required = checked_batch_len(coarse_vertices, columns)?;
        let coarse_rhs = &mut current.coarse_rhs[..coarse_required];
        let coarse_solution = &mut current.coarse_solution[..coarse_required];
        coarse_rhs.fill(0.0);
        let mut work = 0_usize;
        let mut next_checkpoint = 0_usize;
        for vertex in 0..vertices {
            let aggregate =
                usize::try_from(aggregation.assignment[vertex]).expect("validated aggregate index");
            for column in 0..columns {
                if work == next_checkpoint {
                    interrupt.checkpoint("cmg_restrict_batch")?;
                    next_checkpoint = next_checkpoint
                        .checked_add(INTERRUPT_CHECK_CHUNK)
                        .unwrap_or(usize::MAX);
                }
                coarse_rhs[aggregate * columns + column] += residual[vertex * columns + column];
                work += 1;
            }
        }
        center_batch_with_interrupt(coarse_rhs, coarse_vertices, columns, column_sum, interrupt)?;
        self.cycle_batch(
            level_index + 1,
            coarse_rhs,
            coarse_solution,
            columns,
            coarser_workspace,
            interrupt,
        )?;
        work = 0;
        next_checkpoint = 0;
        for vertex in 0..vertices {
            let aggregate =
                usize::try_from(aggregation.assignment[vertex]).expect("validated aggregate index");
            for column in 0..columns {
                if work == next_checkpoint {
                    interrupt.checkpoint("cmg_prolong_batch")?;
                    next_checkpoint = next_checkpoint
                        .checked_add(INTERRUPT_CHECK_CHUNK)
                        .unwrap_or(usize::MAX);
                }
                level_solution[vertex * columns + column] +=
                    coarse_solution[aggregate * columns + column];
                work += 1;
            }
        }
        smooth_batch_with_interrupt(
            graph,
            rhs,
            level_solution,
            action,
            column_sum,
            columns,
            self.options.jacobi_weight,
            self.options.post_sweeps,
            interrupt,
        )?;
        center_batch_with_interrupt(level_solution, vertices, columns, column_sum, interrupt)?;
        output.copy_from_slice(level_solution);
        Ok(())
    }
}

#[derive(Debug)]
struct PreconditionerWorkspace {
    hierarchy: CmgWorkspace,
    full_rhs: Vec<f64>,
    full_solution: Vec<f64>,
    batch: Option<CmgBatchWorkspace>,
}

#[derive(Debug)]
pub struct CmgPreconditioner {
    firms: usize,
    hierarchy: CmgHierarchy,
    workspace: Mutex<PreconditionerWorkspace>,
    receipt: CmgReceipt,
}

impl CmgPreconditioner {
    pub fn new(problem: &CompressedProblem, options: CmgOptions) -> Result<Self> {
        let mut interrupt = NeverInterrupt;
        Self::new_with_interrupt(problem, options, &mut interrupt)
    }

    pub fn new_with_interrupt(
        problem: &CompressedProblem,
        options: CmgOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("cmg_graph_build")?;
        let hybrid = HybridGraph::from_problem_with_interrupt(problem, interrupt)?;
        let firms = hybrid.firms();
        interrupt.checkpoint("cmg_hierarchy_build")?;
        let hierarchy = CmgHierarchy::build_with_interrupt(&hybrid, options, interrupt)?;
        drop(hybrid);
        let vertices = hierarchy.dimension();
        let preconditioner_bytes = to_u64(vertices, "preconditioner vertices")?
            .checked_mul(2)
            .and_then(|value| value.checked_mul(8))
            .ok_or_else(|| resource_error("preconditioner workspace byte forecast overflow"))?;
        let mut receipt = hierarchy.receipt().clone();
        receipt.preconditioner_bytes = preconditioner_bytes;
        let total_bytes = receipt
            .structural_bytes
            .checked_add(receipt.workspace_bytes)
            .and_then(|value| value.checked_add(receipt.preconditioner_bytes))
            .and_then(|value| value.checked_add(receipt.dense_factor_bytes))
            .ok_or_else(|| resource_error("preconditioner total byte receipt overflow"))?;
        if total_bytes > options.memory_limit_bytes {
            return Err(resource_error(
                "CMG preconditioner exceeds the admitted memory limit",
            ));
        }
        let workspace = PreconditionerWorkspace {
            hierarchy: hierarchy.workspace(),
            full_rhs: vec![0.0; vertices],
            full_solution: vec![0.0; vertices],
            batch: None,
        };
        Ok(Self {
            firms,
            hierarchy,
            workspace: Mutex::new(workspace),
            receipt,
        })
    }

    #[must_use]
    pub const fn receipt(&self) -> &CmgReceipt {
        &self.receipt
    }
}

impl Preconditioner for CmgPreconditioner {
    fn dimension(&self) -> usize {
        self.firms
    }

    fn apply(&self, residual: &[f64], output: &mut [f64]) -> Result<()> {
        let mut interrupt = NeverInterrupt;
        self.apply_with_interrupt(residual, output, &mut interrupt)
    }

    fn apply_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if residual.len() != self.dimension()
            || output.len() != self.dimension()
            || residual.iter().any(|value| !value.is_finite())
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "zero-sum CMG preconditioner has incompatible dimensions",
            ));
        }
        let mut state = self.workspace.lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "cmg_apply",
                "CMG workspace lock is poisoned",
            )
        })?;
        state.full_rhs.fill(0.0);
        state.full_rhs[..self.firms].copy_from_slice(residual);
        center_with_interrupt(&mut state.full_rhs[..self.firms], interrupt)?;
        let PreconditionerWorkspace {
            hierarchy,
            full_rhs,
            full_solution,
            ..
        } = &mut *state;
        self.hierarchy
            .apply_with_interrupt(full_rhs, full_solution, hierarchy, interrupt)?;
        center_with_interrupt(&mut full_solution[..self.firms], interrupt)?;
        output.copy_from_slice(&full_solution[..self.firms]);
        if output.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::CmgApplyFailed,
                "cmg_apply",
                "zero-sum CMG preconditioner produced a nonfinite value",
            ));
        }
        Ok(())
    }

    fn apply_columns_with_interrupt(
        &self,
        operator: &dyn SymmetricOperator,
        input: &[f64],
        output: &mut [f64],
        columns: usize,
        active: &[bool],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let required = checked_batch_len(self.firms, columns)?;
        if columns == 0
            || active.len() != columns
            || input.len() != required
            || output.len() != required
            || operator.dimension() != self.firms
            || input.iter().any(|value| !value.is_finite())
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "batched zero-sum CMG preconditioner has incompatible dimensions",
            ));
        }
        let active_columns = active.iter().filter(|&&value| value).count();
        if active_columns == 0 {
            return Ok(());
        }
        for _ in 0..active_columns {
            interrupt.checkpoint("batch_preconditioner")?;
        }
        let mut state = self.workspace.lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "cmg_apply",
                "CMG workspace lock is poisoned",
            )
        })?;
        if state
            .batch
            .as_ref()
            .map_or(true, |workspace| workspace.capacity != columns)
        {
            state.batch = None;
            state.batch = Some(CmgBatchWorkspace::new(&self.hierarchy, columns)?);
        }
        let batch = state.batch.as_mut().expect("batch workspace installed");
        let vertices = self.hierarchy.dimension();
        let full_required = checked_batch_len(vertices, active_columns)?;
        let full_rhs = &mut batch.full_rhs[..full_required];
        let full_solution = &mut batch.full_solution[..full_required];
        full_rhs.fill(0.0);
        full_solution.fill(0.0);
        for firm in 0..self.firms {
            let mut slot = 0_usize;
            for (column, &is_active) in active.iter().enumerate() {
                if is_active {
                    full_rhs[firm * active_columns + slot] = input[column * self.firms + firm];
                    slot += 1;
                }
            }
            debug_assert_eq!(slot, active_columns);
        }
        center_batch_with_interrupt(
            &mut full_rhs[..self.firms * active_columns],
            self.firms,
            active_columns,
            &mut batch.level[0].column_sum,
            interrupt,
        )?;
        self.hierarchy.apply_batch_with_interrupt(
            full_rhs,
            full_solution,
            active_columns,
            &mut batch.level,
            interrupt,
        )?;
        center_batch_with_interrupt(
            &mut full_solution[..self.firms * active_columns],
            self.firms,
            active_columns,
            &mut batch.level[0].column_sum,
            interrupt,
        )?;
        let mut slot = 0_usize;
        for (column, &is_active) in active.iter().enumerate() {
            if !is_active {
                continue;
            }
            let output_column = &mut output[column * self.firms..(column + 1) * self.firms];
            for firm in 0..self.firms {
                output_column[firm] = full_solution[firm * active_columns + slot];
            }
            operator.project(output_column)?;
            slot += 1;
        }
        debug_assert_eq!(slot, active_columns);
        if output.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::CmgApplyFailed,
                "cmg_apply",
                "batched zero-sum CMG preconditioner produced a nonfinite value",
            ));
        }
        Ok(())
    }
}

fn aggregate_with_interrupt(
    graph: &LaplacianGraph,
    options: CmgOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Aggregation> {
    let mut edge_order = (0..graph.edges()).collect::<Vec<_>>();
    interrupt.checkpoint("cmg_aggregate_edge_sort")?;
    edge_order.sort_unstable_by(|&left, &right| {
        let left_edge = graph.edge[left];
        let right_edge = graph.edge[right];
        let left_score = normalized_weight(graph, left_edge);
        let right_score = normalized_weight(graph, right_edge);
        right_score
            .total_cmp(&left_score)
            .then_with(|| right_edge.weight.total_cmp(&left_edge.weight))
            .then_with(|| edge_key(graph, left_edge).cmp(&edge_key(graph, right_edge)))
    });
    interrupt.checkpoint("cmg_aggregate_edge_sort_complete")?;

    let (mut groups, mut used) = structural_twin_groups_with_interrupt(graph, interrupt)?;
    for (index, edge_index) in edge_order.into_iter().enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_aggregate")?;
        let item = graph.edge[edge_index];
        let left = usize::try_from(item.u).expect("endpoint");
        let right = usize::try_from(item.v).expect("endpoint");
        if !used[left] && !used[right] {
            used[left] = true;
            used[right] = true;
            groups.push(vec![left, right]);
        }
    }
    let mut residual = Vec::new();
    for (vertex, &is_used) in used.iter().enumerate() {
        checkpoint_chunk(interrupt, vertex, "cmg_aggregate_residual")?;
        if !is_used {
            residual.push(vertex);
        }
    }
    interrupt.checkpoint("cmg_aggregate_residual_sort")?;
    residual.sort_unstable_by_key(|&vertex| graph.key[vertex]);
    interrupt.checkpoint("cmg_aggregate_residual_sort_complete")?;
    for (index, chunk) in residual.chunks(options.aggregate_cap).enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_aggregate_residual_groups")?;
        groups.push(chunk.to_vec());
    }
    let mut aggregation = finalize_groups_with_interrupt(
        graph,
        groups,
        AggregationMethod::NormalizedHeavyEdge,
        interrupt,
    )?;
    let reduction =
        1.0 - usize_to_f64(aggregation.coarse_vertices)? / usize_to_f64(graph.vertices())?;
    if reduction < options.minimum_reduction && graph.vertices() > 1 {
        aggregation = canonical_pack_with_interrupt(graph, options.aggregate_cap, interrupt)?;
    }
    Ok(aggregation)
}

fn canonical_pack_with_interrupt(
    graph: &LaplacianGraph,
    cap: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Aggregation> {
    let (mut groups, used) = structural_twin_groups_with_interrupt(graph, interrupt)?;
    let mut order = Vec::new();
    for (vertex, &is_used) in used.iter().enumerate() {
        checkpoint_chunk(interrupt, vertex, "cmg_canonical_residual")?;
        if !is_used {
            order.push(vertex);
        }
    }
    interrupt.checkpoint("cmg_canonical_sort")?;
    order.sort_unstable_by_key(|&vertex| graph.key[vertex]);
    interrupt.checkpoint("cmg_canonical_sort_complete")?;
    for (index, chunk) in order.chunks(cap).enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_canonical_groups")?;
        groups.push(chunk.to_vec());
    }
    finalize_groups_with_interrupt(
        graph,
        groups,
        AggregationMethod::CanonicalPacking,
        interrupt,
    )
}

fn structural_twin_groups_with_interrupt(
    graph: &LaplacianGraph,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<Vec<usize>>, Vec<bool>)> {
    let mut by_key = BTreeMap::<VertexKey, Vec<usize>>::new();
    for (vertex, &key) in graph.key.iter().enumerate() {
        checkpoint_chunk(interrupt, vertex, "cmg_structural_twins")?;
        by_key.entry(key).or_default().push(vertex);
    }
    let mut used = vec![false; graph.vertices()];
    let mut groups = Vec::new();
    for (index, group) in by_key
        .into_values()
        .filter(|group| group.len() > 1)
        .enumerate()
    {
        checkpoint_chunk(interrupt, index, "cmg_structural_twin_groups")?;
        // Equal fine-level firm keys mean identical weighted incidence to every
        // worker, hence a full permutation automorphism class.  No proper
        // deterministic subdivision of such a class is label-equivariant, so
        // it must be coarsened collectively.  The group can exceed
        // `aggregate_cap`: that cap bounds ordinary canonical packing, while
        // this one group still has linear storage and strictly reduces the
        // coarse dimension.
        for (index, &vertex) in group.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "cmg_structural_twin_members")?;
            used[vertex] = true;
        }
        groups.push(group);
    }
    Ok((groups, used))
}

fn finalize_groups_with_interrupt(
    graph: &LaplacianGraph,
    mut groups: Vec<Vec<usize>>,
    method: AggregationMethod,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Aggregation> {
    for (index, group) in groups.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_finalize_group_validate")?;
        if group.is_empty() {
            return Err(cmg_setup_error("CMG aggregation contains an empty group"));
        }
    }
    interrupt.checkpoint("cmg_finalize_group_sort")?;
    groups.sort_unstable_by_key(|group| {
        group
            .iter()
            .map(|&vertex| graph.key[vertex])
            .min()
            .expect("nonempty aggregate")
    });
    interrupt.checkpoint("cmg_finalize_group_sort_complete")?;
    if groups.is_empty() || (groups.len() >= graph.vertices() && graph.vertices() > 1) {
        return Err(cmg_setup_error("CMG aggregation did not reduce the graph"));
    }
    let mut assignment = vec![u32::MAX; graph.vertices()];
    for (aggregate, group) in groups.iter().enumerate() {
        checkpoint_chunk(interrupt, aggregate, "cmg_finalize_groups")?;
        let aggregate = u32::try_from(aggregate)
            .map_err(|_| resource_error("aggregate index exceeds the u32 limit"))?;
        for (index, &vertex) in group.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "cmg_finalize_group_members")?;
            if vertex >= graph.vertices() || assignment[vertex] != u32::MAX {
                return Err(cmg_setup_error(
                    "CMG aggregation is overlapping or out of range",
                ));
            }
            assignment[vertex] = aggregate;
        }
    }
    for (vertex, &aggregate) in assignment.iter().enumerate() {
        checkpoint_chunk(interrupt, vertex, "cmg_finalize_assignment")?;
        if aggregate == u32::MAX {
            return Err(cmg_setup_error("CMG aggregation left a vertex unassigned"));
        }
    }
    Ok(Aggregation {
        assignment,
        coarse_vertices: groups.len(),
        method,
    })
}

fn contract_with_interrupt(
    graph: &LaplacianGraph,
    aggregation: &Aggregation,
    interrupt: &mut dyn InterruptCheck,
) -> Result<LaplacianGraph> {
    let mut key = vec![None::<VertexKey>; aggregation.coarse_vertices];
    for (vertex, &aggregate) in aggregation.assignment.iter().enumerate() {
        checkpoint_chunk(interrupt, vertex, "cmg_contract_keys")?;
        let aggregate = usize::try_from(aggregate).expect("aggregate");
        key[aggregate] = Some(
            key[aggregate].map_or(graph.key[vertex], |current| current.min(graph.key[vertex])),
        );
    }
    let mut final_key = Vec::with_capacity(key.len());
    for (index, value) in key.into_iter().enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_contract_keys")?;
        final_key.push(value.expect("nonempty aggregate"));
    }
    let mut contribution = BTreeMap::<(u32, u32), f64>::new();
    for (index, item) in graph.edge.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_contract")?;
        let left = aggregation.assignment[usize::try_from(item.u).expect("endpoint")];
        let right = aggregation.assignment[usize::try_from(item.v).expect("endpoint")];
        if left == right {
            continue;
        }
        let pair = (left.min(right), left.max(right));
        let next = contribution.get(&pair).copied().unwrap_or(0.0) + item.weight;
        if !next.is_finite() || next <= 0.0 {
            return Err(cmg_setup_error("coarse edge weight is invalid"));
        }
        contribution.insert(pair, next);
    }
    let mut edge = Vec::with_capacity(contribution.len());
    for (index, ((u, v), weight)) in contribution.into_iter().enumerate() {
        checkpoint_chunk(interrupt, index, "cmg_contract_edges")?;
        edge.push(WeightedEdge { u, v, weight });
    }
    LaplacianGraph::from_parts_with_interrupt(final_key, edge, interrupt)
}

fn normalized_weight(graph: &LaplacianGraph, edge: WeightedEdge) -> f64 {
    let left = usize::try_from(edge.u).expect("endpoint");
    let right = usize::try_from(edge.v).expect("endpoint");
    edge.weight / (graph.degree[left] * graph.degree[right]).sqrt()
}

fn edge_key(graph: &LaplacianGraph, edge: WeightedEdge) -> (VertexKey, VertexKey) {
    edge_key_from_keys(&graph.key, edge)
}

fn edge_key_from_keys(key: &[VertexKey], edge: WeightedEdge) -> (VertexKey, VertexKey) {
    let left = key[usize::try_from(edge.u).expect("endpoint")];
    let right = key[usize::try_from(edge.v).expect("endpoint")];
    (left.min(right), left.max(right))
}

fn smooth_with_interrupt(
    graph: &LaplacianGraph,
    right_hand_side: &[f64],
    solution: &mut [f64],
    action: &mut [f64],
    weight: f64,
    sweeps: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    for _ in 0..sweeps {
        interrupt.checkpoint("cmg_smooth")?;
        graph.apply_with_interrupt(solution, action, interrupt)?;
        for begin in (0..solution.len()).step_by(INTERRUPT_CHECK_CHUNK) {
            interrupt.checkpoint("cmg_smooth")?;
            let end = begin
                .saturating_add(INTERRUPT_CHECK_CHUNK)
                .min(solution.len());
            for index in begin..end {
                solution[index] +=
                    weight * (right_hand_side[index] - action[index]) / graph.degree[index];
            }
        }
        center_with_interrupt(solution, interrupt)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn smooth_batch_with_interrupt(
    graph: &LaplacianGraph,
    right_hand_side: &[f64],
    solution: &mut [f64],
    action: &mut [f64],
    column_sum: &mut [f64],
    columns: usize,
    weight: f64,
    sweeps: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let vertices = graph.vertices();
    let required = checked_batch_len(vertices, columns)?;
    if right_hand_side.len() != required
        || solution.len() != required
        || action.len() != required
        || column_sum.len() < columns
    {
        return Err(BackendError::invalid(
            "cmg_apply",
            "batched smoother arrays have incompatible dimensions",
        ));
    }
    for _ in 0..sweeps {
        interrupt.checkpoint("cmg_smooth_batch")?;
        graph.apply_batch_with_interrupt(solution, action, columns, interrupt)?;
        let mut work = 0_usize;
        let mut next_checkpoint = 0_usize;
        for vertex in 0..vertices {
            let diagonal = graph.degree[vertex];
            let begin = vertex * columns;
            for column in 0..columns {
                if work == next_checkpoint {
                    interrupt.checkpoint("cmg_smooth_batch")?;
                    next_checkpoint = next_checkpoint
                        .checked_add(INTERRUPT_CHECK_CHUNK)
                        .unwrap_or(usize::MAX);
                }
                let index = begin + column;
                solution[index] += weight * (right_hand_side[index] - action[index]) / diagonal;
                work += 1;
            }
        }
        center_batch_with_interrupt(solution, vertices, columns, column_sum, interrupt)?;
    }
    Ok(())
}

fn center_with_interrupt(values: &mut [f64], interrupt: &mut dyn InterruptCheck) -> Result<()> {
    if values.is_empty() {
        return Err(BackendError::new(
            ErrorCode::CmgApplyFailed,
            "cmg_apply",
            "cannot center an empty or nonfinite vector",
        ));
    }
    let mut total = 0.0_f64;
    for chunk in values.chunks(INTERRUPT_CHECK_CHUNK) {
        interrupt.checkpoint("cmg_center_sum")?;
        for &value in chunk {
            if !value.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::CmgApplyFailed,
                    "cmg_apply",
                    "cannot center an empty or nonfinite vector",
                ));
            }
            total += value;
        }
    }
    let mean = total / usize_to_f64(values.len())?;
    for chunk in values.chunks_mut(INTERRUPT_CHECK_CHUNK) {
        interrupt.checkpoint("cmg_center_apply")?;
        for value in chunk {
            *value -= mean;
        }
    }
    Ok(())
}

fn center_batch_with_interrupt(
    values: &mut [f64],
    vertices: usize,
    columns: usize,
    column_sum: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let required = checked_batch_len(vertices, columns)?;
    if vertices == 0 || columns == 0 || values.len() != required || column_sum.len() < columns {
        return Err(BackendError::invalid(
            "cmg_apply",
            "cannot center an empty or incompatible batched vector",
        ));
    }
    let column_sum = &mut column_sum[..columns];
    column_sum.fill(0.0);
    let mut work = 0_usize;
    let mut next_checkpoint = 0_usize;
    for vertex in 0..vertices {
        let begin = vertex * columns;
        for column in 0..columns {
            if work == next_checkpoint {
                interrupt.checkpoint("cmg_center_batch_sum")?;
                next_checkpoint = next_checkpoint
                    .checked_add(INTERRUPT_CHECK_CHUNK)
                    .unwrap_or(usize::MAX);
            }
            let index = begin + column;
            let value = values[index];
            if !value.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::CmgApplyFailed,
                    "cmg_apply",
                    "cannot center a nonfinite batched vector",
                ));
            }
            column_sum[column] += value;
            work += 1;
        }
    }
    let count = usize_to_f64(vertices)?;
    for value in column_sum.iter_mut() {
        *value /= count;
    }
    work = 0;
    next_checkpoint = 0;
    for vertex in 0..vertices {
        let begin = vertex * columns;
        for column in 0..columns {
            if work == next_checkpoint {
                interrupt.checkpoint("cmg_center_batch_apply")?;
                next_checkpoint = next_checkpoint
                    .checked_add(INTERRUPT_CHECK_CHUNK)
                    .unwrap_or(usize::MAX);
            }
            values[begin + column] -= column_sum[column];
            work += 1;
        }
    }
    Ok(())
}

fn hierarchy_bytes(level: &[CmgLevel]) -> Result<u64> {
    level.iter().try_fold(0_u64, |total, item| {
        let graph = item.graph.structural_bytes()?;
        let assignment = item.aggregation.as_ref().map_or(Ok(0_u64), |aggregation| {
            to_u64(aggregation.assignment.len(), "aggregation assignment")?
                .checked_mul(4)
                .ok_or_else(|| resource_error("aggregation byte forecast overflow"))
        })?;
        total
            .checked_add(graph)
            .and_then(|value| value.checked_add(assignment))
            .ok_or_else(|| resource_error("hierarchy byte forecast overflow"))
    })
}

fn workspace_bytes(level: &[CmgLevel]) -> Result<u64> {
    level.iter().try_fold(0_u64, |total, item| {
        let vertices = to_u64(item.graph.vertices(), "workspace vertices")?;
        let coarse = to_u64(
            item.aggregation
                .as_ref()
                .map_or(0, |aggregation| aggregation.coarse_vertices),
            "workspace coarse vertices",
        )?;
        let entries = vertices
            .checked_mul(4)
            .and_then(|value| value.checked_add(coarse.checked_mul(2)?))
            .ok_or_else(|| resource_error("workspace entry forecast overflow"))?;
        total
            .checked_add(
                entries
                    .checked_mul(8)
                    .ok_or_else(|| resource_error("workspace byte forecast overflow"))?,
            )
            .ok_or_else(|| resource_error("workspace byte total overflow"))
    })
}

fn ratio(numerator: usize, denominator: usize) -> Result<f64> {
    Ok(usize_to_f64(numerator)? / usize_to_f64(denominator)?)
}

fn usize_to_f64(value: usize) -> Result<f64> {
    Ok(f64::from(u32::try_from(value).map_err(|_| {
        resource_error("CMG graph dimension exceeds the exact f64/u32 conversion limit")
    })?))
}

fn to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| resource_error(&format!("{label} is not representable")))
}

fn checked_batch_len(rows: usize, columns: usize) -> Result<usize> {
    rows.checked_mul(columns)
        .ok_or_else(|| resource_error("CMG batch matrix length overflow"))
}

fn zeroed_batch_vector(rows: usize, columns: usize) -> Result<Vec<f64>> {
    zeroed_vector(checked_batch_len(rows, columns)?)
}

fn zeroed_vector(length: usize) -> Result<Vec<f64>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| resource_error("could not allocate CMG batch workspace"))?;
    values.resize(length, 0.0);
    Ok(values)
}

fn cmg_setup_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::CmgSetupFailed, "cmg_setup", message)
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "cmg_setup", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jla::fitted_values;
    use crate::krylov::{pcg, PcgOptions, Preconditioner};
    use crate::operator::{stable_dot, SymmetricOperator, TwoWayOperator};
    use crate::problem::CanonicalInput;
    use crate::solver::{solve_two_way_routed, LinearSolverOptions, LinearSolverRoute};
    use crate::types::InputColumns;

    fn density_four_problem(firms: usize) -> CompressedProblem {
        let firm_label = (1..=u64::try_from(firms).expect("firms")).collect::<Vec<_>>();
        density_four_problem_with_labels(&firm_label)
    }

    fn density_four_problem_with_labels(firm_label: &[u64]) -> CompressedProblem {
        let firms = firm_label.len();
        let mut worker = Vec::with_capacity(firms * 4);
        let mut firm = Vec::with_capacity(firms * 4);
        let mut deletion = Vec::with_capacity(firms * 4);
        let mut outcome = Vec::with_capacity(firms * 4);
        let mut frequency = Vec::with_capacity(firms * 4);
        let mut target_weight = Vec::with_capacity(firms * 4);
        for worker_index in 0..firms {
            for offset in 0..4 {
                worker.push(u64::try_from(worker_index + 1).expect("worker"));
                firm.push(firm_label[(worker_index + offset) % firms]);
                deletion.push(u64::try_from(worker_index * 4 + offset + 1).expect("deletion"));
                let sign = if (worker_index + offset) % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                outcome.push(sign * f64::from(u32::try_from(offset + 1).expect("offset")));
                frequency.push(u64::try_from(offset + 1).expect("frequency"));
                target_weight.push(1.0);
            }
        }
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion,
                outcome,
                frequency,
                target_weight,
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical")
        .compress(&vec![true; rows])
        .expect("compressed")
    }

    fn complete_bipartite_problem_with_labels(
        firm_label: &[u64],
        workers: usize,
    ) -> CompressedProblem {
        let firms = firm_label.len();
        let mut worker = Vec::with_capacity(workers * firms);
        let mut firm = Vec::with_capacity(workers * firms);
        let mut deletion = Vec::with_capacity(workers * firms);
        let mut outcome = Vec::with_capacity(workers * firms);
        let mut frequency = Vec::with_capacity(workers * firms);
        let mut target_weight = Vec::with_capacity(workers * firms);
        for worker_index in 0..workers {
            for firm_index in 0..firms {
                worker.push(u64::try_from(worker_index + 1).expect("worker"));
                firm.push(firm_label[firm_index]);
                deletion
                    .push(u64::try_from(worker_index * firms + firm_index + 1).expect("deletion"));
                outcome.push(
                    f64::from(u32::try_from((7 * firm_index) % 13).expect("firm outcome")) - 6.0
                        + 0.25 * f64::from(u32::try_from(worker_index).expect("worker outcome")),
                );
                frequency.push(1);
                target_weight.push(1.0);
            }
        }
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion,
                outcome,
                frequency,
                target_weight,
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical")
        .compress(&vec![true; rows])
        .expect("compressed")
    }

    fn dense_firm_vector(firm_label: &[u64], conceptual: &[f64]) -> Vec<f64> {
        assert_eq!(firm_label.len(), conceptual.len());
        let mut sorted = firm_label.to_vec();
        sorted.sort_unstable();
        let mut dense = vec![0.0; conceptual.len()];
        for (concept, label) in firm_label.iter().enumerate() {
            let index = sorted
                .iter()
                .position(|candidate| candidate == label)
                .expect("firm label");
            dense[index] = conceptual[concept];
        }
        dense
    }

    fn conceptual_firm_vector(firm_label: &[u64], dense: &[f64]) -> Vec<f64> {
        assert_eq!(firm_label.len(), dense.len());
        let mut sorted = firm_label.to_vec();
        sorted.sort_unstable();
        firm_label
            .iter()
            .map(|label| {
                let index = sorted
                    .iter()
                    .position(|candidate| candidate == label)
                    .expect("firm label");
                dense[index]
            })
            .collect()
    }

    fn assert_vector_close(left: &[f64], right: &[f64], tolerance: f64) {
        assert_eq!(left.len(), right.len());
        for (index, (&left, &right)) in left.iter().zip(right).enumerate() {
            assert!(
                (left - right).abs() <= tolerance,
                "entry {index} differs: {left} versus {right}"
            );
        }
    }

    fn test_options() -> CmgOptions {
        CmgOptions {
            terminal_vertices: 12,
            dense_vertex_cap: 128,
            maximum_levels: 32,
            maximum_edge_complexity: 16.0,
            maximum_vertex_complexity: 6.0,
            memory_limit_bytes: 64_u64 << 20,
            ..CmgOptions::default()
        }
    }

    #[test]
    fn density_four_hierarchy_has_bounded_complexity() {
        let problem = density_four_problem(48);
        let hybrid = HybridGraph::from_problem(&problem).expect("hybrid");
        let hierarchy = CmgHierarchy::build(&hybrid, test_options()).expect("hierarchy");
        let receipt = hierarchy.receipt();
        assert!(receipt.levels >= 2);
        assert!(receipt.terminal_vertices <= 128);
        assert!(receipt.edge_complexity <= 16.0);
        assert!(receipt.vertex_complexity <= 6.0);
        assert!(receipt
            .level
            .windows(2)
            .all(|pair| pair[1].vertices < pair[0].vertices));
    }

    #[test]
    fn zero_sum_preconditioner_is_symmetric_and_positive() {
        let problem = density_four_problem(24);
        let preconditioner = CmgPreconditioner::new(&problem, test_options()).expect("CMG");
        let dimension = preconditioner.dimension();
        let left = (0..dimension)
            .map(|index| f64::from(u32::try_from(index % 7).expect("index")) - 3.0)
            .collect::<Vec<_>>();
        let right = (0..dimension)
            .map(|index| f64::from(u32::try_from(index % 5).expect("index")) - 2.0)
            .collect::<Vec<_>>();
        let mut left_image = vec![0.0; dimension];
        let mut right_image = vec![0.0; dimension];
        preconditioner
            .apply(&left, &mut left_image)
            .expect("left apply");
        preconditioner
            .apply(&right, &mut right_image)
            .expect("right apply");
        let left_curvature = stable_dot(&left, &left_image);
        let symmetry_left = stable_dot(&left, &right_image);
        let symmetry_right = stable_dot(&right, &left_image);
        assert!(left_curvature.is_finite() && left_curvature > 0.0);
        let scale = symmetry_left.abs().max(symmetry_right.abs()).max(1.0);
        assert!((symmetry_left - symmetry_right).abs() <= 1.0e-10 * scale);
    }

    #[test]
    fn batched_preconditioner_matches_scalar_columns_bitwise() {
        let problem = density_four_problem(24);
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner = CmgPreconditioner::new(&problem, test_options()).expect("CMG");
        let dimension = preconditioner.dimension();
        let mut input = Vec::with_capacity(2 * dimension);
        input.extend(
            (0..dimension).map(|index| f64::from(u32::try_from(index % 7).expect("index")) - 3.0),
        );
        input.extend(
            (0..dimension).map(|index| f64::from(u32::try_from(index % 5).expect("index")) - 2.0),
        );
        let mut expected = vec![0.0; input.len()];
        for column in 0..2 {
            let range = column * dimension..(column + 1) * dimension;
            preconditioner
                .apply(&input[range.clone()], &mut expected[range.clone()])
                .expect("scalar apply");
            operator
                .project(&mut expected[range])
                .expect("scalar projection");
        }
        let mut actual = vec![0.0; input.len()];
        preconditioner
            .apply_columns_with_interrupt(
                &operator,
                &input,
                &mut actual,
                2,
                &[true, true],
                &mut NeverInterrupt,
            )
            .expect("batch apply");
        assert_eq!(
            actual
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn batched_workspace_receipt_counts_every_owned_vector() {
        let problem = density_four_problem(48);
        let preconditioner = CmgPreconditioner::new(&problem, test_options()).expect("CMG");
        let columns = 3;
        let workspace =
            CmgBatchWorkspace::new(&preconditioner.hierarchy, columns).expect("workspace");
        let entries = workspace
            .level
            .iter()
            .map(|level| {
                level.rhs.len()
                    + level.solution.len()
                    + level.action.len()
                    + level.residual.len()
                    + level.coarse_rhs.len()
                    + level.coarse_solution.len()
                    + level.column_sum.len()
            })
            .sum::<usize>()
            + workspace.full_rhs.len()
            + workspace.full_solution.len();
        assert_eq!(
            preconditioner
                .receipt()
                .batch_workspace_bytes(columns)
                .expect("receipt"),
            u64::try_from(entries).expect("entries") * 8
        );
    }

    #[test]
    fn batched_cmg_polls_inside_flattened_edge_column_work() {
        #[derive(Default)]
        struct BreakInsideBatch {
            calls: usize,
        }

        impl InterruptCheck for BreakInsideBatch {
            fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
                if phase == "cmg_laplacian_batch" {
                    self.calls += 1;
                    if self.calls == 2 {
                        return Err(BackendError::new(
                            ErrorCode::UserBreak,
                            phase,
                            "injected batch break",
                        ));
                    }
                }
                Ok(())
            }
        }

        let problem = density_four_problem(1_024);
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner = CmgPreconditioner::new(&problem, test_options()).expect("CMG");
        let input = vec![1.0; preconditioner.dimension() * 2];
        let mut output = vec![0.0; input.len()];
        let mut interrupt = BreakInsideBatch::default();
        let error = preconditioner
            .apply_columns_with_interrupt(
                &operator,
                &input,
                &mut output,
                2,
                &[true, true],
                &mut interrupt,
            )
            .expect_err("deep batch break");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(interrupt.calls, 2);
    }

    #[test]
    fn cmg_preconditioned_pcg_converges_on_density_four_graph() {
        let problem = density_four_problem(32);
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner = CmgPreconditioner::new(&problem, test_options()).expect("CMG");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let schur_rhs = operator
            .schur_rhs(&worker_rhs, &firm_rhs)
            .expect("Schur RHS");
        let solve = pcg(
            &operator,
            &preconditioner,
            &schur_rhs,
            PcgOptions {
                tolerance: 1.0e-10,
                maximum_iterations: 500,
                residual_replacement_interval: 20,
            },
        )
        .expect("PCG");
        let firm_solution = operator.expand_firm(&solve.solution).expect("firm");
        let worker_solution = operator
            .reconstruct_worker(&worker_rhs, &firm_solution)
            .expect("worker");
        let residual = operator
            .full_residual(&worker_solution, &firm_solution, &worker_rhs, &firm_rhs)
            .expect("full residual");
        assert!(solve.receipt.iterations > 0);
        assert!(residual.relative_norm < 1.0e-9);
        assert_eq!(operator.dimension(), preconditioner.dimension());
    }

    #[test]
    fn tie_rich_density_four_v_cycle_and_route_are_firm_relabeling_equivariant() {
        let firms = 32_usize;
        let original_label = (0..firms)
            .map(|firm| u64::try_from(firm + 1).expect("firm"))
            .collect::<Vec<_>>();
        let relabeled = (0..firms)
            .map(|firm| 1_000 + u64::try_from((13 * firm) % firms).expect("permuted firm"))
            .collect::<Vec<_>>();
        let original = density_four_problem_with_labels(&original_label);
        let permuted = density_four_problem_with_labels(&relabeled);
        let original_preconditioner =
            CmgPreconditioner::new(&original, test_options()).expect("original CMG");
        let permuted_preconditioner =
            CmgPreconditioner::new(&permuted, test_options()).expect("permuted CMG");

        let mut conceptual_rhs = (0..firms)
            .map(|firm| f64::from(u32::try_from(firm % 9).expect("firm")) - 4.0)
            .collect::<Vec<_>>();
        let mean = conceptual_rhs.iter().copied().sum::<f64>()
            / f64::from(u32::try_from(firms).expect("firms"));
        for value in &mut conceptual_rhs {
            *value -= mean;
        }
        let original_rhs = dense_firm_vector(&original_label, &conceptual_rhs);
        let permuted_rhs = dense_firm_vector(&relabeled, &conceptual_rhs);
        let mut original_action = vec![0.0; firms];
        let mut permuted_action = vec![0.0; firms];
        original_preconditioner
            .apply(&original_rhs, &mut original_action)
            .expect("original V-cycle");
        permuted_preconditioner
            .apply(&permuted_rhs, &mut permuted_action)
            .expect("permuted V-cycle");
        assert_vector_close(
            &conceptual_firm_vector(&original_label, &original_action),
            &conceptual_firm_vector(&relabeled, &permuted_action),
            1.0e-11,
        );

        let original_operator = TwoWayOperator::new(&original).expect("original operator");
        let permuted_operator = TwoWayOperator::new(&permuted).expect("permuted operator");
        let (original_worker_rhs, original_firm_rhs) =
            original_operator.outcome_rhs().expect("original RHS");
        let (permuted_worker_rhs, permuted_firm_rhs) =
            permuted_operator.outcome_rhs().expect("permuted RHS");
        let options = LinearSolverOptions {
            route: LinearSolverRoute::CmgPcg,
            exact_dimension_limit: 1,
            cmg_minimum_dimension: 1,
            pcg: PcgOptions {
                tolerance: 1.0e-10,
                maximum_iterations: 500,
                residual_replacement_interval: 20,
            },
            cmg: test_options(),
            full_residual_tolerance: 1.0e-9,
            ..LinearSolverOptions::default()
        };
        let original_solve =
            solve_two_way_routed(&original, &original_worker_rhs, &original_firm_rhs, options)
                .expect("original forced CMG solve");
        let permuted_solve =
            solve_two_way_routed(&permuted, &permuted_worker_rhs, &permuted_firm_rhs, options)
                .expect("permuted forced CMG solve");
        assert_eq!(original_solve.receipt.selected, LinearSolverRoute::CmgPcg);
        assert_eq!(permuted_solve.receipt.selected, LinearSolverRoute::CmgPcg);
        let original_pcg = original_solve.receipt.pcg.as_ref().expect("original PCG");
        let permuted_pcg = permuted_solve.receipt.pcg.as_ref().expect("permuted PCG");
        assert!(!original_pcg.zero_rhs && !permuted_pcg.zero_rhs);
        assert_eq!(original_pcg.iterations, permuted_pcg.iterations);
        assert!(original_pcg.relative_residual <= options.pcg.tolerance);
        assert!(permuted_pcg.relative_residual <= options.pcg.tolerance);
        let original_prediction = fitted_values(
            &original,
            &original_solve.solution.worker,
            &original_solve.solution.firm,
        )
        .expect("original prediction");
        let permuted_prediction = fitted_values(
            &permuted,
            &permuted_solve.solution.worker,
            &permuted_solve.solution.firm,
        )
        .expect("permuted prediction");
        assert_vector_close(&original_prediction, &permuted_prediction, 1.0e-9);
        assert_vector_close(
            &original_solve.solution.residual.worker,
            &permuted_solve.solution.residual.worker,
            1.0e-9,
        );
        assert_vector_close(
            &conceptual_firm_vector(&original_label, &original_solve.solution.residual.firm),
            &conceptual_firm_vector(&relabeled, &permuted_solve.solution.residual.firm),
            1.0e-9,
        );
        assert!(
            (original_solve.solution.residual.absolute_norm
                - permuted_solve.solution.residual.absolute_norm)
                .abs()
                <= 1.0e-9
        );
        assert!(
            (original_solve.solution.residual.rhs_norm - permuted_solve.solution.residual.rhs_norm)
                .abs()
                <= 1.0e-9
        );
        assert!(original_solve.solution.residual.relative_norm <= 1.0e-9);
        assert!(permuted_solve.solution.residual.relative_norm <= 1.0e-9);
    }

    #[test]
    fn structural_twin_v_cycle_and_route_are_firm_relabeling_equivariant() {
        let firms = 16_usize;
        let original_label = (0..firms)
            .map(|firm| u64::try_from(firm + 1).expect("firm"))
            .collect::<Vec<_>>();
        let relabeled = (0..firms)
            .map(|firm| 1_000 + u64::try_from((5 * firm) % firms).expect("permuted firm"))
            .collect::<Vec<_>>();
        let original = complete_bipartite_problem_with_labels(&original_label, 8);
        let permuted = complete_bipartite_problem_with_labels(&relabeled, 8);
        let options = CmgOptions {
            terminal_vertices: 4,
            ..test_options()
        };
        let original_preconditioner =
            CmgPreconditioner::new(&original, options).expect("original CMG");
        let permuted_preconditioner =
            CmgPreconditioner::new(&permuted, options).expect("permuted CMG");
        assert_eq!(
            original_preconditioner.receipt().level[0].coarse_vertices,
            Some(2)
        );
        assert_eq!(
            permuted_preconditioner.receipt().level[0].coarse_vertices,
            Some(2)
        );
        assert!(original_preconditioner.receipt().terminal_vertices <= 4);
        assert!(permuted_preconditioner.receipt().terminal_vertices <= 4);

        let mut conceptual_rhs = (0..firms)
            .map(|firm| f64::from(u32::try_from((11 * firm) % 17).expect("firm")) - 8.0)
            .collect::<Vec<_>>();
        let mean = conceptual_rhs.iter().copied().sum::<f64>()
            / f64::from(u32::try_from(firms).expect("firms"));
        for value in &mut conceptual_rhs {
            *value -= mean;
        }
        let original_rhs = dense_firm_vector(&original_label, &conceptual_rhs);
        let permuted_rhs = dense_firm_vector(&relabeled, &conceptual_rhs);
        let mut original_action = vec![0.0; firms];
        let mut permuted_action = vec![0.0; firms];
        original_preconditioner
            .apply(&original_rhs, &mut original_action)
            .expect("original V-cycle");
        permuted_preconditioner
            .apply(&permuted_rhs, &mut permuted_action)
            .expect("permuted V-cycle");
        assert_vector_close(
            &conceptual_firm_vector(&original_label, &original_action),
            &conceptual_firm_vector(&relabeled, &permuted_action),
            1.0e-11,
        );

        let original_operator = TwoWayOperator::new(&original).expect("original operator");
        let permuted_operator = TwoWayOperator::new(&permuted).expect("permuted operator");
        let (original_worker_rhs, original_firm_rhs) =
            original_operator.outcome_rhs().expect("original RHS");
        let (permuted_worker_rhs, permuted_firm_rhs) =
            permuted_operator.outcome_rhs().expect("permuted RHS");
        let solver_options = LinearSolverOptions {
            route: LinearSolverRoute::CmgPcg,
            exact_dimension_limit: 1,
            cmg_minimum_dimension: 1,
            pcg: PcgOptions {
                tolerance: 1.0e-10,
                maximum_iterations: 500,
                residual_replacement_interval: 20,
            },
            cmg: options,
            full_residual_tolerance: 1.0e-9,
            ..LinearSolverOptions::default()
        };
        let original_solve = solve_two_way_routed(
            &original,
            &original_worker_rhs,
            &original_firm_rhs,
            solver_options,
        )
        .expect("original forced CMG solve");
        let permuted_solve = solve_two_way_routed(
            &permuted,
            &permuted_worker_rhs,
            &permuted_firm_rhs,
            solver_options,
        )
        .expect("permuted forced CMG solve");
        assert_eq!(original_solve.receipt.selected, LinearSolverRoute::CmgPcg);
        assert_eq!(permuted_solve.receipt.selected, LinearSolverRoute::CmgPcg);
        let original_pcg = original_solve.receipt.pcg.as_ref().expect("original PCG");
        let permuted_pcg = permuted_solve.receipt.pcg.as_ref().expect("permuted PCG");
        assert!(!original_pcg.zero_rhs && !permuted_pcg.zero_rhs);
        assert!(original_pcg.iterations > 0);
        assert_eq!(original_pcg.iterations, permuted_pcg.iterations);
        assert!(original_pcg.relative_residual <= solver_options.pcg.tolerance);
        assert!(permuted_pcg.relative_residual <= solver_options.pcg.tolerance);
        let original_prediction = fitted_values(
            &original,
            &original_solve.solution.worker,
            &original_solve.solution.firm,
        )
        .expect("original prediction");
        let permuted_prediction = fitted_values(
            &permuted,
            &permuted_solve.solution.worker,
            &permuted_solve.solution.firm,
        )
        .expect("permuted prediction");
        assert_vector_close(&original_prediction, &permuted_prediction, 1.0e-9);
        assert_vector_close(
            &original_solve.solution.residual.worker,
            &permuted_solve.solution.residual.worker,
            1.0e-9,
        );
        assert_vector_close(
            &conceptual_firm_vector(&original_label, &original_solve.solution.residual.firm),
            &conceptual_firm_vector(&relabeled, &permuted_solve.solution.residual.firm),
            1.0e-9,
        );
        assert!(original_solve.solution.residual.relative_norm <= 1.0e-9);
        assert!(permuted_solve.solution.residual.relative_norm <= 1.0e-9);
    }
}
