// SPDX-License-Identifier: GPL-3.0-only
// Inserted inside the immutable campaign module in a disposable build only.
    pub fn audit_entry() {
        let arguments = env::args().skip(1).collect::<Vec<_>>();
        assert_eq!(arguments.len(), 3, "cell, comma-separated reps, dense 0/1");
        let (cell, reference) = q1_cell(&arguments[0]).expect("registered cell");
        assert!(!cell.controls, "this audit has no estimated control offset");
        let dense = arguments[2] == "1";
        let settings = Q1Settings {
            numerical: NumericalSettings { estimator_probes: 256, covariance_probes: 512,
                spectrum_probes: 128, spectrum_iterations: 256 },
            critical_simulations: 4000,
        };
        for rep in arguments[1].split(',').map(|r| r.parse::<usize>().unwrap()) {
            assert!(rep < 2500);
            let seed = semantic_seed(CONFIRMATION_SEED, cell.name, 20, rep);
            let fold = semantic_seed(FIXED_FOLD_SEED, cell.name, 20, 0);
            let generated = make_problem(cell, 20, seed);
            println!("{{\"kind\":\"start\",\"cell\":\"{}\",\"replication\":{},\"seed\":{},\"fold_seed\":{},\"dense\":{},\"truth\":{:?}}}",
                cell.name, rep, seed, fold, dense, generated.truth);
            if dense {
                let i = &generated.input;
                println!("{{\"kind\":\"input\",\"worker\":{:?},\"firm\":{:?},\"deletion\":{:?},\"frequency\":{:?},\"target_weight\":{:?},\"outcome\":{:?},\"true_variance\":{:?}}}",
                    i.worker, i.firm, i.deletion, i.frequency, i.target_weight, i.outcome, generated.aggregate_variance);
            }
            match run_q1_attached(cell, reference, &generated, settings, seed, fold) {
                Ok((r, baseline)) => {
                    assert!(point_results_bitwise_equal(&r, &baseline));
                    println!("{{\"kind\":\"end\",\"status\":\"success\",\"replication\":{rep}}}");
                    emit_q1_success(cell, reference, 20, rep, seed, &generated, &r, true);
                }
                Err(e) => println!("{{\"kind\":\"end\",\"status\":\"failed\",\"replication\":{rep},\"phase\":\"{}\",\"code\":\"{}\"}}", e.phase, e.code.as_str()),
            }
        }
    }
