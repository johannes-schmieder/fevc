// SPDX-License-Identifier: GPL-3.0-only
// Appended to the independently constructed frozen RC oracle by build.py.
// Dense coefficient inverses below are research oracles, never production routes.
use vckss_core::residual_moments::{prepare_with_interrupt, ResidualMomentOptions};
use vckss_core::rng::{CounterRng, ProbeDomain};
use vckss_core::structured_variance::fit_grouped_structured_variance_with_interrupt;

fn sparse_transpose_action(x: &[Vec<(usize, f64)>], p: usize, y: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; p];
    for (row, &y) in x.iter().zip(y) {
        for &(j, v) in row {
            out[j] += v * y;
        }
    }
    out
}
fn sparse_action(x: &[Vec<(usize, f64)>], b: &[f64]) -> Vec<f64> {
    x.iter()
        .map(|r| r.iter().map(|&(j, v)| v * b[j]).sum())
        .collect()
}

fn follow_cell(name: &str, k: usize) -> Cell {
    let mut c = cells(Profile::Confirmation)
        .into_iter()
        .find(|c| c.name == name)
        .expect("unknown cell");
    assert!((12..=128).contains(&k) && k % 2 == 0);
    c.k = k;
    c
}

fn row_ids(d: &FactorizedDesign, k: usize) -> (Vec<u64>, Vec<u64>) {
    let w = d
        .sparse_x
        .iter()
        .map(|r| r.iter().find(|(j, _)| *j < k).unwrap().0 as u64)
        .collect();
    let f = d
        .sparse_x
        .iter()
        .map(|r| {
            r.iter()
                .find(|(j, _)| *j >= k && *j < 2 * k - 1)
                .map_or((k - 1) as u64, |(j, _)| (*j - k) as u64)
        })
        .collect();
    (w, f)
}

fn geometry(c: Cell, probes: u32, seed: u64) {
    use vckss_core::{
        batch_plan::BatchRequest,
        cmg::CmgOptions,
        component_inference::{prepare_oracle_component_inference, ComponentInferenceOptions},
        generic_jla::{
            run_generic_jla_routed_with_attachments_and_hybrid_interrupt,
            GenericJlaExecutionOptions, GenericJlaOptions,
        },
        model_solver::{ModelRoutingOptions, ModelSolverRoute},
        problem::CanonicalInput,
        types::{DeletionMode, InputColumns},
    };
    let started = std::time::Instant::now();
    let d = make_design(c.k, c.controls, c.dominant, c.beta_zero);
    let (worker, firm) = row_ids(&d, c.k);
    let controls = if c.controls {
        (2 * c.k - 1..d.parameters)
            .map(|j| (0..d.rows).map(|i| d.x[i * d.parameters + j]).collect())
            .collect()
    } else {
        vec![]
    };
    // Outcome-free deterministic response. It fixes current JLA ordering before
    // any Monte Carlo outcome; the attachment's intervals are not used.
    let beta = (0..d.parameters)
        .map(|j| 0.3 * ((j + 1) as f64).sin() + 0.1 * ((j * 3 + 1) as f64).cos())
        .collect::<Vec<_>>();
    // The dense covariance eigencertificate has an absolute roundoff envelope.
    // Scale only this deterministic capture response so tiny oracle variances
    // do not put all covariance eigenvalues below that envelope. A positive
    // scale preserves row ordering and the leverage/target numerical atoms.
    let response = matrix_vector(&d.x, d.rows, d.parameters, &beta)
        .into_iter()
        .map(|y| 10_000.0 * y)
        .collect();
    let result = (|| -> vckss_core::error::Result<_> {
        let problem = CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion: (0..d.rows as u64).collect(),
                outcome: response,
                frequency: vec![1; d.rows],
                target_weight: vec![1.0; d.rows],
                controls,
            }
            .validate()?,
        )?
        .compress(&vec![true; d.rows])?;
        let attached = prepare_oracle_component_inference(
            &problem,
            &vec![1e-8; d.rows],
            ComponentInferenceOptions {
                seed: seed ^ 0x941a_0765,
                probes: 32,
                batch_width: 16,
                spectrum_probes: 8,
                spectrum_iterations: 1024,
                ..Default::default()
            },
        )?;
        let estimator = GenericJlaOptions {
            seed,
            probes,
            leverage_batch_width: 16,
            target_batch_width: 16,
            deletion: DeletionMode::Observation,
            ..Default::default()
        };
        let options = GenericJlaExecutionOptions {
            estimator,
            routing: ModelRoutingOptions {
                route: ModelSolverRoute::Diagonal,
                cmg_minimum_dimension: 1,
                allow_automatic_cmg_setup_fallback: false,
                solver: estimator.solver,
                cmg: CmgOptions::default(),
            },
            leverage_batch: BatchRequest::Explicit(16),
            target_batch: BatchRequest::Explicit(16),
            wallseconds: None,
        };
        let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            options,
            None,
            Some(&attached),
            None,
            &mut NeverInterrupt,
        )?;
        let a = result.component_inference.expect("requested attachment");
        let mut h = vec![0.0; d.rows];
        let mut b: [Vec<f64>; 3] = core::array::from_fn(|_| vec![0.0; d.rows]);
        for (i, &original) in problem.retained_rows.iter().enumerate() {
            h[original] = a.leverage[i];
            for t in 0..3 {
                b[t][original] = a.target_diagonal[t][i];
            }
        }
        Ok((h, b, a.maximum_complete_residual, a.full_residual_tolerance))
    })();
    match result {
        Ok((h,b,resid,gate))=>println!("{{\"kind\":\"geometry\",\"cell\":\"{}\",\"k\":{},\"n\":{},\"probes\":{},\"seed\":{},\"status\":\"success\",\"h\":{:?},\"b\":{:?},\"exact_h\":{:?},\"exact_b\":{:?},\"full_residual\":{},\"full_residual_gate\":{},\"seconds\":{}}}",c.name,c.k,d.rows,probes,seed,h,b,d.leverage,d.target_diagonal,resid,gate,started.elapsed().as_secs_f64()),
        Err(e)=>println!("{{\"kind\":\"geometry\",\"cell\":\"{}\",\"k\":{},\"n\":{},\"probes\":{},\"seed\":{},\"status\":\"{}\",\"detail\":{:?},\"seconds\":{}}}",c.name,c.k,d.rows,probes,seed,e.code.as_str(),e.to_string(),started.elapsed().as_secs_f64()),
    }
}

