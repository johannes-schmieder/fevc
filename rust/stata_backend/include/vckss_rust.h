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
#define VCKSS_RNG_COUNTER_V1 1u
#define VCKSS_ROUTE_AUTO 0u
#define VCKSS_ROUTE_EXACT 1u
#define VCKSS_ROUTE_DIAGONAL_PCG 2u
#define VCKSS_ROUTE_CMG_PCG 3u
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
#define VCKSS_SUPPORT_EXACT (UINT64_C(1) << 0)
#define VCKSS_SUPPORT_JLA (UINT64_C(1) << 1)
#define VCKSS_SUPPORT_MATCH_DELETION (UINT64_C(1) << 2)
#define VCKSS_SUPPORT_OBSERVATION_DELETION (UINT64_C(1) << 3)
#define VCKSS_SUPPORT_CONTROLS (UINT64_C(1) << 4)
#define VCKSS_SUPPORT_DIAGONAL (UINT64_C(1) << 5)
#define VCKSS_SUPPORT_CMG (UINT64_C(1) << 6)

typedef struct VckssBackendCapabilitiesV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    uint64_t core_ready_flags;
    uint64_t support_flags;
    uint32_t deterministic_parallelism;
    uint32_t reserved;
} VckssBackendCapabilitiesV1;

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

typedef int32_t (*VckssInterruptPollV1)(void *context);

typedef struct VckssEnginePrepareRequestInterruptV1 {
    VckssEnginePrepareRequestV2 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEnginePrepareRequestInterruptV1;

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

typedef struct VckssEngineSolveRequestInterruptV1 {
    VckssEngineSolveRequestV1 options;
    VckssInterruptPollV1 interrupt_poll;
    void *interrupt_context;
    uint32_t checkpoint_interval;
    uint32_t reserved;
} VckssEngineSolveRequestInterruptV1;

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
int32_t vckss_rust_engine_default_prepare_request_interrupt_v1(
    VckssEnginePrepareRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_default_solve_request_interrupt_v1(
    VckssEngineSolveRequestInterruptV1 *output,
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
int32_t vckss_rust_engine_prepare_v2(
    const VckssEnginePrepareRequestV2 *request,
    const VckssEngineColumnsV1 *columns,
    uint64_t *output_handle,
    uint32_t output_capacity_bytes
);
int32_t vckss_rust_engine_prepare_interrupt_v1(
    const VckssEnginePrepareRequestInterruptV1 *request,
    const VckssEngineColumnsV1 *columns,
    uint64_t *output_handle,
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
int32_t vckss_rust_engine_retained_mask_v1(
    uint64_t generation,
    uint8_t *output,
    uint64_t output_capacity_bytes
);
int32_t vckss_rust_engine_solve_v1(
    uint64_t generation,
    const VckssEngineSolveRequestV1 *request
);
int32_t vckss_rust_engine_solve_interrupt_v1(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV1 *request
);
int32_t vckss_rust_engine_result_v1(
    uint64_t generation,
    VckssEngineResultV1 *output,
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
int32_t vckss_rust_engine_rhs_receipts_v1(
    uint64_t generation,
    VckssEngineRhsReceiptV1 *output,
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
_Static_assert(sizeof(VckssEnginePrepareRequestInterruptV1) == 64, "unexpected interrupt prepare request ABI size");
_Static_assert(sizeof(VckssBackendCapabilitiesV1) == 32, "unexpected capabilities ABI size");
_Static_assert(sizeof(VckssEngineColumnsV1) == 64, "unexpected column descriptor ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestV1) == 176, "unexpected solve request ABI size");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV1) == 200, "unexpected interrupt solve request ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV1) == 72, "unexpected preparation receipt ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV2) == 248, "unexpected V2 preparation receipt ABI size");
_Static_assert(sizeof(VckssEnginePreparationReceiptV3) == 256, "unexpected V3 preparation receipt ABI size");
_Static_assert(sizeof(VckssEngineResultV1) == 144, "unexpected result ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV1) == 272, "unexpected detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV2) == 360, "unexpected V2 detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineDetailedReceiptV3) == 384, "unexpected V3 detailed receipt ABI size");
_Static_assert(sizeof(VckssEngineRhsReceiptV1) == 48, "unexpected RHS receipt ABI size");
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
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, options) == 0, "unexpected interrupt prepare prefix offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, interrupt_poll) == 40, "unexpected interrupt prepare callback offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, interrupt_context) == 48, "unexpected interrupt prepare context offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, checkpoint_interval) == 56, "unexpected interrupt prepare interval offset");
_Static_assert(offsetof(VckssEnginePrepareRequestInterruptV1, reserved) == 60, "unexpected interrupt prepare reserved offset");
_Static_assert(offsetof(VckssEnginePreparationReceiptV2, memory_limit_bytes) == 72, "unexpected V2 preparation receipt extension offset");
_Static_assert(offsetof(VckssEnginePreparationReceiptV3, target_weight_sum) == 248, "unexpected V3 preparation receipt extension offset");
_Static_assert(offsetof(VckssEngineSolveRequestV1, cmg_memory_limit_bytes) == 168, "unexpected solve request tail offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, options) == 0, "unexpected interrupt solve prefix offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, interrupt_poll) == 176, "unexpected interrupt callback offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, interrupt_context) == 184, "unexpected interrupt context offset");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV1, checkpoint_interval) == 192, "unexpected interrupt interval offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV1, cmg_dense_factor_bytes) == 264, "unexpected detailed receipt tail offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV2, full_fit_weighted_rss) == 272, "unexpected V2 detailed receipt tail offset");
_Static_assert(offsetof(VckssEngineDetailedReceiptV3, rng_contract) == 360, "unexpected V3 detailed receipt extension offset");
_Static_assert(offsetof(VckssEngineRhsReceiptV1, reduced_residual) == 32, "unexpected RHS receipt residual offset");
#endif

#ifdef __cplusplus
}
#endif

#endif /* VCKSS_RUST_H */
