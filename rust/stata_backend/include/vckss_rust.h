/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef VCKSS_RUST_H
#define VCKSS_RUST_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define VCKSS_RUST_ABI_VERSION_V1 1u
#define VCKSS_DELETION_MATCH 1u
#define VCKSS_RNG_NONE 0u
#define VCKSS_RNG_COUNTER_V1 1u
#define VCKSS_ROUTE_AUTO 0u
#define VCKSS_ROUTE_EXACT 1u
#define VCKSS_ROUTE_DIAGONAL_PCG 2u
#define VCKSS_ROUTE_CMG_PCG 3u
#define VCKSS_ROUTE_NOT_APPLICABLE 4u
#define VCKSS_INTERRUPT_CONTINUE 0
#define VCKSS_INTERRUPT_USER_BREAK 1
#define VCKSS_CORE_MATCH_GRAPH_READY (UINT64_C(1) << 0)
#define VCKSS_CORE_EXACT_READY (UINT64_C(1) << 1)
#define VCKSS_CORE_DIAGONAL_PCG_READY (UINT64_C(1) << 2)
#define VCKSS_CORE_BATCHED_PCG_READY (UINT64_C(1) << 3)
#define VCKSS_CORE_CMG_GRAPH_READY (UINT64_C(1) << 4)
#define VCKSS_CORE_SOLVER_ROUTER_READY (UINT64_C(1) << 5)
#define VCKSS_CORE_COUNTER_RNG_READY (UINT64_C(1) << 6)
#define VCKSS_CORE_JLA_PLAN_READY (UINT64_C(1) << 7)
#define VCKSS_CORE_FULL_CMG_V2_READY (UINT64_C(1) << 8)
#define VCKSS_CORE_PROJECTION_JLA_READY (UINT64_C(1) << 9)
#define VCKSS_CORE_FULL_CMG_POINT_ROUTING_V1_READY (UINT64_C(1) << 10)
#define VCKSS_CORE_FULL_CMG_MODEL_V1_READY (UINT64_C(1) << 11)
#define VCKSS_CORE_GENERIC_EXECUTION_V1_READY (UINT64_C(1) << 12)
#define VCKSS_CORE_AUTOMATIC_COMPONENT_BATCH_V1_READY (UINT64_C(1) << 13)
#define VCKSS_CORE_RESOLVED_EXECUTION_V1_READY (UINT64_C(1) << 14)
#define VCKSS_SUPPORT_EXACT (UINT64_C(1) << 0)
#define VCKSS_SUPPORT_JLA (UINT64_C(1) << 1)
#define VCKSS_SUPPORT_MATCH_DELETION (UINT64_C(1) << 2)
#define VCKSS_SUPPORT_OBSERVATION_DELETION (UINT64_C(1) << 3)
#define VCKSS_SUPPORT_CONTROLS (UINT64_C(1) << 4)
#define VCKSS_SUPPORT_DIAGONAL (UINT64_C(1) << 5)
#define VCKSS_SUPPORT_CMG (UINT64_C(1) << 6)
#define VCKSS_DELETION_OBSERVATION 2u
#define VCKSS_ALGORITHM_AUTO 0u
#define VCKSS_ALGORITHM_EXACT 1u
#define VCKSS_ALGORITHM_JLA 2u
#define VCKSS_NUISANCE_JOINT 1u
#define VCKSS_NUISANCE_FIXED_OFFSET 2u
#define VCKSS_ENGINE_AUTO_OR_UNSPECIFIED 0u
#define VCKSS_ENGINE_COMPRESSED 1u
#define VCKSS_ENGINE_GENERIC 2u
#define VCKSS_ENGINE_NOT_APPLICABLE 3u
#define VCKSS_PROJECTION_EFFECT_WORKER 1u
#define VCKSS_PROJECTION_EFFECT_FIRM 2u
#define VCKSS_PROJECTION_WEIGHT_FREQUENCY 1u
#define VCKSS_PROJECTION_WEIGHT_TARGET 2u
#define VCKSS_PROJECTION_SCHEMA_V1 1u
#define VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON 1u
#define VCKSS_COMPONENT_VARIANCE_STRUCTURED_LEVERAGE 2u
#define VCKSS_COMPONENT_REFERENCE_Q0 0u
#define VCKSS_COMPONENT_REFERENCE_Q1 1u
#define VCKSS_COMPONENT_INFERENCE_SCHEMA_V1 1u
#define VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V2 2u
#define VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V3 3u
#define VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V4 4u
#define VCKSS_REQUEST_CAPABILITY_SCHEMA_V1 1u
#define VCKSS_REQUEST_CAPABILITY_SCHEMA_V2 2u
#define VCKSS_REQUEST_CAPABILITY_SCHEMA_V3 3u
#define VCKSS_REQUEST_FREQUENCY_UNIT 0u
#define VCKSS_REQUEST_FREQUENCY_LITERAL 1u
#define VCKSS_REQUEST_PROFILE_NONE 0u
#define VCKSS_REQUEST_PROFILE_EXACT_V1 1u
#define VCKSS_REQUEST_PROFILE_JLA_COUNTER_V1 2u
#define VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1 3u
#define VCKSS_REQUEST_PROFILE_PLANNED_V1 4u
#define VCKSS_REQUEST_REASON_SUPPORTED 0u
#define VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA 1u
#define VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM 2u
#define VCKSS_REQUEST_REASON_ALGORITHM_AUTO_UNRESOLVED 3u
#define VCKSS_REQUEST_REASON_UNKNOWN_DELETION 4u
#define VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE 5u
#define VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE 6u
#define VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT 7u
#define VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE 8u
#define VCKSS_REQUEST_REASON_CONTROLS_LIMIT 9u
#define VCKSS_REQUEST_REASON_EXACT_RNG 10u
#define VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE 11u
#define VCKSS_REQUEST_REASON_JLA_DELETION 12u
#define VCKSS_REQUEST_REASON_JLA_NUISANCE 13u
#define VCKSS_REQUEST_REASON_JLA_CONTROLS 14u
#define VCKSS_REQUEST_REASON_JLA_RNG 15u
#define VCKSS_REQUEST_REASON_UNKNOWN_ENGINE 16u
#define VCKSS_REQUEST_REASON_EXACT_ENGINE 17u
#define VCKSS_REQUEST_REASON_JLA_ENGINE_AUTO_UNRESOLVED 18u
#define VCKSS_REQUEST_REASON_JLA_GENERIC_SOLVER_ROUTE 19u
#define VCKSS_REQUEST_REASON_UNKNOWN_BATCH_MODE 20u
#define VCKSS_REQUEST_REASON_UNKNOWN_STAYERS_MODE 21u
#define VCKSS_REQUEST_REASON_UNKNOWN_TARGET_WEIGHT_MODE 22u
#define VCKSS_REQUEST_REASON_UNKNOWN_DELETION_UNIT_SOURCE 23u
#define VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED 24u
#define VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED 25u
#define VCKSS_REQUEST_REASON_PHYSICAL_LIMIT 26u
#define VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH 27u
#define VCKSS_REQUEST_REASON_BATCH_MODE_UNSUPPORTED 28u
#define VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED 29u
#define VCKSS_REQUEST_REASON_BATCH_SUMMARY_MISMATCH 30u
#define VCKSS_REQUEST_REASON_WALLSECONDS_VALUE 31u
#define VCKSS_REQUEST_REASON_FALLBACK_ROUTE_MISMATCH 32u
#define VCKSS_REQUEST_REASON_AUTO_RNG 33u
#define VCKSS_REQUEST_REASON_AUTO_ENGINE_COMPRESSED 34u
#define VCKSS_REQUEST_REASON_COMPRESSED_SCIENTIFIC_INELIGIBILITY 35u
#define VCKSS_BATCH_MODE_AUTO 0u
#define VCKSS_BATCH_MODE_EXPLICIT 1u
#define VCKSS_BATCH_MODE_INDEPENDENT 2u
#define VCKSS_BATCH_MODE_NOT_APPLICABLE 3u
#define VCKSS_PLAN_APPLICABILITY_NONE 0u
#define VCKSS_PLAN_APPLICABILITY_EXACT 1u
#define VCKSS_PLAN_APPLICABILITY_COMPRESSED 2u
#define VCKSS_PLAN_APPLICABILITY_GENERIC 3u
#define VCKSS_BATCH_SELECTION_NOT_APPLICABLE 0u
#define VCKSS_BATCH_SELECTION_AUTO 1u
#define VCKSS_BATCH_SELECTION_EXPLICIT 2u
#define VCKSS_WALL_STATUS_NOT_REQUESTED 0u
#define VCKSS_WALL_STATUS_UNCALIBRATED 1u
#define VCKSS_WALL_STATUS_WITHIN 2u
#define VCKSS_WALL_STATUS_EXCEEDS 3u
#define VCKSS_STAYERS_MOVERS 1u
#define VCKSS_STAYERS_ALL 2u
#define VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT 0u
#define VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT 1u
#define VCKSS_DELETION_SOURCE_CELL_DEFAULT 1u
#define VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT 2u
#define VCKSS_DELETION_SOURCE_OBSERVATION_ROW 3u
#define VCKSS_RHS_STATUS_ZERO 1u
#define VCKSS_RHS_STATUS_CONVERGED 2u
#define VCKSS_RESIDUAL_SPACE_WORKER_FIRM 1u
#define VCKSS_RESIDUAL_SPACE_WORKER_FIRM_CONTROL 2u
#define VCKSS_EXACT_DIAGNOSTIC_WORKING_FIT (UINT64_C(1) << 0)
#define VCKSS_EXACT_DIAGNOSTIC_INVERSE_SQRT (UINT64_C(1) << 1)
#define VCKSS_EXACT_DIAGNOSTIC_MAKER (UINT64_C(1) << 2)
#define VCKSS_EXACT_DIAGNOSTIC_CONTROL_BASIS (UINT64_C(1) << 3)
#define VCKSS_EXACT_DIAGNOSTIC_DELETION_RANK (UINT64_C(1) << 4)
#define VCKSS_EXACT_DIAGNOSTIC_FIRM_ZERO_SUM (UINT64_C(1) << 5)
#define VCKSS_EXACT_DIAGNOSTIC_FIT_PEAK (UINT64_C(1) << 6)
#define VCKSS_EXACT_DIAGNOSTIC_CORRECTION_PEAK (UINT64_C(1) << 7)
#define VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING (UINT64_C(1) << 8)
#define VCKSS_GENERIC_DIAGNOSTIC_CONTROL_RANK (UINT64_C(1) << 0)
#define VCKSS_GENERIC_DIAGNOSTIC_DELETION_RANK (UINT64_C(1) << 1)
#define VCKSS_GENERIC_DIAGNOSTIC_FULL_JOINT_FIT (UINT64_C(1) << 2)
#define VCKSS_GENERIC_DIAGNOSTIC_WORKING_FIT (UINT64_C(1) << 3)
#define VCKSS_GENERIC_DIAGNOSTIC_MAKER (UINT64_C(1) << 4)
#define VCKSS_GENERIC_DIAGNOSTIC_MEMORY_PHASES (UINT64_C(1) << 5)
#define VCKSS_GENERIC_DIAGNOSTIC_RHS_V2 (UINT64_C(1) << 6)

