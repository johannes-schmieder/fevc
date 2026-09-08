    // Read-only output in a disposable copy, after all inference probes.
    {
        let members: Vec<Vec<usize>> = match inference_rows {
            ComponentInferenceRows::Observation { .. } => problem.retained_rows.iter().map(|&i| vec![i]).collect(),
            ComponentInferenceRows::Match { plan } => plan.rows.iter().map(|group| group.iter().map(|&i| problem.retained_rows[i]).collect()).collect(),
        };
        println!("{{\"kind\":\"geometry\",\"members\":{:?},\"y\":{:?},\"maker_inverse\":{:?},\"ratio\":{:?},\"variance\":{:?}}}", members, working_y, maker_inverse, ratios, variance);
        let state = q1.as_ref().expect("registered q1 diagnostic");
        for target in 0..4 {
            println!("{{\"kind\":\"vectors\",\"target\":{},\"mode\":{:?},\"ratio\":{:?},\"influence\":{:?},\"eigenvalue\":{}}}", target, state.mode[target], state.ratio[target], state.influence[target], state.eigenvalue[target]);
        }
    }
