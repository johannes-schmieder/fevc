// SPDX-License-Identifier: GPL-3.0-only
// Research adapter only. Independent dense/factorized oracles remain outside core.
use vckss_core::{
    batch_plan::BatchRequest,
    component_inference::{
        ComponentInferenceOptions, ComponentReferenceDistribution, ComponentVarianceSource,
    },
    generic_jla::{
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt as native_run,
        GenericJlaExecutionOptions, GenericJlaOptions, GenericJlaResult,
    },
    model_solver::{ModelRoutingOptions, ModelSolverRoute},
    problem::{CanonicalInput, CompressedProblem},
    residual_moment_inference as native_moments,
    types::{DeletionMode, InputColumns},
};

fn native_problem(d: &FactorizedDesign, c: Cell, y: &[f64], order: &[usize]) -> CompressedProblem {
    let (w, f) = row_ids(d, c.k);
    let controls = (2 * c.k - 1..d.parameters)
        .map(|j| order.iter().map(|&i| d.x[i * d.parameters + j]).collect())
        .collect();
    let mut p = CanonicalInput::from_validated(
        InputColumns {
            worker: order.iter().map(|&i| w[i]).collect(),
            firm: order.iter().map(|&i| f[i]).collect(),
            deletion: order.iter().map(|&i| i as u64).collect(),
            outcome: order.iter().map(|&i| y[i]).collect(),
            frequency: vec![1; d.rows],
            target_weight: vec![1.0; d.rows],
            controls,
        }
        .validate()
        .unwrap(),
    )
    .unwrap()
    .compress(&vec![true; d.rows])
    .unwrap();
    p.probe_order = Some(
        p.retained_rows
            .iter()
            .map(|&i| order[i] as f64 + 1.0)
            .collect(),
    );
    p
}

fn native_options(seed: u64) -> GenericJlaExecutionOptions {
    let estimator = GenericJlaOptions {
        seed,
        probes: 3200,
        leverage_batch_width: 16,
        target_batch_width: 16,
        deletion: DeletionMode::Observation,
        ..Default::default()
    };
    GenericJlaExecutionOptions {
        estimator,
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Diagonal,
            allow_automatic_cmg_setup_fallback: false,
            solver: estimator.solver,
            ..Default::default()
        },
        leverage_batch: BatchRequest::Explicit(16),
        target_batch: BatchRequest::Explicit(16),
        wallseconds: None,
    }
}

fn native_prepared(
    p: &CompressedProblem,
    c: Cell,
    seed: u64,
    batch: usize,
) -> vckss_core::error::Result<vckss_core::component_inference::PreparedComponentInference> {
    native_moments::prepare_with_interrupt(
        p,
        if c.model == StructuredVarianceModel::LeverageOnly {
            ComponentVarianceSource::StructuredLeverage
        } else {
            ComponentVarianceSource::StructuredCommon
        },
        ComponentInferenceOptions {
            seed: seed ^ 0x941a_0765,
            probes: 1000,
            batch_width: 16,
            spectrum_probes: 128,
            spectrum_iterations: 512,
            spectrum_tolerance: 0.002,
            critical_simulations: 100000,
            reference_distribution: if c.reference == Reference::Q0 {
                ComponentReferenceDistribution::Q0
            } else {
                ComponentReferenceDistribution::Q1
            },
            ..Default::default()
        },
        native_moments::Options {
            seed,
            probes: 512,
            batch_width: batch,
        },
        &mut NeverInterrupt,
    )
}

fn native_once(
    p: &CompressedProblem,
    c: Cell,
    seed: u64,
    batch: usize,
) -> vckss_core::error::Result<GenericJlaResult> {
    let a = native_prepared(p, c, seed, batch)?;
    native_run(
        p,
        native_options(seed),
        None,
        Some(&a),
        None,
        &mut NeverInterrupt,
    )
}

fn reported(v: vckss_core::jla::VarianceComponents) -> [f64; 4] {
    [
        v.worker,
        v.firm,
        v.covariance,
        v.worker + v.firm + 2.0 * v.covariance,
    ]
}