typedef struct VckssBackendCapabilitiesV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    uint64_t core_ready_flags;
    uint64_t support_flags;
    uint32_t deterministic_parallelism;
    uint32_t reserved;
} VckssBackendCapabilitiesV1;

typedef struct VckssBackendRequestCapabilityRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t request_schema;
    uint32_t algorithm;
    uint32_t deletion_mode;
    uint32_t nuisance_mode;
    uint32_t solver_route;
    uint32_t rng_contract;
    uint32_t controls_count;
    uint32_t frequency_use;
    uint64_t reserved;
} VckssBackendRequestCapabilityRequestV1;

typedef struct VckssBackendRequestCapabilityReceiptV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    uint32_t request_schema;
    uint32_t supported;
    uint32_t reason_code;
    uint32_t profile_code;
    uint32_t algorithm;
    uint32_t deletion_mode;
    uint32_t nuisance_mode;
    uint32_t solver_route;
    uint32_t rng_contract;
    uint32_t controls_count;
    uint32_t frequency_use;
    uint32_t reserved;
    uint64_t request_signature;
} VckssBackendRequestCapabilityReceiptV1;

typedef struct VckssBackendRequestCapabilityRequestV2 {
    VckssBackendRequestCapabilityRequestV1 v1;
    uint32_t engine;
    uint32_t batch_mode;
    uint32_t stayers_mode;
    uint32_t target_weight_mode;
    uint32_t deletion_unit_source;
    uint32_t probeorder_supplied;
    uint32_t wallseconds_supplied;
    uint32_t reserved_2;
    uint64_t physical_limit;
} VckssBackendRequestCapabilityRequestV2;

typedef struct VckssBackendRequestCapabilityReceiptV2 {
    VckssBackendRequestCapabilityReceiptV1 v1;
    uint32_t engine;
    uint32_t batch_mode;
    uint32_t stayers_mode;
    uint32_t target_weight_mode;
    uint32_t deletion_unit_source;
    uint32_t probeorder_supplied;
    uint32_t wallseconds_supplied;
    uint32_t reserved_2;
    uint64_t physical_limit;
} VckssBackendRequestCapabilityReceiptV2;

typedef struct VckssBackendRequestCapabilityRequestV3 {
    VckssBackendRequestCapabilityRequestV2 v2;
    uint32_t leverage_batch_mode;
    uint32_t target_batch_mode;
    uint32_t allow_automatic_cmg_setup_fallback;
    uint32_t reserved_3;
    double wallseconds;
    uint64_t reserved_4;
} VckssBackendRequestCapabilityRequestV3;

typedef struct VckssBackendRequestCapabilityReceiptV3 {
    VckssBackendRequestCapabilityReceiptV2 v2;
    uint32_t leverage_batch_mode;
    uint32_t target_batch_mode;
    uint32_t allow_automatic_cmg_setup_fallback;
    uint32_t reserved_3;
    double wallseconds;
    uint32_t algorithm_resolution_deferred;
    uint32_t engine_resolution_deferred;
    uint32_t route_resolution_deferred;
    uint32_t leverage_batch_resolution_deferred;
    uint32_t target_batch_resolution_deferred;
    uint32_t wall_advisory_only;
    uint64_t reserved_4;
} VckssBackendRequestCapabilityReceiptV3;

typedef struct VckssEnginePrepareRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint64_t rows;
    uint32_t cleanup_abandoned;
    uint32_t reserved;
} VckssEnginePrepareRequestV1;

typedef struct VckssEnginePrepareRequestV2 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint64_t rows;
    uint32_t cleanup_abandoned;
    uint32_t reserved;
    uint64_t memory_limit_bytes;
    uint64_t caller_copy_bytes;
} VckssEnginePrepareRequestV2;

typedef struct VckssEnginePrepareRequestV3 {
    VckssEnginePrepareRequestV2 v2;
    uint32_t deletion_mode;
    uint32_t controls_count;
    uint64_t reserved_2;
} VckssEnginePrepareRequestV3;

typedef struct VckssEnginePrepareRequestV4 {
    VckssEnginePrepareRequestV3 v3;
    uint32_t implicit_match;
    uint32_t reserved_3;
} VckssEnginePrepareRequestV4;

typedef int32_t (*VckssInterruptPollV1)(void *context);

/* Additive synchronous reporting scope. Existing estimator layouts are frozen. */
typedef struct VckssProgressUpdateV1 {
    uint64_t sequence;
    uint32_t kind;
    uint32_t reserved;
    uint64_t completed;
    uint64_t total;
    uint64_t values[8];
} VckssProgressUpdateV1;
typedef int32_t (*VckssProgressDisplayV1)(void *, const VckssProgressUpdateV1 *, uint64_t, uint32_t);
typedef int32_t (*VckssProgressOperationV1)(void *);
typedef struct VckssProgressOptionsV1 {
    uint32_t struct_size;
    uint32_t schema;
    uint32_t level;
    uint32_t reserved;
    VckssProgressDisplayV1 display;
    void *context;
} VckssProgressOptionsV1;
int32_t vckss_rust_report_call_v1(const VckssProgressOptionsV1 *, VckssProgressOperationV1, void *);


typedef struct VckssEnginePrepareRequestInterruptV1 {
    VckssEnginePrepareRequestV2 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEnginePrepareRequestInterruptV1;

typedef struct VckssEnginePrepareRequestInterruptV2 {
    VckssEnginePrepareRequestV3 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEnginePrepareRequestInterruptV2;

typedef struct VckssEnginePrepareRequestInterruptV3 {
    VckssEnginePrepareRequestV4 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEnginePrepareRequestInterruptV3;

typedef struct VckssEngineColumnsV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t rows;
    const double *worker;
    const double *firm;
    const double *deletion;
    const double *outcome;
    const double *frequency;
    const double *target_weight;
} VckssEngineColumnsV1;

typedef struct VckssEngineColumnsV2 {
    VckssEngineColumnsV1 v1;
    const double *const *controls;
    uint32_t controls_count;
    uint32_t reserved_2;
} VckssEngineColumnsV2;

typedef struct VckssEngineColumnsV3 {
    VckssEngineColumnsV2 v2;
    const double *probe_order;
    uint32_t probeorder_supplied;
    uint32_t reserved_3;
} VckssEngineColumnsV3;

typedef struct VckssStayerAugmentationRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint64_t rows;
    uint32_t controls_count;
    uint32_t reserved;
    uint64_t caller_copy_bytes;
} VckssStayerAugmentationRequestV1;

typedef struct VckssStayerAugmentationRequestInterruptV1 {
    VckssStayerAugmentationRequestV1 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssStayerAugmentationRequestInterruptV1;

typedef struct VckssStayerAugmentationColumnsV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t rows;
    const double *firm;
    const double *worker;
    const double *outcome;
    const double *frequency;
    const double *target_weight;
    const double *const *controls;
    uint32_t controls_count;
    uint32_t reserved_2;
} VckssStayerAugmentationColumnsV1;

typedef struct VckssProjectionAugmentationRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint64_t rows;
    uint32_t project_count;
    uint32_t effect;
    uint32_t weight;
    uint32_t reserved;
    double rank_tolerance;
    uint64_t caller_copy_bytes;
} VckssProjectionAugmentationRequestV1;

typedef struct VckssProjectionAugmentationRequestInterruptV1 {
    VckssProjectionAugmentationRequestV1 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssProjectionAugmentationRequestInterruptV1;

typedef struct VckssProjectionColumnsV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t rows;
    const double *const *project;
    uint32_t project_count;
    uint32_t reserved_2;
} VckssProjectionColumnsV1;

typedef struct VckssComponentInferenceAugmentationRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t variance_source;
    uint32_t reference_distribution;
    uint32_t probes;
    uint32_t batch_width;
    uint32_t spectrum_probes;
    uint32_t spectrum_iterations;
    uint64_t seed;
    double psd_tolerance;
    double spectrum_tolerance;
    double confidence_level;
    uint32_t critical_simulations;
    uint32_t observations_per_term;
    uint64_t fold_seed;
    double variance_rank_tolerance;
    double positivity_multiplier;
    uint64_t reserved;
} VckssComponentInferenceAugmentationRequestV1;

typedef struct VckssComponentInferenceAugmentationRequestInterruptV1 {
    VckssComponentInferenceAugmentationRequestV1 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssComponentInferenceAugmentationRequestInterruptV1;

/* Frozen ABI-1 session spellings.  These distinct struct tags are retained
 * for C source compatibility; the corresponding symbols alias the engine V1
 * registry and layouts. */
typedef struct VckssPrepareRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint64_t rows;
    uint32_t cleanup_abandoned;
    uint32_t reserved;
} VckssPrepareRequestV1;

typedef struct VckssColumnsV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t rows;
    const double *worker;
    const double *firm;
    const double *deletion;
    const double *outcome;
    const double *frequency;
    const double *target_weight;
} VckssColumnsV1;

typedef struct VckssPreparationReceiptV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t generation;
    uint64_t input_rows;
    uint64_t retained_rows;
    uint64_t workers;
    uint64_t firms;
    uint64_t cells;
    uint64_t deletion_units;
    uint64_t target_strata;
} VckssPreparationReceiptV1;

typedef struct VckssSessionSnapshotV1 {
    uint32_t struct_size;
    uint32_t state;
    uint64_t generation;
    uint64_t last_released_generation;
} VckssSessionSnapshotV1;

typedef struct VckssEngineSolveRequestV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint64_t seed;
    uint32_t probes;
    uint32_t leverage_batch_width;
    uint32_t target_batch_width;
    uint32_t deletion_mode;
    uint32_t rng_contract;
    uint32_t solver_route;
    uint32_t allow_automatic_cmg_setup_fallback;
    uint64_t exact_dimension_limit;
    uint64_t cmg_minimum_dimension;
    double pcg_tolerance;
    uint32_t maximum_iterations;
    uint32_t residual_replacement_interval;
    double rank_tolerance;
    double block_tolerance;
    uint64_t cmg_terminal_vertices;
    uint64_t cmg_dense_vertex_cap;
    uint64_t cmg_maximum_levels;
    uint64_t cmg_aggregate_cap;
    double cmg_minimum_reduction;
    double cmg_jacobi_weight;
    uint32_t cmg_pre_sweeps;
    uint32_t cmg_post_sweeps;
    double cmg_maximum_edge_complexity;
    double cmg_maximum_vertex_complexity;
    uint64_t cmg_memory_limit_bytes;
} VckssEngineSolveRequestV1;

