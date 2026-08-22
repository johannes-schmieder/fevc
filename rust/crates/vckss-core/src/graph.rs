// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{BTreeSet, VecDeque};

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::problem::CanonicalInput;

const UNVISITED: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
    select_match_deletion_graph_with_interrupt(input, &mut NeverInterrupt)
}

/// Select the MATLAB-compatible leave-one-observation component.
///
/// Observation deletion differs from match deletion in two important ways:
/// stayers remain in the target population, and the deletion-rank graph gate
/// removes workers with only one *physical* observation rather than workers
/// observed at only one firm.  Coordinate bridges are not deletion units in
/// this mode, so the fixed point ends after the articulation-worker gate.
pub fn select_observation_deletion_graph(input: &CanonicalInput) -> Result<GraphSelection> {
    select_observation_deletion_graph_with_interrupt(input, &mut NeverInterrupt)
}

pub fn select_observation_deletion_graph_with_interrupt(
    input: &CanonicalInput,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GraphSelection> {
    interrupt.checkpoint("graph_observation_entry")?;
    if input.rows() == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "command sample is empty",
        ));
    }

    let mut active = vec![true; input.rows()];
    let mut receipt = GraphSelectionReceipt {
        input_rows: as_u64(input.rows(), "input row count")?,
        input_physical_mass: input.physical_total,
        ..GraphSelectionReceipt::default()
    };

    let initial = largest_component(input, &active, interrupt)?;
    receipt.initial_components = as_u64(initial.components, "component count")?;
    receipt.maximum_components = receipt.initial_components;
    receipt.initial_component_rows = as_u64(initial.retained_rows, "component row count")?;
    intersect_active(&mut active, &initial.keep, interrupt)?;
    receipt.mover_input_rows = as_u64(
        count_true(&active, interrupt, "graph_observation_initial_count")?,
        "observation target row count",
    )?;

    let initial_graph = coordinate_graph(input, &active, interrupt)?;
    receipt.initial_deletion_edges = as_u64(initial_graph.edges, "coordinate edge count")?;
    let iteration_bound = input
        .workers()
        .checked_add(1)
        .ok_or_else(|| counter_overflow("observation iteration bound"))?;

    loop {
        interrupt.checkpoint("graph_observation_fixed_point")?;
        if receipt.fixed_point_iterations > as_u64(iteration_bound, "iteration bound")? {
            return Err(BackendError::new(
                ErrorCode::GraphCertificateFailed,
                "graph",
                "observation-deletion pruning exceeded its finite iteration bound",
            ));
        }

        let component = largest_component(input, &active, interrupt)?;
        receipt.maximum_components = receipt
            .maximum_components
            .max(as_u64(component.components, "component count")?);
        intersect_active(&mut active, &component.keep, interrupt)?;
        if count_true(&active, interrupt, "graph_observation_active_count")? == 0 {
            return Err(BackendError::new(
                ErrorCode::GraphEmpty,
                "graph",
                "no component remains after observation-deletion pruning",
            ));
        }

        let physical = worker_physical_counts(input, &active, interrupt)?;
        let mut singleton = Vec::with_capacity(physical.len());
        for (worker, &count) in physical.iter().enumerate() {
            checkpoint_chunk(interrupt, worker, "graph_observation_degree_scan")?;
            singleton.push(count == 1);
        }
        let removed = count_true(&singleton, interrupt, "graph_observation_degree_count")?;
        if removed > 0 {
            for (row, keep) in active.iter_mut().enumerate() {
                checkpoint_chunk(interrupt, row, "graph_observation_degree_prune")?;
                if *keep {
                    let worker = usize::try_from(input.worker[row]).expect("dense worker");
                    *keep = !singleton[worker];
                }
            }
            receipt.insufficient_workers_removed = receipt
                .insufficient_workers_removed
                .checked_add(as_u64(removed, "removed worker count")?)
                .ok_or_else(|| counter_overflow("insufficient workers"))?;
            receipt.degree_iterations += 1;
            receipt.fixed_point_iterations += 1;
            continue;
        }

        let articulations = worker_articulations(input, &active, interrupt)?;
        let removed = count_true(
            &articulations,
            interrupt,
            "graph_observation_articulation_count",
        )?;
        if removed == 0 {
            break;
        }
        for (row, keep) in active.iter_mut().enumerate() {
            checkpoint_chunk(interrupt, row, "graph_observation_articulation_prune")?;
            if *keep {
                let worker = usize::try_from(input.worker[row]).expect("dense worker");
                *keep = !articulations[worker];
            }
        }
        receipt.articulation_workers_removed = receipt
            .articulation_workers_removed
            .checked_add(as_u64(removed, "articulation count")?)
            .ok_or_else(|| counter_overflow("articulation workers"))?;
        receipt.articulation_iterations += 1;
        receipt.fixed_point_iterations += 1;
    }

    let final_component = largest_component(input, &active, interrupt)?;
    receipt.maximum_components = receipt
        .maximum_components
        .max(as_u64(final_component.components, "component count")?);
    intersect_active(&mut active, &final_component.keep, interrupt)?;

    let final_physical = worker_physical_counts(input, &active, interrupt)?;
    if final_physical.contains(&1) {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final observation-deletion graph still contains a physical-singleton worker",
        ));
    }
    let final_articulations = worker_articulations(input, &active, interrupt)?;
    if count_true(
        &final_articulations,
        interrupt,
        "graph_observation_final_articulation_count",
    )? > 0
    {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final observation-deletion graph still contains a worker articulation",
        ));
    }

    let retained_rows = count_true(&active, interrupt, "graph_observation_retained_count")?;
    if retained_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "no component remains after observation-deletion graph pruning",
        ));
    }
    receipt.retained_rows = as_u64(retained_rows, "retained row count")?;
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_observation_retained_reconcile")?;
        if keep {
            receipt.retained_physical_mass = receipt
                .retained_physical_mass
                .checked_add(input.frequency[row])
                .ok_or_else(|| counter_overflow("retained physical mass"))?;
        }
    }
    receipt.retained_deletion_edges = as_u64(
        coordinate_graph(input, &active, interrupt)?.edges,
        "retained coordinate edges",
    )?;
    interrupt.checkpoint("graph_observation_final")?;
    Ok(GraphSelection { active, receipt })
}

