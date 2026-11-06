#![cfg(feature = "parallel")]

use cmg::{
    BuildContext, Cancellation, CmgError, CmgOptions, CmgPreconditioner, Laplacian,
    ParallelOptions, Team,
};
use std::sync::{
    Barrier,
    atomic::{AtomicUsize, Ordering},
};

const THRESHOLD: usize = 65_536;

// Duplicate endpoints, reversed inputs, unequal/tied weights, awkward tails,
// two disconnected components and isolated vertices. Weight order matters.
fn input(len: usize) -> Vec<(usize, usize, f64)> {
    (0..len)
        .map(|i| {
            let component = (i % 2) * 256;
            let u = component + (i * 73 % 255);
            let v = u + 1;
            let w = [1.0, 1e-12, 2.0, 1e9, 0.25, 1.0][(i / 7) % 6];
            if i % 3 == 0 { (v, u, w) } else { (u, v, w) }
        })
        .collect()
}

fn hosted(
    pool: &rayon::ThreadPool,
    ceiling: usize,
    edges: &[(usize, usize, f64)],
    memory: usize,
    cancel: Cancellation<'_>,
) -> Result<Laplacian, CmgError> {
    pool.install(|| {
        let team = Team::new(pool, ceiling, ParallelOptions::default()).unwrap();
        Laplacian::build(
            514,
            edges,
            BuildContext {
                team: Some(team),
                cancel: Some(cancel),
                budget_bytes: Some(memory),
            },
        )
    })
}

/// Counts polls from every thread and cancels from one zero-based poll onwards.
struct StopAt {
    polls: AtomicUsize,
    stop: usize,
}

impl StopAt {
    fn new(stop: usize) -> Self {
        Self {
            polls: AtomicUsize::new(0),
            stop,
        }
    }
    fn query(&self) -> bool {
        self.polls.fetch_add(1, Ordering::SeqCst) >= self.stop
    }
    fn polls(&self) -> usize {
        self.polls.load(Ordering::SeqCst)
    }
}

fn cancelled_phase(
    pool: &rayon::ThreadPool,
    ceiling: usize,
    edges: &[(usize, usize, f64)],
    stop: usize,
) -> &'static str {
    let cancel = StopAt::new(stop);
    let query = || cancel.query();
    match hosted(pool, ceiling, edges, usize::MAX, &query) {
        Err(CmgError::Cancelled { phase }) => phase,
        other => panic!("cancellation at poll {stop} did not stop: {other:?}"),
    }
}

/// Zero-based index of the first poll issued inside the mapping stage. Every
/// earlier poll belongs to serial admission and bank initialization.
fn first_mapping_poll(pool: &rayon::ThreadPool, edges: &[(usize, usize, f64)]) -> usize {
    (0..)
        .find(|&stop| cancelled_phase(pool, 1, edges, stop) == "hosted input edge mapping")
        .unwrap()
}

#[test]
fn threshold_and_large_uneven_graph_preserve_bits_hierarchy_cycle_and_poll_schedule() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let never = || false;
    for len in [THRESHOLD - 1, THRESHOLD, THRESHOLD + 17] {
        let edges = input(len);
        let serial = Laplacian::build(
            514,
            &edges,
            cmg::BuildContext {
                cancel: Some(&never),
                budget_bytes: Some(usize::MAX),
                ..cmg::BuildContext::default()
            },
        )
        .unwrap();
        let options = CmgOptions {
            direct_threshold: 8,
            ..Default::default()
        };
        let old = CmgPreconditioner::build(&serial, options).unwrap();
        assert!(old.hierarchy().levels().len() > 1);
        let probe: Vec<_> = (0..514).map(|i| (i as f64 * 0.37).sin()).collect();
        let rhs = serial.matvec(&probe).unwrap();
        let expected = old.apply(&rhs).unwrap();
        let mut reference = None;
        for ceiling in [1, 2, 4] {
            for _ in 0..2 {
                let count = StopAt::new(usize::MAX);
                let query = || count.query();
                let graph = hosted(&pool, ceiling, &edges, usize::MAX, &query).unwrap();
                assert_eq!(graph, serial);
                assert_eq!(
                    graph
                        .diagonal()
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>(),
                    serial
                        .diagonal()
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>()
                );
                // Hosted stages poll before every logical chunk of eight edges.
                assert_eq!(count.polls() > len / 8, len >= THRESHOLD);
                // The poll schedule depends on the input, never on the ceiling.
                if let Some(polls) = reference {
                    assert_eq!(count.polls(), polls);
                } else {
                    reference = Some(count.polls());
                }
                let new = CmgPreconditioner::build(&graph, options).unwrap();
                assert_eq!(new.hierarchy(), old.hierarchy());
                assert_eq!(new.terminal_factor(), old.terminal_factor());
                assert_eq!(new.repeat_counts(), old.repeat_counts());
                assert_eq!(new.apply(&rhs).unwrap(), expected);
            }
        }
    }
}

