// SPDX-License-Identifier: GPL-3.0-only

//! Rust half of the permanent Mata matrix-provider differential oracle.
//!
//! `varcomp_kss/tests/stata/test_rust_mata_shared_atoms.do` freezes the same
//! retained design, Counter-V1 atoms, moments, target draws, and components.
//! Keeping the expected values in both language-specific test suites prevents
//! either implementation from becoming the other's runtime oracle.

use vckss_core::engine::{run_jla_no_controls, JlaEngineOptions, NumericalMcse};
use vckss_core::graph::select_match_deletion_graph;
use vckss_core::jla::{JlaPlan, VarianceComponents};
use vckss_core::krylov::PcgOptions;
use vckss_core::problem::CanonicalInput;
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute};
use vckss_core::types::InputColumns;

const TOLERANCE: f64 = 3.0e-12;

fn assert_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < TOLERANCE,
            "entry {index}: {actual:.17e} versus {expected:.17e}"
        );
    }
}

fn components(value: VarianceComponents) -> [f64; 4] {
    [value.worker, value.firm, value.covariance, value.total]
}

fn mcse(value: NumericalMcse) -> [f64; 4] {
    [value.worker, value.firm, value.covariance, value.total]
}

#[test]
fn rust_and_mata_share_the_frozen_counter_atom_oracle() {
    let worker = [1_u64, 1, 1, 1, 2, 2, 2, 2, 9, 9, 10, 10];
    let firm = [1_u64, 1, 2, 2, 1, 1, 2, 2, 9, 10, 9, 10];
    let deletion = [11_u64, 12, 21, 22, 31, 32, 41, 42, 91, 92, 93, 94];
    let outcome = [
        1.0, 3.0, 0.0, 2.0, -1.0, 1.0, 2.0, -2.0, 10.0, 11.0, 12.0, 13.0,
    ];
    let frequency = [1_u64, 2, 1, 2, 1, 3, 2, 1, 1, 1, 1, 1];
    let target = [1.0, 2.0, 2.0, 2.0, 3.0, 9.0, 8.0, 4.0, 1.0, 1.0, 1.0, 1.0];
    let order = [7_usize, 2, 8, 5, 0, 9, 3, 11, 10, 1, 6, 4];

    let canonical = CanonicalInput::from_validated(
        InputColumns {
            worker: order.iter().map(|&row| worker[row]).collect(),
            firm: order.iter().map(|&row| firm[row]).collect(),
            deletion: order.iter().map(|&row| deletion[row]).collect(),
            outcome: order.iter().map(|&row| outcome[row]).collect(),
            frequency: order.iter().map(|&row| frequency[row]).collect(),
            target_weight: order.iter().map(|&row| target[row]).collect(),
            controls: Vec::new(),
        }
        .validate()
        .expect("shared-atom input validates"),
    )
    .expect("shared-atom input canonicalizes");
    let selection = select_match_deletion_graph(&canonical).expect("graph selection");
    let expected_mask = order.map(|source_row| source_row < 8);
    assert_eq!(selection.active, expected_mask);
    assert_eq!(selection.receipt.input_rows, 12);
    assert_eq!(selection.receipt.retained_rows, 8);
    assert_eq!(selection.receipt.retained_physical_mass, 13);

    let problem = canonical
        .compress(&selection.active)
        .expect("shared-atom compression");
    assert_eq!(problem.dimensions.rows_stored, 8);
    assert_eq!(problem.dimensions.rows_physical, 13);
    assert_eq!(problem.dimensions.workers, 2);
    assert_eq!(problem.dimensions.firms, 2);
    assert_eq!(problem.dimensions.cells, 4);
    assert_eq!(problem.dimensions.deletion_units, 8);
    // Compression retains eight exact semantic row groups.  The estimator's
    // scientifically relevant (cell, per-copy target mass) plan has five.
    assert_eq!(problem.dimensions.target_strata, 8);
    assert_eq!(
        JlaPlan::build_no_controls(&problem)
            .expect("shared-atom JLA plan")
            .target_strata(),
        5
    );
    assert_eq!(problem.target_total, 31.0);

    let options = JlaEngineOptions {
        seed: 8_675_309,
        probes: 5,
        leverage_batch_width: 2,
        target_batch_width: 2,
        solver: LinearSolverOptions {
            route: LinearSolverRoute::DiagonalPcg,
            pcg: PcgOptions {
                tolerance: 1.0e-13,
                maximum_iterations: 500,
                residual_replacement_interval: 50,
            },
            full_residual_tolerance: 1.0e-11,
            ..LinearSolverOptions::default()
        },
        ..JlaEngineOptions::default()
    };
    let result = run_jla_no_controls(&problem, options).expect("shared-atom JLA result");

    assert_close(
        &result.fitted_cell,
        &[
            2.022_222_222_222_222,
            1.644_444_444_444_444_4,
            0.733_333_333_333_333_4,
            0.355_555_555_555_555_5,
        ],
    );
    assert_close(
        &result.unit_projection_share,
        &[
            0.152_262_804_075_126_46,
            0.510_455_390_334_572_6,
            0.288_959_204_429_879,
            0.878_996_218_631_832_3,
            0.127_746_135_069_161_9,
            0.425_474_254_742_547_35,
            0.372_936_576_889_661_2,
            0.245_671_769_924_166_53,
        ],
    );
    assert_close(
        &result.unit_residual_share,
        &[
            0.847_737_195_924_873_6,
            0.489_544_609_665_427_45,
            0.711_040_795_570_120_9,
            0.121_003_781_368_167_75,
            0.872_253_864_930_838_1,
            0.574_525_745_257_452_6,
            0.627_063_423_110_338_8,
            0.754_328_230_075_833_4,
        ],
    );
    assert_close(
        &result.unit_finite_bias,
        &[
            -0.010_459_834_917_505_805,
            0.000_856_591_246_347_567,
            0.004_690_655_241_321_344,
            0.017_757_832_298_514_47,
            -0.014_775_372_931_922_968,
            -0.029_847_395_362_842_533,
            -0.014_143_015_377_574_683,
            -0.008_988_934_639_795_521,
        ],
    );
    assert_close(
        &result.unit_finite_variance,
        &[
            0.004_109_429_862_540_695,
            0.015_803_408_041_061_603,
            0.032_130_730_670_155_33,
            0.004_617_151_459_123_924,
            0.003_331_706_170_581_966,
            0.021_332_842_411_230_485,
            0.018_713_182_354_403_637,
            0.007_273_619_149_137_075,
        ],
    );
    assert_close(
        &result.unit_residual_mass,
        &[
            -1.022_222_222_222_222_1,
            1.955_555_555_555_555_7,
            -1.644_444_444_444_444_4,
            0.711_111_111_111_111_2,
            -1.733_333_333_333_333_4,
            0.799_999_999_999_999_8,
            3.288_888_888_888_888_8,
            -2.355_555_555_555_555_6,
        ],
    );
    assert_close(
        &result.unit_deleted_mass,
        &[
            -1.184_051_183_272_414_8,
            3.738_214_731_150_928_4,
            -2.181_006_167_619_276_7,
            4.886_041_088_573_386_5,
            -1.944_825_436_299_603_8,
            1.230_119_697_737_058,
            4.877_000_503_581_939,
            -3.045_590_340_676_231,
        ],
    );
    assert_close(
        &result.cell_correction_weight,
        &[
            21.245_237_203_633_152,
            19.544_164_354_293_546,
            5.635_184_529_510_777,
            25.599_182_695_680_216,
        ],
    );

    let expected_draws = [
        [
            0.000_005_919_592_148_325,
            0.147_954_148_570_416_78,
            -0.000_359_573_162_336_864,
            0.147_240_921_837_891_36,
        ],
        [
            0.323_827_444_037_945_4,
            0.039_106_598_805_320_12,
            0.043_237_408_133_709_15,
            0.449_408_859_110_683_84,
        ],
        [
            0.001_030_790_584_914_836,
            0.287_553_349_471_305_9,
            0.006_614_884_114_337_031,
            0.301_813_908_284_894_8,
        ],
        [
            0.120_809_628_649_632_4,
            0.078_926_039_911_184_26,
            -0.037_517_918_390_190_03,
            0.124_699_831_780_436_59,
        ],
        [
            0.328_321_006_422_348_74,
            0.887_253_202_407_564_8,
            -0.207_372_386_777_208_82,
            0.800_829_435_275_495_9,
        ],
    ];
    for (draw, expected) in result.target_draws.iter().zip(expected_draws) {
        assert_close(&components(*draw), &expected);
    }

    assert_close(
        &components(result.plugin),
        &[
            0.290_413_535_283_462_5,
            0.035_641_885_381_739_71,
            -0.006_080_086_329_826_184,
            0.313_895_248_005_549_85,
        ],
    );
    assert_close(
        &components(result.correction),
        &[
            0.154_798_957_857_397_95,
            0.288_158_667_833_158_36,
            -0.039_079_517_216_337_91,
            0.364_798_591_257_880_5,
        ],
    );
    assert_close(
        &components(result.corrected),
        &[
            0.135_614_577_426_064_53,
            -0.252_516_782_451_418_6,
            0.032_999_430_886_511_72,
            -0.050_903_343_252_330_646,
        ],
    );
    assert_close(
        &mcse(result.numerical_mcse),
        &[
            0.073_294_385_508_162_88,
            0.155_624_092_071_748_77,
            0.043_981_937_835_619_02,
            0.123_739_164_874_365_75,
        ],
    );

    assert_eq!(result.receipt.leverage_rhs.len(), 5);
    assert_eq!(result.receipt.target_rhs.len(), 10);
    assert_eq!(
        result.receipt.solver.selected,
        LinearSolverRoute::DiagonalPcg
    );
    assert!(result.receipt.accounting_residual <= 1.0e-12);
    assert!(result.receipt.max_complete_residual <= 1.0e-11);
    for rhs in std::iter::once(&result.receipt.full_fit)
        .chain(result.receipt.leverage_rhs.iter())
        .chain(result.receipt.target_rhs.iter())
    {
        assert_eq!(rhs.route, LinearSolverRoute::DiagonalPcg);
        assert!(rhs.complete_residual <= 1.0e-11);
    }
}