pub fn select_match_deletion_graph_with_interrupt(
    input: &CanonicalInput,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GraphSelection> {
    interrupt.checkpoint("graph_entry")?;
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

    let initial = largest_component(input, &active, interrupt)?;
    receipt.initial_components = as_u64(initial.components, "component count")?;
    receipt.maximum_components = receipt.initial_components;
    receipt.initial_component_rows = as_u64(initial.retained_rows, "component row count")?;
    intersect_active(&mut active, &initial.keep, interrupt)?;

    let worker_firms = distinct_firm_counts(input, &active, interrupt)?;
    for (row, keep) in active.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_mover_filter")?;
        if *keep {
            let worker = usize::try_from(input.worker[row]).expect("dense worker");
            *keep = worker_firms[worker] > 1;
        }
    }
    let mover_rows = count_true(&active, interrupt, "graph_mover_count")?;
    receipt.mover_input_rows = as_u64(mover_rows, "mover row count")?;
    if mover_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "no observations remain in the mover target population",
        ));
    }

    let mover_component = largest_component(input, &active, interrupt)?;
    receipt.maximum_components = receipt
        .maximum_components
        .max(as_u64(mover_component.components, "component count")?);
    intersect_active(&mut active, &mover_component.keep, interrupt)?;

    let initial_edges = active_deletion_ids(input, &active, interrupt)?.len();
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
        interrupt.checkpoint("graph_fixed_point")?;
        if receipt.fixed_point_iterations > iteration_bound {
            return Err(BackendError::new(
                ErrorCode::GraphCertificateFailed,
                "graph",
                "sample fixed-point pruning exceeded its finite iteration bound",
            ));
        }

        let component = largest_component(input, &active, interrupt)?;
        receipt.maximum_components = receipt
            .maximum_components
            .max(as_u64(component.components, "component count")?);
        intersect_active(&mut active, &component.keep, interrupt)?;
        if count_true(&active, interrupt, "graph_active_count")? == 0 {
            return Err(BackendError::new(
                ErrorCode::GraphEmpty,
                "graph",
                "no component remains after fixed-point pruning",
            ));
        }

        let worker_firms = distinct_firm_counts(input, &active, interrupt)?;
        let mut insufficient = Vec::with_capacity(worker_firms.len());
        for (worker, &count) in worker_firms.iter().enumerate() {
            checkpoint_chunk(interrupt, worker, "graph_degree_scan")?;
            insufficient.push(count == 1);
        }
        let removed_workers = count_true(&insufficient, interrupt, "graph_degree_count")?;
        if removed_workers > 0 {
            for (row, keep) in active.iter_mut().enumerate() {
                checkpoint_chunk(interrupt, row, "graph_degree_prune")?;
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

        let articulations = worker_articulations(input, &active, interrupt)?;
        let articulation_count = count_true(&articulations, interrupt, "graph_articulation_count")?;
        if articulation_count > 0 {
            for (row, keep) in active.iter_mut().enumerate() {
                checkpoint_chunk(interrupt, row, "graph_articulation_prune")?;
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

        let bridges = deletion_bridges(input, &active, interrupt)?;
        if bridges.is_empty() {
            break;
        }
        let bridge_set = bridge_deletion_set(&bridges, interrupt)?;
        let mut removed_rows = 0_usize;
        for (row, &keep) in active.iter().enumerate() {
            checkpoint_chunk(interrupt, row, "graph_bridge_count")?;
            removed_rows += usize::from(keep && bridge_set.contains(&input.deletion[row]));
        }
        for (row, keep) in active.iter_mut().enumerate() {
            checkpoint_chunk(interrupt, row, "graph_bridge_prune")?;
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

    let final_component = largest_component(input, &active, interrupt)?;
    receipt.maximum_components = receipt
        .maximum_components
        .max(as_u64(final_component.components, "component count")?);
    intersect_active(&mut active, &final_component.keep, interrupt)?;

    let final_bridges = deletion_bridges(input, &active, interrupt)?;
    if !final_bridges.is_empty() {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final retained deletion-unit multigraph still contains a bridge",
        ));
    }
    let final_articulations = worker_articulations(input, &active, interrupt)?;
    if count_true(
        &final_articulations,
        interrupt,
        "graph_final_articulation_count",
    )? > 0
    {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final retained graph still contains a worker articulation",
        ));
    }
    let final_worker_firms = distinct_firm_counts(input, &active, interrupt)?;
    let mut one_firm = false;
    for (worker, &count) in final_worker_firms.iter().enumerate() {
        checkpoint_chunk(interrupt, worker, "graph_final_degree_scan")?;
        one_firm |= count == 1;
    }
    if one_firm {
        return Err(BackendError::new(
            ErrorCode::GraphCertificateFailed,
            "graph",
            "final retained graph still contains a one-firm worker",
        ));
    }

    let retained_rows = count_true(&active, interrupt, "graph_retained_count")?;
    if retained_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "no component remains after fixed-point graph pruning",
        ));
    }
    receipt.retained_rows = as_u64(retained_rows, "retained row count")?;
    receipt.retained_physical_mass = 0;
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_retained_reconcile")?;
        if keep {
            receipt.retained_physical_mass = receipt
                .retained_physical_mass
                .checked_add(input.frequency[row])
                .ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "graph",
                        "retained physical mass overflow",
                    )
                })?;
        }
    }
    receipt.retained_deletion_edges = as_u64(
        active_deletion_ids(input, &active, interrupt)?.len(),
        "retained deletion edges",
    )?;

    interrupt.checkpoint("graph_final")?;
    Ok(GraphSelection { active, receipt })
}

