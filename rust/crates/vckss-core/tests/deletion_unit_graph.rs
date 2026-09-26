// SPDX-License-Identifier: GPL-3.0-only
//! Small exhaustive oracle using vertex/edge removal and reachability, not
//! production Tarjan traversals or production support counters.
use std::collections::BTreeSet;
use vckss_core::graph::select_match_deletion_graph;
use vckss_core::problem::CanonicalInput;
use vckss_core::types::InputColumns;

fn components(
    input: &CanonicalInput,
    active: &[bool],
    drop_worker: Option<usize>,
    drop_unit: Option<u32>,
) -> Vec<Vec<usize>> {
    let workers = input.workers();
    let n = workers + input.firms();
    let mut present = vec![false; n];
    let mut adjacency = vec![vec![false; n]; n];
    for (row, &keep) in active.iter().enumerate() {
        if !keep {
            continue;
        }
        let w = input.worker[row] as usize;
        let f = workers + input.firm[row] as usize;
        present[w] = true;
        present[f] = true;
        if drop_worker != Some(w) && drop_unit != Some(input.deletion[row]) {
            adjacency[w][f] = true;
            adjacency[f][w] = true;
        }
    }
    if let Some(w) = drop_worker {
        present[w] = false;
    }
    let mut result = Vec::new();
    for start in 0..n {
        if !present[start] {
            continue;
        }
        let mut reached = vec![start];
        present[start] = false;
        let mut position = 0;
        while position < reached.len() {
            let node = reached[position];
            for next in 0..n {
                if present[next] && adjacency[node][next] {
                    present[next] = false;
                    reached.push(next);
                }
            }
            position += 1;
        }
        result.push(reached);
    }
    result
}

fn largest(input: &CanonicalInput, active: &mut [bool]) -> Result<(), ()> {
    let groups = components(input, active, None, None);
    let ranks: Vec<_> = groups
        .iter()
        .map(|nodes| {
            let firms = nodes
                .iter()
                .filter(|&&node| node >= input.workers())
                .count();
            let mass: u64 = active
                .iter()
                .enumerate()
                .filter(|&(row, keep)| *keep && nodes.contains(&(input.worker[row] as usize)))
                .map(|(row, _)| input.frequency[row])
                .sum();
            (firms, mass)
        })
        .collect();
    let best = ranks.iter().max().ok_or(())?;
    if ranks.iter().filter(|rank| *rank == best).count() != 1 {
        return Err(());
    }
    let selected = &groups[ranks.iter().position(|rank| rank == best).unwrap()];
    for (row, keep) in active.iter_mut().enumerate() {
        *keep &= selected.contains(&(input.worker[row] as usize));
    }
    Ok(())
}

fn reference(input: &CanonicalInput) -> Result<Vec<bool>, ()> {
    let mut active = vec![true; input.rows()];
    largest(input, &mut active)?;
    for _ in 0..=input.rows() + input.workers() {
        largest(input, &mut active)?;
        let units: Vec<BTreeSet<_>> = (0..input.workers())
            .map(|worker| {
                active
                    .iter()
                    .enumerate()
                    .filter(|&(row, keep)| *keep && input.worker[row] as usize == worker)
                    .map(|(row, _)| input.deletion[row])
                    .collect()
            })
            .collect();
        let insufficient: Vec<_> = units.iter().map(|u| u.len() == 1).collect();
        if insufficient.iter().any(|&x| x) {
            for (row, keep) in active.iter_mut().enumerate() {
                *keep &= !insufficient[input.worker[row] as usize];
            }
            continue;
        }
        let cut_workers: Vec<_> = (0..input.workers())
            .map(|worker| {
                !units[worker].is_empty()
                    && components(input, &active, Some(worker), None).len() > 1
            })
            .collect();
        if cut_workers.iter().any(|&x| x) {
            for (row, keep) in active.iter_mut().enumerate() {
                *keep &= !cut_workers[input.worker[row] as usize];
            }
            continue;
        }
        let cut_units: BTreeSet<_> = units
            .iter()
            .flatten()
            .copied()
            .filter(|&unit| components(input, &active, None, Some(unit)).len() > 1)
            .collect();
        if cut_units.is_empty() {
            return Ok(active);
        }
        for (row, keep) in active.iter_mut().enumerate() {
            *keep &= !cut_units.contains(&input.deletion[row]);
        }
    }
    panic!("reference failed to reach a finite deletion fixed point");
}

#[test]
fn all_small_parallel_multigraphs_match_independent_pruning() {
    // Every 3-worker/2-firm support, with zero, one, or two blocks per cell.
    // Repeated rows and unequal literal masses distinguish blocks from copies.
    for code in 1_u32..3_u32.pow(6) {
        let mut value = code;
        let mut columns = InputColumns {
            worker: Vec::new(),
            firm: Vec::new(),
            deletion: Vec::new(),
            outcome: Vec::new(),
            frequency: Vec::new(),
            target_weight: Vec::new(),
            controls: Vec::new(),
        };
        for cell in 0_u64..6 {
            let blocks = value % 3;
            value /= 3;
            for block in 0..u64::from(blocks) {
                for repeated in 0..=block {
                    columns.worker.push(100 + cell / 2);
                    columns.firm.push(200 + cell % 2);
                    columns.deletion.push(300 + 2 * cell + block);
                    columns.outcome.push(0.0);
                    columns.frequency.push(1 + (cell + repeated) % 3);
                    columns.target_weight.push(1.0);
                }
            }
        }
        let input = CanonicalInput::from_validated(columns.validate().unwrap()).unwrap();
        let expected = reference(&input);
        let actual = select_match_deletion_graph(&input);
        match (expected, actual) {
            (Ok(mask), Ok(selected)) => assert_eq!(mask, selected.active, "graph {code}"),
            (Err(()), Err(_)) => (),
            (left, right) => panic!("graph {code}: oracle {left:?}, production {right:?}"),
        }
    }
}
