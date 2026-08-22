// SPDX-License-Identifier: GPL-3.0-only

//! Pure estimator-engine resolution.
//!
//! Resolution is structural and happens before estimator RNG.  In particular,
//! an automatically selected compressed engine never changes to the generic
//! engine because of a later resource or numerical failure.

use crate::error::{BackendError, ErrorCode, Result};
use crate::types::{DeletionMode, NuisanceMode};

pub const ALGORITHM_RESOLUTION_SCHEMA_VERSION: u32 = 1;
pub const ENGINE_RESOLUTION_SCHEMA_VERSION: u32 = 2;
pub const ESTIMATOR_PLAN_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlgorithmRequest {
    Auto,
    Exact,
    Jla,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstimatorAlgorithm {
    Exact,
    Jla,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineRequest {
    Auto,
    Compressed,
    Generic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedEngine {
    NotApplicable,
    Compressed,
    Generic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressedEligibilityReason {
    NotApplicable,
    Eligible,
    ObservationDeletion,
    ControlsPresent,
    SemanticPlanUnavailable,
    PhysicalRngContractUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineResolutionReason {
    ExactNotApplicable,
    JlaExplicitGeneric,
    JlaExplicitCompressed,
    JlaAutomaticCompressed,
    JlaAutomaticGenericScientificIneligibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlgorithmResolutionReason {
    ExplicitExact,
    ExplicitJla,
    AutomaticWithinExactLimit,
    AutomaticAboveExactLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlgorithmResolutionRequest {
    pub requested: AlgorithmRequest,
    pub retained_workers: usize,
    pub retained_firms: usize,
    pub controls: usize,
    pub nuisance: NuisanceMode,
    pub exact_limit: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlgorithmResolutionReceipt {
    pub schema_version: u32,
    pub requested: AlgorithmRequest,
    pub selected: EstimatorAlgorithm,
    pub reason: AlgorithmResolutionReason,
    pub retained_workers: usize,
    pub retained_firms: usize,
    pub controls: usize,
    pub nuisance: NuisanceMode,
    /// Identified W+F+Q coefficient dimension: W + F - 1 + Q.
    pub identified_complexity: usize,
    pub exact_limit: usize,
    pub resolved_after_retained_dimensions: bool,
    pub resolved_before_rng: bool,
    pub rng_draws_consumed: u64,
    pub rng_counter_atoms_consumed: u64,
    /// Frozen false: algorithm(auto) never reroutes on a later memory,
    /// rank, setup, or numerical failure.
    pub opportunistic_fallback_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineResolutionRequest {
    pub algorithm: EstimatorAlgorithm,
    pub engine: EngineRequest,
    pub deletion: DeletionMode,
    pub nuisance: NuisanceMode,
    pub controls: usize,
    /// Whether the compressed semantic match/target plan was certified.
    pub compressed_semantic_plan_ready: bool,
    /// Whether the retained physical counts satisfy the registered RNG
    /// representation used by the compressed engine.
    pub compressed_physical_rng_ready: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineResolutionReceipt {
    pub schema_version: u32,
    pub algorithm: EstimatorAlgorithm,
    pub requested: EngineRequest,
    pub selected: SelectedEngine,
    pub reason: EngineResolutionReason,
    pub compressed_eligibility: CompressedEligibilityReason,
    pub resolved_before_rng: bool,
    pub rng_draws_consumed: u64,
    pub rng_counter_atoms_consumed: u64,
    /// Frozen false: engine(auto) is not a post-selection resource or
    /// numerical fallback mechanism.
    pub opportunistic_fallback_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstimatorPlanRequest {
    pub algorithm: AlgorithmRequest,
    pub engine: EngineRequest,
    pub deletion: DeletionMode,
    pub nuisance: NuisanceMode,
    pub retained_workers: usize,
    pub retained_firms: usize,
    pub controls: usize,
    pub exact_limit: usize,
    pub compressed_semantic_plan_ready: bool,
    pub compressed_physical_rng_ready: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstimatorPlanReceipt {
    pub schema_version: u32,
    pub algorithm: AlgorithmResolutionReceipt,
    pub engine: EngineResolutionReceipt,
    pub resolved_before_rng: bool,
    pub rng_draws_consumed: u64,
    pub rng_counter_atoms_consumed: u64,
    pub opportunistic_fallback_allowed: bool,
}

#[must_use]
pub fn compressed_eligibility(request: EngineResolutionRequest) -> CompressedEligibilityReason {
    if request.algorithm == EstimatorAlgorithm::Exact {
        CompressedEligibilityReason::NotApplicable
    } else if request.deletion != DeletionMode::Match {
        CompressedEligibilityReason::ObservationDeletion
    } else if request.controls != 0 {
        CompressedEligibilityReason::ControlsPresent
    } else if !request.compressed_semantic_plan_ready {
        CompressedEligibilityReason::SemanticPlanUnavailable
    } else if !request.compressed_physical_rng_ready {
        CompressedEligibilityReason::PhysicalRngContractUnavailable
    } else {
        // With Q=0, joint and fixed-offset nuisance modes are identical.  The
        // requested nuisance value is still retained in the estimator receipt.
        CompressedEligibilityReason::Eligible
    }
}

/// Resolve `algorithm(auto)` exactly once, after retained W/F/Q dimensions
/// are known and before any estimator RNG is addressed. Controls contribute
/// to the identified dimension in both nuisance modes.
pub fn resolve_algorithm(
    request: AlgorithmResolutionRequest,
) -> Result<AlgorithmResolutionReceipt> {
    if request.retained_workers == 0 || request.retained_firms == 0 {
        return Err(BackendError::invalid(
            "algorithm_resolution",
            "retained worker and firm dimensions must be positive",
        ));
    }
    if request.exact_limit == 0 {
        return Err(BackendError::invalid(
            "algorithm_resolution",
            "exact algorithm limit must be positive",
        ));
    }
    let identified_complexity = request
        .retained_firms
        .checked_sub(1)
        .and_then(|firm_quotient| request.retained_workers.checked_add(firm_quotient))
        .and_then(|value| value.checked_add(request.controls))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "algorithm_resolution",
                "identified W+F+Q complexity overflow",
            )
        })?;
    let (selected, reason) = match request.requested {
        AlgorithmRequest::Exact => (
            EstimatorAlgorithm::Exact,
            AlgorithmResolutionReason::ExplicitExact,
        ),
        AlgorithmRequest::Jla => (
            EstimatorAlgorithm::Jla,
            AlgorithmResolutionReason::ExplicitJla,
        ),
        AlgorithmRequest::Auto if identified_complexity <= request.exact_limit => (
            EstimatorAlgorithm::Exact,
            AlgorithmResolutionReason::AutomaticWithinExactLimit,
        ),
        AlgorithmRequest::Auto => (
            EstimatorAlgorithm::Jla,
            AlgorithmResolutionReason::AutomaticAboveExactLimit,
        ),
    };
    Ok(AlgorithmResolutionReceipt {
        schema_version: ALGORITHM_RESOLUTION_SCHEMA_VERSION,
        requested: request.requested,
        selected,
        reason,
        retained_workers: request.retained_workers,
        retained_firms: request.retained_firms,
        controls: request.controls,
        nuisance: request.nuisance,
        identified_complexity,
        exact_limit: request.exact_limit,
        resolved_after_retained_dimensions: true,
        resolved_before_rng: true,
        rng_draws_consumed: 0,
        rng_counter_atoms_consumed: 0,
        opportunistic_fallback_allowed: false,
    })
}

pub fn resolve_engine(request: EngineResolutionRequest) -> Result<EngineResolutionReceipt> {
    let eligibility = compressed_eligibility(request);
    let (selected, reason) = match (request.algorithm, request.engine) {
        (EstimatorAlgorithm::Exact, EngineRequest::Compressed) => {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_resolution",
                "the exact algorithm does not support engine(compressed)",
            ));
        }
        (EstimatorAlgorithm::Exact, EngineRequest::Auto | EngineRequest::Generic) => (
            SelectedEngine::NotApplicable,
            EngineResolutionReason::ExactNotApplicable,
        ),
        (EstimatorAlgorithm::Jla, EngineRequest::Generic) => (
            SelectedEngine::Generic,
            EngineResolutionReason::JlaExplicitGeneric,
        ),
        (EstimatorAlgorithm::Jla, EngineRequest::Compressed) => {
            if eligibility != CompressedEligibilityReason::Eligible {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "engine_resolution",
                    format!("engine(compressed) is scientifically ineligible: {eligibility:?}"),
                ));
            }
            (
                SelectedEngine::Compressed,
                EngineResolutionReason::JlaExplicitCompressed,
            )
        }
        (EstimatorAlgorithm::Jla, EngineRequest::Auto)
            if eligibility == CompressedEligibilityReason::Eligible =>
        {
            (
                SelectedEngine::Compressed,
                EngineResolutionReason::JlaAutomaticCompressed,
            )
        }
        (EstimatorAlgorithm::Jla, EngineRequest::Auto) => (
            SelectedEngine::Generic,
            EngineResolutionReason::JlaAutomaticGenericScientificIneligibility,
        ),
    };
    Ok(EngineResolutionReceipt {
        schema_version: ENGINE_RESOLUTION_SCHEMA_VERSION,
        algorithm: request.algorithm,
        requested: request.engine,
        selected,
        reason,
        compressed_eligibility: eligibility,
        resolved_before_rng: true,
        rng_draws_consumed: 0,
        rng_counter_atoms_consumed: 0,
        opportunistic_fallback_allowed: false,
    })
}

/// Freeze algorithm and engine selection as one pre-RNG transaction. Neither
/// stage is reconsidered after a resource, rank, setup, or numerical failure.
pub fn resolve_estimator_plan(request: EstimatorPlanRequest) -> Result<EstimatorPlanReceipt> {
    let algorithm = resolve_algorithm(AlgorithmResolutionRequest {
        requested: request.algorithm,
        retained_workers: request.retained_workers,
        retained_firms: request.retained_firms,
        controls: request.controls,
        nuisance: request.nuisance,
        exact_limit: request.exact_limit,
    })?;
    let engine = resolve_engine(EngineResolutionRequest {
        algorithm: algorithm.selected,
        engine: request.engine,
        deletion: request.deletion,
        nuisance: request.nuisance,
        controls: request.controls,
        compressed_semantic_plan_ready: request.compressed_semantic_plan_ready,
        compressed_physical_rng_ready: request.compressed_physical_rng_ready,
    })?;
    Ok(EstimatorPlanReceipt {
        schema_version: ESTIMATOR_PLAN_SCHEMA_VERSION,
        algorithm,
        engine,
        resolved_before_rng: true,
        rng_draws_consumed: 0,
        rng_counter_atoms_consumed: 0,
        opportunistic_fallback_allowed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(algorithm: EstimatorAlgorithm, engine: EngineRequest) -> EngineResolutionRequest {
        EngineResolutionRequest {
            algorithm,
            engine,
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::Joint,
            controls: 0,
            compressed_semantic_plan_ready: true,
            compressed_physical_rng_ready: true,
        }
    }

    fn algorithm_request(requested: AlgorithmRequest) -> AlgorithmResolutionRequest {
        AlgorithmResolutionRequest {
            requested,
            retained_workers: 3,
            retained_firms: 4,
            controls: 2,
            nuisance: NuisanceMode::Joint,
            exact_limit: 8,
        }
    }

    fn plan_request(algorithm: AlgorithmRequest, engine: EngineRequest) -> EstimatorPlanRequest {
        EstimatorPlanRequest {
            algorithm,
            engine,
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::Joint,
            retained_workers: 3,
            retained_firms: 4,
            controls: 2,
            exact_limit: 8,
            compressed_semantic_plan_ready: true,
            compressed_physical_rng_ready: true,
        }
    }

    #[test]
    fn exact_auto_and_generic_are_not_applicable_but_compressed_fails() {
        let automatic = resolve_engine(request(EstimatorAlgorithm::Exact, EngineRequest::Auto))
            .expect("exact auto");
        let generic = resolve_engine(request(EstimatorAlgorithm::Exact, EngineRequest::Generic))
            .expect("exact generic");
        assert_eq!(automatic.selected, SelectedEngine::NotApplicable);
        assert_eq!(automatic.algorithm, EstimatorAlgorithm::Exact);
        assert_eq!(generic.selected, SelectedEngine::NotApplicable);
        assert_eq!(automatic.reason, EngineResolutionReason::ExactNotApplicable);
        assert_eq!(generic.reason, EngineResolutionReason::ExactNotApplicable);
        assert_eq!(
            automatic.compressed_eligibility,
            CompressedEligibilityReason::NotApplicable
        );
        assert_eq!(automatic.schema_version, ENGINE_RESOLUTION_SCHEMA_VERSION);
        assert!(automatic.resolved_before_rng);
        assert_eq!(automatic.rng_draws_consumed, 0);
        assert_eq!(automatic.rng_counter_atoms_consumed, 0);
        assert!(!automatic.opportunistic_fallback_allowed);
        let error = resolve_engine(request(
            EstimatorAlgorithm::Exact,
            EngineRequest::Compressed,
        ))
        .expect_err("exact compressed must fail");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
        assert_eq!(error.phase, "engine_resolution");
    }

    #[test]
    fn automatic_algorithm_uses_the_complete_identified_dimension_at_the_boundary() {
        let exact = resolve_algorithm(algorithm_request(AlgorithmRequest::Auto))
            .expect("boundary selects exact");
        assert_eq!(exact.identified_complexity, 3 + 4 - 1 + 2);
        assert_eq!(exact.selected, EstimatorAlgorithm::Exact);
        assert_eq!(
            exact.reason,
            AlgorithmResolutionReason::AutomaticWithinExactLimit
        );
        assert_eq!(exact.requested, AlgorithmRequest::Auto);
        assert!(exact.resolved_after_retained_dimensions);
        assert!(exact.resolved_before_rng);
        assert_eq!(exact.rng_draws_consumed, 0);
        assert_eq!(exact.rng_counter_atoms_consumed, 0);
        assert!(!exact.opportunistic_fallback_allowed);

        let above = resolve_algorithm(AlgorithmResolutionRequest {
            exact_limit: 7,
            ..algorithm_request(AlgorithmRequest::Auto)
        })
        .expect("above boundary selects JLA");
        assert_eq!(above.selected, EstimatorAlgorithm::Jla);
        assert_eq!(
            above.reason,
            AlgorithmResolutionReason::AutomaticAboveExactLimit
        );
    }

    #[test]
    fn fixed_offset_controls_count_and_explicit_algorithms_do_not_reroute() {
        let fixed = resolve_algorithm(AlgorithmResolutionRequest {
            retained_workers: 2,
            retained_firms: 2,
            controls: 3,
            nuisance: NuisanceMode::FixedOffset,
            exact_limit: 5,
            ..algorithm_request(AlgorithmRequest::Auto)
        })
        .expect("fixed-offset algorithm plan");
        assert_eq!(fixed.identified_complexity, 6);
        assert_eq!(fixed.selected, EstimatorAlgorithm::Jla);

        let exact = resolve_algorithm(AlgorithmResolutionRequest {
            requested: AlgorithmRequest::Exact,
            exact_limit: 1,
            ..algorithm_request(AlgorithmRequest::Exact)
        })
        .expect("explicit exact remains exact at planning");
        assert_eq!(exact.selected, EstimatorAlgorithm::Exact);
        assert_eq!(exact.reason, AlgorithmResolutionReason::ExplicitExact);

        let jla = resolve_algorithm(AlgorithmResolutionRequest {
            requested: AlgorithmRequest::Jla,
            exact_limit: usize::MAX,
            ..algorithm_request(AlgorithmRequest::Jla)
        })
        .expect("explicit JLA remains JLA at planning");
        assert_eq!(jla.selected, EstimatorAlgorithm::Jla);
        assert_eq!(jla.reason, AlgorithmResolutionReason::ExplicitJla);
    }

    #[test]
    fn algorithm_resolution_rejects_unresolved_dimensions_and_overflow() {
        for invalid_request in [
            AlgorithmResolutionRequest {
                retained_workers: 0,
                ..algorithm_request(AlgorithmRequest::Auto)
            },
            AlgorithmResolutionRequest {
                retained_firms: 0,
                ..algorithm_request(AlgorithmRequest::Auto)
            },
            AlgorithmResolutionRequest {
                exact_limit: 0,
                ..algorithm_request(AlgorithmRequest::Auto)
            },
        ] {
            let error = resolve_algorithm(invalid_request).expect_err("invalid plan rejects");
            assert_eq!(error.code, ErrorCode::InvalidInput);
            assert_eq!(error.phase, "algorithm_resolution");
        }
        let maximum = resolve_algorithm(AlgorithmResolutionRequest {
            retained_workers: usize::MAX,
            retained_firms: 1,
            controls: 0,
            exact_limit: usize::MAX,
            ..algorithm_request(AlgorithmRequest::Auto)
        })
        .expect("W=max, F=1, Q=0 is exactly representable");
        assert_eq!(maximum.identified_complexity, usize::MAX);
        assert_eq!(maximum.selected, EstimatorAlgorithm::Exact);

        let overflow = resolve_algorithm(AlgorithmResolutionRequest {
            retained_workers: usize::MAX,
            retained_firms: 1,
            controls: 1,
            exact_limit: usize::MAX,
            ..algorithm_request(AlgorithmRequest::Auto)
        })
        .expect_err("adding one control to the maximum dimension overflows");
        assert_eq!(overflow.code, ErrorCode::ResourceLimit);
        assert_eq!(overflow.phase, "algorithm_resolution");
    }

    #[test]
    fn combined_plan_preserves_requested_and_selected_routes_before_rng() {
        for engine in [EngineRequest::Auto, EngineRequest::Generic] {
            let exact = resolve_estimator_plan(plan_request(AlgorithmRequest::Auto, engine))
                .expect("automatic exact plan");
            assert_eq!(exact.algorithm.requested, AlgorithmRequest::Auto);
            assert_eq!(exact.algorithm.selected, EstimatorAlgorithm::Exact);
            assert_eq!(exact.engine.requested, engine);
            assert_eq!(exact.engine.selected, SelectedEngine::NotApplicable);
            assert!(exact.resolved_before_rng);
            assert_eq!(exact.rng_draws_consumed, 0);
            assert_eq!(exact.rng_counter_atoms_consumed, 0);
            assert!(!exact.opportunistic_fallback_allowed);
        }
        let error = resolve_estimator_plan(plan_request(
            AlgorithmRequest::Auto,
            EngineRequest::Compressed,
        ))
        .expect_err("automatic exact plus forced compressed rejects");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
        assert_eq!(error.phase, "engine_resolution");
    }

    #[test]
    fn jla_auto_prefers_compressed_only_when_scientifically_eligible() {
        let eligible = resolve_engine(request(EstimatorAlgorithm::Jla, EngineRequest::Auto))
            .expect("eligible auto");
        assert_eq!(eligible.selected, SelectedEngine::Compressed);
        assert_eq!(
            eligible.reason,
            EngineResolutionReason::JlaAutomaticCompressed
        );

        let cases = [
            (
                EngineResolutionRequest {
                    deletion: DeletionMode::Observation,
                    ..request(EstimatorAlgorithm::Jla, EngineRequest::Auto)
                },
                CompressedEligibilityReason::ObservationDeletion,
            ),
            (
                EngineResolutionRequest {
                    controls: 1,
                    ..request(EstimatorAlgorithm::Jla, EngineRequest::Auto)
                },
                CompressedEligibilityReason::ControlsPresent,
            ),
            (
                EngineResolutionRequest {
                    compressed_semantic_plan_ready: false,
                    ..request(EstimatorAlgorithm::Jla, EngineRequest::Auto)
                },
                CompressedEligibilityReason::SemanticPlanUnavailable,
            ),
            (
                EngineResolutionRequest {
                    compressed_physical_rng_ready: false,
                    ..request(EstimatorAlgorithm::Jla, EngineRequest::Auto)
                },
                CompressedEligibilityReason::PhysicalRngContractUnavailable,
            ),
        ];
        for (input, reason) in cases {
            let receipt = resolve_engine(input).expect("auto resolves to generic");
            assert_eq!(receipt.selected, SelectedEngine::Generic);
            assert_eq!(receipt.compressed_eligibility, reason);
            assert_eq!(
                receipt.reason,
                EngineResolutionReason::JlaAutomaticGenericScientificIneligibility
            );
            assert!(!receipt.opportunistic_fallback_allowed);
        }
    }

    #[test]
    fn combined_automatic_jla_engine_resolution_is_scientific_and_fail_closed() {
        let eligible = EstimatorPlanRequest {
            controls: 0,
            exact_limit: 5,
            ..plan_request(AlgorithmRequest::Auto, EngineRequest::Auto)
        };
        let compressed = resolve_estimator_plan(eligible).expect("eligible automatic JLA plan");
        assert_eq!(compressed.algorithm.requested, AlgorithmRequest::Auto);
        assert_eq!(compressed.algorithm.selected, EstimatorAlgorithm::Jla);
        assert_eq!(compressed.engine.selected, SelectedEngine::Compressed);
        assert_eq!(
            compressed.engine.reason,
            EngineResolutionReason::JlaAutomaticCompressed
        );

        for ineligible in [
            EstimatorPlanRequest {
                deletion: DeletionMode::Observation,
                ..eligible
            },
            EstimatorPlanRequest {
                controls: 1,
                ..eligible
            },
            EstimatorPlanRequest {
                compressed_semantic_plan_ready: false,
                ..eligible
            },
            EstimatorPlanRequest {
                compressed_physical_rng_ready: false,
                ..eligible
            },
        ] {
            let generic =
                resolve_estimator_plan(ineligible).expect("ineligible auto selects generic");
            assert_eq!(generic.algorithm.selected, EstimatorAlgorithm::Jla);
            assert_eq!(generic.engine.selected, SelectedEngine::Generic);
            assert_eq!(
                generic.engine.reason,
                EngineResolutionReason::JlaAutomaticGenericScientificIneligibility
            );
            assert!(!generic.engine.opportunistic_fallback_allowed);
        }

        let forced_generic = resolve_estimator_plan(EstimatorPlanRequest {
            engine: EngineRequest::Generic,
            ..eligible
        })
        .expect("forced generic JLA");
        assert_eq!(forced_generic.engine.selected, SelectedEngine::Generic);
        assert_eq!(
            forced_generic.engine.reason,
            EngineResolutionReason::JlaExplicitGeneric
        );

        let forced_error = resolve_estimator_plan(EstimatorPlanRequest {
            engine: EngineRequest::Compressed,
            controls: 1,
            ..eligible
        })
        .expect_err("scientifically ineligible forced compressed route rejects");
        assert_eq!(forced_error.code, ErrorCode::UnsupportedFeature);
        assert_eq!(forced_error.phase, "engine_resolution");
    }

    #[test]
    fn forced_routes_do_not_fallback_and_q_zero_fixedoffset_is_equivalent() {
        let fixed = EngineResolutionRequest {
            nuisance: NuisanceMode::FixedOffset,
            ..request(EstimatorAlgorithm::Jla, EngineRequest::Compressed)
        };
        let receipt = resolve_engine(fixed).expect("Q=0 fixed offset is compressed eligible");
        assert_eq!(receipt.selected, SelectedEngine::Compressed);
        assert!(!receipt.opportunistic_fallback_allowed);

        let ineligible = EngineResolutionRequest {
            controls: 2,
            ..fixed
        };
        let error = resolve_engine(ineligible).expect_err("forced compressed fails closed");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);

        let explicit_generic = EngineResolutionRequest {
            engine: EngineRequest::Generic,
            ..ineligible
        };
        let generic = resolve_engine(explicit_generic).expect("forced generic");
        assert_eq!(generic.selected, SelectedEngine::Generic);
        assert_eq!(generic.reason, EngineResolutionReason::JlaExplicitGeneric);
    }
}
