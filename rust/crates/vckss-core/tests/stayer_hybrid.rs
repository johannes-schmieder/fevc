// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::stayer_hybrid::{
    dense_stayer_observation_correction, dense_stayer_point_oracle,
    validate_stayer_rows, StayerPointOracleInput, StayerRows,
};

fn assert_close(actual: f64, expected: f64) {
    let scale = 1.0_f64.max(actual.abs()).max(expected.abs());
    assert!(
        (actual - expected).abs() <= 1.0e-12 * scale,
        "actual={actual:.17e}, expected={expected:.17e}"
    );
}

fn assert_point_equal(
    left: vckss_core::stayer_hybrid::StayerPointMoments,
    right: vckss_core::stayer_hybrid::StayerPointMoments,
) {
    assert_close(left.target_mass, right.target_mass);
    assert_close(left.worker_mean, right.worker_mean);
    assert_close(left.firm_mean, right.firm_mean);
    assert_close(left.worker_variance, right.worker_variance);
    assert_close(left.firm_variance, right.firm_variance);
    assert_close(left.covariance, right.covariance);
    assert_close(left.total_variance, right.total_variance);
    assert_close(left.accounting_error, right.accounting_error);
}

#[test]
fn collapsed_frequency_matches_expanded_physical_rows() {
    let collapsed = StayerRows {
        firm: &[0, 1],
        worker: &[0, 1],
        deletion_unit: &[0, 1],
        outcome: &[4.0, 9.0],
        frequency: &[3.0, 2.0],
        target_weight: &[3.0, 2.0],
        offset: None,
        controls: &[],
        controls_count: 0,
        retained_firms: 2,
        stayer_workers: 2,
        deletion_units: 2,
    };
    let expanded = StayerRows {
        firm: &[0, 0, 0, 1, 1],
        worker: &[0, 0, 0, 1, 1],
        deletion_unit: &[0, 0, 0, 1, 1],
        outcome: &[4.0, 4.0, 4.0, 9.0, 9.0],
        frequency: &[1.0; 5],
        target_weight: &[1.0; 5],
        offset: None,
        controls: &[],
        controls_count: 0,
        retained_firms: 2,
        stayer_workers: 2,
        deletion_units: 2,
    };
    let firm_effects = [1.0, 3.0];
    let collapsed_point = dense_stayer_point_oracle(StayerPointOracleInput {
        rows: collapsed,
        firm_effects: &firm_effects,
        control_coefficients: &[],
    })
    .expect("collapsed point oracle");
    let expanded_point = dense_stayer_point_oracle(StayerPointOracleInput {
        rows: expanded,
        firm_effects: &firm_effects,
        control_coefficients: &[],
    })
    .expect("expanded point oracle");
    assert_point_equal(collapsed_point, expanded_point);

    let collapsed_correction = dense_stayer_observation_correction(
        StayerPointOracleInput {
            rows: collapsed,
            firm_effects: &firm_effects,
            control_coefficients: &[],
        },
        5.0,
    )
    .expect("collapsed correction");
    let expanded_correction = dense_stayer_observation_correction(
        StayerPointOracleInput {
            rows: expanded,
            firm_effects: &firm_effects,
            control_coefficients: &[],
        },
        5.0,
    )
    .expect("expanded correction");
    assert_close(
        collapsed_correction.worker_variance_correction,
        expanded_correction.worker_variance_correction,
    );
    assert_close(
        collapsed_correction.total_variance_correction,
        expanded_correction.total_variance_correction,
    );
}

