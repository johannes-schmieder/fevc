// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic CMG hierarchy, symmetric V-cycle, and full zero-sum quotient
//! preconditioner for the worker-eliminated two-way fixed-effect system.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use super::{HybridGraph, VertexKey, WeightedEdge};
use crate::error::{BackendError, ErrorCode, Result};
use crate::krylov::Preconditioner;
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
    pub dense_factor_bytes: u64,
    pub level: Vec<CmgLevelReceipt>,
}

#[derive(Clone, Debug)]
struct LaplacianGraph {
    key: Vec<VertexKey>,
    edge: Vec<WeightedEdge>,
    degree: Vec<f64>,
}

impl LaplacianGraph {
    fn from_hybrid(graph: &HybridGraph) -> Result<Self> {
        Self::from_parts(graph.vertex_keys().to_vec(), graph.edges().to_vec())
    }

    fn from_parts(key: Vec<VertexKey>, mut edge: Vec<WeightedEdge>) -> Result<Self> {
        if key.is_empty() || key.len() > u32::MAX as usize {
            return Err(cmg_setup_error(
                "CMG graph vertex count is empty or exceeds the u32 limit",
            ));
        }
        for item in &edge {
            let left = usize::try_from(item.u).expect("validated u32 endpoint");
            let right = usize::try_from(item.v).expect("validated u32 endpoint");
            if left >= right || right >= key.len() || !item.weight.is_finite() || item.weight <= 0.0
            {
                return Err(cmg_setup_error("CMG graph contains an invalid edge"));
            }
        }
        edge.sort_unstable_by(|left, right| {
            edge_key_from_keys(&key, *left)
                .cmp(&edge_key_from_keys(&key, *right))
                .then_with(|| right.weight.total_cmp(&left.weight))
        });
        let mut degree = vec![0.0_f64; key.len()];
        for item in &edge {
            let left = usize::try_from(item.u).expect("validated u32 endpoint");
            let right = usize::try_from(item.v).expect("validated u32 endpoint");
            degree[left] += item.weight;
            degree[right] += item.weight;
        }
        if key.len() > 1
            && (edge.is_empty()
                || degree
                    .iter()
                    .any(|&value| !value.is_finite() || value <= 0.0))
        {
            return Err(cmg_setup_error(
                "CMG graph is disconnected or has a nonpositive degree",
            ));
        }
        let graph = Self { key, edge, degree };
        if !graph.is_connected() {
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

    fn apply(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        if input.len() != self.vertices() || output.len() != self.vertices() {
            return Err(BackendError::invalid(
                "cmg_apply",
                "Laplacian action has incompatible dimensions",
            ));
        }
        output.fill(0.0);
        for item in &self.edge {
            let left = usize::try_from(item.u).expect("validated endpoint");
            let right = usize::try_from(item.v).expect("validated endpoint");
            let value = item.weight * (input[left] - input[right]);
            output[left] += value;
            output[right] -= value;
        }
        if output.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::CmgApplyFailed,
                "cmg_apply",
                "Laplacian action produced a nonfinite value",
            ));
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        if self.vertices() <= 1 {
            return true;
        }
        let mut adjacency = vec![Vec::<u32>::new(); self.vertices()];
        for item in &self.edge {
            let left = usize::try_from(item.u).expect("endpoint");
            let right = usize::try_from(item.v).expect("endpoint");
            adjacency[left].push(item.v);
            adjacency[right].push(item.u);
        }
        let mut seen = vec![false; self.vertices()];
        let mut queue = VecDeque::from([0_usize]);
        seen[0] = true;
        while let Some(vertex) = queue.pop_front() {
            for &neighbor in &adjacency[vertex] {
                let neighbor = usize::try_from(neighbor).expect("neighbor");
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        seen.into_iter().all(|value| value)
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
    fn factor(graph: &LaplacianGraph, memory_limit: u64) -> Result<Self> {
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
        for item in &graph.edge {
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
        let maximum_diagonal = (0..reduced)
            .map(|index| matrix[index * reduced + index])
            .fold(0.0_f64, f64::max);
        let pivot_floor = maximum_diagonal * 1.0e-14;
        let mut lower = vec![0.0_f64; entries];
        for row in 0..reduced {
            for column in 0..=row {
                let mut value = matrix[row * reduced + column];
                for inner in 0..column {
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

    fn solve(
        &self,
        right_hand_side: &[f64],
        solution: &mut [f64],
        intermediate: &mut [f64],
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
            let mut value = right_hand_side[row];
            for column in 0..row {
                value -= self.lower[row * reduced + column] * intermediate[column];
            }
            intermediate[row] = value / self.lower[row * reduced + row];
        }
        for row in (0..reduced).rev() {
            let mut value = intermediate[row];
            for column in (row + 1)..reduced {
                value -= self.lower[column * reduced + row] * solution[column];
            }
            solution[row] = value / self.lower[row * reduced + row];
        }
        center(solution)?;
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
pub struct CmgHierarchy {
    options: CmgOptions,
    level: Vec<CmgLevel>,
    terminal: DenseGroundedSolver,
    receipt: CmgReceipt,
}

impl CmgHierarchy {
    pub fn build(hybrid: &HybridGraph, options: CmgOptions) -> Result<Self> {
        let options = options.validate()?;
        let fine = LaplacianGraph::from_hybrid(hybrid)?;
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
            let current = level.last().expect("fine level exists").graph.vertices();
            if current <= options.terminal_vertices {
                break;
            }
            if level.len() >= options.maximum_levels {
                return Err(cmg_setup_error("CMG hierarchy exceeded the level cap"));
            }
            let aggregation = aggregate(&level.last().expect("level").graph, options)?;
            let coarse = contract(&level.last().expect("level").graph, &aggregation)?;
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
        let terminal = DenseGroundedSolver::factor(
            &level.last().expect("terminal").graph,
            options.memory_limit_bytes - reserved,
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
        if right_hand_side.len() != self.dimension()
            || solution.len() != self.dimension()
            || workspace.level.len() != self.level.len()
        {
            return Err(BackendError::invalid(
                "cmg_apply",
                "CMG application has incompatible dimensions or workspace",
            ));
        }
        if right_hand_side.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "cmg_apply",
                "CMG right-hand side is nonfinite",
            ));
        }
        self.cycle(0, right_hand_side, solution, &mut workspace.level)?;
        if solution.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::CmgApplyFailed,
                "cmg_apply",
                "CMG V-cycle produced a nonfinite value",
            ));
        }
        Ok(())
    }

    fn cycle(
        &self,
        level_index: usize,
        input: &[f64],
        output: &mut [f64],
        workspace: &mut [LevelWorkspace],
    ) -> Result<()> {
        let (current, coarser_workspace) = workspace
            .split_first_mut()
            .ok_or_else(|| BackendError::invariant("cmg_apply", "missing CMG workspace level"))?;
        current.rhs.copy_from_slice(input);
        center(&mut current.rhs)?;
        current.solution.fill(0.0);

        if level_index + 1 == self.level.len() {
            self.terminal.solve(
                &current.rhs,
                &mut current.solution,
                &mut current.action[..current.rhs.len() - 1],
            )?;
            output.copy_from_slice(&current.solution);
            return Ok(());
        }

        let graph = &self.level[level_index].graph;
        let aggregation = self.level[level_index]
            .aggregation
            .as_ref()
            .ok_or_else(|| BackendError::invariant("cmg_apply", "missing aggregation map"))?;
        smooth(
            graph,
            &current.rhs,
            &mut current.solution,
            &mut current.action,
            self.options.jacobi_weight,
            self.options.pre_sweeps,
        )?;
        graph.apply(&current.solution, &mut current.action)?;
        for ((residual, &rhs), &action) in current
            .residual
            .iter_mut()
            .zip(&current.rhs)
            .zip(&current.action)
        {
            *residual = rhs - action;
        }
        current.coarse_rhs.fill(0.0);
        for (vertex, &value) in current.residual.iter().enumerate() {
            let aggregate =
                usize::try_from(aggregation.assignment[vertex]).expect("validated aggregate index");
            current.coarse_rhs[aggregate] += value;
        }
        center(&mut current.coarse_rhs)?;
        self.cycle(
            level_index + 1,
            &current.coarse_rhs,
            &mut current.coarse_solution,
            coarser_workspace,
        )?;
        for (vertex, value) in current.solution.iter_mut().enumerate() {
            let aggregate =
                usize::try_from(aggregation.assignment[vertex]).expect("validated aggregate index");
            *value += current.coarse_solution[aggregate];
        }
        smooth(
            graph,
            &current.rhs,
            &mut current.solution,
            &mut current.action,
            self.options.jacobi_weight,
            self.options.post_sweeps,
        )?;
        center(&mut current.solution)?;
        output.copy_from_slice(&current.solution);
        Ok(())
    }
}

#[derive(Debug)]
struct PreconditionerWorkspace {
    hierarchy: CmgWorkspace,
    full_rhs: Vec<f64>,
    full_solution: Vec<f64>,
}

#[derive(Debug)]
pub struct CmgPreconditioner {
    firms: usize,
    hierarchy: CmgHierarchy,
    workspace: Mutex<PreconditionerWorkspace>,
}

impl CmgPreconditioner {
    pub fn new(problem: &CompressedProblem, options: CmgOptions) -> Result<Self> {
        let hybrid = HybridGraph::from_problem(problem)?;
        let firms = hybrid.firms();
        let hierarchy = CmgHierarchy::build(&hybrid, options)?;
        let vertices = hierarchy.dimension();
        let workspace = PreconditionerWorkspace {
            hierarchy: hierarchy.workspace(),
            full_rhs: vec![0.0; vertices],
            full_solution: vec![0.0; vertices],
        };
        Ok(Self {
            firms,
            hierarchy,
            workspace: Mutex::new(workspace),
        })
    }

    #[must_use]
    pub const fn receipt(&self) -> &CmgReceipt {
        self.hierarchy.receipt()
    }
}

impl Preconditioner for CmgPreconditioner {
    fn dimension(&self) -> usize {
        self.firms
    }

    fn apply(&self, residual: &[f64], output: &mut [f64]) -> Result<()> {
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
        center(&mut state.full_rhs[..self.firms])?;
        let PreconditionerWorkspace {
            hierarchy,
            full_rhs,
            full_solution,
        } = &mut *state;
        self.hierarchy.apply(full_rhs, full_solution, hierarchy)?;
        center(&mut full_solution[..self.firms])?;
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
}

fn aggregate(graph: &LaplacianGraph, options: CmgOptions) -> Result<Aggregation> {
    let mut edge_order = (0..graph.edges()).collect::<Vec<_>>();
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

    let (mut groups, mut used) = structural_twin_groups(graph);
    for edge_index in edge_order {
        let item = graph.edge[edge_index];
        let left = usize::try_from(item.u).expect("endpoint");
        let right = usize::try_from(item.v).expect("endpoint");
        if !used[left] && !used[right] {
            used[left] = true;
            used[right] = true;
            groups.push(vec![left, right]);
        }
    }
    let mut residual = (0..graph.vertices())
        .filter(|&vertex| !used[vertex])
        .collect::<Vec<_>>();
    residual.sort_unstable_by_key(|&vertex| graph.key[vertex]);
    for chunk in residual.chunks(options.aggregate_cap) {
        groups.push(chunk.to_vec());
    }
    let mut aggregation = finalize_groups(graph, groups, AggregationMethod::NormalizedHeavyEdge)?;
    let reduction =
        1.0 - usize_to_f64(aggregation.coarse_vertices)? / usize_to_f64(graph.vertices())?;
    if reduction < options.minimum_reduction && graph.vertices() > 1 {
        aggregation = canonical_pack(graph, options.aggregate_cap)?;
    }
    Ok(aggregation)
}

fn canonical_pack(graph: &LaplacianGraph, cap: usize) -> Result<Aggregation> {
    let (mut groups, used) = structural_twin_groups(graph);
    let mut order = (0..graph.vertices())
        .filter(|&vertex| !used[vertex])
        .collect::<Vec<_>>();
    order.sort_unstable_by_key(|&vertex| graph.key[vertex]);
    groups.extend(order.chunks(cap).map(|chunk| chunk.to_vec()));
    finalize_groups(graph, groups, AggregationMethod::CanonicalPacking)
}

fn structural_twin_groups(graph: &LaplacianGraph) -> (Vec<Vec<usize>>, Vec<bool>) {
    let mut by_key = BTreeMap::<VertexKey, Vec<usize>>::new();
    for (vertex, &key) in graph.key.iter().enumerate() {
        by_key.entry(key).or_default().push(vertex);
    }
    let mut used = vec![false; graph.vertices()];
    let mut groups = Vec::new();
    for group in by_key.into_values().filter(|group| group.len() > 1) {
        // Equal fine-level firm keys mean identical weighted incidence to every
        // worker, hence a full permutation automorphism class.  No proper
        // deterministic subdivision of such a class is label-equivariant, so
        // it must be coarsened collectively.  The group can exceed
        // `aggregate_cap`: that cap bounds ordinary canonical packing, while
        // this one group still has linear storage and strictly reduces the
        // coarse dimension.
        for &vertex in &group {
            used[vertex] = true;
        }
        groups.push(group);
    }
    (groups, used)
}

fn finalize_groups(
    graph: &LaplacianGraph,
    mut groups: Vec<Vec<usize>>,
    method: AggregationMethod,
) -> Result<Aggregation> {
    groups.retain(|group| !group.is_empty());
    groups.sort_unstable_by_key(|group| {
        group
            .iter()
            .map(|&vertex| graph.key[vertex])
            .min()
            .expect("nonempty aggregate")
    });
    if groups.is_empty() || (groups.len() >= graph.vertices() && graph.vertices() > 1) {
        return Err(cmg_setup_error("CMG aggregation did not reduce the graph"));
    }
    let mut assignment = vec![u32::MAX; graph.vertices()];
    for (aggregate, group) in groups.iter().enumerate() {
        let aggregate = u32::try_from(aggregate)
            .map_err(|_| resource_error("aggregate index exceeds the u32 limit"))?;
        for &vertex in group {
            if vertex >= graph.vertices() || assignment[vertex] != u32::MAX {
                return Err(cmg_setup_error(
                    "CMG aggregation is overlapping or out of range",
                ));
            }
            assignment[vertex] = aggregate;
        }
    }
    if assignment.contains(&u32::MAX) {
        return Err(cmg_setup_error("CMG aggregation left a vertex unassigned"));
    }
    Ok(Aggregation {
        assignment,
        coarse_vertices: groups.len(),
        method,
    })
}

fn contract(graph: &LaplacianGraph, aggregation: &Aggregation) -> Result<LaplacianGraph> {
    let mut key = vec![None::<VertexKey>; aggregation.coarse_vertices];
    for (vertex, &aggregate) in aggregation.assignment.iter().enumerate() {
        let aggregate = usize::try_from(aggregate).expect("aggregate");
        key[aggregate] = Some(
            key[aggregate].map_or(graph.key[vertex], |current| current.min(graph.key[vertex])),
        );
    }
    let key = key
        .into_iter()
        .map(|value| value.expect("nonempty aggregate"))
        .collect::<Vec<_>>();
    let mut contribution = BTreeMap::<(u32, u32), f64>::new();
    for item in &graph.edge {
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
    let edge = contribution
        .into_iter()
        .map(|((u, v), weight)| WeightedEdge { u, v, weight })
        .collect();
    LaplacianGraph::from_parts(key, edge)
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

fn smooth(
    graph: &LaplacianGraph,
    right_hand_side: &[f64],
    solution: &mut [f64],
    action: &mut [f64],
    weight: f64,
    sweeps: u32,
) -> Result<()> {
    for _ in 0..sweeps {
        graph.apply(solution, action)?;
        for (((value, &rhs), &applied), &diagonal) in solution
            .iter_mut()
            .zip(right_hand_side)
            .zip(action.iter())
            .zip(&graph.degree)
        {
            *value += weight * (rhs - applied) / diagonal;
        }
        center(solution)?;
    }
    Ok(())
}

fn center(values: &mut [f64]) -> Result<()> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::CmgApplyFailed,
            "cmg_apply",
            "cannot center an empty or nonfinite vector",
        ));
    }
    let mean = values.iter().copied().sum::<f64>() / usize_to_f64(values.len())?;
    for value in values {
        *value -= mean;
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