#[test]
fn memory_refusal_precedes_polls_and_cancellation_stops_every_phase() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let edges = input(THRESHOLD + 17);
    let count = StopAt::new(usize::MAX);
    let query = || count.query();
    hosted(&pool, 4, &edges, usize::MAX, &query).unwrap();
    let total = count.polls();
    let unreachable = || -> bool { panic!("memory refusal must precede every poll") };
    let required = match hosted(&pool, 4, &edges, 0, &unreachable) {
        Err(CmgError::MemoryBudgetExceeded { required_bytes, .. }) => required_bytes,
        other => panic!("expected scratch refusal: {other:?}"),
    };
    // The existing serial bound is exactly the hosted bound.
    let serial_required = match Laplacian::build(
        514,
        &edges,
        cmg::BuildContext {
            cancel: Some(&unreachable),
            budget_bytes: Some(0),
            ..cmg::BuildContext::default()
        },
    ) {
        Err(CmgError::MemoryBudgetExceeded { required_bytes, .. }) => required_bytes,
        _ => panic!("expected serial bound"),
    };
    assert_eq!(required, serial_required);
    let mapping = first_mapping_poll(&pool, &edges);
    let never = || false;
    for ceiling in [1, 2, 4] {
        hosted(&pool, ceiling, &edges, required, &never).unwrap();
        assert!(matches!(
            hosted(&pool, ceiling, &edges, required - 1, &unreachable),
            Err(CmgError::MemoryBudgetExceeded { .. })
        ));
        // Serial admission, mapping, sorting and the final serial reduction.
        let mut phases = Vec::new();
        for stop in [0, mapping - 1, mapping, mapping + 1, total / 2, total - 1] {
            let phase = cancelled_phase(&pool, ceiling, &edges, stop);
            if !phases.contains(&phase) {
                phases.push(phase);
            }
        }
        assert!(phases.contains(&"hosted input edge mapping"));
        assert!(phases.len() >= 3, "{phases:?}");
    }
    eprintln!(
        "hosted graph fixed witness: input={}, vertices=514, bound={}, polls={}, first mapping poll={}",
        edges.len(),
        required,
        total,
        mapping
    );
}

#[test]
fn invalid_edge_errors_match_serial_order_across_workers() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let never = || false;
    for bad in [
        (514, 0, 1.0),
        (1, 1, 1.0),
        (0, 1, -2.0),
        (0, 1, f64::INFINITY),
    ] {
        let mut edges = input(THRESHOLD + 17);
        edges[97] = bad;
        edges[60_000] = (2, 2, 0.0);
        let serial = Laplacian::build(
            514,
            &edges,
            cmg::BuildContext {
                cancel: Some(&never),
                budget_bytes: Some(usize::MAX),
                ..cmg::BuildContext::default()
            },
        )
        .unwrap_err();
        for ceiling in [1, 2, 4] {
            assert_eq!(
                hosted(&pool, ceiling, &edges, usize::MAX, &never).unwrap_err(),
                serial
            );
        }
    }
}

#[test]
fn mapping_reaches_admitted_lanes_and_cancellation_or_panic_joins_them() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let edges = input(THRESHOLD);
    // Arm the test-only query at the first mapping poll, after serial admission.
    // Rendezvous verifies actual independent graph mapping lanes, not only an
    // empty framework stage. It deliberately strengthens the normal cheap query.
    let mapping = first_mapping_poll(&pool, &edges);
    for ceiling in [1, 2, 4] {
        for panic_first in [false, true] {
            let calls = AtomicUsize::new(0);
            let arrived = AtomicUsize::new(0);
            let barrier = Barrier::new(ceiling);
            let query = || {
                let call = calls.fetch_add(1, Ordering::SeqCst);
                if call < mapping {
                    return false;
                }
                let armed = call - mapping;
                if armed < ceiling {
                    arrived.fetch_add(1, Ordering::SeqCst);
                    barrier.wait();
                    if panic_first && armed == 0 {
                        panic!("hosted mapping query panic");
                    }
                }
                !panic_first && armed >= ceiling
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                hosted(&pool, ceiling, &edges, usize::MAX, &query)
            }));
            assert_eq!(arrived.load(Ordering::SeqCst), ceiling);
            if panic_first {
                assert!(result.is_err());
            } else {
                assert!(matches!(result.unwrap(), Err(CmgError::Cancelled { .. })));
            }
            // Joined pool and all borrowed input remain usable after either exit.
            assert_eq!(pool.install(|| 7 + 9), 16);
        }
    }
}