#[test]
fn row_permutation_preserves_point_and_correction_receipts() {
    let original = StayerRows {
        firm: &[0, 0, 1, 1],
        worker: &[0, 0, 1, 1],
        deletion_unit: &[0, 1, 2, 3],
        outcome: &[1.0, 5.0, 8.0, 12.0],
        frequency: &[1.0, 2.0, 3.0, 1.0],
        target_weight: &[2.0, 1.0, 4.0, 3.0],
        offset: Some(&[0.5, 0.0, 1.0, 1.5]),
        controls: &[1.0, 2.0, 3.0, 4.0],
        controls_count: 1,
        retained_firms: 2,
        stayer_workers: 2,
        deletion_units: 4,
    };
    let permuted = StayerRows {
        firm: &[1, 0, 1, 0],
        worker: &[1, 0, 1, 0],
        deletion_unit: &[3, 1, 2, 0],
        outcome: &[12.0, 5.0, 8.0, 1.0],
        frequency: &[1.0, 2.0, 3.0, 1.0],
        target_weight: &[3.0, 1.0, 4.0, 2.0],
        offset: Some(&[1.5, 0.0, 1.0, 0.5]),
        controls: &[4.0, 2.0, 3.0, 1.0],
        controls_count: 1,
        retained_firms: 2,
        stayer_workers: 2,
        deletion_units: 4,
    };
    let firm_effects = [0.25, 2.5];
    let controls = [0.75];
    let original_point = dense_stayer_point_oracle(StayerPointOracleInput {
        rows: original,
        firm_effects: &firm_effects,
        control_coefficients: &controls,
    })
    .expect("original point oracle");
    let permuted_point = dense_stayer_point_oracle(StayerPointOracleInput {
        rows: permuted,
        firm_effects: &firm_effects,
        control_coefficients: &controls,
    })
    .expect("permuted point oracle");
    assert_point_equal(original_point, permuted_point);

    let original_correction = dense_stayer_observation_correction(
        StayerPointOracleInput {
            rows: original,
            firm_effects: &firm_effects,
            control_coefficients: &controls,
        },
        20.0,
    )
    .expect("original correction");
    let permuted_correction = dense_stayer_observation_correction(
        StayerPointOracleInput {
            rows: permuted,
            firm_effects: &firm_effects,
            control_coefficients: &controls,
        },
        20.0,
    )
    .expect("permuted correction");
    assert_close(
        original_correction.worker_second_plugin,
        permuted_correction.worker_second_plugin,
    );
    assert_close(
        original_correction.worker_second_crossfit,
        permuted_correction.worker_second_crossfit,
    );
    assert_close(
        original_correction.worker_variance_correction,
        permuted_correction.worker_variance_correction,
    );
}

#[test]
fn pooled_target_mass_changes_only_pooled_scaling() {
    let rows = StayerRows {
        firm: &[0, 0],
        worker: &[0, 0],
        deletion_unit: &[0, 1],
        outcome: &[1.0, 3.0],
        frequency: &[1.0, 1.0],
        target_weight: &[2.0, 2.0],
        offset: None,
        controls: &[],
        controls_count: 0,
        retained_firms: 1,
        stayer_workers: 1,
        deletion_units: 2,
    };
    let firm_effects = [0.0];
    let own_mass = dense_stayer_observation_correction(
        StayerPointOracleInput {
            rows,
            firm_effects: &firm_effects,
            control_coefficients: &[],
        },
        4.0,
    )
    .expect("own-mass correction");
    let doubled_mass = dense_stayer_observation_correction(
        StayerPointOracleInput {
            rows,
            firm_effects: &firm_effects,
            control_coefficients: &[],
        },
        8.0,
    )
    .expect("doubled pooled-mass correction");
    assert_close(own_mass.stayer_target_share, 1.0);
    assert_close(doubled_mass.stayer_target_share, 0.5);
    assert!(
        doubled_mass.worker_variance_correction
            < own_mass.worker_variance_correction
    );
    assert_close(
        own_mass.total_variance_correction,
        own_mass.worker_variance_correction,
    );
    assert_close(
        doubled_mass.total_variance_correction,
        doubled_mass.worker_variance_correction,
    );
}

#[test]
fn validation_rejects_rows_outside_the_prepared_mover_graph() {
    let rows = StayerRows {
        firm: &[1],
        worker: &[0],
        deletion_unit: &[0],
        outcome: &[1.0],
        frequency: &[2.0],
        target_weight: &[1.0],
        offset: None,
        controls: &[],
        controls_count: 0,
        retained_firms: 1,
        stayer_workers: 1,
        deletion_units: 1,
    };
    let error = validate_stayer_rows(rows).expect_err("firm index must fail");
    assert_eq!(error.field, "firm");
    assert_eq!(error.row, Some(0));
}
