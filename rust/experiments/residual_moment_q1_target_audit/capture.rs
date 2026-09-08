    // Diagnostic-only read of prepared state. The original full-call gate below
    // still runs and returns its original rejection. No state or probe is changed.
    if let Some(state) = &q1 {
        let numbers = |values: &[f64]| -> String {
            format!("[{}]", values.iter().map(|v| {
                if v.is_finite() { v.to_string() } else { "null".to_owned() }
            }).collect::<Vec<_>>().join(","))
        };
        println!("{{\"kind\":\"capture_variance\",\"variance\":{:?}}}", variance);
        for target in 0..REPORTED_TARGETS {
            let (trace, mcse) = state.probe[target].finish().expect("diagnostic trace");
            let mut q = finish_q1_target(
                state.point_estimate[target], state.leading_score[target],
                state.leading_variance_correction[target], state.direct_remainder_estimate[target],
                state.remainder_identity_tolerance[target], state.eigenvalue[target],
                &state.mode[target], &state.influence[target], variance, trace, mcse,
                prepared.options.psd_tolerance,
            ).expect("diagnostic target identity");
            if state.status[target] != ComponentQ1Status::Computed { q.status = state.status[target]; }
            let q = finish_q1_interval(q, prepared.options.seed, target, state.eigenvalue[target],
                prepared.options.confidence_level, prepared.options.critical_simulations)
                .expect("diagnostic critical calculation");
            let s = spectrum.diagnostics[target];
            println!(concat!("{{\"kind\":\"capture_target\",\"target\":{},\"status\":{},",
                "\"mode\":{:?},\"influence\":{:?},\"ratio\":{:?},",
                "\"eigenvalue\":{},\"certified\":{},\"leading_residual\":{},",
                "\"second_residual\":{},\"leading_share\":{},\"remainder_share\":{},",
                "\"mode_max_weight\":{},\"direct_remainder\":{},\"identity_tolerance\":{},",
                "\"q\":{},\"critical_draws\":{}}}"),
                target, q.status as u32, state.mode[target], state.influence[target], state.ratio[target],
                state.eigenvalue[target], s.certified, s.leading_residual, s.second_residual,
                s.leading_share, s.remainder_leading_share, s.maximum_mode_weight_squared,
                state.direct_remainder_estimate[target], state.remainder_identity_tolerance[target],
                numbers(&[q.point_estimate, q.leading_score, q.leading_variance_correction,
                    q.leading_variance, q.leading_recentered_component, q.remainder_estimate,
                    q.remainder_identity_error, q.leading_remainder_covariance,
                    q.remainder_variance, q.remainder_trace_mcse, q.standardized_determinant,
                    q.remainder_influence_variance, q.remainder_trace_variance,
                    q.curvature, q.critical_value, q.confidence_lower, q.confidence_upper,
                    q.leading_f_statistic, q.remainder_influence_concentration]), q.critical_draws);
        }
    }
