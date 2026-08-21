// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{BTreeSet, VecDeque};

use crate::error::{BackendError, ErrorCode, Result};
use crate::problem::CanonicalInput;

const UNVISITED: usize = usize::MAX;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GraphSelectionReceipt {
    pub input_rows: u64,
    pub retained_rows: u64,
    pub input_physical_mass: u64,
    pub retained_physical_mass: u64,
    pub initial_components: u64,
    pub maximum_components: u64,
    pub initial_component_rows: u64,
    pub mover_input_rows: u64,
    pub initial_deletion_edges: u64,
    pub retained_deletion_edges: u64,
    pub insufficient_workers_removed: u64,
    pub articulation_workers_removed: u64,
    pub bridge_units_removed: u64,
    pub bridge_rows_removed: u64,
    pub degree_iterations: u64,
    pub articulation_iterations: u64,
    pub bridge_iterations: u64,
    pub fixed_point_iterations: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphSelection {
    pub active: Vec<bool>,
    pub receipt: GraphSelectionReceipt,
}

#[derive(Clone, Debug)]
struct ComponentResult {
    keep: Vec<bool>,
    components: usize,
    retained_rows: usize,
}

#[derive(Clone, Copy, Debug)]
struct Arc {
    to: usize,
    edge: usize,
}

#[derive(Clone, Debug)]
struct Graph {
    adjacency: Vec<Vec<Arc>>,
    edges: usize,
}

#[derive(Clone, Copy, Debug)]
struct DfsFrame {
    node: usize,
    parent_edge: Option<usize>,
    next_arc: usize,
}

pub fn select_match_deletion_graph(input: &CanonicalInput) -> Result<GraphSelection> {
    if input.rows() == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "command sample is empty",
        ));
    }
    let mut active = vec![true; input.rows()];
    let mut receipt = GraphSelectionReceipt {
        input_rows: u64::try_from(input.rows()).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "graph",
                "input row count is not representable",
            )
        })?,
        input_physical_mass: input.physical_total,
        ..GraphSelectionReceipt::default()
    };

    let initial = largest_component(input, &active)?;
    receipt.initial_components = as_u64(initial.components, "component count")?;
    receipt.maximum_components = receipt.initial_components;
    receipt.initial_component_rows = as_u64(initial.retained_rows, "component row count")?;
    intersect_active(&mut active, &initial.keep);

    let worker_firms = distinct_firm_counts(input, &active);
    for (row, keep) in active.iter_mut().enumerate() {
        if *keep {
            let worker = usize::try_from(input.worker[row]).expect("dense worker");
            *keep = worker_firms[worker] > 1;
        }
    }
    let mover_rows = active.iter().filter(|&&keep| keep).count();
    receipt.mover_input_rows = as_u64(mover_rows, "mover row count")?;
    if mover_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "no observations remain in the mover target population",
        ));
    }

    let mover_component = largest_component(input, &active)?;
    receipt.maximum_components = receipt
        .maximum_components
        .max(as_u64(mover_component.components, "component count")?);
    intersect_active(&mut active, &mover_component.keep);

    let initial_edges = active_deletion_ids(input, &active).len();
    receipt.initial_deletion_edges = as_u64(initial_edges, "deletion edge count")?;
    let iteration_bound = input
        .workers()
        .checked_add(initial_edges)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "graph",
                "fixed-point iteration bound overflow",
            )
        })?;
    let iteration_bound = as_u64(iteration_bound, "fixed-point iteration bound")?;

    loop {
        if receipt.fixed_point_iterations > iteration_bound {
            return Err(BackendError::new(
                ErrorCode::GraphCertificateFailed,
                "graph",
                "sample fixed-point pruning exceeded its finite iteration bound",
            ));
        }

        let component = largest_component(input, &active)?;
        receipt.maximum_components = receipt
            .maximum_components
            .max(as_u64(component.components, "component count")?);
        intersect_active(&mut active, &component.keep);
        if !active.iter().any(|&keep| keep) {
            return Err(BackendError::new(
                ErrorCode::GraphEmpty,
                "graph",
                "no component remains after fixed-point pruning",
            ));
        }

        let worker_firms = distinct_firm_counts(input, &active);
        let insufficient: Vec<bool> = worker_firms.iter().map(|&count| count == 1).collect();
        let removed_workers = insufficient.iter().filter(|&&remove| remove).count();
        if removed_workers > 0 {
            for (row, keep) in active.iter_mut().enumerate() {
                if *keep {
                    let worker = usize::try_from(input.worker[row]).expect("dense worker");
                    if insufficient[worker] {
                        *keep = false;
                    }
                }
            }
            receipt.insufficient_workers_removed = receipt
                .insufficient_workers_removed
                .checked_add(as_u64(removed_workers, "removed worker count")?)
                .ok_or_else(|| counter_overflow("insufficient workers"))?;
            receipt.degree_iterations += 1;
            receipt.fixed_point_iterations += 1;
            continue;
        }

        let articulations = worker_articulations(input, &active)?;
        let articulation_count = articulations.iter().filter(|&&remove| remove).count();
        if articulation_count > 0 {
            for (row, keep) in active.iter_mut().enumerate() {
                if *keep {
                    let worker = usize::try_from(input.worker[row]).expect("dense worker");
                    if articulations[worker] {
                        *keep = false;
                    }
                }
            }
            receipt.articulation_workers_removed = receipt
                .articulation_workers_removed
                .checked_add(as_u64(articulation_count, "articulation count")?)
                .ok_or_else(|| counter_overflow("articulation workers"))?;
            receipt.articulation_iterations += 1;
            receipt.fixed_point_iterations += 1;
            continue;
        }

        let bridges = deletion_bridges(input, &active)?;
        if bridges.is_empty() {
            break;
        }
        let bridge_set: BTreeSet<u32> = bridges.iter().copied().collect();
        let removed_rows = active
            .iter()
            .enumerate()
            .filter(|(row, keep)| **keep && bridge_set.contains(&input.deletion[*row]))
            .count();
        for (row, keep) in active.iter_mut().enumerate() {
            if *keep && bridge_set.contains(&input.deletion[row]) {
                *keep = false;
            }
        }
        receipt.bridge_units_removed = receipt
            .bridge_units_removed
            .checked_add(as_u64(bridges.len(), "bridge count")?)
            .ok_or_else(|| counter_overflow("bridge units"))?;
        receipt.bridge_rows_removed = receipt
            .bridge_rows_removed
            .checked_add(as_u64(removed_rows, "bridge row count")?)
            .ok_or_else(|| counter_overflow("bridge rows"))?;
        receipt.bridge_iterations += 1;
        receipt.fixed_point_iterations += 1;
    }

    if receipt.fixed_point_iterations > iteration_bound {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "sample fixed-point pruning exceeded its finite iteration bound",
        ));
    }

    let final_component = largest_component(input, &active)?;
    receipt.maximum_components = receipt
        .maximum_components
        .max(as_u64(final_component.components, "component count")?);
    intersect_active(&mut active, &final_component.keep);

    let final_bridges = deletion_bridges(input, &active)?;
    if !final_bridges.is_empty() {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final retained deletion-unit multigraph still contains a bridge",
        ));
    }
    let final_articulations = worker_articulations(input, &active)?;
    if final_articulations.iter().any(|&remove| remove) {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final retained graph still contains a worker articulation",
        ));
    }
    let final_worker_firms = distinct_firm_counts(input, &active);
    if final_worker_firms.iter().any(|&count| count == 1) {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final retained graph still contains a one-firm worker",
        ));
    }

    let retained_rows = active.iter().filter(|&&keep| keep).count();
    if retained_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "no component remains after fixed-point graph pruning",
        ));
    }
    receipt.retained_rows = as_u64(retained_rows, "retained row count")?;
    receipt.retained_physical_mass = active
        .iter()
        .enumerate()
        .filter(|(_, keep)| **keep)
        .try_fold(0_u64, |total, (row, _)| {
            total.checked_add(input.frequency[row]).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "graph",
                    "retained physical mass overflow",
                )
            })
        })?;
    receipt.retained_deletion_edges =
        as_u64(active_deletion_ids(input, &active).len(), "retained deletion edges")?;

    Ok(GraphSelection { active, receipt })
}