typedef struct VckssEngineSolveRequestV2 {
    VckssEngineSolveRequestV1 v1;
    uint32_t algorithm;
    uint32_t nuisance_mode;
    uint64_t exact_estimator_limit;
    uint64_t blocksize_limit;
} VckssEngineSolveRequestV2;

typedef struct VckssEngineSolveRequestV3 {
    VckssEngineSolveRequestV2 v2;
    uint32_t engine;
    uint32_t batch_mode;
    uint32_t stayers_mode;
    uint32_t target_weight_mode;
    uint32_t deletion_unit_source;
    uint32_t probeorder_supplied;
    uint32_t wallseconds_supplied;
    uint32_t capability_schema;
    uint32_t capability_profile;
    uint32_t frequency_use;
    uint64_t physical_limit;
    uint64_t request_signature;
    uint64_t reserved_3;
} VckssEngineSolveRequestV3;

typedef struct VckssEngineSolveRequestV4 {
    VckssEngineSolveRequestV3 v3;
    uint32_t leverage_batch_mode;
    uint32_t target_batch_mode;
    double wallseconds;
    uint64_t reserved_4;
} VckssEngineSolveRequestV4;

typedef struct VckssEngineSolveRequestV5 {
    VckssEngineSolveRequestV4 v4;
    uint32_t threads;
    uint32_t tolerance_supplied;
    uint32_t full_cmg_v2;
    uint32_t reserved_5;
} VckssEngineSolveRequestV5;

/* Execution-only extension of V4; V5 meanings and layouts remain frozen. */
#define VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE 1
#define VCKSS_GENERIC_EXECUTION_DIRECT_ATTACHMENTS 2
#define VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO 3
typedef struct VckssEngineSolveRequestV6 {
    VckssEngineSolveRequestV4 v4;
    uint32_t execution_mode;
    uint32_t threads;
    uint64_t reserved_6;
} VckssEngineSolveRequestV6;

typedef struct VckssEngineSolveRequestV7 {
    VckssEngineSolveRequestV6 v6;
    uint32_t component_batch_mode;
    uint32_t reserved_7;
} VckssEngineSolveRequestV7;

/* Additive resolver-aware diagonal execution. Exact/compressed selections
 * retain their ordinary paths and do not create generic-work receipts. */
typedef struct VckssEngineSolveRequestV8 {
    VckssEngineSolveRequestV7 v7;
    uint32_t resolved_execution_mode;
    uint32_t tolerance_supplied;
} VckssEngineSolveRequestV8;

typedef struct VckssEngineSolveRequestInterruptV1 {
    VckssEngineSolveRequestV1 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV1;

typedef struct VckssEngineSolveRequestInterruptV2 {
    VckssEngineSolveRequestV2 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV2;

typedef struct VckssEngineSolveRequestInterruptV3 {
    VckssEngineSolveRequestV3 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV3;

typedef struct VckssEngineSolveRequestInterruptV4 {
    VckssEngineSolveRequestV4 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV4;

typedef struct VckssEngineSolveRequestInterruptV5 {
    VckssEngineSolveRequestV5 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV5;

typedef struct VckssEngineSolveRequestInterruptV6 {
    VckssEngineSolveRequestV6 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV6;

typedef struct VckssEngineSolveRequestInterruptV7 {
    VckssEngineSolveRequestV7 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV7;

typedef struct VckssEngineSolveRequestInterruptV8 {
    VckssEngineSolveRequestV8 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV8;

/* Experimental opt-in exact executor. No legacy entrypoint selects it and
 * the ordinary core-ready capability mask remains unchanged. */
typedef struct VckssExactExecutionRequestV1 {
    VckssEngineSolveRequestV4 v4;
    uint32_t threads;
    uint32_t reserved;
} VckssExactExecutionRequestV1;

typedef struct VckssExactExecutionRequestInterruptV1 {
    VckssExactExecutionRequestV1 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssExactExecutionRequestInterruptV1;

typedef struct VckssExactExecutionReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint32_t requested_threads;
    uint32_t worker_limit; /* Selected bound, not measured active cores. */
    uint64_t parallel_regions;
    uint64_t estimator_passes;
    uint64_t incremental_forecast_bytes;
    uint64_t command_peak_forecast_bytes;
    double maximum_original_fit_residual;
} VckssExactExecutionReceiptV1;

uint32_t vckss_rust_exact_execution_schema_v1(void);
uint32_t vckss_rust_exact_legacy_execution_schema_v1(void);
int32_t vckss_rust_engine_solve_exact_legacy_execution_interrupt_v1(
    uint64_t generation, const VckssEngineSolveRequestInterruptV2 *request,
    uint32_t threads);
/* Resolved-auto-only selector. Reuses the frozen payload layout without
 * changing V1's explicit-exact-only contract or the request signature. */
uint32_t vckss_rust_exact_resolved_execution_schema_v2(void);
int32_t vckss_rust_engine_solve_exact_resolved_execution_v2(
    uint64_t generation, const VckssExactExecutionRequestV1 *request);
int32_t vckss_rust_engine_solve_exact_resolved_execution_interrupt_v2(
    uint64_t generation, const VckssExactExecutionRequestInterruptV1 *request);
int32_t vckss_rust_engine_solve_exact_execution_v1(
    uint64_t generation, const VckssExactExecutionRequestV1 *request);
int32_t vckss_rust_engine_solve_exact_execution_interrupt_v1(
    uint64_t generation, const VckssExactExecutionRequestInterruptV1 *request);
int32_t vckss_rust_engine_exact_execution_receipt_v1(
    uint64_t generation, VckssExactExecutionReceiptV1 *output,
    uint32_t output_capacity_bytes);

typedef struct VckssEnginePreparationReceiptV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t generation;
    uint64_t input_rows;
    uint64_t retained_rows;
    uint64_t workers;
    uint64_t firms;
    uint64_t cells;
    uint64_t deletion_units;
    uint64_t target_strata;
} VckssEnginePreparationReceiptV1;

typedef struct VckssEnginePreparationReceiptV2 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t generation;
    uint64_t input_rows;
    uint64_t retained_rows;
    uint64_t workers;
    uint64_t firms;
    uint64_t cells;
    uint64_t deletion_units;
    uint64_t target_strata;
    uint64_t memory_limit_bytes;
    uint64_t caller_copy_bytes;
    uint64_t preparation_peak_forecast_bytes;
    uint64_t prepared_resident_bytes;
    uint64_t graph_input_rows;
    uint64_t graph_retained_rows;
    uint64_t graph_input_physical_mass;
    uint64_t graph_retained_physical_mass;
    uint64_t graph_initial_components;
    uint64_t graph_maximum_components;
    uint64_t graph_initial_component_rows;
    uint64_t graph_mover_input_rows;
    uint64_t graph_initial_deletion_edges;
    uint64_t graph_retained_deletion_edges;
    uint64_t graph_insufficient_workers_removed;
    uint64_t graph_articulation_workers_removed;
    uint64_t graph_bridge_units_removed;
    uint64_t graph_bridge_rows_removed;
    uint64_t graph_degree_iterations;
    uint64_t graph_articulation_iterations;
    uint64_t graph_bridge_iterations;
    uint64_t graph_fixed_point_iterations;
} VckssEnginePreparationReceiptV2;

typedef struct VckssEnginePreparationReceiptV3 {
    VckssEnginePreparationReceiptV2 v2;
    double target_weight_sum;
} VckssEnginePreparationReceiptV3;

typedef struct VckssEnginePreparationReceiptV4 {
    VckssEnginePreparationReceiptV3 v3;
    uint32_t controls_count;
    uint32_t deletion_mode;
} VckssEnginePreparationReceiptV4;

typedef struct VckssStayerAugmentationReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint64_t mover_stored_rows;
    uint64_t stayer_stored_rows;
    uint64_t combined_stored_rows;
    uint64_t mover_physical_mass;
    uint64_t stayer_physical_mass;
    uint64_t combined_physical_mass;
    uint64_t mover_workers;
    uint64_t stayer_workers;
    uint64_t combined_workers;
    uint64_t firms;
    uint64_t mover_deletion_units;
    uint64_t stayer_deletion_units;
    uint64_t combined_deletion_units;
    double mover_target_mass;
    double stayer_target_mass;
    double combined_target_mass;
    uint64_t topology_checksum;
    uint64_t memory_limit_bytes;
    uint64_t caller_copy_bytes;
    uint64_t augmentation_peak_forecast_bytes;
    uint64_t augmented_resident_bytes;
    uint64_t total_prepared_resident_bytes;
} VckssStayerAugmentationReceiptV1;

typedef struct VckssProjectionAugmentationReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint64_t rows;
    uint64_t columns;
    uint32_t effect;
    uint32_t weight;
    uint64_t caller_copy_bytes;
    uint64_t augmentation_peak_forecast_bytes;
    uint64_t projection_persistent_bytes;
    uint64_t total_prepared_resident_bytes;
    double gram_rcond;
    double gram_relres;
    double gram_original_relres;
} VckssProjectionAugmentationReceiptV1;

typedef struct VckssProjectionResultReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint64_t columns;
    uint32_t effect;
    uint32_t weight;
    double gram_rcond;
    double gram_relres;
    double gram_original_relres;
    double covariance_smallest_eigenvalue;
    double covariance_largest_eigenvalue;
    double psd_cleanup;
    double proxy_minimum;
    double proxy_maximum;
    uint32_t maximum_iterations;
    uint32_t reserved;
    double maximum_reduced_residual;
    double maximum_complete_residual;
    double full_residual_tolerance;
    uint64_t projection_peak_forecast_bytes;
    uint64_t persistent_bytes;
    uint64_t result_bytes;
} VckssProjectionResultReceiptV1;

typedef struct VckssComponentInferenceAugmentationReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint64_t rows;
    uint32_t variance_source;
    uint32_t reference_distribution;
    uint64_t augmentation_peak_forecast_bytes;
    uint64_t component_persistent_bytes;
    uint64_t total_prepared_resident_bytes;
    uint64_t reserved;
} VckssComponentInferenceAugmentationReceiptV1;

typedef struct VckssComponentInferenceResultReceiptV2 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint32_t variance_source;
    uint32_t reference_distribution;
    uint32_t probes;
    uint32_t q1_present;
    uint64_t counter_atoms;
    uint64_t counter_words;
    uint64_t peak_forecast_bytes;
    double psd_cleanup;
    double smallest_eigenvalue_before_cleanup;
    double largest_eigenvalue_before_cleanup;
    double point_correction_identity_error;
    uint32_t maximum_iterations;
    uint32_t reserved;
    double maximum_reduced_residual;
    double maximum_complete_residual;
    double full_residual_tolerance;
    uint32_t structured_schema_version;
    uint32_t fold_rows;
    uint32_t cv_rows;
    uint32_t reserved_2;
    double median_absolute_log_ratio;
    double p90_absolute_log_ratio;
    double maximum_absolute_log_ratio;
    double log_variance_correlation;
} VckssComponentInferenceResultReceiptV2;

typedef struct VckssComponentInferenceResultReceiptV3 {
    VckssComponentInferenceResultReceiptV2 v2;
    uint32_t q1_columns;
    uint32_t critical_simulations;
    double maximum_remainder_identity_error;
    uint64_t reserved;
} VckssComponentInferenceResultReceiptV3;

typedef struct VckssComponentInferenceResultReceiptV4 {
    VckssComponentInferenceResultReceiptV3 v3;
    uint32_t computed_targets;
    uint32_t solver_columns;
    uint64_t critical_draws;
} VckssComponentInferenceResultReceiptV4;

typedef struct VckssComponentInferenceResultReceiptV5 {
    VckssComponentInferenceResultReceiptV4 v4;
    uint32_t joint_status;
    uint32_t variance_fit;
    uint32_t q0_status[4];
    uint32_t gram_probes;
    uint32_t ordering;
    double gram_rcond;
    double gram_inverse_relres;
    double variance_fit_relres;
    double positivity_floor;
    uint64_t floored_predictions;
    uint64_t nonpositive_predictions;
} VckssComponentInferenceResultReceiptV5;

/* Additive inference-unit metadata. Deletion codes use the preparation ABI. */
typedef struct VckssComponentInferenceUnitReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint32_t deletion_mode;
    uint32_t nuisance_uncertainty_omitted;
    uint64_t independent_units;
    double effective_match_count;
    double largest_match_mass_share;
    double largest_match_leverage;
    double smallest_maker_denominator;
} VckssComponentInferenceUnitReceiptV1;