fn largest_component(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentResult> {
    validate_mask(input, active, interrupt)?;
    let graph = coordinate_graph(input, active, interrupt)?;
    let mut label = vec![UNVISITED; graph.adjacency.len()];
    let mut components = 0_usize;
    let mut queue = VecDeque::new();

    for root in 0..graph.adjacency.len() {
        checkpoint_chunk(interrupt, root, "graph_component_roots")?;
        if graph.adjacency[root].is_empty() || label[root] != UNVISITED {
            continue;
        }
        label[root] = components;
        queue.push_back(root);
        while let Some(node) = queue.pop_front() {
            for (arc_index, arc) in graph.adjacency[node].iter().enumerate() {
                checkpoint_chunk(interrupt, arc_index, "graph_component_bfs")?;
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

    let mut firms = vec![0_usize; components];
    let mut mass = vec![0_u64; components];
    let mut row_component = vec![UNVISITED; input.rows()];
    for firm in 0..input.firms() {
        checkpoint_chunk(interrupt, firm, "graph_component_firms")?;
        let node = input
            .workers()
            .checked_add(firm)
            .ok_or_else(|| counter_overflow("firm node index"))?;
        let component = label[node];
        if component != UNVISITED {
            firms[component] = firms[component]
                .checked_add(1)
                .ok_or_else(|| counter_overflow("component firm count"))?;
        }
    }
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_component_rows")?;
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
    let largest = maximum_component_rank(&firms, &mass, interrupt)?.ok_or_else(|| {
        BackendError::new(ErrorCode::GraphEmpty, "graph", "component rank is empty")
    })?;
    let mut winners = Vec::new();
    for (component, (&firm_count, &component_mass)) in firms.iter().zip(&mass).enumerate() {
        checkpoint_chunk(interrupt, component, "graph_component_winners")?;
        if (firm_count, component_mass) == largest {
            winners.push(component);
        }
    }
    if winners.len() != 1 {
        return Err(BackendError::new(
            ErrorCode::GraphUnidentified,
            "graph",
            "multiple connected components tie for largest-component selection",
        ));
    }
    let winner = winners[0];
    let mut keep = Vec::with_capacity(active.len());
    for (row, &was_active) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_component_keep")?;
        keep.push(was_active && row_component[row] == winner);
    }
    let retained_rows = count_true(&keep, interrupt, "graph_component_keep_count")?;
    Ok(ComponentResult {
        keep,
        components,
        retained_rows,
    })
}

fn coordinate_graph(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Graph> {
    validate_mask(input, active, interrupt)?;
    let mut coordinates = BTreeSet::new();
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_coordinate_collect")?;
        if keep {
            coordinates.insert((input.worker[row], input.firm[row]));
        }
    }
    let nodes = input.workers().checked_add(input.firms()).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "graph",
            "bipartite node count overflow",
        )
    })?;
    let mut adjacency = vec![Vec::new(); nodes];
    for (edge, &(worker, firm)) in coordinates.iter().enumerate() {
        checkpoint_chunk(interrupt, edge, "graph_coordinate_edges")?;
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
    for (node, arcs) in adjacency.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, node, "graph_coordinate_adjacency")?;
        stable_sort_by_with_interrupt(
            arcs,
            |left, right| (left.to, left.edge).cmp(&(right.to, right.edge)),
            interrupt,
            "graph_coordinate_sort",
        )?;
    }
    Ok(Graph {
        adjacency,
        edges: coordinates.len(),
    })
}