fn follow_basis(h: &[f64], b: &[Vec<f64>; 3], model: StructuredVarianceModel) -> (usize, Vec<f64>) {
    let ranks = [
        midranks(h),
        midranks(&b[0]),
        midranks(&b[1]),
        midranks(&b[2]),
    ];
    if model == StructuredVarianceModel::LeverageOnly {
        (
            3,
            (0..h.len())
                .flat_map(|i| [1.0, ranks[0][i], ranks[0][i] * ranks[0][i]])
                .collect(),
        )
    } else {
        (
            15,
            (0..h.len()).flat_map(|i| basis_row(&ranks, i)).collect(),
        )
    }
}

fn follow_normal(rng: CounterRng, row: u64, normal: u64) -> f64 {
    let word = |j| rng.word(ProbeDomain::Diagnostic, 0, row, j);
    let u = ((word(2 * normal) >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0;
    let v = ((word(2 * normal + 1) >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0;
    (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
}

fn follow_y(
    d: &FactorizedDesign,
    c: Cell,
    s: &[f64],
    mu: &[f64],
    master: u64,
    rep: usize,
) -> (u64, Vec<f64>) {
    let seed = semantic_seed(master, c.name, c.k, rep);
    let rng = CounterRng::new(seed);
    let y = (0..d.rows)
        .map(|i| {
            let mut e = follow_normal(rng, i as u64, 0);
            if c.error == ErrorDgp::StudentT8 {
                let q = (1..=8)
                    .map(|j| follow_normal(rng, i as u64, j).powi(2))
                    .sum::<f64>();
                e *= (6.0 / q).sqrt();
            }
            mu[i] + s[i].sqrt() * e
        })
        .collect();
    (seed, y)
}

fn collapsed(
    d: &FactorizedDesign,
    k: usize,
    s: &[f64],
) -> (FactorizedDesign, Vec<Vec<usize>>, Vec<f64>, Vec<f64>) {
    assert_eq!(
        d.parameters,
        2 * k - 1,
        "no control collapse in this diagnostic"
    );
    let (w, f) = row_ids(d, k);
    let mut map = std::collections::BTreeMap::<u64, Vec<usize>>::new();
    for i in 0..d.rows {
        map.entry(w[i] * k as u64 + f[i]).or_default().push(i);
    }
    let groups = map.values().cloned().collect::<Vec<_>>();
    let mass = groups.iter().map(|g| g.len() as f64).collect::<Vec<_>>();
    let mut a = d.clone();
    a.rows = groups.len();
    a.x = groups
        .iter()
        .flat_map(|g| {
            d.x[g[0] * d.parameters..(g[0] + 1) * d.parameters]
                .iter()
                .map(|x| x * (g.len() as f64).sqrt())
        })
        .collect();
    a.sparse_x = groups
        .iter()
        .map(|g| {
            d.sparse_x[g[0]]
                .iter()
                .map(|&(j, x)| (j, x * (g.len() as f64).sqrt()))
                .collect()
        })
        .collect();
    a.leverage = groups
        .iter()
        .map(|g| d.leverage[g[0]] * g.len() as f64)
        .collect();
    assert!(a.leverage.iter().all(|h| *h < 1.0 - 1e-10));
    a.maker_inverse = a.leverage.iter().map(|h| 1.0 / (1.0 - h)).collect();
    a.fold_entity = map.keys().copied().collect();
    for t in 0..3 {
        a.target_diagonal[t] = groups
            .iter()
            .map(|g| d.target_diagonal[t][g[0]] * g.len() as f64)
            .collect();
    }
    for t in 0..4 {
        a.leading_mode[t] = groups
            .iter()
            .map(|g| d.leading_mode[t][g[0]] * (g.len() as f64).sqrt())
            .collect();
        a.ratio[t] = groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                d.ratio[t][g[0]] * (1.0 - d.leverage[g[0]]) * g.len() as f64 * a.maker_inverse[i]
            })
            .collect();
        a.remainder_ratio[t] = groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                d.remainder_ratio[t][g[0]]
                    * (1.0 - d.leverage[g[0]])
                    * g.len() as f64
                    * a.maker_inverse[i]
            })
            .collect();
    }
    a.variance_ranks = [
        midranks(&a.leverage),
        midranks(&a.target_diagonal[0]),
        midranks(&a.target_diagonal[1]),
        midranks(&a.target_diagonal[2]),
    ];
    a.hidden_driver = vec![0.0; a.rows];
    let variance = groups
        .iter()
        .map(|g| g.iter().map(|&i| s[i]).sum::<f64>() / g.len() as f64)
        .collect();
    (a, groups, mass, variance)
}

fn follow_evaluate(
    d: &FactorizedDesign,
    c: Cell,
    y: &[f64],
    s: Option<&[f64]>,
    prefix: &str,
    floor: usize,
) {
    let residual = maker_action(d, y);
    let proxy = (0..d.rows)
        .map(|i| y[i] * residual[i] * d.maker_inverse[i])
        .collect::<Vec<_>>();
    for t in 0..4 {
        let prefix = format!(
            "{prefix},\"target\":\"{}\",\"leading_share\":{},\"remainder_share\":{},\"floored\":{}",
            target_name(t),
            d.leading_share[t],
            d.remainder_share[t],
            floor
        );
        let Some(s) = s else {
            println!("{prefix},\"status\":\"variance_fit_failed\"}}");
            continue;
        };
        let u = kernel_action(d, &d.kernel_factor[t], &d.ratio[t], y);
        let point = dot(y, &u);
        let trace = factorized_trace_variance(d, &d.kernel_factor[t], &d.ratio[t], s);
        let variance = 4.0 * u.iter().zip(s).map(|(u, s)| u * u * s).sum::<f64>() - trace;
        if !variance.is_finite() || variance <= 0.0 {
            println!("{prefix},\"status\":\"q0_variance_failed\"}}");
            continue;
        }
        let mut output = format!(
            "{prefix},\"point_error\":{},\"variance\":{},\"estimated_sd\":{}",
            point - d.truth[t],
            variance,
            variance.sqrt()
        );
        let interval = if c.reference == Reference::Q0 {
            let r = 1.959963984540054 * variance.sqrt();
            Some([point - r, point + r])
        } else {
            let u = kernel_action(d, &d.remainder_factor[t], &d.remainder_ratio[t], y);
            let trace =
                factorized_trace_variance(d, &d.remainder_factor[t], &d.remainder_ratio[t], s);
            let correction = d.leading_mode[t]
                .iter()
                .zip(&proxy)
                .map(|(v, p)| v * v * p)
                .sum::<f64>();
            match finish_q1_target(
                point,
                dot(&d.leading_mode[t], y),
                correction,
                dot(y, &u),
                1e-9,
                d.leading_value[t],
                &d.leading_mode[t],
                &u,
                s,
                trace,
                0.0,
                1e-8,
            )
            .and_then(require_computed_q1)
            {
                Ok(q) => v4_interval(
                    [q.leading_score, q.remainder_estimate],
                    V4Covariance {
                        leading: q.leading_variance,
                        cross: q.leading_remainder_covariance,
                        remainder: q.remainder_variance,
                    },
                    d.leading_value[t],
                    d.truth[t],
                )
                .map(|i| [i.lower, i.upper]),
                Err(_) => None,
            }
        };
        match interval {Some([lo,hi])=>write!(&mut output,",\"status\":\"success\",\"covered\":{},\"lower_miss\":{},\"upper_miss\":{},\"width\":{}",lo<=d.truth[t]&&d.truth[t]<=hi,d.truth[t]<lo,d.truth[t]>hi,hi-lo).unwrap(),None=>output.push_str(",\"status\":\"q1_covariance_failed\"")};
        println!("{output}}}");
    }
}

fn follow_run(
    c: Cell,
    start: usize,
    reps: usize,
    master: u64,
    numseed: u64,
    mode: &str,
    input: &str,
) {
    let d = make_design(c.k, c.controls, c.dominant, c.beta_zero);
    let s = make_variance(&d, c.variance);
    let mu = matrix_vector(&d.x, d.rows, d.parameters, &d.beta);
    let arms: &[&str] = match mode {
        "development" => &["exact", "jla_h", "jla_z", "jla_both"],
        "confirmation" => &["exact", "jla_both"],
        "paired" => &[
            "obs_old",
            "exact",
            "obs_oracle",
            "match_old",
            "match_oracle",
        ],
        _ => panic!("bad mode"),
    };
    let approximate = if mode != "paired" {
        let bytes = std::fs::read(input).expect("geometry input");
        assert_eq!(bytes.len(), 32 * d.rows);
        bytes
            .chunks_exact(8)
            .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let (jh, jb) = if approximate.is_empty() {
        (d.leverage.clone(), d.target_diagonal.clone())
    } else {
        (
            approximate[..d.rows].to_vec(),
            core::array::from_fn(|t| approximate[(t + 1) * d.rows..(t + 2) * d.rows].to_vec()),
        )
    };
    let zb = follow_basis(&d.leverage, &d.target_diagonal, c.model);
    let zj = if jh.iter().all(|h| h.is_finite()) {
        follow_basis(&jh, &jb, c.model)
    } else {
        (zb.0, vec![f64::NAN; zb.1.len()])
    };
    let ids = (0..d.rows).map(|i| (i as u64, 0)).collect::<Vec<_>>();
    let information = sparse_crossproduct(&d.sparse_x, d.parameters, None);
    let mut prepared = Vec::new();
    for &arm in arms {
        if !["exact", "jla_h", "jla_z", "jla_both"].contains(&arm) {
            prepared.push(None);
            continue;
        }
        let z = if ["jla_z", "jla_both"].contains(&arm) {
            &zj.1
        } else {
            &zb.1
        };
        let h = if ["jla_h", "jla_both"].contains(&arm) {
            &jh
        } else {
            &d.leverage
        };
        let now = std::time::Instant::now();
        // Approximate-h arms deliberately stress the exact-h API off contract.
        // They do not expand its production claim or bypass any failure gate.
        let fit = prepare_with_interrupt(
            z,
            zb.0,
            h,
            &ids,
            ResidualMomentOptions {
                seed: numseed,
                probes: 512,
                projection_workspace_bytes: 8 * (d.rows + 4 * d.parameters * d.parameters),
                additional_memory_limit_bytes: 512 * 1024 * 1024,
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
                    let fitted = sparse_action(&d.sparse_x, &beta);
                    pg[col * d.rows..(col + 1) * d.rows].copy_from_slice(&fitted);
                    let check = matrix_vector(&information, d.parameters, d.parameters, &beta);
                    let rn = check
                        .iter()
                        .zip(&rhs)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt();
                    let bn = rhs.iter().map(|a| a * a).sum::<f64>().sqrt();
                    cert[col] = if bn == 0.0 { rn } else { rn / bn };
                }
                Ok(())
            },
            &mut NeverInterrupt,
        );
        match &fit {
            Ok(f)=>println!("{{\"kind\":\"prepare\",\"arm\":\"{arm}\",\"status\":\"success\",\"n\":{},\"terms\":{},\"rcond\":{},\"inverse_residual\":{},\"projection_residual\":{},\"memory_bytes\":{},\"gram\":{:?},\"seconds\":{}}}",d.rows,zb.0,f.diagnostic.gram_rcond,f.diagnostic.gram_inverse_relres,f.diagnostic.maximum_full_residual,f.diagnostic.memory.additional_peak_bytes,f.gram(),now.elapsed().as_secs_f64()),
            Err(e)=>println!("{{\"kind\":\"prepare\",\"arm\":\"{arm}\",\"status\":\"{}\",\"detail\":{:?},\"seconds\":{}}}",e.code.as_str(),e.to_string(),now.elapsed().as_secs_f64()),
        }
        prepared.push(fit.ok());
    }
    let group = if mode == "paired" {
        Some(collapsed(&d, c.k, &s))
    } else {
        None
    };
    println!("{{\"kind\":\"design\",\"cell\":\"{}\",\"k\":{},\"n\":{},\"p\":{},\"truth\":{:?},\"leading_share\":{:?},\"remainder_share\":{:?},\"max_h\":{},\"min_variance\":{},\"groups\":{}}}",c.name,c.k,d.rows,d.parameters,d.truth,d.leading_share,d.remainder_share,d.leverage.iter().copied().fold(0.0,f64::max),s.iter().copied().fold(f64::INFINITY,f64::min),group.as_ref().map_or(0,|g|g.0.rows));
    for rep in start..start + reps {
        let (seed, y) = follow_y(&d, c, &s, &mu, master, rep);
        let e = maker_action(&d, &y);
        let proxy = (0..d.rows)
            .map(|i| y[i] * e[i] * d.maker_inverse[i])
            .collect::<Vec<_>>();
        for (j, &arm) in arms.iter().enumerate() {
            let prefix=format!("{{\"kind\":\"target\",\"cell\":\"{}\",\"k\":{},\"replication\":{},\"seed\":{},\"arm\":\"{}\",\"gate\":\"{}\",\"reference\":\"{:?}\"",c.name,c.k,rep,seed,arm,c.gate,c.reference);
            if arm.starts_with("match") {
                let (g, groups, mass, gs) = group.as_ref().unwrap();
                let gy = groups
                    .iter()
                    .map(|r| r.iter().map(|&i| y[i]).sum::<f64>() / (r.len() as f64).sqrt())
                    .collect::<Vec<_>>();
                let selected = if arm == "match_oracle" {
                    Some(gs.clone())
                } else {
                    let ge = maker_action(g, &gy);
                    let gp = (0..g.rows)
                        .map(|i| gy[i] * ge[i] * g.maker_inverse[i])
                        .collect::<Vec<_>>();
                    fit_grouped_structured_variance_with_interrupt(
                        &gp,
                        &ge,
                        &g.maker_inverse,
                        &g.leverage,
                        &g.target_diagonal,
                        mass,
                        &g.fold_entity,
                        StructuredVarianceOptions {
                            seed: INTERVAL_SEED,
                            ..Default::default()
                        },
                        &mut NeverInterrupt,
                    )
                    .ok()
                    .map(|f| f.selected(c.model).to_vec())
                };
                follow_evaluate(g, c, &gy, selected.as_deref(), &prefix, 0);
            } else {
                let (selected, floor) = if arm == "obs_oracle" {
                    (Some(s.clone()), 0)
                } else if arm == "obs_old" {
                    (
                        fit_structured_variance_with_interrupt(
                            &proxy,
                            &e,
                            &d.maker_inverse,
                            &d.leverage,
                            &d.target_diagonal,
                            &d.fold_entity,
                            StructuredVarianceOptions {
                                seed: INTERVAL_SEED,
                                ..Default::default()
                            },
                            &mut NeverInterrupt,
                        )
                        .ok()
                        .map(|f| f.selected(c.model).to_vec()),
                        0,
                    )
                } else {
                    match prepared[j]
                        .as_ref()
                        .and_then(|p| p.fit_with_interrupt(&e, &mut NeverInterrupt).ok())
                    {
                        Some(f) => (Some(f.positive_variance), f.floored_predictions),
                        None => (None, 0),
                    }
                };
                follow_evaluate(&d, c, &y, selected.as_deref(), &prefix, floor);
            }
        }
    }
}

fn scale_candidate(c: Cell, seed: u64) {
    use vckss_core::model_operator::{CanonicalModelData, ModelRhs};
    use vckss_core::model_solver::{ModelSolverOptions, PreparedModelSolver};
    let d = make_design(c.k, c.controls, c.dominant, false);
    let (w, f) = row_ids(&d, c.k);
    let w = w.iter().map(|&x| x as u32).collect::<Vec<_>>();
    let f = f.iter().map(|&x| x as u32).collect::<Vec<_>>();
    let controls = if c.controls {
        (2 * c.k - 1..d.parameters)
            .map(|j| {
                (0..d.rows)
                    .map(|i| d.x[i * d.parameters + j])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let weight = vec![1.0; d.rows];
    let (terms, z) = follow_basis(&d.leverage, &d.target_diagonal, c.model);
    let ids = (0..d.rows).map(|i| (i as u64, 0)).collect::<Vec<_>>();
    let start = std::time::Instant::now();
    let solver = PreparedModelSolver::prepare(
        CanonicalModelData {
            workers: c.k,
            firms: c.k,
            row_worker: &w,
            row_firm: &f,
            weight: &weight,
            controls: &controls,
        },
        ModelSolverOptions::default(),
    )
    .unwrap();
    let solver_seconds = start.elapsed().as_secs_f64();
    let start = std::time::Instant::now();
    let prepared = prepare_with_interrupt(
        &z,
        terms,
        &d.leverage,
        &ids,
        ResidualMomentOptions {
            seed,
            projection_workspace_bytes: 8 * (16 * d.rows + 128 * d.parameters),
            additional_memory_limit_bytes: 512 * 1024 * 1024,
            ..Default::default()
        },
        |input, cols, output, cert, interrupt| {
            for col in 0..cols {
                let mut wr = vec![0.0; c.k];
                let mut fr = wr.clone();
                let mut cr = vec![0.0; controls.len()];
                for i in 0..d.rows {
                    let v = input[col * d.rows + i];
                    wr[w[i] as usize] += v;
                    fr[f[i] as usize] += v;
                    for j in 0..cr.len() {
                        cr[j] += controls[j][i] * v;
                    }
                }
                let a = solver.solve_with_interrupt(
                    ModelRhs {
                        worker: &wr,
                        firm: &fr,
                        control: &cr,
                    },
                    interrupt,
                )?;
                cert[col] = a.receipt.full_residual;
                solver.operator().predict_into_with_interrupt(
                    &a.coefficients.worker,
                    &a.coefficients.firm,
                    &a.coefficients.control,
                    &mut output[col * d.rows..(col + 1) * d.rows],
                    interrupt,
                )?;
            }
            Ok(())
        },
        &mut NeverInterrupt,
    )
    .unwrap();
    let prepare_seconds = start.elapsed().as_secs_f64();
    let s = make_variance(&d, c.variance);
    let mu = matrix_vector(&d.x, d.rows, d.parameters, &d.beta);
    let mut fit_seconds = 0.0;
    let mut total = 0;
    for rep in 0..10 {
        let (_, y) = follow_y(&d, c, &s, &mu, 7346928104512301, rep);
        let e = maker_action(&d, &y);
        let start = std::time::Instant::now();
        let result = prepared.fit_with_interrupt(&e, &mut NeverInterrupt);
        fit_seconds += start.elapsed().as_secs_f64();
        total += usize::from(result.is_ok());
    }
    println!("{{\"kind\":\"scaling\",\"cell\":\"{}\",\"k\":{},\"n\":{},\"terms\":{},\"solver_seconds\":{},\"prepare_seconds\":{},\"mean_fit_seconds\":{},\"fits\":10,\"successful_fits\":{},\"additional_memory_bytes\":{},\"borrowed_input_bytes\":{},\"projection_residual\":{},\"rcond\":{}}}",c.name,c.k,d.rows,terms,solver_seconds,prepare_seconds,fit_seconds/10.0,total,prepared.diagnostic.memory.additional_peak_bytes,prepared.diagnostic.memory.borrowed_input_bytes,prepared.diagnostic.maximum_full_residual,prepared.diagnostic.gram_rcond);
}

fn collapse_diagnostics(c: Cell) {
    let d = make_design(c.k, false, c.dominant, false);
    let s = make_variance(&d, c.variance);
    let (g, _, mass, variance) = collapsed(&d, c.k, &s);
    let e = variance
        .iter()
        .zip(&g.maker_inverse)
        .map(|(s, m)| (s / m).sqrt())
        .collect::<Vec<_>>();
    let check = fit_grouped_structured_variance_with_interrupt(
        &variance,
        &e,
        &g.maker_inverse,
        &g.leverage,
        &g.target_diagonal,
        &mass,
        &g.fold_entity,
        StructuredVarianceOptions {
            seed: INTERVAL_SEED,
            ..Default::default()
        },
        &mut NeverInterrupt,
    );
    let (status, detail) = match check {
        Ok(_) => ("success", String::new()),
        Err(e) => (e.code.as_str(), e.to_string()),
    };
    println!("{{\"cell\":\"{}\",\"k\":{},\"n\":{},\"groups\":{},\"mass\":{:?},\"h\":{:?},\"b\":{:?},\"variance\":{:?},\"model\":{},\"support_status\":\"{}\",\"support_detail\":{:?}}}",c.name,c.k,d.rows,g.rows,mass,g.leverage,g.target_diagonal,variance,c.model as u32,status,detail);
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("geometry")=>{assert_eq!(args.len(),6);geometry(follow_cell(&args[2],args[3].parse().unwrap()),args[4].parse().unwrap(),args[5].parse().unwrap());},
        Some("scale")=>{assert_eq!(args.len(),5);scale_candidate(follow_cell(&args[2],args[3].parse().unwrap()),args[4].parse().unwrap());},
        Some("collapse")=>{assert_eq!(args.len(),4);collapse_diagnostics(follow_cell(&args[2],args[3].parse().unwrap()));},
        Some("preflight")=>{assert_eq!(args.len(),8);follow_run(follow_cell(&args[2],args[3].parse().unwrap()),0,0,0,args[4].parse().unwrap(),&args[5],&args[6]);assert_eq!(args[7],"v1");},
        Some("run")=>{assert_eq!(args.len(),11);let reps=args[5].parse().unwrap();assert!((1..=2500).contains(&reps));follow_run(follow_cell(&args[2],args[3].parse().unwrap()),args[4].parse().unwrap(),reps,args[6].parse().unwrap(),args[7].parse().unwrap(),&args[8],&args[9]);assert_eq!(args[10],"v1");},
        Some("cells")=>{for c in cells(Profile::Confirmation){println!("{{\"cell\":\"{}\",\"k\":{},\"gate\":\"{}\",\"reference\":\"{:?}\"}}",c.name,c.k,c.gate,c.reference);}},
        _=>panic!("geometry CELL K PROBES SEED | run CELL K START REPS MASTER NUMSEED MODE INPUT v1 | cells"),
    }
}

#[cfg(test)]
mod followup_tests {
    use super::*;
    #[test]
    fn physical_block_deletion_equals_whitened_match_oracle() {
        for dominant in [false, true] {
            let d = make_design(6, false, dominant, false);
            let s = make_variance(&d, VarianceDgp::Common);
            let (g, groups, _, gs) = collapsed(&d, 6, &s);
            let n = d.rows;
            let p = d.parameters;
            let m = g.rows;
            let xt = transpose(&d.x, n, p);
            let projection = multiply(
                &multiply(&d.x, n, p, &d.information_inverse, p),
                n,
                p,
                &xt,
                n,
            );
            let mut maker = projection.iter().map(|x| -x).collect::<Vec<_>>();
            for i in 0..n {
                maker[i * n + i] += 1.0;
            }
            let orig_info = sparse_crossproduct(&d.sparse_x, p, None);
            let group_info = sparse_crossproduct(&g.sparse_x, p, None);
            for (a, b) in orig_info.iter().zip(group_info) {
                assert!((a - b).abs() < 1e-10);
            }
            let mu = matrix_vector(&d.x, n, p, &d.beta);
            let y = mu
                .iter()
                .enumerate()
                .map(|(i, v)| v + ((i * 7 + 3) as f64).sin())
                .collect::<Vec<_>>();
            let gy = groups
                .iter()
                .map(|r| r.iter().map(|&i| y[i]).sum::<f64>() / (r.len() as f64).sqrt())
                .collect::<Vec<_>>();
            for t in 0..4 {
                let block = (0..p)
                    .flat_map(|i| (0..p).map(move |j| (i, j)))
                    .map(|(i, j)| d.kernel_factor[t][i * 2 * p + j])
                    .collect::<Vec<_>>();
                let b = multiply(&multiply(&d.x, n, p, &block, p), n, p, &xt, n);
                let mut correction = vec![0.0; n * n];
                for group in &groups {
                    let q = group.len();
                    let mut block = vec![0.0; q * q];
                    let mut bb = block.clone();
                    for (i, &r) in group.iter().enumerate() {
                        for (j, &c) in group.iter().enumerate() {
                            block[i * q + j] = maker[r * n + c];
                            bb[i * q + j] = b[r * n + c];
                        }
                    }
                    let inv = inverse(&block, q);
                    let weight = multiply(&bb, q, q, &inv, q);
                    for (i, &r) in group.iter().enumerate() {
                        for (j, &c) in group.iter().enumerate() {
                            correction[r * n + c] = weight[i * q + j];
                        }
                    }
                }
                let dm = multiply(&correction, n, n, &maker, n);
                let mut physical = b;
                for i in 0..n {
                    for j in 0..n {
                        physical[i * n + j] -= 0.5 * (dm[i * n + j] + dm[j * n + i]);
                    }
                }
                let mut cg = vec![0.0; m * m];
                for j in 0..m {
                    let mut unit = vec![0.0; m];
                    unit[j] = 1.0;
                    let a = kernel_action(&g, &g.kernel_factor[t], &g.ratio[t], &unit);
                    for i in 0..m {
                        cg[i * m + j] = a[i];
                    }
                }
                for (i, gi) in groups.iter().enumerate() {
                    for (j, gj) in groups.iter().enumerate() {
                        for &r in gi {
                            for &c in gj {
                                let expanded =
                                    cg[i * m + j] / ((gi.len() * gj.len()) as f64).sqrt();
                                assert!((physical[r * n + c] - expanded).abs() < 2e-10);
                                if i == j {
                                    assert!(physical[r * n + c].abs() < 2e-10);
                                }
                            }
                        }
                    }
                }
                let point = dot(&y, &matrix_vector(&physical, n, n, &y));
                let gp = dot(
                    &gy,
                    &kernel_action(&g, &g.kernel_factor[t], &g.ratio[t], &gy),
                );
                assert!((point - gp).abs() < 1e-8);
                let trace = trace_variance(&physical, &s, n);
                let gt = factorized_trace_variance(&g, &g.kernel_factor[t], &g.ratio[t], &gs);
                assert!((trace - gt).abs() < 1e-8);
            }
        }
    }
    #[test]
    fn counter_outcomes_do_not_depend_on_traversal_or_other_arms() {
        let c = follow_cell("dominant_common_t8", 12);
        let d = make_design(c.k, c.controls, c.dominant, c.beta_zero);
        let s = make_variance(&d, c.variance);
        let mu = matrix_vector(&d.x, d.rows, d.parameters, &d.beta);
        let forward = (0..5)
            .map(|r| follow_y(&d, c, &s, &mu, 9423780156204917, r))
            .collect::<Vec<_>>();
        for r in (0..5).rev() {
            assert_eq!(forward[r], follow_y(&d, c, &s, &mu, 9423780156204917, r));
        }
        assert_ne!(forward[0], follow_y(&d, c, &s, &mu, 7346928104512301, 0));
    }
}
