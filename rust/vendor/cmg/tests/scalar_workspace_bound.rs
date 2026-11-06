//! Dimension-only admission compared with actual scalar workspace layouts.
use cmg::{CmgError, CmgOptions, CmgPreconditioner, Laplacian, TerminalReason};

fn check(graph: &Laplacian, options: CmgOptions) -> CmgPreconditioner {
    let cmg = CmgPreconditioner::build(graph, options).unwrap();
    let actual = cmg.workspace_allocation_bytes().unwrap();
    let workspace = cmg.workspace();
    assert!(
        workspace.byte_len() <= actual,
        "descriptor storage is included"
    );
    assert!(workspace.level_count() <= options.max_levels);
    assert!(
        workspace
            .dimensions()
            .iter()
            .all(|&n| n <= graph.vertex_count())
    );
    assert!(
        cmg.terminal_factor()
            .is_none_or(|factor| factor.active_dimension() <= graph.vertex_count())
    );
    for vertices in [graph.vertex_count(), graph.vertex_count() + 7] {
        for max_levels in [options.max_levels, options.max_levels + 3] {
            let bound = CmgPreconditioner::scalar_workspace_allocation_bound(
                vertices,
                CmgOptions {
                    max_levels,
                    ..options
                },
            )
            .unwrap();
            assert!(
                actual <= bound,
                "{} vertices, {:?}: {actual} > {bound}",
                graph.vertex_count(),
                workspace.dimensions()
            );
        }
    }
    cmg
}

#[test]
fn empty_singleton_and_disconnected_workspaces_include_projection_and_descriptors() {
    let options = CmgOptions {
        max_levels: 1,
        ..CmgOptions::default()
    };
    for vertices in [0, 1, 9] {
        let graph = Laplacian::from_edges(vertices, []).unwrap();
        let cmg = check(&graph, options);
        assert_eq!(cmg.hierarchy().levels().len(), 1);
        assert_eq!(cmg.terminal_factor().unwrap().active_dimension(), 0);
        let bound =
            CmgPreconditioner::scalar_workspace_allocation_bound(vertices, options).unwrap();
        // Each isolated vertex realizes the maximum component workspace. With
        // one level, only the deliberately reserved terminal banks are unused.
        assert_eq!(
            bound - cmg.workspace_allocation_bytes().unwrap(),
            2 * vertices * size_of::<f64>()
        );
    }
    let graph =
        Laplacian::from_edges(10, [(0, 1, 0.3), (1, 2, 1.2), (3, 4, 2.4), (5, 6, 0.8)]).unwrap();
    let cmg = check(&graph, options);
    assert!(cmg.terminal_factor().unwrap().active_dimension() > 0);
}

#[test]
fn multilevel_direct_and_iterative_terminals_fit_the_dimension_bound() {
    let graph = Laplacian::from_edges(96, (0..95).map(|v| (v, v + 1, 1.0))).unwrap();
    let direct = check(
        &graph,
        CmgOptions {
            direct_threshold: 20,
            max_levels: 8,
            ..CmgOptions::default()
        },
    );
    assert!(direct.hierarchy().levels().len() > 1);
    assert_eq!(
        direct.hierarchy().report().terminal_reason(),
        TerminalReason::Direct
    );
    assert!(direct.terminal_factor().unwrap().active_dimension() > 0);
    let iterative = check(
        &graph,
        CmgOptions {
            direct_threshold: 1,
            max_levels: 2,
            ..CmgOptions::default()
        },
    );
    assert_eq!(iterative.hierarchy().levels().len(), 2);
    assert_eq!(
        iterative.hierarchy().report().terminal_reason(),
        TerminalReason::MaximumLevels
    );
    assert!(iterative.terminal_factor().is_none());
    // Two independent long paths and isolated vertices exercise coarse
    // component-centering banks, rather than only a connected centering plan.
    let disconnected = Laplacian::from_edges(
        131,
        (0..2).flat_map(|component| {
            (0..63).map(move |v| (component * 64 + v, component * 64 + v + 1, 1.0))
        }),
    )
    .unwrap();
    let cmg = check(
        &disconnected,
        CmgOptions {
            direct_threshold: 20,
            max_levels: 8,
            ..CmgOptions::default()
        },
    );
    assert!(cmg.hierarchy().levels().len() > 1);
}

#[test]
fn invalid_options_and_overflow_fail_before_any_hierarchy_exists() {
    let default = CmgOptions::default();
    let invalid = [
        (
            "direct_threshold",
            CmgOptions {
                direct_threshold: 0,
                ..default
            },
        ),
        (
            "max_levels",
            CmgOptions {
                max_levels: 0,
                ..default
            },
        ),
        (
            "max_hierarchy_nnz_factor",
            CmgOptions {
                max_hierarchy_nnz_factor: 0.0,
                ..default
            },
        ),
        (
            "max_hierarchy_nnz_factor",
            CmgOptions {
                max_hierarchy_nnz_factor: f64::INFINITY,
                ..default
            },
        ),
        (
            "low_effective_degree_threshold",
            CmgOptions {
                low_effective_degree_threshold: f64::NAN,
                ..default
            },
        ),
        (
            "low_effective_degree_threshold",
            CmgOptions {
                low_effective_degree_threshold: 1.1,
                ..default
            },
        ),
    ];
    for (expected, options) in invalid {
        for vertices in [0, usize::MAX] {
            assert!(
                matches!(CmgPreconditioner::scalar_workspace_allocation_bound(vertices, options),
                Err(CmgError::InvalidOption { name, .. }) if name == expected)
            );
        }
    }
    assert!(CmgPreconditioner::scalar_workspace_allocation_bound(usize::MAX, default).is_err());
    assert!(
        CmgPreconditioner::scalar_workspace_allocation_bound(
            0,
            CmgOptions {
                max_levels: usize::MAX,
                ..default
            }
        )
        .is_err()
    );
    // Individual layouts fit while the cross-level product does not.
    assert!(matches!(
        CmgPreconditioner::scalar_workspace_allocation_bound(
            1_000_000,
            CmgOptions {
                max_levels: (isize::MAX as usize) / 1024,
                ..default
            },
        ),
        Err(CmgError::InvalidHierarchy { .. })
    ));
}