fn native_campaign(c: Cell, start: usize, reps: usize, master: u64, numseed: u64) {
    let d = make_design(c.k, c.controls, c.dominant, c.beta_zero);
    let s = make_variance(&d, c.variance);
    let mu = matrix_vector(&d.x, d.rows, d.parameters, &d.beta);
    let order = (0..d.rows).collect::<Vec<_>>();
    println!("{{\"kind\":\"design\",\"cell\":\"{}\",\"k\":{},\"n\":{},\"p\":{},\"truth\":{:?},\"leading_share\":{:?},\"remainder_share\":{:?},\"max_h\":{},\"min_variance\":{}}}",c.name,c.k,d.rows,d.parameters,d.truth,d.leading_share,d.remainder_share,d.leverage.iter().copied().fold(0.0,f64::max),s.iter().copied().fold(f64::INFINITY,f64::min));
    for rep in start..start + reps {
        let (seed, y) = follow_y(&d, c, &s, &mu, master, rep);
        let p = native_problem(&d, c, &y, &order);
        let now = std::time::Instant::now();
        let prefix=format!("{{\"kind\":\"target\",\"cell\":\"{}\",\"k\":{},\"replication\":{},\"seed\":{},\"numseed\":{}",c.name,c.k,rep,seed,numseed);
        match native_once(&p, c, numseed, 16) {
            Err(e) => {
                println!("{{\"kind\":\"call\",\"replication\":{rep},\"status\":\"{}\",\"phase\":{:?},\"detail\":{:?},\"seconds\":{}}}",e.code.as_str(),e.phase,e.to_string(),now.elapsed().as_secs_f64());
                for t in 0..4 {
                    for arm in ["native", "exact_same_variance"] {
                        println!("{prefix},\"target\":\"{}\",\"arm\":\"{arm}\",\"status\":\"native_call_failed\"}}",target_name(t));
                    }
                }
            }
            Ok(result) => {
                let a = result.component_inference.as_ref().unwrap();
                let fit = a.residual_moments.as_ref().unwrap();
                let mut variance = vec![0.0; d.rows];
                let mut approximate = d.clone();
                for (i, &original) in p.retained_rows.iter().enumerate() {
                    variance[original] = fit.fit.positive_variance[i];
                    let b = [
                        a.target_diagonal[0][i],
                        a.target_diagonal[1][i],
                        a.target_diagonal[2][i],
                        a.target_diagonal[0][i]
                            + a.target_diagonal[1][i]
                            + 2.0 * a.target_diagonal[2][i],
                    ];
                    for t in 0..4 {
                        approximate.ratio[t][original] = b[t] * a.maker_inverse[i];
                    }
                }
                println!("{{\"kind\":\"call\",\"replication\":{rep},\"status\":\"success\",\"seconds\":{},\"gram\":{:?},\"h\":{:?},\"b\":{:?},\"rcond\":{},\"floored\":{},\"atoms\":{},\"words\":{},\"gram_atoms\":{},\"gram_words\":{},\"projection_count\":{},\"projection_residual\":{},\"projection_gate\":{},\"full_residual\":{},\"full_gate\":{},\"memory\":{}}}",now.elapsed().as_secs_f64(),fit.gram,a.leverage,a.target_diagonal,fit.preparation.gram_rcond,fit.fit.floored_predictions, a.counter_atoms,a.counter_words,fit.preparation.counter_atoms,fit.preparation.counter_words,fit.projections.len(),fit.preparation.maximum_full_residual,fit.preparation.full_residual_gate,a.maximum_complete_residual,a.full_residual_tolerance,a.peak_forecast_bytes);
                follow_evaluate(
                    &d,
                    c,
                    &y,
                    Some(&variance),
                    &format!("{prefix},\"arm\":\"exact_same_variance\""),
                    fit.fit.floored_predictions,
                );
                let points = reported(result.corrected);
                let point_mcse = reported(result.numerical_mcse);
                for t in 0..4 {
                    let native_v = a.covariance[t * 4 + t];
                    let u = kernel_action(
                        &approximate,
                        &approximate.kernel_factor[t],
                        &approximate.ratio[t],
                        &y,
                    );
                    let trace = factorized_trace_variance(
                        &approximate,
                        &approximate.kernel_factor[t],
                        &approximate.ratio[t],
                        &variance,
                    );
                    let exact_v =
                        4.0 * u.iter().zip(&variance).map(|(u, s)| u * u * s).sum::<f64>() - trace;
                    let exact_point =
                        dot(&y, &kernel_action(&d, &d.kernel_factor[t], &d.ratio[t], &y));
                    let mut extra=format!(",\"point_delta\":{},\"point_kernel_identity\":{},\"covariance_exact_native_kernel\":{},\"covariance_delta\":{}",points[t]-exact_point,points[t]-dot(&y,&u),exact_v,native_v-exact_v);
                    if t < 3 {
                        write!(&mut extra, ",\"point_mcse\":{}", point_mcse[t]).unwrap();
                        write!(&mut extra, ",\"trace_mcse\":{}", a.trace_mcse[t * 3 + t]).unwrap();
                    }
                    let interval = if let Some(q) = a.q1 {
                        let q = q[t];
                        write!(&mut extra, ",\"q1_status\":{}", q.status as u32).unwrap();
                        if q.status == vckss_core::component_inference::ComponentQ1Status::Computed
                        {
                            let crit = q1_reference::reference_q1_critical(q.curvature, 0.95);
                            write!(&mut extra,",\"critical\":{},\"critical_exact\":{},\"critical_delta\":{},\"remainder_identity\":{}",q.critical_value,crit,q.critical_value-crit,q.remainder_identity_error).unwrap();
                            Some([q.confidence_lower, q.confidence_upper])
                        } else {
                            None
                        }
                    } else {
                        let radius = 1.959963984540054 * native_v.sqrt();
                        Some([points[t] - radius, points[t] + radius])
                    };
                    if let Some([lo, hi]) = interval {
                        println!("{prefix},\"target\":\"{}\",\"arm\":\"native\",\"status\":\"success\",\"point_error\":{},\"variance\":{},\"estimated_sd\":{},\"covered\":{},\"lower_miss\":{},\"upper_miss\":{},\"width\":{}{extra}}}",target_name(t),points[t]-d.truth[t],native_v,native_v.sqrt(),lo<=d.truth[t]&&d.truth[t]<=hi,d.truth[t]<lo,d.truth[t]>hi,hi-lo);
                    } else {
                        println!("{prefix},\"target\":\"{}\",\"arm\":\"native\",\"status\":\"q1_target_failed\",\"point_error\":{},\"variance\":{}{extra}}}",target_name(t),points[t]-d.truth[t],native_v);
                    }
                }
            }
        }
    }
}