fn largest_component(input: &CanonicalInput, active: &[bool]) -> Result<ComponentResult> {
    validate_mask(input, active)?;
    let graph = coordinate_graph(input, active)?;
    let mut label = vec![UNVISITED; graph.adjacency.len()];
    let mut components = 0_usize;
    let mut queue = VecDeque::new();

    for root in 0..graph.adjacency.len() {
        if graph.adjacency[root].is_empty() || label[root] != UNVISITED {
            continue;
        }
        label[root] = components;
        queue.push_back(root);
        while let Some(node) = queue.pop_front() {
            for arc in &graph.adjacency[node] {
                if label[arc.to] == UNVISITED {
                    label[arc.to] = components;
                    queue.push_back(arc.to);
                }
            }
        }
        components += 1;
    }
    if components == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "no connected worker-firm component remains",
        ));
    }

    let mut mass = vec![0_u64; components];
    let mut row_component = vec![UNVISITED; input.rows()];
    for (row, &keep) in active.iter().enumerate() {
        if !keep {
            continue;
        }
        let worker = usize::try_from(input.worker[row]).expect("dense worker");
        let component = label[worker];
        if component == UNVISITED {
            return Err(BackendError::invariant(
                "graph",
                "active row has no connected-component label",
            ));
        }
        row_component[row] = component;
        mass[component] = mass[component]
            .checked_add(input.frequency[row])
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "graph",
                    "component physical mass overflow",
                )
            })?;
    }
    let largest = mass.iter().copied().max().ok_or_else(|| {
        BackendError::new(ErrorCode::GraphEmpty, "graph", "component mass is empty")
    })?;
    let winners: Vec<usize> = mass
        .iter()
        .enumerate()
        .filter_map(|(component, &value)| (value == largest).then_some(component))
        .collect();
    if winners.len() != 1 {
        return Err(BackendError::new(
            ErrorCode::GraphUnidentified,
            "graph",
            "multiple connected components tie for largest-component selection",
        ));
    }
    let winner = winners[0];
    let keep: Vec<bool> = active
        .iter()
        .enumerate()
        .map(|(row, &was_active)| was_active && row_component[row] == winner)
        .collect();
    let retained_rows = keep.iter().filter(|&&value| value).count();
    Ok(ComponentResult {
        keep,
        components,
        retained_rows,
    })
}

