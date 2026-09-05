// SPDX-License-Identifier: GPL-3.0-only

//! Fresh match-q0 confirmation on repaired production code.
//!
//! The immutable repair harness supplies the physical-row DGP and diagnostics.
//! This separately registered copy changes only cells, seed domains, and schema;
//! folds remain outcome-free. It does not change the estimator.

#[path = "common/q1_reference.rs"]
mod q1_reference;

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
mod campaign {
    use std::env;

    use vckss_core::batch_plan::BatchRequest;
    use vckss_core::cmg::CmgOptions;
    use vckss_core::component_inference::{
        prepare_grouped_oracle_component_inference, prepare_grouped_structured_component_inference,
        ComponentInferenceOptions, ComponentReferenceDistribution, ComponentVarianceSource,
    };
    use vckss_core::error::BackendError;
    use vckss_core::generic_jla::{
        run_generic_jla_routed, run_generic_jla_routed_with_attachments_and_hybrid_interrupt,
        GenericJlaExecutionOptions, GenericJlaOptions,
    };
    use vckss_core::interrupt::NeverInterrupt;
    use vckss_core::jla::VarianceComponents;
    use vckss_core::krylov::PcgOptions;
    use vckss_core::model_solver::{ModelRoutingOptions, ModelSolverOptions, ModelSolverRoute};
    use vckss_core::problem::{CanonicalInput, CompressedProblem};
    use vckss_core::structured_variance::StructuredVarianceOptions;
    use vckss_core::types::{DeletionMode, InputColumns, NuisanceMode};

