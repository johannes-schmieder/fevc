use cmg::{CmgOptions, CmgPreconditioner, Laplacian, PcgOptions, solve_pcg};

#[test]
fn absolute_residual_request_retains_the_weak_bridge_forward_oracle() {
    // Exactly representable path-current reference, independent of CMG.
    const N: usize = 64;
    const BRIDGE: f64 = 16_777_216.0;
    const LIMIT: f64 = 1e-8 * std::f64::consts::SQRT_2;
    let graph = Laplacian::from_edges(
        N,
        (1..N).map(|v| (v - 1, v, if v == 32 { 1.0 / BRIDGE } else { 1.0 })),
    )
    .unwrap();
    let mut reference = [0.0; N];
    for v in (1..N).rev() {
        reference[v - 1] = reference[v] + if v == 32 { BRIDGE } else { 1.0 };
    }
    let mean = reference.iter().sum::<f64>() / N as f64;
    for x in &mut reference {
        *x -= mean;
    }
    let scale = reference.iter().copied().map(f64::abs).fold(0.0, f64::max);
    let mut rhs = [0.0; N];
    rhs[0] = 1.0;
    rhs[N - 1] = -1.0;
    for direct_threshold in [8, N + 1] {
        let pre = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold,
                ..Default::default()
            },
        )
        .unwrap();
        // The historical default is unchanged. Its backward certificate must
        // still be valid, but it makes neither of the stronger promises below.
        let default = solve_pcg(&graph, &pre, &rhs, PcgOptions::default()).unwrap();
        assert!(default.residual_norm() <= default.tolerance());
        assert_eq!(PcgOptions::default().relative_tolerance, 1e-8);
        let strict = solve_pcg(
            &graph,
            &pre,
            &rhs,
            PcgOptions {
                relative_tolerance: 0.0,
                absolute_tolerance: LIMIT,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(strict.tolerance(), LIMIT);
        assert!(strict.residual_norm() <= LIMIT);
        let forward = strict
            .solution()
            .iter()
            .zip(reference)
            .map(|(x, r)| (x - r).abs())
            .fold(0.0, f64::max)
            / scale;
        assert!(forward <= 1e-6, "forward error {forward:e}");
        let mut residual = rhs;
        for v in 1..N {
            let current = (strict.solution()[v - 1] - strict.solution()[v])
                / if v == 32 { BRIDGE } else { 1.0 };
            residual[v - 1] -= current;
            residual[v] += current;
        }
        let residual = residual.iter().map(|r| r * r).sum::<f64>().sqrt();
        assert!(residual <= LIMIT, "independent residual {residual:e}");
        println!(
            "STRICT_RESIDUAL_ORACLE_SUCCESS direct={direct_threshold} iterations={} forward={forward:e} residual={residual:e}",
            strict.iterations()
        );
    }
}