fn coordinate_graph(input: &CanonicalInput, active: &[bool]) -> Result<Graph> {
    validate_mask(input, active)?;
    let coordinates: BTreeSet<(u32, u32)> = active
        .iter()
        .enumerate()
        .filter_map(|(row, &keep)| keep.then_some((input.worker[row], input.firm[row])))
        .collect();
    let nodes = input.workers().checked_add(input.firms()).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "graph",
            "bipartite node count overflow",
        )
    })?;
    let mut adjacency = vec![Vec::new(); nodes];
    for (edge, &(worker, firm)) in coordinates.iter().enumerate() {
        let worker_node = usize::try_from(worker).expect("dense worker");
        let firm_node = input
            .workers()
            .checked_add(usize::try_from(firm).expect("dense firm"))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "graph",
                    "firm node index overflow",
                )
            })?;
        adjacency[worker_node].push(Arc {
            to: firm_node,
            edge,
        });
        adjacency[firm_node].push(Arc {
            to: worker_node,
            edge,
        });
    }
    for arcs in &mut adjacency {
        arcs.sort_by_key(|arc| (arc.to, arc.edge));
    }
    Ok(Graph {
        adjacency,
        edges: coordinates.len(),
    })
}

fn distinct_firm_counts(input: &CanonicalInput, active: &[bool]) -> Vec<usize> {
    let coordinates: BTreeSet<(u32, u32)> = active
        .iter()
        .enumerate()
        .filter_map(|(row, &keep)| keep.then_some((input.worker[row], input.firm[row])))
        .collect();
    let mut counts = vec![0_usize; input.workers()];
    for (worker, _) in coordinates {
        counts[usize::try_from(worker).expect("dense worker")] += 1;
    }
    counts
}

fn worker_articulations(input: &CanonicalInput, active: &[bool]) -> Result<Vec<bool>> {
    let graph = coordinate_graph(input, active)?;
    let articulation = articulation_vertices(&graph)?;
    Ok(articulation[..input.workers()].to_vec())
}