fn distinct_firm_counts(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut coordinates = BTreeSet::new();
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_distinct_firms")?;
        if keep {
            coordinates.insert((input.worker[row], input.firm[row]));
        }
    }
    let mut counts = vec![0_usize; input.workers()];
    for (coordinate, (worker, _)) in coordinates.into_iter().enumerate() {
        checkpoint_chunk(interrupt, coordinate, "graph_distinct_firm_consume")?;
        counts[usize::try_from(worker).expect("dense worker")] += 1;
    }
    Ok(counts)
}

fn worker_physical_counts(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    validate_mask(input, active, interrupt)?;
    let mut counts = vec![0_u64; input.workers()];
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_observation_physical_count")?;
        if keep {
            let worker = usize::try_from(input.worker[row]).expect("dense worker");
            counts[worker] = counts[worker]
                .checked_add(input.frequency[row])
                .ok_or_else(|| counter_overflow("worker physical mass"))?;
        }
    }
    Ok(counts)
}

fn worker_articulations(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<bool>> {
    let graph = coordinate_graph(input, active, interrupt)?;
    let articulation = articulation_vertices(&graph, interrupt)?;
    let mut workers = Vec::with_capacity(input.workers());
    for (worker, &is_articulation) in articulation[..input.workers()].iter().enumerate() {
        checkpoint_chunk(interrupt, worker, "graph_worker_articulation_copy")?;
        workers.push(is_articulation);
    }
    Ok(workers)
}

fn bridge_deletion_set(
    bridges: &[u32],
    interrupt: &mut dyn InterruptCheck,
) -> Result<BTreeSet<u32>> {
    let mut bridge_set = BTreeSet::new();
    for (bridge, &deletion) in bridges.iter().enumerate() {
        checkpoint_chunk(interrupt, bridge, "graph_bridge_set")?;
        bridge_set.insert(deletion);
    }
    Ok(bridge_set)
}

fn maximum_component_rank(
    firms: &[usize],
    mass: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Option<(usize, u64)>> {
    debug_assert_eq!(firms.len(), mass.len());
    let mut maximum: Option<(usize, u64)> = None;
    for (component, (&firm_count, &component_mass)) in firms.iter().zip(mass).enumerate() {
        checkpoint_chunk(interrupt, component, "graph_component_max")?;
        let rank = (firm_count, component_mass);
        maximum = Some(maximum.map_or(rank, |current| current.max(rank)));
    }
    Ok(maximum)
}

fn articulation_vertices(graph: &Graph, interrupt: &mut dyn InterruptCheck) -> Result<Vec<bool>> {
    let nodes = graph.adjacency.len();
    let mut discovery = vec![0_usize; nodes];
    let mut low = vec![0_usize; nodes];
    let mut parent = vec![None; nodes];
    let mut child_count = vec![0_usize; nodes];
    let mut articulation = vec![false; nodes];
    let mut clock = 0_usize;

    for root in 0..nodes {
        checkpoint_chunk(interrupt, root, "graph_articulation_roots")?;
        if graph.adjacency[root].is_empty() || discovery[root] != 0 {
            continue;
        }
        clock = clock
            .checked_add(1)
            .ok_or_else(|| counter_overflow("DFS clock"))?;
        discovery[root] = clock;
        low[root] = clock;
        let mut stack = vec![DfsFrame {
            node: root,
            parent_edge: None,
            next_arc: 0,
        }];

        while !stack.is_empty() {
            interrupt.checkpoint("graph_articulation_dfs")?;
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

fn deletion_bridges(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u32>> {
    validate_mask(input, active, interrupt)?;
    let mut coordinate: Vec<Option<(u32, u32)>> = vec![None; input.deletion_units()];
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_bridge_coordinates")?;
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

    let mut edges = Vec::new();
    for (deletion, pair) in coordinate.into_iter().enumerate() {
        checkpoint_chunk(interrupt, deletion, "graph_bridge_edges")?;
        if let Some((worker, firm)) = pair {
            edges.push((
                u32::try_from(deletion).expect("canonical deletion is u32"),
                worker,
                firm,
            ));
        }
    }
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
        checkpoint_chunk(interrupt, edge, "graph_bridge_adjacency")?;
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
    for (node, arcs) in adjacency.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, node, "graph_bridge_nodes")?;
        stable_sort_by_with_interrupt(
            arcs,
            |left, right| (left.to, left.edge).cmp(&(right.to, right.edge)),
            interrupt,
            "graph_bridge_sort",
        )?;
    }
    let graph = Graph {
        adjacency,
        edges: edges.len(),
    };
    let bridge_flags = bridge_edges(&graph, interrupt)?;
    let mut output = Vec::new();
    for (edge, &(deletion, _, _)) in edges.iter().enumerate() {
        checkpoint_chunk(interrupt, edge, "graph_bridge_collect")?;
        if bridge_flags[edge] {
            output.push(deletion);
        }
    }
    Ok(output)
}

fn bridge_edges(graph: &Graph, interrupt: &mut dyn InterruptCheck) -> Result<Vec<bool>> {
    let nodes = graph.adjacency.len();
    let mut discovery = vec![0_usize; nodes];
    let mut low = vec![0_usize; nodes];
    let mut parent = vec![None; nodes];
    let mut bridge = vec![false; graph.edges];
    let mut clock = 0_usize;

    for root in 0..nodes {
        checkpoint_chunk(interrupt, root, "graph_bridge_roots")?;
        if graph.adjacency[root].is_empty() || discovery[root] != 0 {
            continue;
        }
        clock = clock
            .checked_add(1)
            .ok_or_else(|| counter_overflow("DFS clock"))?;
        discovery[root] = clock;
        low[root] = clock;
        let mut stack = vec![DfsFrame {
            node: root,
            parent_edge: None,
            next_arc: 0,
        }];

        while !stack.is_empty() {
            interrupt.checkpoint("graph_bridge_dfs")?;
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

fn active_deletion_ids(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<BTreeSet<u32>> {
    let mut output = BTreeSet::new();
    for (row, &keep) in active.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "graph_active_deletions")?;
        if keep {
            output.insert(input.deletion[row]);
        }
    }
    Ok(output)
}

fn intersect_active(
    active: &mut [bool],
    keep: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    for (row, (active_value, &keep_value)) in active.iter_mut().zip(keep).enumerate() {
        checkpoint_chunk(interrupt, row, "graph_mask_intersection")?;
        *active_value &= keep_value;
    }
    Ok(())
}

fn validate_mask(
    input: &CanonicalInput,
    active: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if active.len() != input.rows() {
        return Err(BackendError::invalid(
            "graph",
            "active mask has the wrong length",
        ));
    }
    if count_true(active, interrupt, "graph_validate_mask")? == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "graph",
            "active graph is empty",
        ));
    }
    Ok(())
}

fn count_true(
    values: &[bool],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<usize> {
    let mut count = 0_usize;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        count += usize::from(value);
    }
    Ok(count)
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
    use crate::error::BackendError;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    #[derive(Debug)]
    struct BreakOnPhase {
        phase: &'static str,
        hits: usize,
        stop_at: usize,
    }

    impl BreakOnPhase {
        fn new(phase: &'static str, stop_at: usize) -> Self {
            Self {
                phase,
                hits: 0,
                stop_at,
            }
        }
    }

    impl InterruptCheck for BreakOnPhase {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.phase {
                self.hits += 1;
                if self.hits == self.stop_at {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "injected large graph-pass break",
                    ));
                }
            }
            Ok(())
        }
    }

    fn canonical_with_frequency(rows: &[(u64, u64, u64, u64)]) -> CanonicalInput {
        CanonicalInput::from_validated(
            InputColumns {
                worker: rows.iter().map(|row| row.0).collect(),
                firm: rows.iter().map(|row| row.1).collect(),
                deletion: rows.iter().map(|row| row.2).collect(),
                outcome: vec![0.0; rows.len()],
                frequency: rows.iter().map(|row| row.3).collect(),
                target_weight: vec![1.0; rows.len()],
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical fixture")
    }

    fn canonical(rows: &[(u64, u64, u64)]) -> CanonicalInput {
        let weighted = rows
            .iter()
            .map(|&(worker, firm, deletion)| (worker, firm, deletion, 1))
            .collect::<Vec<_>>();
        canonical_with_frequency(&weighted)
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
    fn observation_deletion_retains_connected_stayers_with_multiple_copies() {
        let input = canonical_with_frequency(&[
            (1, 1, 1, 2),
            (2, 1, 2, 1),
            (2, 2, 3, 1),
            (3, 1, 4, 1),
            (3, 2, 5, 1),
        ]);
        let selection = select_observation_deletion_graph(&input).expect("selection");
        assert_eq!(selection.active, vec![true; 5]);
        assert_eq!(selection.receipt.mover_input_rows, 5);
        assert_eq!(selection.receipt.retained_physical_mass, 6);
        assert_eq!(selection.receipt.fixed_point_iterations, 0);
    }

    #[test]
    fn observation_deletion_prunes_physical_singletons_to_a_fixed_point() {
        let input = canonical_with_frequency(&[
            (1, 1, 1, 1),
            (2, 1, 2, 1),
            (2, 2, 3, 1),
            (3, 1, 4, 1),
            (3, 2, 5, 1),
        ]);
        let selection = select_observation_deletion_graph(&input).expect("selection");
        assert_eq!(selection.active, vec![false, true, true, true, true]);
        assert_eq!(selection.receipt.insufficient_workers_removed, 1);
        assert_eq!(selection.receipt.degree_iterations, 1);
        assert_eq!(selection.receipt.fixed_point_iterations, 1);
    }

    #[test]
    fn parallel_deletion_units_are_not_bridges() {
        let input = canonical(&[(1, 1, 1), (1, 1, 2), (1, 2, 3), (2, 1, 4), (2, 2, 5)]);
        let bridges = deletion_bridges(&input, &vec![true; input.rows()], &mut NeverInterrupt)
            .expect("bridges");
        assert!(bridges.is_empty());
    }

    #[test]
    fn firm_count_precedes_physical_mass_in_component_rank() {
        let input = canonical_with_frequency(&[
            (1, 1, 1, 100),
            (1, 2, 2, 100),
            (2, 1, 3, 100),
            (2, 2, 4, 100),
            (3, 3, 5, 1),
            (3, 4, 6, 1),
            (3, 5, 7, 1),
            (4, 3, 8, 1),
            (4, 4, 9, 1),
            (4, 5, 10, 1),
        ]);
        let selection = select_match_deletion_graph(&input).expect("selection");
        assert_eq!(
            selection.active,
            vec![false, false, false, false, true, true, true, true, true, true]
        );
        assert_eq!(selection.receipt.retained_physical_mass, 6);
    }

    #[test]
    fn tied_firm_count_and_physical_mass_fail_closed() {
        let input = canonical_with_frequency(&[
            (1, 1, 1, 3),
            (1, 2, 2, 1),
            (2, 1, 3, 1),
            (2, 2, 4, 1),
            (3, 3, 5, 1),
            (3, 4, 6, 1),
            (4, 3, 7, 2),
            (4, 4, 8, 2),
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
        let bad = worker_articulations(&input, &vec![true; input.rows()], &mut NeverInterrupt)
            .expect("articulations");
        assert_eq!(bad, vec![false, false, false]);
    }

    #[test]
    fn long_graph_iterator_and_copy_passes_are_interruptible() {
        const ITEMS: usize = 9_001;

        let bridges = (0..ITEMS as u32).collect::<Vec<_>>();
        let mut interrupt = BreakOnPhase::new("graph_bridge_set", 2);
        let error = bridge_deletion_set(&bridges, &mut interrupt)
            .expect_err("bridge-set construction must break within bounded work");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(interrupt.hits, 2);

        let firms = (0..ITEMS).collect::<Vec<_>>();
        let mass = (0..ITEMS as u64).collect::<Vec<_>>();
        let mut interrupt = BreakOnPhase::new("graph_component_max", 2);
        let error = maximum_component_rank(&firms, &mass, &mut interrupt)
            .expect_err("component maximum must break within bounded work");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(interrupt.hits, 2);

        let rows = (0..ITEMS)
            .map(|row| (row as u64 + 1, row as u64 + 1, row as u64 + 1))
            .collect::<Vec<_>>();
        let input = canonical(&rows);
        let active = vec![true; ITEMS];

        let mut interrupt = BreakOnPhase::new("graph_distinct_firm_consume", 2);
        let error = distinct_firm_counts(&input, &active, &mut interrupt)
            .expect_err("coordinate consumption must break within bounded work");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(interrupt.hits, 2);

        let mut interrupt = BreakOnPhase::new("graph_worker_articulation_copy", 2);
        let error = worker_articulations(&input, &active, &mut interrupt)
            .expect_err("worker-articulation copy must break within bounded work");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(interrupt.hits, 2);
    }
}