    const Q1_ROW_SCHEMA: &str = "fevc-rc-match-q0-row-v1";
    const Q1_PREFLIGHT_SCHEMA: &str = "fevc-match-inference-q1-design-preflight-v1";
    const Q1_MASTER_SEED: u64 = 0x73bd_6a80_912f_c4e5;
    const CONFIRMATION_SEED: u64 = 0xeca1_8f43_7d60_b295;
    const FIXED_FOLD_SEED: u64 = 0x593f_7d82_a106_ce4b;
    const NORMAL_CRITICAL: f64 = 1.959_963_984_540_054;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum MatchMass {
        Equal,
        Unequal,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum WithinMatch {
        Independent,
        CommonShock,
        Serial,
        EqualAggregateShapes,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum VarianceDgp {
        Correct,
        MildOmitted,
        SevereOmitted,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TargetShape {
        Diffuse,
        OneMode,
        MultiMode,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Signal {
        Regular,
        Weak,
        Null,
    }

    #[derive(Clone, Copy, Debug)]
    struct Cell {
        name: &'static str,
        gate: &'static str,
        mass: MatchMass,
        dependence: WithinMatch,
        variance_dgp: VarianceDgp,
        variance_source: ComponentVarianceSource,
        target_shape: TargetShape,
        signal: Signal,
        controls: bool,
        route: ModelSolverRoute,
    }

    impl Cell {
        fn named(name: &str) -> Option<Self> {
            let common = ComponentVarianceSource::StructuredCommon;
            let leverage = ComponentVarianceSource::StructuredLeverage;
            let diagonal = ModelSolverRoute::Diagonal;
            let cmg = ModelSolverRoute::Cmg;
            Some(match name {
                "diffuse_equal_independent" => Self {
                    name: "diffuse_equal_independent",
                    gate: "correct",
                    mass: MatchMass::Equal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_equal_independent_cmg" => Self {
                    name: "diffuse_equal_independent_cmg",
                    gate: "solver_diagnostic",
                    mass: MatchMass::Equal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: cmg,
                },
                "diffuse_unequal_independent" => Self {
                    name: "diffuse_unequal_independent",
                    gate: "correct",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_unequal_common_shock" => Self {
                    name: "diffuse_unequal_common_shock",
                    gate: "correct",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::CommonShock,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_unequal_serial" => Self {
                    name: "diffuse_unequal_serial",
                    gate: "correct",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Serial,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_equal_aggregate_shapes" => Self {
                    name: "diffuse_equal_aggregate_shapes",
                    gate: "correct",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::EqualAggregateShapes,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_leverage_sensitivity" => Self {
                    name: "diffuse_leverage_sensitivity",
                    gate: "correct",
                    mass: MatchMass::Equal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: leverage,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_mild_omitted" => Self {
                    name: "diffuse_mild_omitted",
                    gate: "mild",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::MildOmitted,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "diffuse_severe_omitted" => Self {
                    name: "diffuse_severe_omitted",
                    gate: "descriptive",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::SevereOmitted,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "one_mode_diagnostic" => Self {
                    name: "one_mode_diagnostic",
                    gate: "spectral_diagnostic",
                    mass: MatchMass::Equal,
                    dependence: WithinMatch::CommonShock,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::OneMode,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "multi_mode_diagnostic" => Self {
                    name: "multi_mode_diagnostic",
                    gate: "spectral_diagnostic",
                    mass: MatchMass::Equal,
                    dependence: WithinMatch::CommonShock,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::MultiMode,
                    signal: Signal::Regular,
                    controls: false,
                    route: diagonal,
                },
                "weak_signal" => Self {
                    name: "weak_signal",
                    gate: "failure_diagnostic",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Weak,
                    controls: false,
                    route: diagonal,
                },
                "null_signal" => Self {
                    name: "null_signal",
                    gate: "failure_diagnostic",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Independent,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Null,
                    controls: false,
                    route: diagonal,
                },
                "controls_varying_fixedoffset" => Self {
                    name: "controls_varying_fixedoffset",
                    gate: "conditioning_diagnostic",
                    mass: MatchMass::Unequal,
                    dependence: WithinMatch::Serial,
                    variance_dgp: VarianceDgp::Correct,
                    variance_source: common,
                    target_shape: TargetShape::Diffuse,
                    signal: Signal::Regular,
                    controls: true,
                    route: diagonal,
                },
                _ => return None,
            })
        }

        fn variance_model(self) -> &'static str {
            match self.variance_source {
                ComponentVarianceSource::StructuredCommon => "structured_common",
                ComponentVarianceSource::StructuredLeverage => "structured_leverage",
                ComponentVarianceSource::Oracle => "oracle",
            }
        }

        const fn mass_name(self) -> &'static str {
            match self.mass {
                MatchMass::Equal => "equal",
                MatchMass::Unequal => "highly_unequal",
            }
        }

        const fn dependence_name(self) -> &'static str {
            match self.dependence {
                WithinMatch::Independent => "independent",
                WithinMatch::CommonShock => "common_shock",
                WithinMatch::Serial => "serial_ar1",
                WithinMatch::EqualAggregateShapes => "equal_aggregate_mixed_shapes",
            }
        }

        const fn variance_dgp_name(self) -> &'static str {
            match self.variance_dgp {
                VarianceDgp::Correct => "correct",
                VarianceDgp::MildOmitted => "mild_omitted",
                VarianceDgp::SevereOmitted => "severe_omitted",
            }
        }

        const fn target_shape_name(self) -> &'static str {
            match self.target_shape {
                TargetShape::Diffuse => "diffuse",
                TargetShape::OneMode => "one_mode",
                TargetShape::MultiMode => "multi_mode",
            }
        }

        const fn signal_name(self) -> &'static str {
            match self.signal {
                Signal::Regular => "regular",
                Signal::Weak => "weak",
                Signal::Null => "null",
            }
        }

        const fn route_name(self) -> &'static str {
            match self.route {
                ModelSolverRoute::Diagonal => "diagonal",
                ModelSolverRoute::Cmg => "cmg",
                ModelSolverRoute::Auto => unreachable!(),
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct NumericalSettings {
        estimator_probes: u32,
        covariance_probes: u32,
        spectrum_probes: u32,
        spectrum_iterations: u32,
    }

    #[derive(Debug)]
    struct GeneratedProblem {
        problem: CompressedProblem,
        input: InputColumns,
        truth: [f64; 4],
        aggregate_variance: Vec<f64>,
    }

    // Preserve the frozen harness's result transport while selecting only Q0.
    #[allow(dead_code)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Reference {
        Q0,
        Q1,
    }

    impl Reference {
        const fn name(self) -> &'static str {
            match self {
                Self::Q0 => "q0",
                Self::Q1 => "q1",
            }
        }
    }

    fn q1_cell(name: &str) -> Option<(Cell, Reference)> {
        Cell::named(name).map(|cell| (cell, Reference::Q0))
    }

    #[derive(Clone, Copy, Debug)]
    struct Q1Settings {
        numerical: NumericalSettings,
        critical_simulations: u32,
    }

    fn parse_q1_request(
        arguments: &[String],
        with_range: bool,
    ) -> (Cell, Reference, usize, usize, usize, Q1Settings) {
        let expected = if with_range { 9 } else { 7 };
        assert_eq!(
            arguments.len(),
            expected,
            "invalid grouped q1 argument count"
        );
        let (cell, reference) = q1_cell(&arguments[0]).expect("unknown grouped q1 cell");
        let dimension = arguments[1]
            .parse::<usize>()
            .expect("dimension is an integer");
        assert!(dimension >= 8, "dimension must be at least eight");
        let (start, replications, offset) = if with_range {
            let start = arguments[2].parse::<usize>().expect("start is an integer");
            let count = arguments[3]
                .parse::<usize>()
                .expect("replications is an integer");
            assert!(
                count > 0 && start.checked_add(count).is_some(),
                "invalid replication range"
            );
            (start, count, 4)
        } else {
            (0, 1, 2)
        };
        let settings = Q1Settings {
            numerical: NumericalSettings {
                estimator_probes: arguments[offset]
                    .parse()
                    .expect("estimator probes are an integer"),
                covariance_probes: arguments[offset + 1]
                    .parse()
                    .expect("covariance probes are an integer"),
                spectrum_probes: arguments[offset + 2]
                    .parse()
                    .expect("spectrum probes are an integer"),
                spectrum_iterations: arguments[offset + 3]
                    .parse()
                    .expect("spectrum iterations are an integer"),
            },
            critical_simulations: arguments[offset + 4]
                .parse()
                .expect("critical simulations are an integer"),
        };
        assert!(
            settings.numerical.estimator_probes >= 2
                && settings.numerical.covariance_probes >= 2
                && settings.numerical.spectrum_probes >= 2
                && settings.numerical.spectrum_iterations >= 2
                && settings.critical_simulations >= 1_000,
            "invalid grouped q1 numerical settings"
        );
        (cell, reference, dimension, start, replications, settings)
    }

    fn coverage_eligible(cell: Cell, reference: Reference, target: usize) -> bool {
        match reference {
            Reference::Q0 => {
                !cell.controls
                    && cell.signal == Signal::Regular
                    && cell.target_shape == TargetShape::Diffuse
                    && cell.gate != "solver_diagnostic"
            }
            Reference::Q1 => {
                target != 2
                    && !cell.controls
                    && cell.signal == Signal::Regular
                    && cell.target_shape == TargetShape::OneMode
                    && cell.gate != "solver_diagnostic"
            }
        }
    }

    pub fn entry() {
        let arguments = env::args().skip(1).collect::<Vec<_>>();
        match arguments.first().map(String::as_str) {
            Some("q1-matrix-v1") => run_q1_matrix(&arguments[1..], false, 0),
            Some("repair-replay-v1") => run_q1_matrix(&arguments[1..], true, 0),
            Some("repair-development-v1") => run_q1_matrix(&arguments[1..], false, 1),
            Some("repair-confirmation-v1") => run_q1_matrix(&arguments[1..], false, 2),
            Some("design-preflight-v1") => run_q1_preflight(&arguments[1..]),
            _ => panic!("expected q1-matrix-v1 or design-preflight-v1"),
        }
    }

    fn run_q1_matrix(arguments: &[String], export_state: bool, campaign: u32) {
        let (cell, reference, dimension, start, replications, settings) =
            parse_q1_request(arguments, true);
        for replication in start..start + replications {
            let seed = semantic_seed(
                if campaign == 2 {
                    CONFIRMATION_SEED
                } else {
                    Q1_MASTER_SEED
                },
                cell.name,
                dimension,
                replication,
            );
            let fold_seed = if campaign == 0 {
                seed ^ 0xa51e_79d3_6c42_f807
            } else {
                semantic_seed(FIXED_FOLD_SEED, cell.name, dimension, 0)
            };
            let generated = make_problem(cell, dimension, seed);
            if export_state {
                let input = &generated.input;
                println!("{{\"kind\":\"input\",\"cell\":\"{}\",\"replication\":{},\"semantic_seed\":{},\"worker\":{:?},\"firm\":{:?},\"deletion\":{:?},\"outcome\":{:?},\"frequency\":{:?},\"target_weight\":{:?},\"controls\":{:?},\"aggregate_variance\":{:?},\"truth\":{:?}}}", cell.name, replication, seed, input.worker, input.firm, input.deletion, input.outcome, input.frequency, input.target_weight, input.controls, generated.aggregate_variance, generated.truth);
            }
            match run_q1_attached(cell, reference, &generated, settings, seed, fold_seed) {
                Ok((result, baseline)) => {
                    if export_state {
                        let inference = result.component_inference.as_ref().expect("attachment");
                        let fit = inference.structured_variance.as_ref().expect("structured");
                        let variance =
                            if cell.variance_source == ComponentVarianceSource::StructuredCommon {
                                &fit.common
                            } else {
                                &fit.leverage_only
                            };
                        println!("{{\"kind\":\"state\",\"cell\":\"{}\",\"replication\":{},\"leverage\":{:?},\"maker_inverse\":{:?},\"target_diagonal\":{:?},\"fitted_variance\":{:?}}}",cell.name,replication,inference.leverage,inference.maker_inverse,inference.target_diagonal,variance);
                    }
                    if !point_results_bitwise_equal(&result, &baseline) {
                        emit_q1_failure(
                            cell,
                            reference,
                            dimension,
                            replication,
                            seed,
                            "point_invariance_failed",
                            None,
                        );
                        continue;
                    }
                    emit_q1_success(
                        cell,
                        reference,
                        dimension,
                        replication,
                        seed,
                        &generated,
                        &result,
                        campaign != 0,
                    );
                }
                Err(error) => emit_q1_failure(
                    cell,
                    reference,
                    dimension,
                    replication,
                    seed,
                    "backend_failure",
                    Some(&error),
                ),
            }
        }
    }

    fn run_q1_preflight(arguments: &[String]) {
        let (cell, reference, dimension, _, _, settings) = parse_q1_request(arguments, false);
        let seed = semantic_seed(Q1_MASTER_SEED, cell.name, dimension, 0);
        let generated = make_problem(
            Cell {
                signal: Signal::Regular,
                ..cell
            },
            dimension,
            seed,
        );
        let estimator = estimator_options(cell, settings.numerical, seed);
        let prepared = prepare_grouped_oracle_component_inference(
            &generated.problem,
            &generated.aggregate_variance,
            inference_options(settings.numerical, seed),
        )
        .expect("registered q1 preflight oracle attachment must prepare");
        let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &generated.problem,
            routed_options(estimator, cell.route),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
        .expect("registered q1 design preflight must execute");
        let inference = result
            .component_inference
            .expect("q1 preflight inference result");
        for target in 0..4 {
            let spectrum = inference.spectrum[target];
            println!(
                concat!(
                    "{{\"schema\":\"{}\",\"cell\":\"{}\",\"k\":{},\"target\":\"{}\",",
                    "\"target_shape\":\"{}\",\"reference_distribution\":\"{}\",",
                    "\"coverage_eligible\":{},\"independent_matches\":{},",
                    "\"effective_match_count\":{:.17e},\"largest_match_mass_share\":{:.17e},",
                    "\"largest_match_leverage\":{:.17e},\"smallest_maker_denominator\":{:.17e},",
                    "\"leading_eigenvalue\":{:.17e},\"second_eigenvalue\":{:.17e},",
                    "\"leading_share\":{:.17e},\"remainder_share\":{:.17e},",
                    "\"maximum_mode_weight\":{:.17e},\"maximum_influence_share\":{:.17e},",
                    "\"leading_residual\":{:.17e},\"second_residual\":{:.17e},",
                    "\"nuisance_uncertainty_conditioned_away\":{}}}"
                ),
                Q1_PREFLIGHT_SCHEMA,
                cell.name,
                dimension,
                target_name(target),
                cell.target_shape_name(),
                reference.name(),
                coverage_eligible(cell, reference, target),
                inference.independent_units,
                inference.effective_match_count,
                inference.largest_match_mass_share,
                inference.largest_match_leverage,
                inference.smallest_maker_denominator,
                spectrum.leading_eigenvalue,
                spectrum.second_eigenvalue,
                spectrum.leading_share,
                spectrum.remainder_leading_share,
                spectrum.maximum_mode_weight_squared,
                inference.influence_concentration[target],
                spectrum.leading_residual,
                spectrum.second_residual,
                inference.nuisance_uncertainty_conditioned_away,
            );
        }
    }

    fn run_q1_attached(
        cell: Cell,
        reference: Reference,
        generated: &GeneratedProblem,
        settings: Q1Settings,
        seed: u64,
        fold_seed: u64,
    ) -> Result<
        (
            vckss_core::generic_jla::GenericJlaResult,
            vckss_core::generic_jla::GenericJlaResult,
        ),
        BackendError,
    > {
        let estimator = estimator_options(cell, settings.numerical, seed);
        let routed = routed_options(estimator, cell.route);
        let baseline = run_generic_jla_routed(&generated.problem, routed)?;
        let mut options = inference_options(settings.numerical, seed);
        options.reference_distribution = match reference {
            Reference::Q0 => ComponentReferenceDistribution::Q0,
            Reference::Q1 => ComponentReferenceDistribution::Q1,
        };
        options.critical_simulations = settings.critical_simulations;
        let prepared = prepare_grouped_structured_component_inference(
            &generated.problem,
            cell.variance_source,
            options,
            StructuredVarianceOptions {
                seed: fold_seed,
                ..StructuredVarianceOptions::default()
            },
        )?;
        let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &generated.problem,
            routed,
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )?;
        Ok((result, baseline))
    }