fn articulation_vertices(graph: &Graph) -> Result<Vec<bool>> {
    let nodes = graph.adjacency.len();
    let mut discovery = vec![0_usize; nodes];
    let mut low = vec![0_usize; nodes];
    let mut parent = vec![None; nodes];
    let mut child_count = vec![0_usize; nodes];
    let mut articulation = vec![false; nodes];
    let mut clock = 0_usize;

    for root in 0..nodes {
        if graph.adjacency[root].is_empty() || discovery[root] != 0 {
            continue;
        }
        clock = clock.checked_add(1).ok_or_else(|| counter_overflow("DFS clock"))?;
        discovery[root] = clock;
        low[root] = clock;
        let mut stack = vec![DfsFrame {
            node: root,
            parent_edge: None,
            next_arc: 0,
        }];

        while !stack.is_empty() {
            let top = stack.len() - 1;
            let node = stack[top].node;
            if stack[top].next_arc < graph.adjacency[node].len() {
                let arc = graph.adjacency[node][stack[top].next_arc];
                stack[top].next_arc += 1;
                if Some(arc.edge) == stack[top].parent_edge {
                    continue;
                }
                if discovery[arc.to] == 0 {
                    parent[arc.to] = Some(node);
                    child_count[node] += 1;
                    clock = clock
                        .checked_add(1)
                        .ok_or_else(|| counter_overflow("DFS clock"))?;
                    discovery[arc.to] = clock;
                    low[arc.to] = clock;
                    stack.push(DfsFrame {
                        node: arc.to,
                        parent_edge: Some(arc.edge),
                        next_arc: 0,
                    });
                } else {
                    low[node] = low[node].min(discovery[arc.to]);
                }
            } else {
                stack.pop();
                if let Some(parent_node) = parent[node] {
                    low[parent_node] = low[parent_node].min(low[node]);
                    if parent[parent_node].is_some() && low[node] >= discovery[parent_node] {
                        articulation[parent_node] = true;
                    }
                }
            }
        }
        articulation[root] = child_count[root] > 1;
    }
    Ok(articulation)
}

fn deletion_bridges(input: &CanonicalInput, active: &[bool]) -> Result<Vec<u32>> {
    validate_mask(input, active)?;
    let mut coordinate: Vec<Option<(u32, u32)>> = vec![None; input.deletion_units()];
    for (row, &keep) in active.iter().enumerate() {
        if !keep {
            continue;
        }
        let deletion = usize::try_from(input.deletion[row]).expect("dense deletion");
        let pair = (input.worker[row], input.firm[row]);
        match coordinate[deletion] {
            None => coordinate[deletion] = Some(pair),
            Some(previous) if previous == pair => {}
            Some(_) => {
                return Err(BackendError::invariant(
                    "graph",
                    "deletion unit crosses worker-firm coordinates",
                ));
            }
        }
    }

    let edges: Vec<(u32, u32, u32)> = coordinate
        .into_iter()
        .enumerate()
        .filter_map(|(deletion, pair)| {
            pair.map(|(worker, firm)| {
                (
                    u32::try_from(deletion).expect("canonical deletion is u32"),
                    worker,
                    firm,
                )
            })
        })
        .collect();
    if edges.is_empty() {
        return Ok(Vec::new());
    }
    let nodes = input.workers().checked_add(input.firms()).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "graph",
            "bipartite node count overflow",
        )
    })?;
    let mut adjacency = vec![Vec::new(); nodes];
    for (edge, &(_, worker, firm)) in edges.iter().enumerate() {
        let worker_node = usize::try_from(worker).expect("dense worker");
        let firm_node = input.workers() + usize::try_from(firm).expect("dense firm");
        adjacency[worker_node].push(Arc {
            to: firm_node,
            edge,
        });
        adjacency[firm_node].push(Arc {
            to: worker_node,
            edge,
        });
    }
    for arcs in &mut adjacency {
        arcs.sort_by_key(|arc| (arc.to, arc.edge));
    }
    let graph = Graph {
        adjacency,
        edges: edges.len(),
    };
    let bridge_flags = bridge_edges(&graph)?;
    Ok(edges
        .iter()
        .enumerate()
        .filter_map(|(edge, &(deletion, _, _))| bridge_flags[edge].then_some(deletion))
        .collect())
}