typedef struct VckssComponentVectorV1 {
    double worker;
    double firm;
    double covariance;
    double total;
} VckssComponentVectorV1;

typedef struct VckssEngineResultV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t generation;
    VckssComponentVectorV1 plugin;
    VckssComponentVectorV1 correction;
    VckssComponentVectorV1 corrected;
    VckssComponentVectorV1 numerical_mcse;
} VckssEngineResultV1;

typedef struct VckssStayerHybridResultV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    VckssComponentVectorV1 plugin;
    VckssComponentVectorV1 correction;
    VckssComponentVectorV1 corrected;
    VckssComponentVectorV1 mover_correction;
    VckssComponentVectorV1 stayer_correction;
    double weighted_rss;
    uint64_t parameters;
    uint64_t full_parameters;
    uint64_t correction_parameters;
    uint64_t deletion_units;
    double max_leverage;
    double information_rcond;
    double inverse_relres;
    double inverse_original_relres;
    double inverse_sqrt_relres;
    double maker_relres;
    double full_fit_relres;
    double working_fit_relres;
    double fit_residual_tolerance;
    double control_basis_relres;
    double control_basis_forward_error;
    double deletion_rank_gap;
    double firm_zero_sum_residual;
    uint64_t peak_forecast_bytes;
    uint64_t fit_peak_forecast_bytes;
    uint64_t correction_peak_forecast_bytes;
    uint64_t topology_checksum;
    double accounting_residual;
} VckssStayerHybridResultV1;

typedef struct VckssEngineDetailedReceiptV1 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t generation;
    uint64_t seed;
    uint32_t probes_requested;
    uint32_t leverage_probes_accepted;
    uint32_t target_probes_accepted;
    uint32_t solver_requested;
    uint32_t solver_selected;
    uint32_t solver_fallback;
    int32_t solver_fallback_error;
    uint64_t solver_dimension;
    uint64_t leverage_batch_width;
    uint64_t target_batch_width;
    double rank_tolerance;
    double block_tolerance;
    double full_residual_tolerance;
    uint32_t full_fit_route;
    uint32_t full_fit_iterations;
    double full_fit_reduced_residual;
    double full_fit_complete_residual;
    uint32_t full_fit_zero_rhs;
    uint32_t reserved_1;
    uint64_t leverage_rhs_count;
    uint64_t target_rhs_count;
    double max_reduced_residual;
    double max_complete_residual;
    double max_leverage;
    double max_reciprocal_residual;
    double accounting_residual;
    uint64_t topology_checksum;
    uint64_t cmg_levels;
    uint64_t cmg_fine_vertices;
    uint64_t cmg_fine_edges;
    uint64_t cmg_terminal_vertices;
    double cmg_edge_complexity;
    double cmg_vertex_complexity;
    uint64_t cmg_structural_bytes;
    uint64_t cmg_workspace_bytes;
    uint64_t cmg_dense_factor_bytes;
} VckssEngineDetailedReceiptV1;

typedef struct VckssEngineDetailedReceiptV2 {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t generation;
    uint64_t seed;
    uint32_t probes_requested;
    uint32_t leverage_probes_accepted;
    uint32_t target_probes_accepted;
    uint32_t solver_requested;
    uint32_t solver_selected;
    uint32_t solver_fallback;
    int32_t solver_fallback_error;
    uint64_t solver_dimension;
    uint64_t leverage_batch_width;
    uint64_t target_batch_width;
    double rank_tolerance;
    double block_tolerance;
    double full_residual_tolerance;
    uint32_t full_fit_route;
    uint32_t full_fit_iterations;
    double full_fit_reduced_residual;
    double full_fit_complete_residual;
    uint32_t full_fit_zero_rhs;
    uint32_t reserved_1;
    uint64_t leverage_rhs_count;
    uint64_t target_rhs_count;
    double max_reduced_residual;
    double max_complete_residual;
    double max_leverage;
    double max_reciprocal_residual;
    double accounting_residual;
    uint64_t topology_checksum;
    uint64_t cmg_levels;
    uint64_t cmg_fine_vertices;
    uint64_t cmg_fine_edges;
    uint64_t cmg_terminal_vertices;
    double cmg_edge_complexity;
    double cmg_vertex_complexity;
    uint64_t cmg_structural_bytes;
    uint64_t cmg_workspace_bytes;
    uint64_t cmg_dense_factor_bytes;
    double full_fit_weighted_rss;
    uint64_t memory_limit_bytes;
    uint64_t caller_copy_bytes;
    uint64_t preparation_peak_forecast_bytes;
    uint64_t prepared_resident_bytes;
    uint64_t solver_setup_forecast_bytes;
    uint64_t leverage_phase_forecast_bytes;
    uint64_t target_phase_forecast_bytes;
    uint64_t result_forecast_bytes;
    uint64_t solve_peak_forecast_bytes;
    uint64_t command_peak_forecast_bytes;
} VckssEngineDetailedReceiptV2;

typedef struct VckssEngineDetailedReceiptV3 {
    VckssEngineDetailedReceiptV2 v2;
    uint32_t rng_contract;
    uint32_t reserved_2;
    uint64_t rhs_receipt_rows;
    uint64_t caller_result_copy_bytes;
} VckssEngineDetailedReceiptV3;

typedef struct VckssEngineDetailedReceiptV4 {
    VckssEngineDetailedReceiptV3 v3;
    uint32_t algorithm_requested;
    uint32_t algorithm_selected;
    uint32_t deletion_mode;
    uint32_t nuisance_mode;
    uint64_t parameters;
    uint64_t full_parameters;
    uint64_t correction_parameters;
    double information_rcond;
    double inverse_relres;
    uint64_t exact_peak_forecast_bytes;
} VckssEngineDetailedReceiptV4;

typedef struct VckssEngineDetailedReceiptV5 {
    VckssEngineDetailedReceiptV4 v4;
    uint64_t applicability_flags;
    double working_fit_complete_residual;
    double inverse_sqrt_relres;
    double maker_relres;
    double control_basis_relres;
    double control_basis_forward_error;
    double deletion_rank_gap;
    double firm_zero_sum_residual;
    uint64_t fit_peak_forecast_bytes;
    uint64_t correction_peak_forecast_bytes;
    double actual_accounting_residual;
} VckssEngineDetailedReceiptV5;

typedef struct VckssEngineDetailedReceiptV6 {
    VckssEngineDetailedReceiptV5 v5;
    uint32_t engine_requested;
    uint32_t engine_selected;
    uint64_t generic_applicability_flags;
    uint32_t controls_count;
    uint32_t rhs_receipt_schema;
    uint64_t control_projection_rhs_count;
    double control_rank_rcond;
    double control_rank_smallest_generalized_eigenvalue_lower;
    double control_rank_largest_generalized_eigenvalue_upper;
    double control_rank_projection_error_bound;
    double control_rank_normalization_error_bound;
    double control_rank_fe_information_eigenvalue_lower_bound;
    double control_rank_maximum_projection_residual;
    double control_rank_effective_tolerance;
    double control_rank_projection_pcg_tolerance;
    double control_rank_projection_residual_gate;
    double generic_control_basis_relres;
    double generic_control_basis_forward_error;
    double generic_control_schur_rcond;
    double generic_control_schur_relres;
    double generic_deletion_rank_gap;
    double full_joint_fit_complete_residual;
    double generic_working_fit_complete_residual;
    double generic_maker_relres;
    uint64_t canonicalization_peak_forecast_bytes;
    uint64_t generic_fit_peak_forecast_bytes;
    uint64_t geometry_peak_forecast_bytes;
    uint64_t generic_leverage_peak_forecast_bytes;
    uint64_t generic_target_peak_forecast_bytes;
    uint64_t maker_peak_forecast_bytes;
    uint64_t generic_result_forecast_bytes;
    uint64_t generic_peak_forecast_bytes;
    /* Combined simultaneously-live export workspace: one native 96-byte
       V2 row plus fifteen caller-owned doubles (120 bytes) per RHS row. */
    uint64_t rhs_v2_caller_copy_bytes;
    uint32_t capability_schema;
    uint32_t capability_profile;
    uint32_t batch_mode;
    uint32_t stayers_mode;
    uint32_t target_weight_mode;
    uint32_t deletion_unit_source;
    uint32_t probeorder_supplied;
    uint32_t wallseconds_supplied;
    uint32_t frequency_use;
    uint32_t reserved_6;
    uint64_t physical_limit;
    uint64_t request_signature;
} VckssEngineDetailedReceiptV6;

typedef struct VckssEstimatorResolutionReceiptV1 {
    uint32_t algorithm_schema;
    uint32_t algorithm_requested;
    uint32_t algorithm_selected;
    uint32_t algorithm_reason;
    uint32_t engine_schema;
    uint32_t engine_requested;
    uint32_t engine_selected;
    uint32_t engine_reason;
    uint32_t compressed_eligibility;
    uint32_t resolved_before_rng;
    uint32_t opportunistic_engine_fallback_allowed;
    uint32_t reserved;
    uint64_t identified_complexity;
    uint64_t exact_limit;
    uint64_t rng_draws_before_resolution;
    uint64_t counter_atoms_before_resolution;
} VckssEstimatorResolutionReceiptV1;