    fn estimator_options(_cell: Cell, settings: NumericalSettings, seed: u64) -> GenericJlaOptions {
        GenericJlaOptions {
            seed: seed ^ 0xd07a_95c3_184e_b62f,
            probes: settings.estimator_probes,
            leverage_batch_width: 16,
            target_batch_width: 16,
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::FixedOffset,
            rank_tolerance: 1.0e-10,
            block_tolerance: 1.0e-10,
            solver: ModelSolverOptions {
                pcg: PcgOptions {
                    tolerance: 1.0e-11,
                    maximum_iterations: 10_000,
                    residual_replacement_interval: 37,
                },
                rank_tolerance: 1.0e-11,
            },
            ..GenericJlaOptions::default()
        }
    }

    fn inference_options(settings: NumericalSettings, seed: u64) -> ComponentInferenceOptions {
        ComponentInferenceOptions {
            seed: seed ^ 0x347c_2a91_e85f_d60b,
            probes: settings.covariance_probes,
            batch_width: 16,
            spectrum_probes: settings.spectrum_probes,
            spectrum_iterations: settings.spectrum_iterations,
            spectrum_tolerance: 2.0e-2,
            ..ComponentInferenceOptions::default()
        }
    }

    fn routed_options(
        estimator: GenericJlaOptions,
        route: ModelSolverRoute,
    ) -> GenericJlaExecutionOptions {
        GenericJlaExecutionOptions {
            estimator,
            routing: ModelRoutingOptions {
                route,
                cmg_minimum_dimension: 1,
                allow_automatic_cmg_setup_fallback: false,
                solver: estimator.solver,
                cmg: CmgOptions::default(),
            },
            leverage_batch: BatchRequest::Explicit(estimator.leverage_batch_width),
            target_batch: BatchRequest::Explicit(estimator.target_batch_width),
            wallseconds: None,
        }
    }