fn bridge_edges(graph: &Graph) -> Result<Vec<bool>> {
    let nodes = graph.adjacency.len();
    let mut discovery = vec![0_usize; nodes];
    let mut low = vec![0_usize; nodes];
    let mut parent = vec![None; nodes];
    let mut bridge = vec![false; graph.edges];
    let mut clock = 0_usize;

    for root in 0..nodes {
        if graph.adjacency[root].is_empty() || discovery[root] != 0 {
            continue;
        }
        clock = clock.checked_add(1).ok_or_else(|| counter_overflow("DFS clock"))?;
        discovery[root] = clock;
        low[root] = clock;
        let mut stack = vec![DfsFrame {
            node: root,
            parent_edge: None,
            next_arc: 0,
        }];

        while !stack.is_empty() {
            let top = stack.len() - 1;
            let node = stack[top].node;
            if stack[top].next_arc < graph.adjacency[node].len() {
                let arc = graph.adjacency[node][stack[top].next_arc];
                stack[top].next_arc += 1;
                if Some(arc.edge) == stack[top].parent_edge {
                    continue;
                }
                if discovery[arc.to] == 0 {
                    parent[arc.to] = Some(node);
                    clock = clock
                        .checked_add(1)
                        .ok_or_else(|| counter_overflow("DFS clock"))?;
                    discovery[arc.to] = clock;
                    low[arc.to] = clock;
                    stack.push(DfsFrame {
                        node: arc.to,
                        parent_edge: Some(arc.edge),
                        next_arc: 0,
                    });
                } else {
                    low[node] = low[node].min(discovery[arc.to]);
                }
            } else {
                let frame = stack.pop().expect("nonempty stack");
                if let Some(parent_node) = parent[frame.node] {
                    if low[frame.node] > discovery[parent_node] {
                        bridge[frame.parent_edge.expect("nonroot parent edge")] = true;
                    }
                    low[parent_node] = low[parent_node].min(low[frame.node]);
                }
            }
        }
    }
    Ok(bridge)
}

fn active_deletion_ids(input: &CanonicalInput, active: &[bool]) -> BTreeSet<u32> {
    active
        .iter()
        .enumerate()
        .filter_map(|(row, &keep)| keep.then_some(input.deletion[row]))
        .collect()
}

fn intersect_active(active: &mut [bool], keep: &[bool]) {
    for (active_value, &keep_value) in active.iter_mut().zip(keep) {
        *active_value &= keep_value;
    }
}

fn validate_mask(input: &CanonicalInput, active: &[bool]) -> Result<()> {
    if active.len() != input.rows() {
        return Err(BackendError::invalid(
            "graph",
            "active mask has the wrong length",
        ));
    }
    if !active.iter().any(|&keep| keep) {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "active graph is empty",
        ));
    }
    Ok(())
}

fn as_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "graph",
            format!("{label} is not representable as u64"),
        )
    })
}

fn counter_overflow(label: &str) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "graph",
        format!("{label} counter overflow"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn canonical(rows: &[(u64, u64, u64)]) -> CanonicalInput {
        CanonicalInput::from_validated(
            InputColumns {
                worker: rows.iter().map(|row| row.0).collect(),
                firm: rows.iter().map(|row| row.1).collect(),
                deletion: rows.iter().map(|row| row.2).collect(),
                outcome: vec![0.0; rows.len()],
                frequency: vec![1; rows.len()],
                target_weight: vec![1.0; rows.len()],
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical fixture")
    }

    #[test]
    fn complete_two_by_two_is_certified() {
        let input = canonical(&[(1, 1, 1), (1, 2, 2), (2, 1, 3), (2, 2, 4)]);
        let selection = select_match_deletion_graph(&input).expect("selection");
        assert_eq!(selection.active, vec![true; 4]);
        assert_eq!(selection.receipt.retained_deletion_edges, 4);
        assert_eq!(selection.receipt.fixed_point_iterations, 0);
    }

    #[test]
    fn parallel_deletion_units_are_not_bridges() {
        let input = canonical(&[
            (1, 1, 1),
            (1, 1, 2),
            (1, 2, 3),
            (2, 1, 4),
            (2, 2, 5),
        ]);
        let bridges = deletion_bridges(&input, &vec![true; input.rows()]).expect("bridges");
        assert!(bridges.is_empty());
    }

    #[test]
    fn tied_largest_components_fail_closed() {
        let input = canonical(&[
            (1, 1, 1),
            (1, 2, 2),
            (2, 1, 3),
            (2, 2, 4),
            (3, 3, 5),
            (3, 4, 6),
            (4, 3, 7),
            (4, 4, 8),
        ]);
        let error = select_match_deletion_graph(&input).expect_err("tie must fail");
        assert_eq!(error.code, ErrorCode::GraphUnidentified);
    }

    #[test]
    fn cycle_has_no_worker_articulation() {
        let input = canonical(&[
            (1, 1, 1),
            (1, 2, 2),
            (2, 2, 3),
            (2, 3, 4),
            (3, 3, 5),
            (3, 1, 6),
        ]);
        let bad = worker_articulations(&input, &vec![true; input.rows()]).expect("articulations");
        assert_eq!(bad, vec![false, false, false]);
    }
}