typedef struct VckssSolverExecutionReceiptV1 {
    uint32_t schema_version;
    uint32_t requested_route;
    uint32_t selected_route;
    uint32_t fallback_used;
    int32_t fallback_error;
    uint32_t full_setup_complete;
    uint32_t fe_setup_complete;
    uint32_t fe_hierarchy_reused;
    uint32_t plan_frozen_before_rng;
    uint32_t auto_route_contract;
    uint32_t threads_requested;
    uint32_t threads_used;
    uint32_t parallel_regions;
    uint32_t applicability;
    uint32_t reserved_1;
    uint32_t reserved_2;
    uint64_t planned_rhs;
    uint64_t auto_firm_threshold;
    uint64_t auto_rhs_threshold;
    uint64_t full_solver_dimension;
    uint64_t fe_solver_dimension;
    uint64_t logical_atoms_before_plan_freeze;
    uint64_t unique_words_before_plan_freeze;
    uint64_t physical_trials_before_plan_freeze;
} VckssSolverExecutionReceiptV1;

typedef struct VckssBatchPhasePlanReceiptV1 {
    uint32_t request_mode;
    uint32_t selection_reason;
    uint32_t applicability;
    uint32_t reserved;
    uint64_t requested_width;
    uint64_t selected_width;
    uint64_t probe_width_cap;
    uint64_t declared_threads;
    uint64_t thread_width_cap;
    uint64_t route_width_cap;
    uint64_t effective_width_cap;
    uint64_t hard_memory_bytes;
    uint64_t width_one_forecast_bytes;
    uint64_t selected_forecast_bytes;
} VckssBatchPhasePlanReceiptV1;

typedef struct VckssBatchExecutionReceiptV1 {
    uint32_t schema_version;
    uint32_t deterministic;
    uint32_t width_invariance_required;
    uint32_t arithmetic_contract;
    uint32_t whole_command_admitted;
    uint32_t applicability;
    uint64_t non_batched_peak_bytes;
    uint64_t selected_command_peak_bytes;
    VckssBatchPhasePlanReceiptV1 leverage;
    VckssBatchPhasePlanReceiptV1 target;
} VckssBatchExecutionReceiptV1;

typedef struct VckssWallExecutionReceiptV1 {
    uint32_t schema_version;
    uint32_t model_code;
    uint32_t status;
    uint32_t routing_effect;
    uint32_t requested_applicable;
    uint32_t forecast_applicable;
    uint32_t advisory_applicable;
    uint32_t margin_applicable;
    double requested_seconds;
    double forecast_seconds;
    double advisory_seconds;
    double advisory_margin_fraction;
    uint64_t preparation_work;
    uint64_t engine_setup_work;
    uint64_t full_fit_work;
    uint64_t leverage_work;
    uint64_t target_work;
    uint64_t result_export_work;
    uint64_t total_work;
} VckssWallExecutionReceiptV1;

typedef struct VckssCounterPhaseExecutionReceiptV1 {
    uint64_t planned_logical_atoms;
    uint64_t actual_logical_atoms;
    uint64_t planned_unique_packed_words;
    uint64_t actual_unique_packed_words;
    uint64_t planned_physical_trials;
    uint64_t actual_physical_trials;
    uint64_t planned_generator_work;
    uint64_t actual_generator_work;
} VckssCounterPhaseExecutionReceiptV1;

typedef struct VckssCounterExecutionReceiptV1 {
    uint32_t schema_version;
    uint32_t rng_contract;
    uint32_t generator_work_applicable;
    uint32_t completed;
    VckssCounterPhaseExecutionReceiptV1 leverage;
    VckssCounterPhaseExecutionReceiptV1 target;
    VckssCounterPhaseExecutionReceiptV1 total;
    uint64_t logical_atoms_before_plan_freeze;
    uint64_t unique_words_before_plan_freeze;
    uint64_t physical_trials_before_plan_freeze;
} VckssCounterExecutionReceiptV1;

typedef struct VckssExecutionMemoryReceiptV1 {
    uint32_t schema_version;
    uint32_t applicability;
    uint32_t peak_phase;
    uint32_t reserved;
    uint64_t hard_limit_bytes;
    uint64_t prepared_persistent_bytes;
    uint64_t setup_peak_bytes;
    uint64_t fit_peak_bytes;
    uint64_t correction_peak_bytes;
    uint64_t leverage_peak_bytes;
    uint64_t target_peak_bytes;
    uint64_t result_peak_bytes;
    uint64_t non_batched_peak_bytes;
    uint64_t command_peak_bytes;
    uint64_t shared_cmg_persistent_bytes;
    uint64_t full_control_block_persistent_bytes;
    uint64_t setup_transient_bytes;
    uint64_t cmg_preconditioner_workspace_bytes;
    uint64_t cmg_aggregated_cell_capacity_bytes;
    uint64_t cmg_group_index_bytes;
    uint64_t cmg_hybrid_graph_bytes;
    uint64_t retained_nq_bytes;
    uint64_t retained_q2_bytes;
} VckssExecutionMemoryReceiptV1;

typedef struct VckssExecutionPlanReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint64_t applicability_flags;
    uint64_t contract_flags;
    uint64_t request_signature;
    VckssEstimatorResolutionReceiptV1 resolution;
    VckssSolverExecutionReceiptV1 solver;
    VckssBatchExecutionReceiptV1 batch;
    VckssWallExecutionReceiptV1 wall;
    VckssCounterExecutionReceiptV1 counter;
    VckssExecutionMemoryReceiptV1 memory;
} VckssExecutionPlanReceiptV1;

typedef struct VckssEngineDetailedReceiptV7 {
    VckssEngineDetailedReceiptV6 v6;
    VckssExecutionPlanReceiptV1 execution;
} VckssEngineDetailedReceiptV7;

typedef struct VckssEnginePerformanceReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint64_t applicability_flags;
    uint32_t algorithm_selected;
    uint32_t engine_selected;
    uint64_t ingest_ns;
    uint64_t canonicalize_ns;
    uint64_t graph_ns;
    uint64_t compress_ns;
    uint64_t plan_ns;
    uint64_t stayer_augmentation_ns;
    uint64_t solve_ns;
    uint64_t native_total_ns;
} VckssEnginePerformanceReceiptV1;

typedef struct VckssFullCmgReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint32_t backend_identity;
    uint32_t platform_os;
    uint32_t platform_arch;
    uint32_t batch_strategy_mask;
    uint8_t cmg_source_commit[40];
    uint32_t threads_requested;
    uint32_t threads_used;
    uint64_t maximum_concurrency;
    uint64_t vertices;
    uint64_t edges;
    uint64_t hierarchy_levels;
    uint64_t terminal_vertices;
    uint64_t graph_copy_bytes;
    uint64_t hierarchy_bytes;
    uint64_t plan_bytes;
    uint64_t workspace_bytes_each;
    uint64_t workspace_pool_bytes;
    uint64_t admitted_peak_bytes;
    double fit_effective_tolerance;
    double probe_effective_tolerance;
    double fit_initial_inner_tolerance;
    double probe_initial_inner_tolerance;
    uint64_t refinement_attempts;
    uint64_t refined_columns;
    uint64_t batch_calls;
    uint64_t rhs_count;
    uint64_t serial_batches;
    uint64_t planned_batches;
    uint64_t across_rhs_batches;
    uint64_t total_iterations;
    uint64_t total_operator_applications;
    uint64_t total_preconditioner_applications;
    double maximum_reduced_residual;
    double maximum_complete_residual;
    uint64_t graph_ns;
    uint64_t hierarchy_plan_ns;
    uint64_t rhs_ns;
    uint64_t solve_ns;
    uint64_t extraction_ns;
    uint64_t preparation_peak_bytes;
    uint64_t prepared_persistent_bytes;
    uint64_t non_cmg_command_peak_bytes;
    uint64_t pre_rng_forecast_bytes;
    uint64_t actual_retained_bytes;
    uint64_t allocator_allowance_bytes;
    uint64_t maximum_batch_rhs;
    uint64_t workspace_count;
} VckssFullCmgReceiptV1;

/* Additive; never append model accounting to the frozen 400-byte receipt. */
typedef struct VckssFullCmgModelReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint32_t controls_count;
    uint32_t nuisance_mode;
    uint64_t logical_rhs_count;
    uint64_t explicit_options_rhs_count;
    uint64_t controlled_rhs_count;
    uint64_t control_refinement_rhs_count;
} VckssFullCmgModelReceiptV1;

int32_t vckss_rust_engine_full_cmg_model_receipt_v1(
    uint64_t generation, VckssFullCmgModelReceiptV1 *output,
    uint32_t output_capacity_bytes);

/* Actual complete logical work. Queued RHSs exclude scalar diagonal fits;
   CMG RHSs include fits and complete-model refinements. Active workers are
   measured only for the diagonal queue (zero for CMG); CMG reports selected
   concurrency separately. Allocation forecasts are not RSS. */
typedef struct VckssGenericExecutionReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t generation;
    uint32_t execution_mode;
    uint32_t permitted_threads;
    uint32_t planned_workers;
    uint32_t maximum_active_workers;
    uint32_t cmg_selected_concurrency;
    uint32_t reserved;
    uint64_t maximum_rhs_capacity;
    uint64_t leverage_batch_width;
    uint64_t target_batch_width;
    uint64_t fit_rhs_count;
    uint64_t control_projection_rhs_count;
    uint64_t point_probe_rhs_count;
    uint64_t projection_rhs_count;
    uint64_t component_rhs_count;
    uint64_t gram_rhs_count;
    uint64_t logical_rhs_count;
    uint64_t queued_rhs_count;
    uint64_t cmg_rhs_count;
    uint64_t control_refinement_rhs_count;
    uint64_t command_peak_forecast_bytes;
    double maximum_complete_residual;
} VckssGenericExecutionReceiptV1;

typedef struct VckssComponentBatchReceiptV1 {
    uint32_t struct_size;
    uint32_t schema_version;
    uint32_t policy;
    uint32_t selection_reason;
    uint64_t generation;
    uint64_t declared_component_width;
    uint64_t declared_gram_width;
    uint64_t component_width;
    uint64_t gram_width;
    uint64_t automatic_cap;
    uint64_t permitted_threads;
    uint64_t maximum_rhs_capacity;
    uint64_t command_peak_forecast_bytes;
} VckssComponentBatchReceiptV1;