    fn make_problem(cell: Cell, dimension: usize, seed: u64) -> GeneratedProblem {
        let mut worker = Vec::new();
        let mut firm = Vec::new();
        let mut deletion = Vec::new();
        let mut outcome = Vec::new();
        let mut frequency = Vec::new();
        let mut target_weight = Vec::new();
        let mut first_control = Vec::new();
        let mut second_control = Vec::new();
        let mut aggregate_variance = Vec::with_capacity(dimension * dimension);
        let center = (dimension - 1) as f64 / 2.0;
        let signal_scale = match cell.signal {
            Signal::Regular => 2.5,
            Signal::Weak => 0.04,
            Signal::Null => 0.0,
        };
        let worker_effect = (0..dimension)
            .map(|index| {
                signal_scale
                    * (((index as f64 - center) / dimension as f64)
                        + 0.18 * ((index + 1) as f64 * 0.73).sin())
            })
            .collect::<Vec<_>>();
        let firm_effect = (0..dimension)
            .map(|index| {
                signal_scale
                    * (-0.8 * (index as f64 - center) / dimension as f64
                        + 0.14 * ((index + 1) as f64 * 1.07).cos())
            })
            .collect::<Vec<_>>();
        let match_frequencies = (0..dimension * dimension)
            .map(|group| frequencies_for_match(cell.mass, dimension, group))
            .collect::<Vec<_>>();
        let match_mass = match_frequencies
            .iter()
            .map(|values| values.iter().sum::<u64>() as f64)
            .collect::<Vec<_>>();
        let match_mass_rank = normalized_midranks(&match_mass);
        let mut rng = IndependentRng::new(seed);
        for worker_index in 0..dimension {
            for firm_index in 0..dimension {
                let group = worker_index * dimension + firm_index;
                let frequencies = &match_frequencies[group];
                let stored = frequencies.len();
                let signal_location = (worker_effect[worker_index] + firm_effect[firm_index]).abs();
                let hidden = (0.85 * (signal_location / 1.8).min(1.0)
                    + 0.15 * hidden_driver(worker_index, firm_index))
                .clamp(0.0, 1.0);
                let mut tau = 0.26 + 0.08 * match_mass_rank[group];
                tau *= match cell.variance_dgp {
                    VarianceDgp::Correct => 1.0,
                    VarianceDgp::MildOmitted => 0.85 + 0.30 * hidden,
                    VarianceDgp::SevereOmitted => 0.12 + 4.88 * hidden.powi(3),
                };
                aggregate_variance.push(tau);
                let dependence = match cell.dependence {
                    WithinMatch::EqualAggregateShapes if group % 2 == 0 => WithinMatch::Independent,
                    WithinMatch::EqualAggregateShapes => WithinMatch::CommonShock,
                    value => value,
                };
                let errors = match_errors(frequencies, tau, dependence, &mut rng);
                for local in 0..stored {
                    let row = worker.len();
                    let control_one = 0.17 * worker_index as f64 - 0.11 * firm_index as f64
                        + 0.37 * local as f64
                        + ((row * 7 + 3) % 19) as f64 / 23.0;
                    let control_two = -0.08 * worker_index as f64 + 0.14 * firm_index as f64
                        - 0.29 * local as f64
                        + ((row * 13 + 5) % 29) as f64 / 31.0;
                    let control_index = if cell.controls {
                        0.72 * control_one - 0.43 * control_two
                    } else {
                        0.0
                    };
                    worker.push(10_000 + worker_index as u64);
                    firm.push(20_000 + firm_index as u64);
                    deletion.push(100_000 + group as u64);
                    frequency.push(frequencies[local]);
                    target_weight.push(target_mass(
                        cell.target_shape,
                        worker_index,
                        firm_index,
                        stored,
                    ));
                    first_control.push(control_one);
                    second_control.push(control_two);
                    outcome.push(
                        worker_effect[worker_index]
                            + firm_effect[firm_index]
                            + control_index
                            + errors[local],
                    );
                }
            }
        }
        let truth = component_truth(&worker, &firm, &target_weight, &worker_effect, &firm_effect);
        let rows = outcome.len();
        let input = InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: if cell.controls {
                vec![first_control, second_control]
            } else {
                Vec::new()
            },
        };
        let problem = CanonicalInput::from_validated(
            input
                .clone()
                .validate()
                .expect("registered generated columns validate"),
        )
        .expect("registered generated columns canonicalize")
        .compress(&vec![true; rows])
        .expect("registered generated problem compresses");
        GeneratedProblem {
            problem,
            input,
            truth,
            aggregate_variance,
        }
    }

    fn frequencies_for_match(mass: MatchMass, dimension: usize, group: usize) -> Vec<u64> {
        let stored = match mass {
            MatchMass::Equal => 3,
            MatchMass::Unequal => 2 + (group % 3),
        };
        let worker = group / dimension;
        let firm = group % dimension;
        (0..stored)
            .map(|local| match mass {
                MatchMass::Equal => 1,
                MatchMass::Unequal => {
                    1 + ((group * 17 + local * 11 + worker * 5 + firm * 3) % 13) as u64
                }
            })
            .collect()
    }

    fn normalized_midranks(values: &[f64]) -> Vec<f64> {
        let mut order = (0..values.len()).collect::<Vec<_>>();
        order.sort_by(|&left, &right| {
            values[left]
                .total_cmp(&values[right])
                .then_with(|| left.cmp(&right))
        });
        let mut output = vec![0.0; values.len()];
        let mut begin = 0;
        while begin < order.len() {
            let mut end = begin + 1;
            while end < order.len()
                && values[order[begin]].total_cmp(&values[order[end]]) == core::cmp::Ordering::Equal
            {
                end += 1;
            }
            let midrank = 0.5 * ((begin + 1) as f64 + end as f64);
            let normalized = 2.0 * ((midrank - 0.5) / values.len() as f64) - 1.0;
            for &row in &order[begin..end] {
                output[row] = normalized;
            }
            begin = end;
        }
        output
    }

    fn target_mass(shape: TargetShape, worker: usize, firm: usize, stored: usize) -> f64 {
        let base = (0.85 + ((worker * 19 + firm * 11 + 3) % 31) as f64 / 100.0) / stored as f64;
        match shape {
            TargetShape::Diffuse => base,
            TargetShape::OneMode => {
                base * if worker == 0 && firm == 0 {
                    1_000.0
                } else {
                    1.0
                }
            }
            TargetShape::MultiMode => {
                let multiplier = if worker == firm {
                    [900.0, 600.0, 300.0].get(worker).copied().unwrap_or(1.0)
                } else {
                    1.0
                };
                base * multiplier
            }
        }
    }

    fn hidden_driver(worker: usize, firm: usize) -> f64 {
        let value = splitmix64(((worker as u64) << 32) | firm as u64);
        (((value >> 11) as f64) + 0.5) / 9_007_199_254_740_992.0
    }

    fn match_errors(
        frequency: &[u64],
        tau: f64,
        dependence: WithinMatch,
        rng: &mut IndependentRng,
    ) -> Vec<f64> {
        let mass = frequency.iter().sum::<u64>() as f64;
        match dependence {
            WithinMatch::Independent => {
                let denominator = frequency
                    .iter()
                    .map(|value| (*value as f64).powi(2))
                    .sum::<f64>();
                let scale = (tau * mass / denominator).sqrt();
                frequency.iter().map(|_| scale * rng.normal()).collect()
            }
            WithinMatch::CommonShock => {
                let shock = (tau / mass).sqrt() * rng.normal();
                vec![shock; frequency.len()]
            }
            WithinMatch::Serial => {
                let rho = 0.65_f64;
                let mut denominator = 0.0;
                for left in 0..frequency.len() {
                    for right in 0..frequency.len() {
                        denominator += frequency[left] as f64
                            * frequency[right] as f64
                            * rho.powi(left.abs_diff(right) as i32);
                    }
                }
                let scale = (tau * mass / denominator).sqrt();
                let mut output = Vec::with_capacity(frequency.len());
                let mut state = rng.normal();
                output.push(scale * state);
                for _ in 1..frequency.len() {
                    state = rho * state + (1.0 - rho * rho).sqrt() * rng.normal();
                    output.push(scale * state);
                }
                output
            }
            WithinMatch::EqualAggregateShapes => {
                unreachable!("mixed shape resolves before drawing")
            }
        }
    }

    fn component_truth(
        worker_id: &[u64],
        firm_id: &[u64],
        target: &[f64],
        worker_effect: &[f64],
        firm_effect: &[f64],
    ) -> [f64; 4] {
        let total = target.iter().sum::<f64>();
        let worker_mean = worker_id
            .iter()
            .zip(target)
            .map(|(id, mass)| mass * worker_effect[(*id - 10_000) as usize])
            .sum::<f64>()
            / total;
        let firm_mean = firm_id
            .iter()
            .zip(target)
            .map(|(id, mass)| mass * firm_effect[(*id - 20_000) as usize])
            .sum::<f64>()
            / total;
        let mut worker = 0.0;
        let mut firm = 0.0;
        let mut covariance = 0.0;
        for row in 0..target.len() {
            let worker_value = worker_effect[(worker_id[row] - 10_000) as usize] - worker_mean;
            let firm_value = firm_effect[(firm_id[row] - 20_000) as usize] - firm_mean;
            let share = target[row] / total;
            worker += share * worker_value * worker_value;
            firm += share * firm_value * firm_value;
            covariance += share * worker_value * firm_value;
        }
        [worker, firm, covariance, worker + firm + 2.0 * covariance]
    }

    fn point_results_bitwise_equal(
        left: &vckss_core::generic_jla::GenericJlaResult,
        right: &vckss_core::generic_jla::GenericJlaResult,
    ) -> bool {
        components(left.plugin)
            .into_iter()
            .zip(components(right.plugin))
            .chain(
                components(left.correction)
                    .into_iter()
                    .zip(components(right.correction)),
            )
            .chain(
                components(left.corrected)
                    .into_iter()
                    .zip(components(right.corrected)),
            )
            .all(|(left, right)| left.to_bits() == right.to_bits())
    }

    const fn components(value: VarianceComponents) -> [f64; 4] {
        [value.worker, value.firm, value.covariance, value.total]
    }

    fn target_name(target: usize) -> &'static str {
        ["worker", "firm", "covariance", "total"][target]
    }

    fn semantic_seed(master: u64, cell: &str, dimension: usize, replication: usize) -> u64 {
        splitmix64(
            master
                ^ splitmix64(hash_label(cell))
                ^ splitmix64(dimension as u64)
                ^ splitmix64(replication as u64),
        )
    }

    fn hash_label(value: &str) -> u64 {
        value.bytes().fold(0xcbf2_9ce4_8422_2325, |state, byte| {
            (state ^ u64::from(byte)).wrapping_mul(0x1000_0000_01b3)
        })
    }

    fn splitmix64(mut value: u64) -> u64 {
        value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    #[derive(Clone, Copy, Debug)]
    struct IndependentRng {
        state: u64,
    }

    impl IndependentRng {
        const fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn uniform(&mut self) -> f64 {
            self.state = splitmix64(self.state);
            (((self.state >> 11) as f64) + 0.5) / 9_007_199_254_740_992.0
        }

        fn normal(&mut self) -> f64 {
            (-2.0 * self.uniform().ln()).sqrt() * (core::f64::consts::TAU * self.uniform()).cos()
        }
    }

    fn emit_q1_failure(
        cell: Cell,
        reference: Reference,
        dimension: usize,
        replication: usize,
        seed: u64,
        status: &str,
        error: Option<&BackendError>,
    ) {
        let (code, phase) =
            error.map_or(("NONE", "none"), |value| (value.code.as_str(), value.phase));
        if let Some(error) = error {
            eprintln!(
                "REPAIR_FAILURE cell={} replication={} seed={} {}",
                cell.name, replication, seed, error
            );
        }
        for target in 0..4 {
            println!(
                concat!(
                    "{{\"schema\":\"{}\",\"cell\":\"{}\",\"gate\":\"{}\",\"k\":{},",
                    "\"replication\":{},\"target\":\"{}\",\"semantic_seed\":{},",
                    "\"status\":\"{}\",\"error_code\":\"{}\",\"error_phase\":\"{}\",",
                    "\"reference_distribution\":\"{}\",\"variance_model\":\"{}\",",
                    "\"variance_dgp\":\"{}\",\"within_match_dependence\":\"{}\",",
                    "\"match_mass\":\"{}\",\"target_shape\":\"{}\",\"signal\":\"{}\",",
                    "\"controls\":{},\"solver\":\"{}\",\"coverage_eligible\":{}}}"
                ),
                Q1_ROW_SCHEMA,
                cell.name,
                cell.gate,
                dimension,
                replication,
                target_name(target),
                seed,
                status,
                code,
                phase,
                reference.name(),
                cell.variance_model(),
                cell.variance_dgp_name(),
                cell.dependence_name(),
                cell.mass_name(),
                cell.target_shape_name(),
                cell.signal_name(),
                cell.controls,
                cell.route_name(),
                coverage_eligible(cell, reference, target),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn common_success_json(
        cell: Cell,
        reference: Reference,
        dimension: usize,
        replication: usize,
        seed: u64,
        generated: &GeneratedProblem,
        result: &vckss_core::generic_jla::GenericJlaResult,
        target: usize,
        lower: f64,
        upper: f64,
    ) -> String {
        let points = components(result.corrected);
        let inference = result
            .component_inference
            .as_ref()
            .expect("attached result");
        let structured = inference
            .structured_variance
            .as_ref()
            .expect("structured fit");
        let model_index =
            usize::from(cell.variance_source == ComponentVarianceSource::StructuredLeverage);
        let summary = structured.summary[model_index];
        let model_code = if model_index == 0 { 1 } else { 2 };
        let minimum_active_terms = structured
            .folds
            .iter()
            .filter(|fold| fold.model == model_code)
            .map(|fold| fold.active_terms)
            .min()
            .unwrap_or(0);
        let minimum_training_matches = structured
            .folds
            .iter()
            .filter(|fold| fold.model == model_code)
            .map(|fold| fold.training_observations)
            .min()
            .unwrap_or(0);
        let maximum_trace_mcse = inference.trace_mcse.iter().copied().fold(0.0_f64, f64::max);
        let aggregate_variance_mean = generated.aggregate_variance.iter().sum::<f64>()
            / generated.aggregate_variance.len() as f64;
        let estimated_variance = inference.covariance[target * 4 + target];
        let standard_error = estimated_variance.sqrt();
        let point_error = points[target] - generated.truth[target];
        let spectrum = inference.spectrum[target];
        format!(
            concat!(
                "\"schema\":\"{}\",\"cell\":\"{}\",\"gate\":\"{}\",\"k\":{},",
                "\"replication\":{},\"target\":\"{}\",\"semantic_seed\":{},\"status\":\"{}\",",
                "\"reference_distribution\":\"{}\",\"variance_model\":\"{}\",\"variance_dgp\":\"{}\",",
                "\"within_match_dependence\":\"{}\",\"match_mass\":\"{}\",\"target_shape\":\"{}\",",
                "\"signal\":\"{}\",\"controls\":{},\"solver\":\"{}\",\"coverage_eligible\":{},",
                "\"truth\":{:.17e},\"point_estimate\":{:.17e},\"point_error\":{:.17e},",
                "\"estimated_variance\":{:.17e},\"estimated_sd\":{:.17e},",
                "\"confidence_lower\":{},\"confidence_upper\":{},",
                "\"covered\":{},\"lower_miss\":{},\"upper_miss\":{},",
                "\"independent_matches\":{},\"effective_match_count\":{:.17e},",
                "\"largest_match_mass_share\":{:.17e},\"largest_match_leverage\":{:.17e},",
                "\"smallest_maker_denominator\":{:.17e},\"maximum_influence_share\":{:.17e},",
                "\"leading_eigenvalue\":{},\"second_eigenvalue\":{},",
                "\"leading_share\":{},\"remainder_share\":{},",
                "\"maximum_mode_weight\":{},\"leading_residual\":{},",
                "\"second_residual\":{},\"minimum_active_terms\":{},",
                "\"minimum_training_matches\":{},\"variance_floor_count\":{},",
                "\"variance_floor_share\":{:.17e},\"boundary_share\":{:.17e},",
                "\"maximum_boundary_excess\":{:.17e},\"maximum_prediction_leverage\":{:.17e},",
                "\"minimum_fitted_rcond\":{:.17e},\"sensitivity_median_log_ratio\":{:.17e},",
                "\"sensitivity_p90_log_ratio\":{:.17e},\"sensitivity_maximum_log_ratio\":{:.17e},",
                "\"sensitivity_log_variance_correlation\":{:.17e},",
                "\"maximum_complete_residual\":{:.17e},\"full_residual_tolerance\":{:.17e},",
                "\"maximum_trace_mcse\":{:.17e},\"psd_cleanup\":{:.17e},",
                "\"smallest_covariance_eigenvalue\":{:.17e},",
                "\"largest_covariance_eigenvalue\":{:.17e},",
                "\"point_correction_identity_error\":{:.17e},",
                "\"aggregate_variance_mean\":{:.17e},",
                "\"nuisance_uncertainty_conditioned_away\":{},\"outer_fold_fingerprint\":{}"
            ),
            Q1_ROW_SCHEMA,
            cell.name,
            cell.gate,
            dimension,
            replication,
            target_name(target),
            seed,
            if lower.is_finite() && upper.is_finite() { "success" } else { "target_unavailable" },
            reference.name(),
            cell.variance_model(),
            cell.variance_dgp_name(),
            cell.dependence_name(),
            cell.mass_name(),
            cell.target_shape_name(),
            cell.signal_name(),
            cell.controls,
            cell.route_name(),
            coverage_eligible(cell, reference, target),
            generated.truth[target],
            points[target],
            point_error,
            estimated_variance,
            standard_error,
            nullable(lower),
            nullable(upper),
            generated.truth[target] >= lower && generated.truth[target] <= upper,
            generated.truth[target] < lower,
            generated.truth[target] > upper,
            inference.independent_units,
            inference.effective_match_count,
            inference.largest_match_mass_share,
            inference.largest_match_leverage,
            inference.smallest_maker_denominator,
            inference.influence_concentration[target],
            nullable(spectrum.leading_eigenvalue),
            nullable(spectrum.second_eigenvalue),
            nullable(spectrum.leading_share),
            nullable(spectrum.remainder_leading_share),
            nullable(spectrum.maximum_mode_weight_squared),
            nullable(spectrum.leading_residual),
            nullable(spectrum.second_residual),
            minimum_active_terms,
            minimum_training_matches,
            summary.floor_count,
            summary.floor_share,
            summary.boundary_share,
            summary.maximum_boundary_excess,
            summary.maximum_prediction_leverage,
            summary.minimum_fitted_rcond,
            structured.sensitivity.median_absolute_log_ratio,
            structured.sensitivity.p90_absolute_log_ratio,
            structured.sensitivity.maximum_absolute_log_ratio,
            structured.sensitivity.log_variance_correlation,
            inference.maximum_complete_residual,
            inference.full_residual_tolerance,
            maximum_trace_mcse,
            inference.psd_cleanup,
            inference.smallest_eigenvalue_before_cleanup,
            inference.largest_eigenvalue_before_cleanup,
            inference.point_correction_identity_error,
            aggregate_variance_mean,
            inference.nuisance_uncertainty_conditioned_away,
            structured.outer_fold.iter().fold(0xcbf2_9ce4_8422_2325_u64, |state, &fold| (state ^ u64::from(fold)).wrapping_mul(0x100_0000_01b3)),
        )
    }

    fn nullable(value: f64) -> String {
        if value.is_finite() {
            format!("{value:.17e}")
        } else {
            "null".to_owned()
        }
    }

    fn emit_q1_success(
        cell: Cell,
        reference: Reference,
        dimension: usize,
        replication: usize,
        seed: u64,
        generated: &GeneratedProblem,
        result: &vckss_core::generic_jla::GenericJlaResult,
        quadrature: bool,
    ) {
        let points = components(result.corrected);
        let inference = result
            .component_inference
            .as_ref()
            .expect("attached result");
        if (0..4).any(|target| {
            let value = inference.covariance[target * 4 + target];
            !value.is_finite() || value <= 0.0
        }) {
            emit_q1_failure(
                cell,
                reference,
                dimension,
                replication,
                seed,
                "covariance_failed",
                None,
            );
            return;
        }
        match reference {
            Reference::Q0 => {
                for target in 0..4 {
                    let standard_error = inference.covariance[target * 4 + target].sqrt();
                    let lower = points[target] - NORMAL_CRITICAL * standard_error;
                    let upper = points[target] + NORMAL_CRITICAL * standard_error;
                    let common = common_success_json(
                        cell,
                        reference,
                        dimension,
                        replication,
                        seed,
                        generated,
                        result,
                        target,
                        lower,
                        upper,
                    );
                    println!(
                        "{{{common},\"q1_diagnostics_applicable\":false,\"critical_simulations\":0,\"leading_score\":null,\"raw_leading_variance_correction\":null,\"leading_variance\":null,\"leading_recentered_component\":null,\"remainder_estimate\":null,\"remainder_identity_error\":null,\"leading_remainder_covariance\":null,\"remainder_variance\":null,\"remainder_trace_mcse\":null,\"curvature\":null,\"critical_value\":null,\"leading_f_statistic\":null,\"remainder_influence_concentration\":null,\"q1_point_identity_error\":null}}"
                    );
                }
            }
            Reference::Q1 => {
                let Some(q1) = inference.q1 else {
                    emit_q1_failure(
                        cell,
                        reference,
                        dimension,
                        replication,
                        seed,
                        "q1_result_failed",
                        None,
                    );
                    return;
                };
                for target in 0..4 {
                    let mut value = q1[target];
                    let production_critical = value.critical_value;
                    if quadrature
                        && value.status
                            == vckss_core::component_inference::ComponentQ1Status::Computed
                    {
                        let critical =
                            super::q1_reference::reference_q1_critical(value.curvature, 0.95);
                        let interval = vckss_core::component_inference::q1_am_interval(
                            [value.leading_score, value.remainder_estimate],
                            [
                                value.leading_variance,
                                value.leading_remainder_covariance,
                                value.leading_remainder_covariance,
                                value.remainder_variance,
                            ],
                            critical,
                            inference.spectrum[target].leading_eigenvalue,
                        )
                        .expect("qualified positive covariance maps to a finite ellipse image");
                        value.critical_value = critical;
                        value.confidence_lower = interval[0];
                        value.confidence_upper = interval[1];
                    }
                    let common = common_success_json(
                        cell,
                        reference,
                        dimension,
                        replication,
                        seed,
                        generated,
                        result,
                        target,
                        value.confidence_lower,
                        value.confidence_upper,
                    );
                    println!(
                        concat!(
                            "{{{},\"q1_diagnostics_applicable\":true,\"critical_simulations\":{},",
                            "\"leading_score\":{:.17e},\"raw_leading_variance_correction\":{:.17e},",
                            "\"leading_variance\":{:.17e},\"leading_recentered_component\":{:.17e},",
                            "\"remainder_estimate\":{:.17e},\"remainder_identity_error\":{:.17e},",
                            "\"leading_remainder_covariance\":{:.17e},\"remainder_variance\":{:.17e},",
                            "\"remainder_trace_mcse\":{:.17e},\"curvature\":{},",
                            "\"critical_value\":{},\"leading_f_statistic\":{},",
                            "\"remainder_influence_concentration\":{:.17e},",
                            "\"q1_point_identity_error\":{:.17e},\"quadrature_critical\":{},\"production_critical_value\":{},",
                            "\"q1_status\":{},\"standardized_determinant\":{},",
                            "\"remainder_influence_variance\":{:.17e},\"remainder_trace_variance\":{:.17e}}}"
                        ),
                        common,
                        value.critical_draws,
                        value.leading_score,
                        value.leading_variance_correction,
                        value.leading_variance,
                        value.leading_recentered_component,
                        value.remainder_estimate,
                        value.remainder_identity_error,
                        value.leading_remainder_covariance,
                        value.remainder_variance,
                        value.remainder_trace_mcse,
                        nullable(value.curvature),
                        nullable(value.critical_value),
                        nullable(value.leading_f_statistic),
                        value.remainder_influence_concentration,
                        (value.point_estimate - points[target]).abs(),
                        quadrature,
                        nullable(production_critical),
                        value.status as u32,
                        nullable(value.standardized_determinant),
                        value.remainder_influence_variance,
                        value.remainder_trace_variance,
                    );
                }
            }
        }
    }

    #[cfg(test)]
    mod q0_tests {
        use super::*;

        #[test]
        fn semantic_seed_is_frozen() {
            assert_eq!(
                semantic_seed(Q1_MASTER_SEED, "diffuse_equal_independent", 20, 0),
                4_226_334_232_446_896_308
            );
            assert_eq!(
                semantic_seed(CONFIRMATION_SEED, "diffuse_equal_independent", 20, 0),
                856_281_763_097_141_742
            );
            assert_eq!(
                semantic_seed(FIXED_FOLD_SEED, "diffuse_equal_independent", 20, 0),
                12_413_852_455_707_087_318
            );
        }

        #[test]
        fn repair_folds_are_fixed_across_independent_outcomes() {
            let (cell, reference) = q1_cell("diffuse_equal_independent").unwrap();
            let settings = Q1Settings {
                numerical: NumericalSettings {
                    estimator_probes: 256,
                    covariance_probes: 512,
                    spectrum_probes: 128,
                    spectrum_iterations: 256,
                },
                critical_simulations: 1000,
            };
            let fixed = semantic_seed(FIXED_FOLD_SEED, cell.name, 20, 0);
            let folds = [0, 1].map(|replication| {
                let seed = semantic_seed(CONFIRMATION_SEED, cell.name, 20, replication);
                assert_ne!(
                    seed,
                    semantic_seed(Q1_MASTER_SEED, cell.name, 20, replication)
                );
                let generated = make_problem(cell, 20, seed);
                run_q1_attached(cell, reference, &generated, settings, seed, fixed)
                    .unwrap()
                    .0
                    .component_inference
                    .unwrap()
                    .structured_variance
                    .unwrap()
                    .outer_fold
            });
            assert_eq!(folds[0], folds[1]);
        }

        #[test]
        fn target_specific_eligibility_is_frozen() {
            let (cell, reference) = q1_cell("diffuse_equal_independent").expect("q0 cell");
            assert!(matches!(reference, Reference::Q0));
            assert!((0..4).all(|target| coverage_eligible(cell, reference, target)));
            let (multi, reference) = q1_cell("multi_mode_diagnostic").expect("multi-mode cell");
            assert!((0..4).all(|target| !coverage_eligible(multi, reference, target)));
            let (controls, reference) =
                q1_cell("controls_varying_fixedoffset").expect("controls diagnostic");
            assert!((0..4).all(|target| !coverage_eligible(controls, reference, target)));
        }
    }
}

fn main() {
    campaign::entry();
}
