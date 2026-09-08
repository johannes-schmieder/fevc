    // Read-only diagnostic, generated into a disposable frozen-source core.
    // The original full-call PSD gate immediately below remains authoritative.
    {
        let dense = std::env::var("FEVC_MATCH_AUDIT_DENSE").as_deref() == Ok("1");
        let numbers = |values: &[f64]| -> String {
            format!("[{}]", values.iter().map(|v| if v.is_finite() {
                v.to_string() } else { "null".to_owned() }).collect::<Vec<_>>().join(","))
        };
        let pm = moments.finish().expect("diagnostic moments");
        let mut primitive = [0.0; 9];
        for l in 0..3 { for r in 0..3 {
            primitive[3*l+r] = variance.iter().enumerate().map(|(i, s)|
                4.0 * s * influence[l][i] * influence[r][i]).sum::<f64>() - pm.covariance[3*l+r];
        }}
        println!("{{\"kind\":\"joint\",\"primitive\":{:?},\"trace\":{:?}}}", primitive, pm.covariance);
        if dense {
            let ComponentInferenceRows::Match { plan } = inference_rows else { panic!("match only"); };
            let order = plan.rows.iter().map(|r| problem.retained_rows[r[0]]).collect::<Vec<_>>();
            println!("{{\"kind\":\"geometry\",\"physical_representative\":{:?},\"y\":{:?},\"variance\":{:?},\"maker_inverse\":{:?},\"ratio\":{:?},\"influence\":{:?}}}",
                order, working_y, variance, maker_inverse, ratios, influence);
        }
        for target in 0..REPORTED_TARGETS {
            let s = spectrum.diagnostics[target];
            println!("{{\"kind\":\"spectrum\",\"target\":{},\"certified\":{},\"values\":{}}}", target, s.certified,
                numbers(&[s.leading_eigenvalue, s.second_eigenvalue, s.leading_share,
                    s.remainder_leading_share, s.maximum_mode_weight_squared,
                    s.leading_residual, s.second_residual]));
        }
        if let Some(state) = &q1 {
            for target in 0..REPORTED_TARGETS {
                let (trace, mcse) = state.probe[target].finish().expect("diagnostic trace");
                let mut q = finish_q1_target(state.point_estimate[target], state.leading_score[target],
                    state.leading_variance_correction[target], state.direct_remainder_estimate[target],
                    state.remainder_identity_tolerance[target], state.eigenvalue[target],
                    &state.mode[target], &state.influence[target], variance, trace, mcse,
                    prepared.options.psd_tolerance).expect("diagnostic target");
                if state.status[target] != ComponentQ1Status::Computed { q.status = state.status[target]; }
                let q = finish_q1_interval(q, prepared.options.seed, target, state.eigenvalue[target],
                    prepared.options.confidence_level, prepared.options.critical_simulations).expect("diagnostic interval");
                println!("{{\"kind\":\"q1\",\"target\":{},\"status\":{},\"critical_draws\":{},\"eigenvalue\":{},\"q\":{}}}",
                    target, q.status as u32, q.critical_draws, state.eigenvalue[target],
                    numbers(&[q.point_estimate, q.leading_score, q.leading_variance_correction,
                        q.leading_variance, q.leading_recentered_component, q.remainder_estimate,
                        q.remainder_identity_error, q.leading_remainder_covariance,
                        q.remainder_variance, q.remainder_trace_mcse, q.standardized_determinant,
                        q.remainder_influence_variance, q.remainder_trace_variance,
                        q.curvature, q.critical_value, q.confidence_lower, q.confidence_upper,
                        q.leading_f_statistic, q.remainder_influence_concentration]));
                if dense {
                    println!("{{\"kind\":\"q1_vectors\",\"target\":{},\"mode\":{:?},\"influence\":{:?},\"ratio\":{:?}}}",
                        target, state.mode[target], state.influence[target], state.ratio[target]);
                }
            }
        }
    }