int32_t vckss_rust_engine_default_solve_request_v6(
    VckssEngineSolveRequestV6 *output, uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_default_solve_request_interrupt_v6(
    VckssEngineSolveRequestInterruptV6 *output, uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_solve_v6(
    uint64_t generation, const VckssEngineSolveRequestV6 *request);
int32_t vckss_rust_engine_solve_interrupt_v6(
    uint64_t generation, const VckssEngineSolveRequestInterruptV6 *request);
int32_t vckss_rust_engine_generic_execution_receipt_v1(
    uint64_t generation, VckssGenericExecutionReceiptV1 *output,
    uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_default_solve_request_interrupt_v7(
    VckssEngineSolveRequestInterruptV7 *output, uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_solve_interrupt_v7(
    uint64_t generation, const VckssEngineSolveRequestInterruptV7 *request);
int32_t vckss_rust_engine_component_batch_receipt_v1(
    uint64_t generation, VckssComponentBatchReceiptV1 *output,
    uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_default_solve_request_v8(
    VckssEngineSolveRequestV8 *output, uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_default_solve_request_interrupt_v8(
    VckssEngineSolveRequestInterruptV8 *output, uint32_t output_capacity_bytes);
int32_t vckss_rust_engine_solve_v8(
    uint64_t generation, const VckssEngineSolveRequestV8 *request);
int32_t vckss_rust_engine_solve_interrupt_v8(
    uint64_t generation, const VckssEngineSolveRequestInterruptV8 *request);

typedef struct VckssEngineRhsReceiptV1 {
    uint32_t phase;
    uint32_t side;
    int64_t probe;
    uint32_t route;
    uint32_t iterations;
    uint32_t zero_rhs;
    uint32_t reserved;
    double reduced_residual;
    double complete_residual;
} VckssEngineRhsReceiptV1;

typedef struct VckssEngineRhsReceiptV2 {
    VckssEngineRhsReceiptV1 v1;
    uint32_t status;
    uint32_t residual_replacements;
    uint64_t operator_applications;
    uint64_t preconditioner_applications;
    double full_residual_tolerance;
    uint32_t residual_space;
    uint32_t reserved_2;
    uint64_t solver_dimension;
} VckssEngineRhsReceiptV2;

typedef struct VckssEngineSnapshotV1 {
    uint32_t struct_size;
    uint32_t state;
    uint64_t generation;
    uint64_t last_released_generation;
} VckssEngineSnapshotV1;

uint32_t vckss_rust_abi_version(void);
const char *vckss_rust_backend_version(void);
const char *vckss_rust_capabilities_json(void);
const char *vckss_rust_last_error(void);
int32_t vckss_rust_selftest(void);
int32_t vckss_rust_backend_capabilities_v1(
    VckssBackendCapabilitiesV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_backend_request_capability_v1(
    const VckssBackendRequestCapabilityRequestV1 *request,
    VckssBackendRequestCapabilityReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_backend_request_capability_v2(
    const VckssBackendRequestCapabilityRequestV2 *request,
    VckssBackendRequestCapabilityReceiptV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_backend_request_capability_v3(
    const VckssBackendRequestCapabilityRequestV3 *request,
    VckssBackendRequestCapabilityReceiptV3 *output,
    uint32_t output_capacity_bytes
);

int32_t vckss_rust_session_prepare_v1(
    const VckssPrepareRequestV1 *request,
    const VckssColumnsV1 *columns,
    uint64_t *output_handle
);
int32_t vckss_rust_session_preparation_receipt_v1(
    uint64_t generation,
    VckssPreparationReceiptV1 *output
);
int32_t vckss_rust_session_release_v1(uint64_t generation);
int32_t vckss_rust_session_clear_abandoned_v1(void);
int32_t vckss_rust_session_snapshot_v1(VckssSessionSnapshotV1 *output);
const char *vckss_rust_session_last_error(void);

int32_t vckss_rust_engine_default_solve_request_v1(
    VckssEngineSolveRequestV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_prepare_request_v3(
    VckssEnginePrepareRequestV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_v2(
    VckssEngineSolveRequestV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_v3(
    VckssEngineSolveRequestV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_v4(
    VckssEngineSolveRequestV4 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_v5(
    VckssEngineSolveRequestV5 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_prepare_request_interrupt_v1(
    VckssEnginePrepareRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_prepare_request_interrupt_v2(
    VckssEnginePrepareRequestInterruptV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_prepare_request_interrupt_v3(
    VckssEnginePrepareRequestInterruptV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_stayer_augmentation_request_interrupt_v1(
    VckssStayerAugmentationRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_projection_augmentation_request_interrupt_v1(
    VckssProjectionAugmentationRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(
    VckssComponentInferenceAugmentationRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_interrupt_v1(
    VckssEngineSolveRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_interrupt_v2(
    VckssEngineSolveRequestInterruptV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_interrupt_v3(
    VckssEngineSolveRequestInterruptV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_interrupt_v4(
    VckssEngineSolveRequestInterruptV4 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_interrupt_v5(
    VckssEngineSolveRequestInterruptV5 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_v1(
    const VckssEnginePrepareRequestV1 *request,
    const VckssEngineColumnsV1 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_admit_prepare_v2(
    const VckssEnginePrepareRequestV2 *request
);
int32_t vckss_rust_engine_admit_prepare_v3(
    const VckssEnginePrepareRequestV3 *request
);
int32_t vckss_rust_engine_admit_prepare_probe_order_v1(
    const VckssEnginePrepareRequestV3 *request,
    uint32_t probeorder_supplied
);
int32_t vckss_rust_engine_admit_prepare_v4(
    const VckssEnginePrepareRequestV4 *request,
    uint32_t probeorder_supplied
);
int32_t vckss_rust_engine_prepare_v2(
    const VckssEnginePrepareRequestV2 *request,
    const VckssEngineColumnsV1 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_v3(
    const VckssEnginePrepareRequestV3 *request,
    const VckssEngineColumnsV2 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_interrupt_v1(
    const VckssEnginePrepareRequestInterruptV1 *request,
    const VckssEngineColumnsV1 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_interrupt_v2(
    const VckssEnginePrepareRequestInterruptV2 *request,
    const VckssEngineColumnsV2 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_interrupt_v3(
    const VckssEnginePrepareRequestInterruptV2 *request,
    const VckssEngineColumnsV3 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_interrupt_v4(
    const VckssEnginePrepareRequestInterruptV3 *request,
    const VckssEngineColumnsV3 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_augment_stayers_v1(
    uint64_t generation,
    const VckssStayerAugmentationRequestV1 *request,
    const VckssStayerAugmentationColumnsV1 *columns
);
int32_t vckss_rust_engine_augment_stayers_interrupt_v1(
    uint64_t generation,
    const VckssStayerAugmentationRequestInterruptV1 *request,
    const VckssStayerAugmentationColumnsV1 *columns
);
int32_t vckss_rust_engine_stayer_augmentation_receipt_v1(
    uint64_t generation,
    VckssStayerAugmentationReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_augment_projection_interrupt_v1(
    uint64_t generation,
    const VckssProjectionAugmentationRequestInterruptV1 *request,
    const VckssProjectionColumnsV1 *columns
);
int32_t vckss_rust_engine_projection_augmentation_receipt_v1(
    uint64_t generation,
    VckssProjectionAugmentationReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_augment_component_inference_interrupt_v1(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request
);
int32_t vckss_rust_engine_augment_match_component_inference_interrupt_v1(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request
);
int32_t vckss_rust_engine_component_inference_unit_receipt_v1(
    uint64_t generation,
    VckssComponentInferenceUnitReceiptV1 *output,
    uint32_t output_capacity_bytes
);
/* V2 changes policy, not the frozen V1 tuning-request layout. */
uint32_t vckss_rust_component_inference_interface_version(void);
int32_t vckss_rust_engine_augment_component_inference_interrupt_v2(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request
);
int32_t vckss_rust_engine_augment_match_component_inference_interrupt_v2(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request
);
/* V3 selects unified residual moments for either deletion unit. */
int32_t vckss_rust_engine_augment_component_inference_interrupt_v3(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request
);
int32_t vckss_rust_engine_augment_match_component_inference_interrupt_v3(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request
);
/* V4 selects direct residual covariance; gram_probes is in [512, INT32_MAX]. */
int32_t vckss_rust_engine_augment_component_inference_interrupt_v4(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request,
    uint32_t gram_probes
);
int32_t vckss_rust_engine_augment_match_component_inference_interrupt_v4(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request,
    uint32_t gram_probes
);
int32_t vckss_rust_engine_augment_component_inference_interrupt_v5(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request,
    uint32_t gram_probes
);
int32_t vckss_rust_engine_augment_match_component_inference_interrupt_v5(
    uint64_t generation,
    const VckssComponentInferenceAugmentationRequestInterruptV1 *request,
    uint32_t gram_probes
);
int32_t vckss_rust_engine_component_inference_augmentation_receipt_v1(
    uint64_t generation,
    VckssComponentInferenceAugmentationReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_preparation_receipt_v1(
    uint64_t generation,
    VckssEnginePreparationReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_preparation_receipt_v2(
    uint64_t generation,
    VckssEnginePreparationReceiptV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_preparation_receipt_v3(
    uint64_t generation,
    VckssEnginePreparationReceiptV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_preparation_receipt_v4(
    uint64_t generation,
    VckssEnginePreparationReceiptV4 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_retained_mask_v1(
    uint64_t generation,
    uint8_t *output,
    uint64_t output_capacity_bytes
);
int32_t vckss_rust_engine_solve_v1(
    uint64_t generation,
    const VckssEngineSolveRequestV1 *request
);
int32_t vckss_rust_engine_solve_v2(
    uint64_t generation,
    const VckssEngineSolveRequestV2 *request
);
int32_t vckss_rust_engine_solve_v3(
    uint64_t generation,
    const VckssEngineSolveRequestV3 *request
);
int32_t vckss_rust_engine_solve_v4(
    uint64_t generation,
    const VckssEngineSolveRequestV4 *request
);
int32_t vckss_rust_engine_solve_v5(
    uint64_t generation,
    const VckssEngineSolveRequestV5 *request
);
int32_t vckss_rust_engine_solve_interrupt_v1(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV1 *request
);
int32_t vckss_rust_engine_solve_interrupt_v2(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV2 *request
);
int32_t vckss_rust_engine_solve_interrupt_v3(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV3 *request
);
int32_t vckss_rust_engine_solve_interrupt_v4(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV4 *request
);
int32_t vckss_rust_engine_solve_interrupt_v5(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV5 *request
);
int32_t vckss_rust_engine_result_v1(
    uint64_t generation,
    VckssEngineResultV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_stayer_hybrid_result_v1(
    uint64_t generation,
    VckssStayerHybridResultV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_projection_result_v1(
    uint64_t generation,
    double *coefficients,
    uint64_t coefficients_capacity,
    double *covariance,
    uint64_t covariance_capacity,
    double *naive_covariance,
    uint64_t naive_covariance_capacity,
    VckssProjectionResultReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_projection_result_receipt_v1(
    uint64_t generation,
    VckssProjectionResultReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_component_inference_result_v2(
    uint64_t generation,
    double *primitive_covariance,
    uint64_t primitive_covariance_capacity,
    double *covariance,
    uint64_t covariance_capacity,
    double *trace_mcse,
    uint64_t trace_mcse_capacity,
    double *spectrum,
    uint64_t spectrum_capacity,
    double *q1,
    uint64_t q1_capacity,
    double *variance_summary,
    uint64_t variance_summary_capacity,
    double *fold_diagnostics,
    uint64_t fold_diagnostics_capacity,
    double *cv_diagnostics,
    uint64_t cv_diagnostics_capacity,
    VckssComponentInferenceResultReceiptV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_component_inference_result_v3(
    uint64_t generation,
    double *primitive_covariance,
    uint64_t primitive_covariance_capacity,
    double *covariance,
    uint64_t covariance_capacity,
    double *trace_mcse,
    uint64_t trace_mcse_capacity,
    double *spectrum,
    uint64_t spectrum_capacity,
    double *q1,
    uint64_t q1_capacity,
    double *variance_summary,
    uint64_t variance_summary_capacity,
    double *fold_diagnostics,
    uint64_t fold_diagnostics_capacity,
    double *cv_diagnostics,
    uint64_t cv_diagnostics_capacity,
    VckssComponentInferenceResultReceiptV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_component_inference_result_v4(
    uint64_t generation,
    double *primitive_covariance,
    uint64_t primitive_covariance_capacity,
    double *covariance,
    uint64_t covariance_capacity,
    double *trace_mcse,
    uint64_t trace_mcse_capacity,
    double *spectrum,
    uint64_t spectrum_capacity,
    double *q1,
    uint64_t q1_capacity,
    double *variance_summary,
    uint64_t variance_summary_capacity,
    double *fold_diagnostics,
    uint64_t fold_diagnostics_capacity,
    double *cv_diagnostics,
    uint64_t cv_diagnostics_capacity,
    VckssComponentInferenceResultReceiptV4 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_component_inference_result_v5(
    uint64_t generation,
    double *primitive, uint64_t primitive_capacity,
    double *covariance, uint64_t covariance_capacity,
    double *mcse, uint64_t mcse_capacity,
    double *spectrum, uint64_t spectrum_capacity,
    double *q1, uint64_t q1_capacity,
    double *summaries, uint64_t summaries_capacity,
    double *folds, uint64_t folds_capacity,
    double *cv, uint64_t cv_capacity,
    double *targets, uint64_t targets_capacity,
    VckssComponentInferenceResultReceiptV5 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v1(
    uint64_t generation,
    VckssEngineDetailedReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v2(
    uint64_t generation,
    VckssEngineDetailedReceiptV2 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v3(
    uint64_t generation,
    VckssEngineDetailedReceiptV3 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v4(
    uint64_t generation,
    VckssEngineDetailedReceiptV4 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v5(
    uint64_t generation,
    VckssEngineDetailedReceiptV5 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v6(
    uint64_t generation,
    VckssEngineDetailedReceiptV6 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_execution_plan_receipt_v1(
    uint64_t generation,
    VckssExecutionPlanReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_detailed_receipt_v7(
    uint64_t generation,
    VckssEngineDetailedReceiptV7 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_performance_receipt_v1(
    uint64_t generation,
    VckssEnginePerformanceReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_full_cmg_receipt_v1(
    uint64_t generation,
    VckssFullCmgReceiptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_rhs_receipts_v1(
    uint64_t generation,
    VckssEngineRhsReceiptV1 *output,
    uint64_t output_capacity_rows
);
int32_t vckss_rust_engine_rhs_receipts_v2(
    uint64_t generation,
    VckssEngineRhsReceiptV2 *output,
    uint64_t output_capacity_rows
);
int32_t vckss_rust_engine_release_v1(uint64_t generation);
int32_t vckss_rust_engine_clear_abandoned_v1(void);
int32_t vckss_rust_engine_snapshot_v1(
    VckssEngineSnapshotV1 *output,
    uint32_t output_capacity_bytes
);
const char *vckss_rust_engine_last_error(void);

#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(VckssEnginePrepareRequestV1) == 24, "unexpected prepare request ABI size");
_Static_assert(sizeof(VckssEnginePrepareRequestV2) == 40, "unexpected V2 prepare request ABI size");
_Static_assert(sizeof(VckssEnginePrepareRequestV3) == 56, "unexpected V3 prepare request ABI size");
_Static_assert(sizeof(VckssEnginePrepareRequestInterruptV1) == 64, "unexpected interrupt prepare request ABI size");
_Static_assert(sizeof(VckssEnginePrepareRequestInterruptV2) == 80, "unexpected V2 interrupt prepare request ABI size");
_Static_assert(sizeof(VckssBackendCapabilitiesV1) == 32, "unexpected capabilities ABI size");
_Static_assert(sizeof(VckssBackendRequestCapabilityRequestV1) == 48, "unexpected request capability ABI size");
_Static_assert(sizeof(VckssBackendRequestCapabilityReceiptV1) == 64, "unexpected request capability receipt ABI size");
_Static_assert(sizeof(VckssBackendRequestCapabilityRequestV2) == 88, "unexpected V2 request capability ABI size");
_Static_assert(sizeof(VckssBackendRequestCapabilityReceiptV2) == 104, "unexpected V2 request capability receipt ABI size");
_Static_assert(sizeof(VckssBackendRequestCapabilityRequestV3) == 120, "unexpected V3 request capability ABI size");
_Static_assert(sizeof(VckssBackendRequestCapabilityReceiptV3) == 160, "unexpected V3 request capability receipt ABI size");
_Static_assert(sizeof(VckssEngineColumnsV1) == 64, "unexpected column descriptor ABI size");
_Static_assert(sizeof(VckssEngineColumnsV2) == 80, "unexpected V2 column descriptor ABI size");
_Static_assert(sizeof(VckssEngineColumnsV3) == 96, "unexpected V3 column descriptor ABI size");
_Static_assert(sizeof(VckssStayerAugmentationRequestV1) == 32, "unexpected stayer augmentation request ABI size");
_Static_assert(sizeof(VckssStayerAugmentationRequestInterruptV1) == 56, "unexpected interrupt stayer augmentation request ABI size");
_Static_assert(sizeof(VckssStayerAugmentationColumnsV1) == 72, "unexpected stayer augmentation columns ABI size");
_Static_assert(sizeof(VckssProjectionAugmentationRequestV1) == 48, "unexpected projection augmentation request ABI size");
_Static_assert(sizeof(VckssProjectionAugmentationRequestInterruptV1) == 72, "unexpected interrupt projection augmentation request ABI size");
_Static_assert(sizeof(VckssProjectionColumnsV1) == 32, "unexpected projection column descriptor ABI size");
_Static_assert(sizeof(VckssComponentInferenceAugmentationRequestV1) == 104, "unexpected component inference augmentation request ABI size");
_Static_assert(sizeof(VckssComponentInferenceAugmentationRequestInterruptV1) == 128, "unexpected interrupt component inference augmentation request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV1) == 176, "unexpected solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV2) == 200, "unexpected V2 solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV3) == 264, "unexpected V3 solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV4) == 288, "unexpected V4 solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV1) == 200, "unexpected interrupt solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV2) == 224, "unexpected V2 interrupt solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV3) == 288, "unexpected V3 interrupt solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV4) == 312, "unexpected V4 interrupt solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV5) == 304, "unexpected V5 solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV6) == 304, "unexpected V6 solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV6) == 328, "unexpected V6 interrupt solve ABI size");
_Static_assert(sizeof(VckssGenericExecutionReceiptV1) == 160, "unexpected generic execution receipt ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV7) == 312, "unexpected V7 solve ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV7) == 336, "unexpected V7 interrupt solve ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV8) == 320, "unexpected V8 solve ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV8) == 344, "unexpected V8 interrupt solve ABI size");
_Static_assert(offsetof(VckssEngineSolveRequestV8, resolved_execution_mode) == 312, "unexpected V8 execution offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV8, interrupt_poll) == 320, "unexpected V8 callback offset");
_Static_assert(sizeof(VckssComponentBatchReceiptV1) == 88, "unexpected component batch receipt ABI size");
_Static_assert(sizeof(VckssExactExecutionRequestV1) == 296, "unexpected exact execution request size");
_Static_assert(sizeof(VckssExactExecutionRequestInterruptV1) == 320, "unexpected interrupt exact execution size");
_Static_assert(sizeof(VckssExactExecutionReceiptV1) == 64, "unexpected exact execution receipt size");
_Static_assert(offsetof(VckssExactExecutionRequestV1, threads) == 288, "unexpected exact execution threads offset");
_Static_assert(offsetof(VckssExactExecutionRequestInterruptV1, interrupt_poll) == 296, "unexpected exact callback offset");
_Static_assert(offsetof(VckssEngineSolveRequestV6, execution_mode) == 288, "unexpected V6 execution offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV6, interrupt_poll) == 304, "unexpected V6 callback offset");
_Static_assert(offsetof(VckssGenericExecutionReceiptV1, fit_rhs_count) == 64, "unexpected generic work offset");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV5) == 328, "unexpected V5 interrupt solve request ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV1) == 72, "unexpected preparation receipt ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV2) == 248, "unexpected V2 preparation receipt ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV3) == 256, "unexpected V3 preparation receipt ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV4) == 264, "unexpected V4 preparation receipt ABI size");
_Static_assert(sizeof(VckssStayerAugmentationReceiptV1) == 192, "unexpected stayer augmentation receipt ABI size");
_Static_assert(sizeof(VckssProjectionAugmentationReceiptV1) == 96, "unexpected projection augmentation receipt ABI size");
_Static_assert(sizeof(VckssProjectionResultReceiptV1) == 152, "unexpected projection result receipt ABI size");
_Static_assert(sizeof(VckssComponentInferenceAugmentationReceiptV1) == 64, "unexpected component inference augmentation receipt ABI size");
_Static_assert(sizeof(VckssComponentInferenceResultReceiptV2) == 168, "unexpected component inference result receipt ABI size");
_Static_assert(sizeof(VckssComponentInferenceResultReceiptV3) == 192, "unexpected component inference V3 result receipt ABI size");
_Static_assert(sizeof(VckssComponentInferenceResultReceiptV4) == 208, "unexpected component inference V4 result receipt ABI size");
_Static_assert(sizeof(VckssComponentInferenceUnitReceiptV1) == 64, "unexpected component inference unit receipt ABI size");
_Static_assert(sizeof(VckssEngineResultV1) == 144, "unexpected result ABI size");
_Static_assert(sizeof(VckssStayerHybridResultV1) == 360, "unexpected stayer hybrid result ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV1) == 272, "unexpected detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV2) == 360, "unexpected V2 detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV3) == 384, "unexpected V3 detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV4) == 448, "unexpected V4 detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV5) == 536, "unexpected V5 detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV6) == 840, "unexpected V6 detailed receipt ABI size");
_Static_assert(sizeof(VckssEstimatorResolutionReceiptV1) == 80, "unexpected estimator resolution receipt ABI size");
_Static_assert(sizeof(VckssSolverExecutionReceiptV1) == 128, "unexpected solver execution receipt ABI size");
_Static_assert(sizeof(VckssBatchPhasePlanReceiptV1) == 96, "unexpected batch-phase receipt ABI size");
_Static_assert(sizeof(VckssBatchExecutionReceiptV1) == 232, "unexpected batch receipt ABI size");
_Static_assert(sizeof(VckssWallExecutionReceiptV1) == 120, "unexpected wall receipt ABI size");
_Static_assert(sizeof(VckssCounterPhaseExecutionReceiptV1) == 64, "unexpected counter-phase receipt ABI size");
_Static_assert(sizeof(VckssCounterExecutionReceiptV1) == 232, "unexpected counter receipt ABI size");
_Static_assert(sizeof(VckssExecutionMemoryReceiptV1) == 168, "unexpected execution memory receipt ABI size");
_Static_assert(sizeof(VckssExecutionPlanReceiptV1) == 1000, "unexpected execution-plan receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV7) == 1840, "unexpected V7 detailed receipt ABI size");
_Static_assert(sizeof(VckssEnginePerformanceReceiptV1) == 96, "unexpected performance receipt ABI size");
_Static_assert(sizeof(VckssFullCmgReceiptV1) == 400, "unexpected full-CMG receipt ABI size");
_Static_assert(sizeof(VckssFullCmgModelReceiptV1) == 56, "unexpected full-CMG model receipt ABI size");
_Static_assert(offsetof(VckssFullCmgModelReceiptV1, logical_rhs_count) == 24, "unexpected full-CMG model receipt ABI offset");
_Static_assert(sizeof(VckssEngineRhsReceiptV1) == 48, "unexpected RHS receipt ABI size");
_Static_assert(sizeof(VckssEngineRhsReceiptV2) == 96, "unexpected V2 RHS receipt ABI size");
_Static_assert(sizeof(VckssEngineSnapshotV1) == 24, "unexpected snapshot ABI size");
_Static_assert(sizeof(VckssPrepareRequestV1) == 24, "unexpected legacy prepare request ABI size");
_Static_assert(sizeof(VckssColumnsV1) == 64, "unexpected legacy column descriptor ABI size");
_Static_assert(sizeof(VckssPreparationReceiptV1) == 72, "unexpected legacy preparation receipt ABI size");
_Static_assert(sizeof(VckssSessionSnapshotV1) == 24, "unexpected legacy snapshot ABI size");
_Static_assert(sizeof(VckssPrepareRequestV1) == sizeof(VckssEnginePrepareRequestV1), "legacy prepare layout changed");
_Static_assert(sizeof(VckssColumnsV1) == sizeof(VckssEngineColumnsV1), "legacy columns layout changed");
_Static_assert(sizeof(VckssPreparationReceiptV1) == sizeof(VckssEnginePreparationReceiptV1), "legacy receipt layout changed");
_Static_assert(sizeof(VckssSessionSnapshotV1) == sizeof(VckssEngineSnapshotV1), "legacy snapshot layout changed");
_Static_assert(offsetof(VckssEnginePrepareRequestV2, memory_limit_bytes) == 24, "unexpected V2 prepare extension offset");
_Static_assert(offsetof(VckssEnginePrepareRequestV3, deletion_mode) == 40, "unexpected V3 prepare extension offset");
_Static_assert(sizeof(VckssEnginePrepareRequestV4) == 64, "unexpected V4 prepare request ABI size");
_Static_assert(offsetof(VckssEnginePrepareRequestV4, implicit_match) == 56, "unexpected V4 prepare extension offset");
_Static_assert(offsetof(VckssEngineColumnsV2, controls) == 64, "unexpected V2 columns extension offset");
_Static_assert(offsetof(VckssEngineColumnsV3, probe_order) == 80, "unexpected V3 columns extension offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, options) == 0, "unexpected interrupt prepare prefix offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, interrupt_poll) == 40, "unexpected interrupt prepare callback offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, interrupt_context) == 48, "unexpected interrupt prepare context offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, checkpoint_interval) == 56, "unexpected interrupt prepare interval offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, reserved) == 60, "unexpected interrupt prepare reserved offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV2, options) == 0, "unexpected V2 interrupt prepare prefix offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV2, interrupt_poll) == 56, "unexpected V2 interrupt prepare callback offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV2, interrupt_context) == 64, "unexpected V2 interrupt prepare context offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV2, checkpoint_interval) == 72, "unexpected V2 interrupt prepare interval offset");
_Static_assert(sizeof(VckssEnginePrepareRequestInterruptV3) == 88, "unexpected V3 interrupt prepare request ABI size");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV3, interrupt_poll) == 64, "unexpected V3 interrupt prepare callback offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV3, interrupt_context) == 72, "unexpected V3 interrupt prepare context offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV3, checkpoint_interval) == 80, "unexpected V3 interrupt prepare interval offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV3, reserved) == 84, "unexpected V3 interrupt prepare reserved offset");
_Static_assert(offsetof(VckssEnginePreparationReceiptV2, memory_limit_bytes) == 72, "unexpected V2 preparation receipt extension offset");
_Static_assert(offsetof(VckssEnginePreparationReceiptV3, target_weight_sum) == 248, "unexpected V3 preparation receipt extension offset");
_Static_assert(offsetof(VckssEnginePreparationReceiptV4, controls_count) == 256, "unexpected V4 preparation receipt extension offset");
_Static_assert(offsetof(VckssEngineSolveRequestV1, cmg_memory_limit_bytes) == 168, "unexpected solve request tail offset");
_Static_assert(offsetof(VckssEngineSolveRequestV2, algorithm) == 176, "unexpected V2 solve extension offset");
_Static_assert(offsetof(VckssEngineSolveRequestV3, engine) == 200, "unexpected V3 solve extension offset");
_Static_assert(offsetof(VckssEngineSolveRequestV4, leverage_batch_mode) == 264, "unexpected V4 solve extension offset");
_Static_assert(offsetof(VckssEngineSolveRequestV5, threads) == 288, "unexpected V5 solve extension offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, options) == 0, "unexpected interrupt solve prefix offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, interrupt_poll) == 176, "unexpected interrupt callback offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, interrupt_context) == 184, "unexpected interrupt context offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, checkpoint_interval) == 192, "unexpected interrupt interval offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV2, options) == 0, "unexpected V2 interrupt solve prefix offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV2, interrupt_poll) == 200, "unexpected V2 interrupt solve callback offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV2, interrupt_context) == 208, "unexpected V2 interrupt solve context offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV2, checkpoint_interval) == 216, "unexpected V2 interrupt solve interval offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV3, interrupt_poll) == 264, "unexpected V3 interrupt solve callback offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV4, interrupt_poll) == 288, "unexpected V4 interrupt solve callback offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV5, interrupt_poll) == 304, "unexpected V5 interrupt solve callback offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV1, cmg_dense_factor_bytes) == 264, "unexpected detailed receipt tail offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV2, full_fit_weighted_rss) == 272, "unexpected V2 detailed receipt tail offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV3, rng_contract) == 360, "unexpected V3 detailed receipt extension offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV4, algorithm_requested) == 384, "unexpected V4 detailed receipt extension offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV5, applicability_flags) == 448, "unexpected V5 detailed receipt extension offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV5, actual_accounting_residual) == 528, "unexpected V5 detailed receipt tail offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, engine_requested) == 536, "unexpected V6 detailed receipt extension offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, rhs_v2_caller_copy_bytes) == 776, "unexpected V6 detailed receipt RHS copy offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, capability_schema) == 784, "unexpected V6 capability echo offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, request_signature) == 832, "unexpected V6 detailed receipt tail offset");
_Static_assert(offsetof(VckssExecutionPlanReceiptV1, resolution) == 40, "unexpected execution-plan resolution offset");
_Static_assert(offsetof(VckssExecutionPlanReceiptV1, solver) == 120, "unexpected execution-plan solver offset");
_Static_assert(offsetof(VckssExecutionPlanReceiptV1, batch) == 248, "unexpected execution-plan batch offset");
_Static_assert(offsetof(VckssExecutionPlanReceiptV1, wall) == 480, "unexpected execution-plan wall offset");
_Static_assert(offsetof(VckssExecutionPlanReceiptV1, counter) == 600, "unexpected execution-plan counter offset");
_Static_assert(offsetof(VckssExecutionPlanReceiptV1, memory) == 832, "unexpected execution-plan memory offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV7, execution) == 840, "unexpected V7 detailed receipt extension offset");
_Static_assert(offsetof(VckssEnginePerformanceReceiptV1, ingest_ns) == 32, "unexpected performance receipt timing offset");
_Static_assert(offsetof(VckssEngineRhsReceiptV1, reduced_residual) == 32, "unexpected RHS receipt residual offset");
_Static_assert(offsetof(VckssEngineRhsReceiptV2, status) == 48, "unexpected V2 RHS receipt extension offset");
#endif

/* Additive policy: absent budget is represented by budget_present = 0 and
 * budget_bytes = 0. Legacy entrypoints retain strict numeric admission. */
typedef struct {
    uint32_t struct_size;
    uint32_t schema_version;
    uint32_t budget_present;
    uint32_t check_mode; /* 1 error, 2 warn, 3 off */
    uint64_t budget_bytes;
} VckssMemoryPolicyV1;
int32_t vckss_rust_engine_admit_prepare_memory_v1(
    const VckssEnginePrepareRequestV4 *, const VckssMemoryPolicyV1 *, uint32_t);
int32_t vckss_rust_engine_prepare_memory_interrupt_v1(
    const VckssEnginePrepareRequestInterruptV3 *, const VckssEngineColumnsV3 *,
    const VckssMemoryPolicyV1 *, uint64_t *, uint32_t);
int32_t vckss_rust_engine_memory_policy_v1(uint64_t, VckssMemoryPolicyV1 *, uint32_t);
#if !defined(__cplusplus)
_Static_assert(sizeof(VckssMemoryPolicyV1) == 24, "unexpected memory policy size");
_Static_assert(offsetof(VckssMemoryPolicyV1, budget_bytes) == 16, "unexpected budget offset");
#endif

typedef struct {
    uint32_t struct_size;
    uint32_t schema_version;
    uint64_t expected_peak_bytes;
    uint64_t admission_peak_bytes;
    uint64_t conditional_reserve_bytes;
} VckssMemoryForecastV1;
#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(VckssMemoryForecastV1) == 32, "unexpected memory forecast size");
#endif
int32_t vckss_rust_engine_memory_forecast_v1(uint64_t, VckssMemoryForecastV1 *, uint32_t);

#ifdef __cplusplus
}
#endif

#endif /* VCKSS_RUST_H */
