// SPDX-License-Identifier: GPL-3.0-only
// Replays original draws. The instrumented core retains its full-call rejection.
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 2, "comma-separated original replication keys");
    let reps = args[1].split(',').map(|v| v.parse::<usize>().unwrap()).collect::<Vec<_>>();
    assert!(!reps.is_empty() && reps.windows(2).all(|w| w[0] < w[1]));
    assert!(reps.iter().all(|&r| r < 2500));
    let c = cells(Profile::Confirmation).into_iter()
        .find(|c| c.name == "dominant_common_t8" && c.k == 16).unwrap();
    let d = make_design(c.k, c.controls, c.dominant, c.beta_zero);
    let s = make_variance(&d, c.variance);
    let mu = matrix_vector(&d.x, d.rows, d.parameters, &d.beta);
    println!("{{\"kind\":\"exact_modes\",\"eigenvalues\":{:?},\"modes\":{:?},\"truth\":{:?},\"leading_share\":{:?},\"remainder_share\":{:?}}}",
        d.leading_value, d.leading_mode, d.truth, d.leading_share, d.remainder_share);
    for rep in reps {
        let (seed, y) = follow_y(&d, c, &s, &mu, 1956048372195603, rep);
        let p = native_problem(&d, c, &y, &(0..d.rows).collect::<Vec<_>>());
        println!("{{\"kind\":\"start\",\"replication\":{rep},\"seed\":{seed},\"y\":{:?},\"order\":{:?}}}", y, p.retained_rows);
        match native_once(&p, c, 326417, 16) {
            Ok(r) => {
                let a = r.component_inference.unwrap();
                let qs = a.q1.unwrap();
                assert!(qs.iter().all(|q| q.status == vckss_core::component_inference::ComponentQ1Status::Computed));
                println!("{{\"kind\":\"end\",\"replication\":{rep},\"status\":\"success\",\"points\":{:?},\"covariance\":{:?},\"widths\":{:?},\"critical\":{:?}}}",
                    reported(r.corrected), a.covariance,
                    qs.map(|q| q.confidence_upper - q.confidence_lower), qs.map(|q| q.critical_value));
            },
            Err(e) => println!("{{\"kind\":\"end\",\"replication\":{rep},\"status\":\"failed\",\"phase\":\"{}\",\"code\":\"{}\"}}", e.phase, e.code.as_str()),
        }
    }
}