fn main() {
    let a = std::env::args().collect::<Vec<_>>();
    assert_eq!(a.len(), 7, "cell k start reps master numerical-seed");
    native_campaign(
        follow_cell(&a[1], a[2].parse().unwrap()),
        a[3].parse().unwrap(),
        a[4].parse().unwrap(),
        a[5].parse().unwrap(),
        a[6].parse().unwrap(),
    );
}

#[cfg(test)]
mod native_tests {
    use super::*;
    fn fixture() -> (Cell, FactorizedDesign, Vec<f64>) {
        let c = follow_cell("dominant_common_controls", 12);
        let d = make_design(c.k, c.controls, c.dominant, false);
        let s = make_variance(&d, c.variance);
        let mu = matrix_vector(&d.x, d.rows, d.parameters, &d.beta);
        let (_, y) = follow_y(&d, c, &s, &mu, 10382619457023651, 0);
        (c, d, y)
    }
    #[test]
    fn integration_keys_frequency_and_options_rejected() {
        let (c, d, y) = fixture();
        let order = (0..d.rows).collect::<Vec<_>>();
        let good = native_problem(&d, c, &y, &order);
        let mut p = good.clone();
        p.probe_order = None;
        assert!(native_prepared(&p, c, 570239, 16).is_err());
        let mut p = good.clone();
        p.probe_order.as_mut().unwrap()[1] = p.probe_order.as_ref().unwrap()[0];
        assert!(native_prepared(&p, c, 570239, 16).is_err());
        let mut p = good.clone();
        p.probe_order.as_mut().unwrap()[0] = f64::NAN;
        assert!(native_prepared(&p, c, 570239, 16).is_err());
        let mut p = good.clone();
        p.frequency[0] = 2;
        assert!(native_prepared(&p, c, 570239, 16).is_err());
        assert!(native_prepared(&good, c, 570239, 0).is_err());
    }
    #[test]
    fn integration_live_geometry_dense_gram_counter_and_permutation() {
        let (c, d, y) = fixture();
        let order = (0..d.rows).collect::<Vec<_>>();
        let p = native_problem(&d, c, &y, &order);
        let first = native_once(&p, c, 570239, 16).unwrap();
        let a = first.component_inference.as_ref().unwrap();
        let f = a.residual_moments.as_ref().unwrap();
        assert!(a.structured_variance.is_none());
        assert_eq!(f.projections.len(), 512);
        assert_eq!(f.preparation.counter_atoms, 512 * d.rows as u64);
        let draws =
            a.q1.unwrap()
                .iter()
                .map(|q| q.critical_draws as u64)
                .sum::<u64>();
        let atoms = d.rows as u64 * (1000 + 128 + 2 + 512) + 2 * draws;
        assert_eq!(a.counter_atoms, atoms);
        assert_eq!(a.counter_words, 2 * atoms);
        for r in &f.projections {
            assert!(r.complete_residual <= r.full_residual_tolerance);
        }
        let mut h = vec![0.0; d.rows];
        let mut b = std::array::from_fn(|_| vec![0.0; d.rows]);
        for (i, &j) in p.retained_rows.iter().enumerate() {
            h[j] = a.leverage[i];
            for t in 0..3 {
                b[t][j] = a.target_diagonal[t][i];
            }
        }
        let (terms, z) = follow_basis(&h, &b, c.model);
        let ids = (0..d.rows).map(|i| (i as u64 + 1, 0)).collect::<Vec<_>>();
        let dense = prepare_with_interrupt(
            &z,
            terms,
            &h,
            &ids,
            ResidualMomentOptions {
                seed: 570239,
                ..Default::default()
            },
            |g, cols, pg, cert, _| {
                for col in 0..cols {
                    let rhs = sparse_transpose_action(
                        &d.sparse_x,
                        d.parameters,
                        &g[col * d.rows..(col + 1) * d.rows],
                    );
                    let beta =
                        matrix_vector(&d.information_inverse, d.parameters, d.parameters, &rhs);
                    pg[col * d.rows..(col + 1) * d.rows]
                        .copy_from_slice(&sparse_action(&d.sparse_x, &beta));
                    cert[col] = 1e-13;
                }
                Ok(())
            },
            &mut NeverInterrupt,
        )
        .unwrap();
        for (x, y) in f.gram.iter().zip(dense.gram()) {
            assert!((x - y).abs() < 1e-7, "{x} {y}");
        }
        let mut y2 = y.clone();
        for (i, y) in y2.iter_mut().enumerate() {
            *y += 0.2 * ((i + 5) as f64).cos();
        }
        let second = native_once(&native_problem(&d, c, &y2, &order), c, 570239, 7).unwrap();
        let second = second.component_inference.unwrap();
        assert_eq!(a.leverage, second.leverage);
        assert_eq!(a.target_diagonal, second.target_diagonal);
        assert_eq!(f.gram, second.residual_moments.unwrap().gram);
        let reverse = order.into_iter().rev().collect::<Vec<_>>();
        let p2 = native_problem(&d, c, &y, &reverse);
        let perm = native_once(&p2, c, 570239, 1).unwrap();
        for (x, y) in reported(first.corrected)
            .iter()
            .zip(reported(perm.corrected))
        {
            assert!((x - y).abs() < 1e-9);
        }
        assert_eq!(
            f.gram,
            perm.component_inference
                .as_ref()
                .unwrap()
                .residual_moments
                .as_ref()
                .unwrap()
                .gram
        );
        // Different variance-learning probes cannot change the JLA point path.
        let prepared = native_prepared(&p, c, 791503, 16).unwrap();
        let changed = native_run(
            &p,
            native_options(570239),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(reported(first.corrected), reported(changed.corrected));
        let mut low = native_options(570239);
        low.estimator.memory_limit_bytes = changed.receipt.peak_forecast_bytes - 1;
        assert!(native_run(&p, low, None, Some(&prepared), None, &mut NeverInterrupt).is_err());
        low.estimator.memory_limit_bytes += 1;
        assert!(native_run(&p, low, None, Some(&prepared), None, &mut NeverInterrupt).is_ok());
        low = native_options(570239);
        low.estimator.nuisance = vckss_core::types::NuisanceMode::FixedOffset;
        assert!(native_run(&p, low, None, Some(&prepared), None, &mut NeverInterrupt).is_err());
        low=native_options(570239);low.routing.route=ModelSolverRoute::Auto;
        assert!(native_run(&p, low, None, Some(&prepared), None, &mut NeverInterrupt).is_err());
        low=native_options(570239);low.estimator.deletion=DeletionMode::Match;
        assert!(native_run(&p, low, None, Some(&prepared), None, &mut NeverInterrupt).is_err());
        struct StopMoment;
        impl vckss_core::interrupt::InterruptCheck for StopMoment {
            fn checkpoint(&mut self,phase:&'static str)->vckss_core::error::Result<()> {
                if phase=="observation_residual_moments" {
                    Err(vckss_core::error::BackendError::new(vckss_core::error::ErrorCode::UserBreak,phase,"test interruption"))
                }else{Ok(())}
            }
        }
        let error=native_run(&p,native_options(570239),None,Some(&prepared),None,&mut StopMoment).unwrap_err();
        assert_eq!(error.code,vckss_core::error::ErrorCode::UserBreak);
        let mut cmg=native_options(570239);cmg.routing.route=ModelSolverRoute::Cmg;
        let cmg=native_run(&p,cmg,None,Some(&native_prepared(&p,c,570239,16).unwrap()),None,&mut NeverInterrupt).unwrap();
        let cf=cmg.component_inference.unwrap().residual_moments.unwrap();
        for (x,y) in f.gram.iter().zip(cf.gram){assert!((x-y).abs()<1e-7);}
    }
}
