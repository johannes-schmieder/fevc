// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::component_inference::{
    finish_component_covariance, finish_component_covariance_with_reporting, ComponentJointStatus,
    ComponentQ0Status, ProbeMomentResult,
};
use vckss_core::interrupt::NeverInterrupt;

fn influence() -> [Vec<f64>; 3] {
    [
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ]
}

fn probe(covariance: [f64; 9]) -> ProbeMomentResult {
    ProbeMomentResult {
        covariance,
        mean: [0.0; 3],
        mcse: [0.0; 9],
        probes: 1000,
    }
}

#[test]
fn indefinite_joint_does_not_reject_positive_individual_variances() {
    // Independently specified PSD trace term, with influence term 4 I.
    // The difference has eigenvalues -1, 1, 3 but four positive scalar targets.
    let covariance = [3.0, -2.0, 0.0, -2.0, 3.0, 0.0, 0.0, 0.0, 3.0];
    let strict = finish_component_covariance(
        &influence(),
        &[1.0; 3],
        probe(covariance),
        1e-8,
        &mut NeverInterrupt,
    )
    .unwrap_err();
    assert_eq!(strict.phase, "component_inference_psd");
    let result = finish_component_covariance_with_reporting(
        &influence(),
        &[1.0; 3],
        probe(covariance),
        1e-8,
        true,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(result.joint_status, ComponentJointStatus::Indefinite);
    assert_eq!(result.q0_status, [ComponentQ0Status::Computed; 4]);
    assert_eq!(result.psd_cleanup, 0.0);
    assert_eq!(
        result.primitive_covariance,
        [1.0, 2.0, 0.0, 2.0, 1.0, 0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(result.covariance[15], 10.0);
}

#[test]
fn negative_target_is_not_floored_and_does_not_hide_total() {
    let result = finish_component_covariance_with_reporting(
        &influence(),
        &[1.0; 3],
        probe([5.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 3.0]),
        1e-8,
        true,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(
        result.joint_status,
        ComponentJointStatus::NonpositiveDiagonal
    );
    assert_eq!(
        result.q0_status,
        [
            ComponentQ0Status::NonpositiveVariance,
            ComponentQ0Status::Computed,
            ComponentQ0Status::Computed,
            ComponentQ0Status::Computed
        ]
    );
    assert_eq!(result.covariance[0], -1.0);
    assert_eq!(result.covariance[15], 4.0);
    assert_eq!(result.psd_cleanup, 0.0);
}

#[test]
fn valid_joint_matches_strict_result() {
    let trace = [3.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0];
    let strict = finish_component_covariance(
        &influence(),
        &[1.0; 3],
        probe(trace),
        1e-8,
        &mut NeverInterrupt,
    )
    .unwrap();
    let individual = finish_component_covariance_with_reporting(
        &influence(),
        &[1.0; 3],
        probe(trace),
        1e-8,
        true,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(individual.joint_status, ComponentJointStatus::Computed);
    assert_eq!(strict.covariance, individual.covariance);
    assert_eq!(strict.trace_term, individual.trace_term);
}

#[test]
fn invalid_shared_variance_is_still_fatal() {
    assert!(finish_component_covariance_with_reporting(
        &influence(),
        &[1.0, 0.0, 1.0],
        probe([0.0; 9]),
        1e-8,
        true,
        &mut NeverInterrupt,
    )
    .is_err());
}

#[test]
fn zero_linear_influence_remains_an_unavailable_target() {
    let result = finish_component_covariance_with_reporting(
        &[vec![0.0; 3], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0]],
        &[1.0; 3],
        probe([0.0; 9]),
        1e-8,
        true,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(result.q0_status[0], ComponentQ0Status::NoLinearInfluence);
    assert_eq!(result.q0_status[3], ComponentQ0Status::Computed);
}
