/* SPDX-License-Identifier: GPL-3.0-only */

#include <assert.h>
#include <stdint.h>
#include <string.h>

static int mock_cleanup_release(uint64_t generation);
static int mock_cleanup_clear(void);

#define VCKSS_CSHIM_TEST 1
#define VCKSS_CLEANUP_RELEASE(generation) mock_cleanup_release((generation))
#define VCKSS_CLEANUP_CLEAR() mock_cleanup_clear()
#include "../cshim/stata_entry.c"

ST_plugin *_stata_;

int32_t vckss_rust_report_call_v1(const VckssProgressOptionsV1 *options,
    VckssProgressOperationV1 operation, void *context)
{
    assert(options->struct_size == sizeof(*options) && options->schema == 1u);
    return operation(context);
}

uint32_t vckss_rust_abi_version(void) { return VCKSS_RUST_ABI_VERSION_V1; }
const char *vckss_rust_backend_version(void) { return VCKSS_RUST_RUNTIME_BUILD_ID; }
int32_t vckss_rust_backend_capabilities_v1(VckssBackendCapabilitiesV1 *output, uint32_t capacity)
{
    assert(capacity == sizeof(*output));
    *output = (VckssBackendCapabilitiesV1){
        .struct_size = sizeof(*output), .abi_version = VCKSS_RUST_ABI_VERSION_V1,
        .core_ready_flags = 32767, .support_flags = 38, .deterministic_parallelism = 1};
    return 0;
}

int32_t vckss_rust_engine_memory_policy_v1(uint64_t generation, VckssMemoryPolicyV1 *output, uint32_t capacity)
{
    (void)generation;
    assert(capacity == sizeof(*output));
    *output = (VckssMemoryPolicyV1){sizeof(*output), 1u, 1u, 1u, 8192u};
    return 0;
}
int32_t vckss_rust_engine_memory_forecast_v1(uint64_t generation, VckssMemoryForecastV1 *output, uint32_t capacity)
{
    (void)generation;
    assert(capacity == sizeof(*output));
    *output = (VckssMemoryForecastV1){sizeof(*output), 1u, 1024u, 1024u, 0u};
    return 0;
}


static int fail_scalar;
static int fail_plan_scalar;
static int fail_execution_api_scalar;
static double saved_execution_api;
static double saved_progress_api;
static int fail_matrix;
static int error_calls;
static int release_calls;
static int clear_calls;
static int release_status;
static int clear_status;
static uint64_t released_generation;
static int selftest_status;
static const char *selftest_error;
static int scalar_calls;
static int request_capability_calls;
static int corrupt_request_echo;
static int result_status;
static int stayer_result_status;
static int stayer_augmentation_status;
static int corrupt_stayer_receipt;
static int detailed_receipt_status;
static int detailed_receipt_v7_status;
static int corrupt_v7_receipt;
static uint32_t detailed_capability_schema;
static int rhs_v1_status;
static int rhs_v2_status;
static uint64_t rhs_row_count;
static uint32_t rhs_receipt_schema;
static int rhs_copy_override;
static uint64_t rhs_v1_copy_bytes;
static uint64_t rhs_v2_copy_bytes;
static ST_int matrix_rows;
static ST_int matrix_columns;
static uint64_t active_generation;
static uint64_t last_released_generation;
static double saved_error_code;
static char saved_error_status[VCKSS_ERROR_STATUS_CAPACITY];
static char saved_error_detail[VCKSS_ERROR_DETAIL_CAPACITY];
static int solve_v4_calls;
static int solve_v5_calls;
static int solve_v6_calls;
static int solve_v8_calls;
static VckssEngineSolveRequestInterruptV6 captured_solve_v6;
static VckssEngineSolveRequestInterruptV8 captured_solve_v8;
static VckssGenericExecutionReceiptV1 execution_receipt;
static VckssExactExecutionReceiptV1 exact_execution_receipt;
static VckssExactExecutionRequestInterruptV1 captured_exact_execution;
static int exact_execution_calls;
static uint64_t solved_generation;
static VckssEngineSolveRequestInterruptV4 captured_solve_v4;
static VckssEngineSolveRequestInterruptV5 captured_solve_v5;
static int full_cmg_receipt_status;
static int corrupt_full_cmg_receipt;
static int corrupt_full_cmg_model_receipt;
static char saved_cmg_backend[32];
static char saved_cmg_source[64];

int32_t vckss_rust_backend_request_capability_v1(
    const VckssBackendRequestCapabilityRequestV1 *request,
    VckssBackendRequestCapabilityReceiptV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(request != NULL);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    ++request_capability_calls;
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->abi_version = VCKSS_RUST_ABI_VERSION_V1;
    output->request_schema = request->request_schema;
    output->supported = 1;
    output->reason_code = VCKSS_REQUEST_REASON_SUPPORTED;
    output->profile_code = VCKSS_REQUEST_PROFILE_EXACT_V1;
    output->algorithm = request->algorithm;
    output->deletion_mode = request->deletion_mode;
    output->nuisance_mode = request->nuisance_mode;
    output->solver_route = request->solver_route;
    output->rng_contract = request->rng_contract;
    output->controls_count = request->controls_count + (corrupt_request_echo ? 1u : 0u);
    output->frequency_use = request->frequency_use;
    output->request_signature = UINT64_C(0x123456789abcdef0);
    return 0;
}

int32_t vckss_rust_backend_request_capability_v2(
    const VckssBackendRequestCapabilityRequestV2 *request,
    VckssBackendRequestCapabilityReceiptV2 *output,
    uint32_t output_capacity_bytes
)
{
    assert(request != NULL);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    ++request_capability_calls;
    memset(output, 0, sizeof(*output));
    output->v1.struct_size = (uint32_t)sizeof(*output);
    output->v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    output->v1.request_schema = request->v1.request_schema;
    output->v1.supported = 1;
    output->v1.reason_code = VCKSS_REQUEST_REASON_SUPPORTED;
    output->v1.profile_code = VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1;
    output->v1.algorithm = request->v1.algorithm;
    output->v1.deletion_mode = request->v1.deletion_mode;
    output->v1.nuisance_mode = request->v1.nuisance_mode;
    output->v1.solver_route = request->v1.solver_route;
    output->v1.rng_contract = request->v1.rng_contract;
    output->v1.controls_count = request->v1.controls_count;
    output->v1.frequency_use = request->v1.frequency_use;
    output->v1.request_signature = UINT64_C(0xfedcba9876543210);
    output->engine = request->engine + (corrupt_request_echo ? 1u : 0u);
    output->batch_mode = request->batch_mode;
    output->stayers_mode = request->stayers_mode;
    output->target_weight_mode = request->target_weight_mode;
    output->deletion_unit_source = request->deletion_unit_source;
    output->probeorder_supplied = request->probeorder_supplied;
    output->wallseconds_supplied = request->wallseconds_supplied;
    output->physical_limit = request->physical_limit;
    return 0;
}

int32_t vckss_rust_backend_request_capability_v3(
    const VckssBackendRequestCapabilityRequestV3 *request,
    VckssBackendRequestCapabilityReceiptV3 *output,
    uint32_t output_capacity_bytes
)
{
    assert(request != NULL);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    ++request_capability_calls;
    memset(output, 0, sizeof(*output));
    output->v2.v1.struct_size = (uint32_t)sizeof(*output);
    output->v2.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    output->v2.v1.request_schema = request->v2.v1.request_schema;
    output->v2.v1.supported = 1;
    output->v2.v1.reason_code = VCKSS_REQUEST_REASON_SUPPORTED;
    output->v2.v1.profile_code = VCKSS_REQUEST_PROFILE_PLANNED_V1;
    output->v2.v1.algorithm = request->v2.v1.algorithm;
    output->v2.v1.deletion_mode = request->v2.v1.deletion_mode;
    output->v2.v1.nuisance_mode = request->v2.v1.nuisance_mode;
    output->v2.v1.solver_route = request->v2.v1.solver_route;
    output->v2.v1.rng_contract = request->v2.v1.rng_contract;
    output->v2.v1.controls_count = request->v2.v1.controls_count;
    output->v2.v1.frequency_use = request->v2.v1.frequency_use;
    output->v2.v1.request_signature = UINT64_C(0x1020304050607080);
    output->v2.engine = request->v2.engine;
    output->v2.batch_mode = request->v2.batch_mode;
    output->v2.stayers_mode = request->v2.stayers_mode;
    output->v2.target_weight_mode = request->v2.target_weight_mode;
    output->v2.deletion_unit_source = request->v2.deletion_unit_source;
    output->v2.probeorder_supplied = request->v2.probeorder_supplied;
    output->v2.wallseconds_supplied = request->v2.wallseconds_supplied;
    output->v2.physical_limit = request->v2.physical_limit;
    output->leverage_batch_mode = request->leverage_batch_mode;
    output->target_batch_mode = request->target_batch_mode;
    output->allow_automatic_cmg_setup_fallback =
        request->allow_automatic_cmg_setup_fallback + (corrupt_request_echo ? 1u : 0u);
    output->wallseconds = request->wallseconds;
    output->algorithm_resolution_deferred = request->v2.v1.algorithm == VCKSS_ALGORITHM_AUTO;
    output->engine_resolution_deferred = request->v2.engine == VCKSS_ENGINE_AUTO_OR_UNSPECIFIED;
    output->route_resolution_deferred = request->v2.v1.solver_route == VCKSS_ROUTE_AUTO;
    output->leverage_batch_resolution_deferred =
        request->leverage_batch_mode == VCKSS_BATCH_MODE_AUTO;
    output->target_batch_resolution_deferred =
        request->target_batch_mode == VCKSS_BATCH_MODE_AUTO;
    output->wall_advisory_only = 1;
    return 0;
}

int32_t vckss_rust_selftest(void)
{
    return selftest_status;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v1(
    VckssEngineSolveRequestInterruptV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(output != NULL && output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.struct_size = (uint32_t)sizeof(output->options);
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v2(
    VckssEngineSolveRequestInterruptV2 *output,
    uint32_t output_capacity_bytes
)
{
    assert(output != NULL && output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v1.struct_size = (uint32_t)sizeof(output->options);
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v3(
    VckssEngineSolveRequestInterruptV3 *output,
    uint32_t output_capacity_bytes
)
{
    assert(output != NULL && output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v2.v1.struct_size = (uint32_t)sizeof(output->options);
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v4(
    VckssEngineSolveRequestInterruptV4 *output,
    uint32_t output_capacity_bytes
)
{
    assert(output != NULL && output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v3.v2.v1.struct_size = (uint32_t)sizeof(output->options);
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v5(
    VckssEngineSolveRequestInterruptV5 *output,
    uint32_t output_capacity_bytes
)
{
    assert(output != NULL && output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v4.v3.v2.v1.struct_size = (uint32_t)sizeof(output->options);
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v1(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV1 *request
)
{
    assert(generation != 0 && request != NULL);
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v2(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV2 *request
)
{
    assert(generation != 0 && request != NULL);
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v3(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV3 *request
)
{
    assert(generation != 0 && request != NULL);
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v4(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV4 *request
)
{
    assert(generation != 0 && request != NULL);
    ++solve_v4_calls;
    solved_generation = generation;
    captured_solve_v4 = *request;
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v5(
    uint64_t generation,
    const VckssEngineSolveRequestInterruptV5 *request
)
{
    assert(generation != 0 && request != NULL);
    ++solve_v5_calls;
    solved_generation = generation;
    captured_solve_v5 = *request;
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v6(
    VckssEngineSolveRequestInterruptV6 *output, uint32_t capacity)
{
    assert(output != NULL && capacity == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v4.v3.v2.v1.struct_size = sizeof(*output);
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v6(
    uint64_t generation, const VckssEngineSolveRequestInterruptV6 *request)
{
    assert(generation != 0 && request != NULL);
    ++solve_v6_calls;
    solved_generation = generation;
    captured_solve_v6 = *request;
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v7(
    VckssEngineSolveRequestInterruptV7 *output, uint32_t capacity)
{
    assert(output != NULL && capacity == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v6.v4.v3.v2.v1.struct_size = sizeof(*output);
    output->options.component_batch_mode = 1u;
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v7(
    uint64_t generation, const VckssEngineSolveRequestInterruptV7 *request)
{
    assert(generation != 0 && request != NULL);
    ++solve_v6_calls;
    solved_generation = generation;
    captured_solve_v6 = (VckssEngineSolveRequestInterruptV6){0};
    captured_solve_v6.options = request->options.v6;
    return 0;
}

int32_t vckss_rust_engine_default_solve_request_interrupt_v8(
    VckssEngineSolveRequestInterruptV8 *output, uint32_t capacity)
{
    assert(output != NULL && capacity == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->options.v7.v6.v4.v3.v2.v1.struct_size = sizeof(*output);
    output->options.v7.v6.execution_mode = VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO;
    output->options.resolved_execution_mode = VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO;
    return 0;
}

int32_t vckss_rust_engine_solve_interrupt_v8(
    uint64_t generation, const VckssEngineSolveRequestInterruptV8 *request)
{
    assert(generation != 0 && request != NULL);
    ++solve_v8_calls;
    solved_generation = generation;
    captured_solve_v8 = *request;
    return 0;
}

uint32_t vckss_rust_exact_execution_schema_v1(void) { return 1u; }
uint32_t vckss_rust_exact_legacy_execution_schema_v1(void) { return 1u; }
int32_t vckss_rust_engine_solve_exact_legacy_execution_interrupt_v1(
    uint64_t generation, const VckssEngineSolveRequestInterruptV2 *request, uint32_t threads)
{
    assert(threads > 0 && request->options.algorithm == VCKSS_ALGORITHM_EXACT);
    return vckss_rust_engine_solve_interrupt_v2(generation, request);
}
uint32_t vckss_rust_exact_resolved_execution_schema_v2(void) { return 2u; }
int32_t vckss_rust_engine_solve_exact_execution_interrupt_v1(
    uint64_t generation, const VckssExactExecutionRequestInterruptV1 *request)
{
    assert(generation != 0 && request != NULL);
    ++exact_execution_calls;
    solved_generation = generation;
    captured_exact_execution = *request;
    return 0;
}
int32_t vckss_rust_engine_solve_exact_resolved_execution_interrupt_v2(
    uint64_t generation, const VckssExactExecutionRequestInterruptV1 *request)
{
    assert(request->options.v4.v3.v2.algorithm == VCKSS_ALGORITHM_AUTO);
    assert(request->options.v4.v3.v2.v1.rng_contract == VCKSS_RNG_COUNTER_V1);
    return vckss_rust_engine_solve_exact_execution_interrupt_v1(generation, request);
}
int32_t vckss_rust_engine_exact_execution_receipt_v1(
    uint64_t generation, VckssExactExecutionReceiptV1 *output, uint32_t capacity)
{
    assert(generation == active_generation && capacity == sizeof(*output));
    *output = exact_execution_receipt;
    return 0;
}

int32_t vckss_rust_engine_component_batch_receipt_v1(
    uint64_t generation, VckssComponentBatchReceiptV1 *output, uint32_t capacity)
{
    assert(generation == active_generation && output != NULL && capacity == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->struct_size = sizeof(*output);
    output->schema_version = 1u;
    output->generation = generation;
    return 0;
}

int32_t vckss_rust_engine_generic_execution_receipt_v1(
    uint64_t generation, VckssGenericExecutionReceiptV1 *output, uint32_t capacity)
{
    assert(generation == active_generation && capacity == sizeof(*output));
    *output = execution_receipt;
    return 0;
}

const char *vckss_rust_last_error(void)
{
    return selftest_error;
}

const char *vckss_rust_engine_last_error(void)
{
    return selftest_error;
}

int32_t vckss_rust_engine_result_v1(
    uint64_t generation,
    VckssEngineResultV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    if (result_status == 0) memset(output, 0, sizeof(*output));
    return result_status;
}

int32_t vckss_rust_engine_stayer_hybrid_result_v1(
    uint64_t generation,
    VckssStayerHybridResultV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    if (stayer_result_status != 0) return stayer_result_status;
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->schema_version = 1u;
    output->generation = generation;
    output->deletion_units = 12;
    output->topology_checksum = UINT64_C(0x123456789abcdef0);
    output->peak_forecast_bytes = 4096;
    output->fit_peak_forecast_bytes = 3072;
    output->correction_peak_forecast_bytes = 4096;
    if (corrupt_stayer_receipt) ++output->deletion_units;
    return 0;
}

int32_t vckss_rust_engine_stayer_augmentation_receipt_v1(
    uint64_t generation,
    VckssStayerAugmentationReceiptV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    if (stayer_augmentation_status != 0) return stayer_augmentation_status;
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->schema_version = 1u;
    output->generation = generation;
    output->mover_stored_rows = 8;
    output->stayer_stored_rows = 2;
    output->combined_stored_rows = 10;
    output->mover_physical_mass = 8;
    output->stayer_physical_mass = 4;
    output->combined_physical_mass = 12;
    output->mover_workers = 4;
    output->stayer_workers = 1;
    output->combined_workers = 5;
    output->firms = 2;
    output->mover_deletion_units = 8;
    output->stayer_deletion_units = 4;
    output->combined_deletion_units = 12;
    output->topology_checksum = UINT64_C(0x123456789abcdef0);
    output->memory_limit_bytes = 8192;
    output->augmentation_peak_forecast_bytes = 4096;
    output->augmented_resident_bytes = 1024;
    output->total_prepared_resident_bytes = 2048;
    return 0;
}

int32_t vckss_rust_engine_detailed_receipt_v6(
    uint64_t generation,
    VckssEngineDetailedReceiptV6 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    if (detailed_receipt_status != 0) return detailed_receipt_status;
    memset(output, 0, sizeof(*output));
    output->v5.v4.v3.rhs_receipt_rows = rhs_row_count;
    output->rhs_receipt_schema = rhs_receipt_schema;
    output->capability_schema = detailed_capability_schema;
    if (detailed_capability_schema == VCKSS_REQUEST_CAPABILITY_SCHEMA_V3) {
        output->capability_profile = VCKSS_REQUEST_PROFILE_PLANNED_V1;
        output->request_signature = UINT64_C(0x1020304050607080);
        output->engine_requested = VCKSS_ENGINE_GENERIC;
        output->engine_selected = VCKSS_ENGINE_GENERIC;
    }
    if (rhs_receipt_schema == 1) {
        output->v5.v4.v3.caller_result_copy_bytes = rhs_row_count *
            ((uint64_t)sizeof(VckssEngineRhsReceiptV1) + UINT64_C(8) * sizeof(double));
    } else if (rhs_receipt_schema == 2) {
        output->rhs_v2_caller_copy_bytes = rhs_row_count *
            ((uint64_t)sizeof(VckssEngineRhsReceiptV2) + UINT64_C(15) * sizeof(double));
    }
    if (rhs_copy_override) {
        output->v5.v4.v3.caller_result_copy_bytes = rhs_v1_copy_bytes;
        output->rhs_v2_caller_copy_bytes = rhs_v2_copy_bytes;
    }
    return 0;
}

int32_t vckss_rust_engine_detailed_receipt_v7(
    uint64_t generation,
    VckssEngineDetailedReceiptV7 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    if (detailed_receipt_v7_status != 0) return detailed_receipt_v7_status;
    memset(output, 0, sizeof(*output));
    assert(vckss_rust_engine_detailed_receipt_v6(
        generation, &output->v6, (uint32_t)sizeof(output->v6)
    ) == 0);
    output->execution.struct_size = (uint32_t)sizeof(output->execution);
    output->execution.schema_version = 1;
    output->execution.generation = generation;
    output->execution.request_signature = output->v6.request_signature;
    output->execution.resolution.algorithm_schema = 1;
    output->execution.resolution.algorithm_requested = VCKSS_ALGORITHM_JLA;
    output->execution.resolution.algorithm_selected = VCKSS_ALGORITHM_JLA;
    output->execution.resolution.engine_schema = 1;
    output->execution.resolution.engine_requested = output->v6.engine_requested;
    output->execution.resolution.engine_selected = output->v6.engine_selected;
    output->execution.resolution.resolved_before_rng = 1;
    output->execution.solver.schema_version = 1;
    output->execution.solver.requested_route = VCKSS_ROUTE_DIAGONAL_PCG;
    output->execution.solver.selected_route = VCKSS_ROUTE_DIAGONAL_PCG;
    output->execution.solver.plan_frozen_before_rng = 1;
    output->execution.solver.threads_requested = 1;
    output->execution.solver.threads_used = 1;
    output->execution.batch.schema_version = 1;
    output->execution.batch.deterministic = 1;
    output->execution.batch.width_invariance_required = 1;
    output->execution.batch.whole_command_admitted = 1;
    output->execution.wall.schema_version = 1;
    output->execution.counter.schema_version = 1;
    output->execution.counter.rng_contract = VCKSS_RNG_COUNTER_V1;
    output->execution.counter.completed = 1;
    output->execution.memory.schema_version = 1;
    if (corrupt_v7_receipt) ++output->execution.struct_size;
    return 0;
}

int32_t vckss_rust_engine_performance_receipt_v1(
    uint64_t generation,
    VckssEnginePerformanceReceiptV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->schema_version = 1;
    output->generation = generation;
    output->applicability_flags = UINT64_C(3);
    output->algorithm_selected = 0;
    output->engine_selected = detailed_capability_schema == VCKSS_REQUEST_CAPABILITY_SCHEMA_V3
        ? VCKSS_ENGINE_GENERIC : 0;
    output->ingest_ns = 10;
    output->canonicalize_ns = 20;
    output->graph_ns = 30;
    output->compress_ns = 40;
    output->plan_ns = 50;
    output->solve_ns = 60;
    output->native_total_ns = 210;
    return 0;
}

int32_t vckss_rust_engine_full_cmg_model_receipt_v1(
    uint64_t generation, VckssFullCmgModelReceiptV1 *output, uint32_t output_capacity_bytes)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->schema_version = 1;
    output->generation = generation;
    output->nuisance_mode = VCKSS_NUISANCE_JOINT;
    output->logical_rhs_count = 16;
    if (corrupt_full_cmg_model_receipt == 1) output->schema_version = 2;
    if (corrupt_full_cmg_model_receipt == 2) output->explicit_options_rhs_count = 1;
    if (corrupt_full_cmg_model_receipt == 3) output->control_refinement_rhs_count = 1;
    return 0;
}

int32_t vckss_rust_engine_full_cmg_receipt_v1(
    uint64_t generation,
    VckssFullCmgReceiptV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_capacity_bytes == sizeof(*output));
    if (full_cmg_receipt_status != 0) return full_cmg_receipt_status;
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->schema_version = 1u;
    output->generation = generation;
    output->backend_identity = VCKSS_FULL_CMG_BACKEND_IDENTITY;
    output->platform_os = 1u;
    output->platform_arch = 1u;
    output->batch_strategy_mask = 2u;
    memcpy(output->cmg_source_commit, VCKSS_FULL_CMG_SOURCE_COMMIT, 40);
    output->threads_requested = 4u;
    output->threads_used = 4u;
    output->maximum_concurrency = 4u;
    output->vertices = 12u;
    output->edges = 20u;
    output->hierarchy_levels = 2u;
    output->terminal_vertices = 4u;
    output->graph_copy_bytes = 1024u;
    output->hierarchy_bytes = 2048u;
    output->plan_bytes = 512u;
    output->workspace_bytes_each = 512u;
    output->workspace_pool_bytes = 2048u;
    output->preparation_peak_bytes = 4096u;
    output->prepared_persistent_bytes = 512u;
    output->non_cmg_command_peak_bytes = 1024u;
    output->actual_retained_bytes = 7000u;
    output->allocator_allowance_bytes = 1400u;
    output->admitted_peak_bytes = 9424u;
    output->pre_rng_forecast_bytes = 12000u;
    output->maximum_batch_rhs = 64u;
    output->workspace_count = 4u;
    output->fit_effective_tolerance = 1e-10;
    output->probe_effective_tolerance = 1e-6;
    output->fit_initial_inner_tolerance = 1e-12;
    output->probe_initial_inner_tolerance = 1e-6;
    output->batch_calls = 3u;
    output->rhs_count = 5u;
    output->planned_batches = 3u;
    output->total_iterations = 20u;
    output->total_operator_applications = 25u;
    output->total_preconditioner_applications = 20u;
    output->maximum_reduced_residual = 1e-7;
    output->maximum_complete_residual = 1e-7;
    if (corrupt_full_cmg_receipt) ++output->backend_identity;
    return 0;
}

int32_t vckss_rust_engine_projection_result_receipt_v1(
    uint64_t generation,
    VckssProjectionResultReceiptV1 *output,
    uint32_t output_capacity_bytes
)
{
    assert(generation == active_generation);
    assert(output != NULL && output_capacity_bytes == sizeof(*output));
    memset(output, 0, sizeof(*output));
    output->struct_size = (uint32_t)sizeof(*output);
    output->schema_version = VCKSS_PROJECTION_SCHEMA_V1;
    output->generation = generation;
    return 0;
}

int32_t vckss_rust_engine_rhs_receipts_v1(
    uint64_t generation,
    VckssEngineRhsReceiptV1 *output,
    uint64_t output_count
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_count == rhs_row_count);
    if (rhs_v1_status == 0) memset(output, 0, (size_t)output_count * sizeof(*output));
    return rhs_v1_status;
}

int32_t vckss_rust_engine_rhs_receipts_v2(
    uint64_t generation,
    VckssEngineRhsReceiptV2 *output,
    uint64_t output_count
)
{
    assert(generation == active_generation);
    assert(output != NULL);
    assert(output_count == rhs_row_count);
    if (rhs_v2_status == 0) memset(output, 0, (size_t)output_count * sizeof(*output));
    return rhs_v2_status;
}

static ST_int mock_error(char *message)
{
    assert(message != NULL);
    ++error_calls;
    return 0;
}

static ST_int mock_scalar_save(char *name, ST_double value)
{
    ++scalar_calls;
    if (strcmp(name, "__vckss_rust_execution_api") == 0) {
        if (fail_execution_api_scalar) return 1;
        saved_execution_api = value;
    }
    if (strcmp(name, "__vckss_rust_progress_api") == 0) saved_progress_api = value;
    if (strcmp(name, "__vckss_rust_error_code") == 0) saved_error_code = value;
    if (fail_plan_scalar && strcmp(name, "__vckss_plan_struct") == 0) return 1;
    return fail_scalar;
}

static ST_int mock_macro_save(char *name, char *value)
{
    if (strcmp(name, "__vckss_rust_error_status") == 0) {
        (void)snprintf(saved_error_status, sizeof(saved_error_status), "%s", value);
    } else if (strcmp(name, "__vckss_rust_error_detail") == 0) {
        (void)snprintf(saved_error_detail, sizeof(saved_error_detail), "%s", value);
    } else if (strcmp(name, "__vckss_cmg_backend") == 0) {
        (void)snprintf(saved_cmg_backend, sizeof(saved_cmg_backend), "%s", value);
    } else if (strcmp(name, "__vckss_cmg_source") == 0) {
        (void)snprintf(saved_cmg_source, sizeof(saved_cmg_source), "%s", value);
    }
    return 0;
}

static ST_int mock_matrix_store(char *name, ST_int row, ST_int column, ST_double value)
{
    (void)name;
    (void)row;
    (void)column;
    (void)value;
    return fail_matrix;
}

static ST_int mock_matrix_rows(char *name)
{
    assert(strcmp(name, "R") == 0);
    return matrix_rows;
}

static ST_int mock_matrix_columns(char *name)
{
    assert(strcmp(name, "R") == 0);
    return matrix_columns;
}

static int mock_cleanup_release(uint64_t generation)
{
    ++release_calls;
    released_generation = generation;
    if (release_status == 0 && generation == active_generation) {
        active_generation = 0;
        last_released_generation = generation;
    }
    return release_status;
}

static int mock_cleanup_clear(void)
{
    ++clear_calls;
    if (clear_status == 0) active_generation = 0;
    return clear_status;
}

static void reset_transport(void)
{
    fail_scalar = 0;
    fail_plan_scalar = 0;
    fail_execution_api_scalar = 0;
    saved_execution_api = -1;
    saved_progress_api = -1;
    fail_matrix = 0;
    error_calls = 0;
    release_calls = 0;
    clear_calls = 0;
    release_status = 0;
    clear_status = 0;
    released_generation = 0;
    selftest_status = 0;
    selftest_error = "OK";
    scalar_calls = 0;
    request_capability_calls = 0;
    corrupt_request_echo = 0;
    result_status = 0;
    stayer_result_status = 0;
    stayer_augmentation_status = 0;
    corrupt_stayer_receipt = 0;
    detailed_receipt_status = 0;
    detailed_receipt_v7_status = 0;
    corrupt_v7_receipt = 0;
    detailed_capability_schema = 0;
    rhs_v1_status = 0;
    rhs_v2_status = 0;
    rhs_row_count = 2;
    rhs_receipt_schema = 2;
    rhs_copy_override = 0;
    rhs_v1_copy_bytes = 0;
    rhs_v2_copy_bytes = 0;
    matrix_rows = 2;
    matrix_columns = 15;
    active_generation = 0;
    last_released_generation = 0;
    saved_error_code = 0.0;
    saved_error_status[0] = '\0';
    saved_error_detail[0] = '\0';
    solve_v4_calls = 0;
    solve_v5_calls = 0;
    solve_v6_calls = 0;
    solve_v8_calls = 0;
    memset(&captured_solve_v6, 0, sizeof(captured_solve_v6));
    memset(&captured_solve_v8, 0, sizeof(captured_solve_v8));
    execution_receipt = (VckssGenericExecutionReceiptV1){
        .struct_size = sizeof(execution_receipt), .schema_version = 1, .generation = 705,
        .execution_mode = 1, .permitted_threads = 7, .planned_workers = 4,
        .maximum_active_workers = 3, .maximum_rhs_capacity = 4,
        .leverage_batch_width = 3, .target_batch_width = 2,
        .fit_rhs_count = 1, .point_probe_rhs_count = 51, .logical_rhs_count = 52,
        .queued_rhs_count = 51, .command_peak_forecast_bytes = 4096,
        .maximum_complete_residual = 1e-12};
    solved_generation = 0;
    memset(&captured_solve_v4, 0, sizeof(captured_solve_v4));
    memset(&captured_solve_v5, 0, sizeof(captured_solve_v5));
    full_cmg_receipt_status = 0;
    corrupt_full_cmg_receipt = 0;
    corrupt_full_cmg_model_receipt = 0;
    saved_cmg_backend[0] = '\0';
    saved_cmg_source[0] = '\0';
    vckss_test_fail_allocation = 0;
    vckss_clear_error_transport();
}

static void assert_released_idle(uint64_t generation)
{
    assert(release_calls == 1);
    assert(clear_calls == 1);
    assert(released_generation == generation);
    assert(active_generation == 0);
    assert(last_released_generation == generation);
}

static void assert_exported_primary(int code, const char *status, const char *detail_fragment)
{
    fail_scalar = 0;
    assert(vckss_export_last_error() == 0);
    assert(saved_error_code == (double)code);
    assert(strcmp(saved_error_status, status) == 0);
    assert(strstr(saved_error_detail, detail_fragment) != NULL);
}

static void assert_result_receipt_failure(uint64_t generation, const char *detail_fragment)
{
    active_generation = generation;
    assert(vckss_result(active_generation) == 498);
    assert_released_idle(generation);
    assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
    assert(strcmp(vckss_last_native_status, "INTERNAL_INVARIANT_FAILED") == 0);
    assert_exported_primary(
        VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
        "INTERNAL_INVARIANT_FAILED",
        detail_fragment
    );
}

int main(void)
{
    ST_plugin plugin = {0};
    plugin.spouterr = mock_error;
    plugin.macresave = mock_macro_save;
    plugin.scalsave = mock_scalar_save;
    plugin.safematstore = mock_matrix_store;
    plugin.matstore = mock_matrix_store;
    plugin.rowsof = mock_matrix_rows;
    plugin.colsof = mock_matrix_columns;
    _stata_ = &plugin;

    {
        reset_transport();
        char *args[] = {"solve", "1", "81227", "7", "0", "0", "auto", "1e-12", "10000",
            "exact", "observation", "joint", "500", "5000", "1e-10", "1e-10", "auto",
            "auto", "movers", "explicit", "observation", "0", "0", "50000000", "3",
            "4", "1", "2290599403", "1137066349", "auto", "auto", "1", "0"};
        assert(vckss_solve(33, args) == 0);
        assert(solve_v4_calls == 1);
    }

    {
        reset_transport();
        char *args[] = {"solveexactexecution", "705", "17", "17", "0", "0", "auto",
            "1e-10", "100", "exact", "observation", "joint", "1000", "100",
            "1e-12", "1e-10", "auto", "auto", "all", "explicit", "observation",
            "0", "0", "50000000", "3", "4", "1", "270544960", "1348497536",
            "auto", "auto", "1", "0", "7"};
        assert(vckss_solve_generic_execution(34, args, 3) == 0);
        assert(exact_execution_calls == 1 && solve_v4_calls == 0 && solve_v8_calls == 0);
        assert(captured_exact_execution.options.v4.v3.v2.v1.struct_size == 320);
        assert(captured_exact_execution.options.threads == 7);
        assert(captured_exact_execution.options.reserved == 0 && captured_exact_execution.reserved == 0);
        assert(captured_exact_execution.options.v4.v3.request_signature == UINT64_C(0x1020304050607080));
        assert(captured_exact_execution.interrupt_poll == vckss_stata_interrupt_poll);
        assert(captured_exact_execution.checkpoint_interval == 1);
        assert(vckss_solve_generic_execution(33, args, 3) == 198);
        args[9] = "jla";
        assert(vckss_solve_generic_execution(34, args, 3) == 198);
        args[9] = "exact"; args[33] = "0";
        assert(vckss_solve_generic_execution(34, args, 3) == 198);
        assert(exact_execution_calls == 1);
        args[0] = "solveexactresolvedv2"; args[9] = "auto"; args[33] = "7";
        assert(vckss_solve_generic_execution(34, args, 4) == 0);
        assert(exact_execution_calls == 2);
        assert(captured_exact_execution.options.v4.v3.request_signature == UINT64_C(0x1020304050607080));
        assert(vckss_solve_generic_execution(33, args, 4) == 198);
        args[9] = "exact";
        assert(vckss_solve_generic_execution(34, args, 4) == 198);
    }
    for (int corrupt = 0; corrupt <= 9; ++corrupt) {
        reset_transport();
        active_generation = 705;
        exact_execution_receipt = (VckssExactExecutionReceiptV1){
            sizeof(VckssExactExecutionReceiptV1), 1, 705, 7, 7, 20, 2, 4096, 8192, 1e-12};
        switch (corrupt) {
            case 1: exact_execution_receipt.struct_size = 63; break;
            case 2: exact_execution_receipt.schema_version = 2; break;
            case 3: exact_execution_receipt.generation = 706; break;
            case 4: exact_execution_receipt.worker_limit = 8; break;
            case 5: exact_execution_receipt.worker_limit = 1; break;
            case 6: exact_execution_receipt.estimator_passes = 3; break;
            case 7: exact_execution_receipt.command_peak_forecast_bytes = 1; break;
            case 8: exact_execution_receipt.maximum_original_fit_residual = NAN; break;
            case 9: exact_execution_receipt.parallel_regions = UINT64_MAX; break;
        }
        assert(vckss_exact_execution_receipt(705) == (corrupt ? 498 : 0));
        assert(release_calls == (corrupt ? 1 : 0));
        if (corrupt) assert(scalar_calls == 0);
    }

    reset_transport();
    assert(vckss_probe() == 0);
    assert(saved_execution_api == 3 && saved_progress_api == 1 && scalar_calls == 10);
    reset_transport();
    fail_execution_api_scalar = 1;
    assert(vckss_probe() == VCKSS_STATA_MEMORY_ERROR);
    assert(saved_execution_api == -1 && release_calls == 0 && clear_calls == 0);

    for (int mode = 1; mode <= 2; ++mode) {
        reset_transport();
        char *args[] = {"solveexecution", "705", "17", "17", "3", "2", "diagonal",
            "1e-11", "100", "jla", "observation", "fixedoffset", "1000", "100",
            "1e-12", "1e-10", "generic", "explicit", "all", "explicit", "observation",
            "1", "0", "50000000", "3", "4", "1", "270544960", "1348497536",
            "explicit", "explicit", "0", "0", "7", "1"};
        if (mode == 2) {
            args[4] = "0"; args[5] = "0"; args[6] = "cmg"; args[17] = "auto";
            args[29] = "auto"; args[30] = "auto"; args[34] = "2";
        }
        assert(vckss_solve_generic_execution(35, args, 0) == 0);
        assert(solve_v6_calls == 1 && solve_v5_calls == 0 && solve_v4_calls == 0);
        assert(solved_generation == 705 && captured_solve_v6.options.threads == 7);
        assert(captured_solve_v6.options.execution_mode == (uint32_t)mode);
        assert(captured_solve_v6.options.v4.v3.request_signature == UINT64_C(0x1020304050607080));
        assert(captured_solve_v6.options.v4.v3.v2.v1.rng_contract == VCKSS_RNG_COUNTER_V1);
        assert(captured_solve_v6.interrupt_poll == vckss_stata_interrupt_poll);
        assert(captured_solve_v6.interrupt_context == NULL && captured_solve_v6.checkpoint_interval == 1);
        assert(vckss_solve_generic_execution(34, args, 0) == 198);
        args[33] = "0";
        assert(vckss_solve_generic_execution(35, args, 0) == 198);
        args[33] = "7"; args[34] = "3";
        assert(vckss_solve_generic_execution(35, args, 0) == 198);
        args[34] = "0";
        assert(vckss_solve_generic_execution(35, args, 0) == 198);
        assert(solve_v6_calls == 1);
    }
    {
        reset_transport();
        char *args[] = {"solveexecutionv8", "705", "17", "17", "0", "0", "auto",
            "1e-10", "100", "auto", "observation", "joint", "1000", "100",
            "1e-12", "1e-10", "auto", "auto", "all", "explicit", "observation",
            "0", "0", "50000000", "3", "4", "1", "270544960", "1348497536",
            "auto", "auto", "1", "0", "7", "3", "0"};
        assert(vckss_solve_generic_execution(36, args, 2) == 0);
        assert(solve_v8_calls == 1 && solve_v6_calls == 0 && solve_v5_calls == 0 && solve_v4_calls == 0);
        assert(solved_generation == 705 && captured_solve_v8.options.v7.v6.threads == 7);
        assert(captured_solve_v8.options.v7.v6.execution_mode == VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO);
        assert(captured_solve_v8.options.resolved_execution_mode == VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO);
        assert(captured_solve_v8.options.tolerance_supplied == 0);
        assert(captured_solve_v8.options.v7.v6.v4.v3.request_signature == UINT64_C(0x1020304050607080));
        assert(captured_solve_v8.interrupt_poll == vckss_stata_interrupt_poll);
        assert(vckss_solve_generic_execution(35, args, 2) == 198);
        args[35] = "2";
        assert(vckss_solve_generic_execution(36, args, 2) == 198);
        assert(solve_v8_calls == 1);
    }
    for (int mode = 1; mode <= 2; ++mode) {
        for (int corrupt = 0; corrupt <= 14; ++corrupt) {
            reset_transport();
            active_generation = 705;
            if (mode == 2) {
                execution_receipt.execution_mode = 2;
                execution_receipt.maximum_active_workers = 0;
                execution_receipt.cmg_selected_concurrency = 4;
                execution_receipt.projection_rhs_count = 2;
                execution_receipt.logical_rhs_count = 54;
                execution_receipt.queued_rhs_count = 0;
                execution_receipt.cmg_rhs_count = 55;
                execution_receipt.control_refinement_rhs_count = 1;
            }
            switch (corrupt) {
                case 1: execution_receipt.struct_size = 159; break;
                case 2: execution_receipt.schema_version = 2; break;
                case 3: execution_receipt.generation = 706; break;
                case 4: execution_receipt.reserved = 1; break;
                case 5: execution_receipt.planned_workers = 8; break;
                case 6: execution_receipt.maximum_rhs_capacity = 3; break;
                case 7: execution_receipt.logical_rhs_count++; break;
                case 8: execution_receipt.queued_rhs_count++; break;
                case 9: execution_receipt.cmg_rhs_count++; break;
                case 10: execution_receipt.maximum_complete_residual = NAN; break;
                case 11: execution_receipt.command_peak_forecast_bytes = UINT64_MAX; break;
                case 12: execution_receipt.component_rhs_count = UINT64_MAX; break;
                case 13: execution_receipt.maximum_active_workers = mode == 1 ? 0 : 1; break;
                case 14: execution_receipt.execution_mode = 0; break;
            }
            assert(vckss_generic_execution_receipt(705) == (corrupt ? 498 : 0));
            if (corrupt) assert_released_idle(705);
            else assert(active_generation == 705 && scalar_calls == 23);
        }
    }
    reset_transport();
    active_generation = 705;
    fail_scalar = 1;
    assert(vckss_generic_execution_receipt(705) != 0);
    assert_released_idle(705);

    reset_transport();
    active_generation = UINT64_C(701);
    assert(vckss_full_cmg_model_receipt(active_generation) == 0);
    assert(active_generation == UINT64_C(701));
    for (int corrupt = 1; corrupt <= 3; ++corrupt) {
        reset_transport();
        active_generation = UINT64_C(701);
        corrupt_full_cmg_model_receipt = corrupt;
        assert(vckss_full_cmg_model_receipt(active_generation) == 498);
        assert_released_idle(UINT64_C(701));
    }

    reset_transport();
    {
        char *request_argv[] = {
            "requestcapability", "auto", "observation", "joint",
            "auto", "counter_v1", "2", "literal", "auto",
            "independent", "movers", "explicit", "observation", "0", "1",
            "50000000", "auto", "explicit", "1", "3.5"
        };
        assert(vckss_request_capability(20, request_argv) == 0);
        assert(request_capability_calls == 1);
        assert(vckss_last_native_code == 0);
    }

    reset_transport();
    {
        char *request_argv[] = {
            "requestcapability", "jla", "match", "joint",
            "auto", "counter_v1", "0", "literal", "auto",
            "independent", "movers", "frequency", "cell", "0", "0",
            "50000000", "auto", "auto", "1", "0"
        };
        corrupt_request_echo = 1;
        assert(vckss_request_capability(20, request_argv) == 498);
        assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
        assert(strstr(vckss_last_native_detail, "did not reconcile") != NULL);
    }

    reset_transport();
    {
        char *solve_argv[] = {
            "solve", "701", "17", "3", "2", "3", "auto", "1e-11", "100",
            "jla", "observation", "fixedoffset", "1000", "100", "1e-12", "1e-10",
            "generic", "independent", "movers", "explicit", "observation", "0", "1",
            "50000000", "3", "4", "1", "270544960", "1348497536", "auto",
            "explicit", "1", "3.5"
        };
        assert(vckss_solve(33, solve_argv) == 0);
        assert(solve_v4_calls == 1);
        assert(solved_generation == UINT64_C(701));
        assert(captured_solve_v4.options.v3.v2.algorithm == VCKSS_ALGORITHM_JLA);
        assert(captured_solve_v4.options.v3.engine == VCKSS_ENGINE_GENERIC);
        assert(captured_solve_v4.options.v3.batch_mode == VCKSS_BATCH_MODE_INDEPENDENT);
        assert(captured_solve_v4.options.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO);
        assert(captured_solve_v4.options.target_batch_mode == VCKSS_BATCH_MODE_EXPLICIT);
        assert(captured_solve_v4.options.wallseconds == 3.5);
        assert(captured_solve_v4.options.v3.request_signature ==
            UINT64_C(0x1020304050607080));
        assert(captured_solve_v4.options.v3.v2.v1.allow_automatic_cmg_setup_fallback == 1);
        assert(captured_solve_v4.interrupt_poll == vckss_stata_interrupt_poll);
        assert(captured_solve_v4.checkpoint_interval == 1);
    }

    reset_transport();
    {
        char *solve_argv[] = {
            "solvefull", "702", "17", "3", "0", "0", "auto", "1e-10", "100",
            "jla", "match", "joint", "1000", "100", "1e-12", "1e-10",
            "auto", "auto", "movers", "frequency", "cell", "0", "0",
            "50000000", "3", "4", "0", "270544960", "1348497536", "auto",
            "auto", "0", "0", "4", "1"
        };
        assert(vckss_solve_full(35, solve_argv) == 0);
        assert(solve_v5_calls == 1);
        assert(solve_v4_calls == 0);
        assert(solved_generation == UINT64_C(702));
        assert(captured_solve_v5.options.v4.v3.v2.algorithm == VCKSS_ALGORITHM_JLA);
        assert(captured_solve_v5.options.v4.v3.engine == VCKSS_ENGINE_AUTO_OR_UNSPECIFIED);
        assert(captured_solve_v5.options.v4.v3.v2.v1.deletion_mode == VCKSS_DELETION_MATCH);
        assert(captured_solve_v5.options.v4.v3.v2.nuisance_mode == VCKSS_NUISANCE_JOINT);
        assert(captured_solve_v5.options.v4.v3.v2.v1.rng_contract == VCKSS_RNG_COUNTER_V1);
        assert(captured_solve_v5.options.threads == 4);
        assert(captured_solve_v5.options.tolerance_supplied == 1);
        assert(captured_solve_v5.options.full_cmg_v2 == 1);
        assert(captured_solve_v5.options.v4.v3.request_signature ==
            UINT64_C(0x1020304050607080));
        assert(captured_solve_v5.interrupt_poll == vckss_stata_interrupt_poll);
        assert(captured_solve_v5.checkpoint_interval == 1);
    }

    reset_transport();
    active_generation = UINT64_C(703);
    {
        assert(vckss_full_cmg_receipt(UINT64_C(703)) == 0);
        assert(strcmp(saved_cmg_backend, "CMG_FULL_V2") == 0);
        assert(strcmp(saved_cmg_source, VCKSS_FULL_CMG_SOURCE_COMMIT) == 0);
        assert(release_calls == 0);
        assert(clear_calls == 0);
    }

    reset_transport();
    active_generation = UINT64_C(704);
    corrupt_full_cmg_receipt = 1;
    {
        assert(vckss_full_cmg_receipt(UINT64_C(704)) == 498);
        assert_released_idle(UINT64_C(704));
        assert(strstr(vckss_last_native_detail, "full-CMG receipt identity") != NULL);
    }

    reset_transport();
    {
        char *request_argv[] = {
            "requestcapability", "jla", "observation", "fixedoffset",
            "diagonal", "counter_v1", "2", "literal", "generic",
            "explicit", "movers", "explicit", "observation", "0", "0",
            "50000000"
        };
        assert(vckss_request_capability(16, request_argv) == 0);
        assert(request_capability_calls == 1);
        assert(scalar_calls == 23);
        assert(vckss_last_native_code == 0);
    }

    reset_transport();
    {
        char *request_argv[] = {
            "requestcapability", "jla", "match", "joint",
            "diagonal", "counter_v1", "0", "literal", "generic",
            "explicit", "movers", "frequency", "cell", "0", "0",
            "50000000"
        };
        corrupt_request_echo = 1;
        assert(vckss_request_capability(16, request_argv) == 498);
        assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
        assert(strstr(vckss_last_native_detail, "did not reconcile") != NULL);
    }

    reset_transport();
    {
        char *request_argv[] = {
            "requestcapability", "exact", "observation", "fixedoffset",
            "auto", "none", "2", "literal"
        };
        assert(vckss_request_capability(8, request_argv) == 0);
        assert(request_capability_calls == 1);
        assert(scalar_calls == 15);
        assert(vckss_last_native_code == 0);
    }

    reset_transport();
    {
        char *request_argv[] = {
            "requestcapability", "exact", "match", "joint",
            "auto", "none", "0", "unit"
        };
        corrupt_request_echo = 1;
        assert(vckss_request_capability(8, request_argv) == 498);
        assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
        assert(strstr(vckss_last_native_detail, "did not reconcile") != NULL);
    }

    reset_transport();
    fail_scalar = 1;
    assert(vckss_save_double("__vckss_test", 1.0) == VCKSS_STATA_MEMORY_ERROR);
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strcmp(vckss_last_native_status, "ALLOCATION_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "scalar") != NULL);
    assert(error_calls == 1);

    reset_transport();
    fail_matrix = 1;
    assert(vckss_store_rhs_matrix_cell("M", 1, 1, 1.0) == VCKSS_STATA_MEMORY_ERROR);
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strstr(vckss_last_native_detail, "matrix") != NULL);
    assert(error_calls == 1);

    reset_transport();
    assert(vckss_cshim_test_allocation_failure() == VCKSS_STATA_MEMORY_ERROR);
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strstr(vckss_last_native_detail, "allocation") != NULL);
    assert(error_calls == 1);

    reset_transport();
    rhs_receipt_schema = 0;
    rhs_row_count = 1;
    assert_result_receipt_failure(UINT64_C(9201), "schema 0");

    reset_transport();
    rhs_receipt_schema = 0;
    rhs_row_count = 0;
    rhs_copy_override = 1;
    rhs_v2_copy_bytes = 1;
    assert_result_receipt_failure(UINT64_C(9202), "schema 0");

    reset_transport();
    rhs_receipt_schema = 1;
    rhs_row_count = 0;
    assert_result_receipt_failure(UINT64_C(9203), "schema 1");

    reset_transport();
    rhs_receipt_schema = 1;
    rhs_copy_override = 1;
    rhs_v1_copy_bytes = UINT64_C(223);
    assert_result_receipt_failure(UINT64_C(9204), "schema 1");

    reset_transport();
    rhs_receipt_schema = 1;
    rhs_copy_override = 1;
    rhs_v1_copy_bytes = UINT64_C(224);
    rhs_v2_copy_bytes = 1;
    assert_result_receipt_failure(UINT64_C(9205), "schema 1");

    reset_transport();
    rhs_receipt_schema = 2;
    rhs_copy_override = 1;
    rhs_v1_copy_bytes = 1;
    rhs_v2_copy_bytes = UINT64_C(432);
    assert_result_receipt_failure(UINT64_C(9206), "schema 2");

    reset_transport();
    rhs_receipt_schema = 9;
    assert_result_receipt_failure(UINT64_C(9207), "unknown schema");

    reset_transport();
    rhs_receipt_schema = 1;
    rhs_row_count = UINT64_MAX;
    assert_result_receipt_failure(UINT64_C(9208), "schema 1");

    reset_transport();
    rhs_receipt_schema = 2;
    rhs_row_count = UINT64_MAX;
    assert_result_receipt_failure(UINT64_C(9209), "schema 2");

    reset_transport();
    active_generation = UINT64_C(9210);
    rhs_receipt_schema = 1;
    rhs_copy_override = 1;
    rhs_v1_copy_bytes = 1;
    matrix_columns = 8;
    assert(vckss_rhsresult(active_generation, "R") == 498);
    assert_released_idle(UINT64_C(9210));
    assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
    assert(strcmp(vckss_last_native_status, "INTERNAL_INVARIANT_FAILED") == 0);
    assert_exported_primary(
        VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
        "INTERNAL_INVARIANT_FAILED",
        "RHS V1 export memory receipt"
    );

    reset_transport();
    active_generation = UINT64_C(9301);
    fail_scalar = 1;
    assert(vckss_result(active_generation) == VCKSS_STATA_MEMORY_ERROR);
    assert_released_idle(UINT64_C(9301));
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strcmp(vckss_last_native_status, "ALLOCATION_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "scalar") != NULL);
    assert_exported_primary(
        VCKSS_ERROR_ALLOCATION_FAILED,
        "ALLOCATION_FAILED",
        "could not save a Rust backend scalar"
    );

    reset_transport();
    active_generation = UINT64_C(9302);
    vckss_test_fail_allocation = 1;
    assert(vckss_rhsresult(active_generation, "R") == VCKSS_STATA_MEMORY_ERROR);
    assert_released_idle(UINT64_C(9302));
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strcmp(vckss_last_native_status, "ALLOCATION_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "caller copy") != NULL);
    assert_exported_primary(
        VCKSS_ERROR_ALLOCATION_FAILED,
        "ALLOCATION_FAILED",
        "could not allocate the Rust RHS V2 receipt caller copy"
    );

    reset_transport();
    active_generation = UINT64_C(9303);
    fail_matrix = 1;
    assert(vckss_rhsresult(active_generation, "R") == VCKSS_STATA_MEMORY_ERROR);
    assert_released_idle(UINT64_C(9303));
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strcmp(vckss_last_native_status, "ALLOCATION_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "matrix") != NULL);
    assert_exported_primary(
        VCKSS_ERROR_ALLOCATION_FAILED,
        "ALLOCATION_FAILED",
        "could not store the Rust RHS receipt matrix"
    );

    reset_transport();
    active_generation = UINT64_C(9304);
    result_status = 90;
    selftest_error = "INTERNAL_INVARIANT_FAILED [engine_result]: injected native fetch failure";
    assert(vckss_result(active_generation) == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9304));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9305);
    detailed_receipt_status = 90;
    selftest_error = "INTERNAL_INVARIANT_FAILED [detailed_receipt]: injected native fetch failure";
    assert(vckss_rhsresult(active_generation, "R") == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9305));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9306);
    detailed_receipt_status = 90;
    selftest_error =
        "INTERNAL_INVARIANT_FAILED [detailed_receipt]: injected result receipt fetch failure";
    assert(vckss_result(active_generation) == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9306));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9307);
    rhs_v2_status = 90;
    selftest_error = "INTERNAL_INVARIANT_FAILED [rhs_receipts]: injected row fetch failure";
    assert(vckss_rhsresult(active_generation, "R") == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9307));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9308);
    assert(vckss_result(active_generation) == 0);
    assert(vckss_rhsresult(active_generation, "R") == 0);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9308));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9312);
    detailed_capability_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V3;
    rhs_row_count = 1;
    assert(vckss_result(active_generation) == 0);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9312));

    reset_transport();
    active_generation = UINT64_C(9313);
    detailed_capability_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V3;
    rhs_row_count = 1;
    corrupt_v7_receipt = 1;
    assert(vckss_result(active_generation) == 498);
    assert_released_idle(UINT64_C(9313));
    assert_exported_primary(
        VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
        "INTERNAL_INVARIANT_FAILED",
        "V7 execution-plan receipt"
    );

    reset_transport();
    active_generation = UINT64_C(9314);
    detailed_capability_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V3;
    rhs_row_count = 1;
    fail_plan_scalar = 1;
    assert(vckss_result(active_generation) == VCKSS_STATA_MEMORY_ERROR);
    assert_released_idle(UINT64_C(9314));
    assert_exported_primary(
        VCKSS_ERROR_ALLOCATION_FAILED,
        "ALLOCATION_FAILED",
        "could not save a Rust backend scalar"
    );

    reset_transport();
    active_generation = UINT64_C(9315);
    detailed_capability_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V3;
    rhs_row_count = 1;
    detailed_receipt_v7_status = 90;
    selftest_error = "INTERNAL_INVARIANT_FAILED [detailed_receipt_v7]: injected native fetch failure";
    assert(vckss_result(active_generation) == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9315));

    reset_transport();
    active_generation = UINT64_C(9310);
    rhs_receipt_schema = 1;
    matrix_columns = 8;
    assert(vckss_result(active_generation) == 0);
    assert(vckss_rhsresult(active_generation, "R") == 0);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9310));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9311);
    rhs_receipt_schema = 0;
    rhs_row_count = 0;
    assert(vckss_result(active_generation) == 0);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9311));
    assert(last_released_generation == 0);

    reset_transport();
    active_generation = UINT64_C(9309);
    rhs_receipt_schema = 1;
    matrix_columns = 8;
    fail_matrix = 1;
    assert(vckss_rhsresult(active_generation, "R") == VCKSS_STATA_MEMORY_ERROR);
    assert_released_idle(UINT64_C(9309));
    assert(vckss_last_native_code == VCKSS_ERROR_ALLOCATION_FAILED);
    assert(strcmp(vckss_last_native_status, "ALLOCATION_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "matrix") != NULL);
    assert_exported_primary(
        VCKSS_ERROR_ALLOCATION_FAILED,
        "ALLOCATION_FAILED",
        "could not store the Rust RHS receipt matrix"
    );

    reset_transport();
    active_generation = UINT64_C(9320);
    assert(vckss_stayer_result(active_generation) == 0);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9320));

    reset_transport();
    active_generation = UINT64_C(9321);
    corrupt_stayer_receipt = 1;
    assert(vckss_stayer_result(active_generation) == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9321));
    assert_exported_primary(
        VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
        "INTERNAL_INVARIANT_FAILED",
        "stayer-hybrid result"
    );

    reset_transport();
    active_generation = UINT64_C(9322);
    stayer_result_status = VCKSS_ERROR_INTERNAL_INVARIANT_FAILED;
    selftest_error =
        "INTERNAL_INVARIANT_FAILED [stayer_result]: injected native fetch failure";
    assert(vckss_stayer_result(active_generation) == 498);
    assert(release_calls == 0);
    assert(clear_calls == 0);
    assert(active_generation == UINT64_C(9322));

    reset_transport();
    selftest_status = VCKSS_ERROR_INTERNAL_INVARIANT_FAILED;
    selftest_error =
        "SELFTEST_FAILED [standalone_selftest]: injected self-test failure";
    assert(vckss_run_selftest() == 498);
    assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
    assert(strcmp(vckss_last_native_status, "SELFTEST_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "injected self-test failure") != NULL);
    assert(error_calls == 1);

    reset_transport();
    vckss_store_error_transport(
        VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
        "INTERNAL_INVARIANT_FAILED",
        "INTERNAL_INVARIANT_FAILED [stata_spi]: primary export failure"
    );
    release_status = 77;
    clear_status = 88;
    vckss_cleanup_preserving_primary(UINT64_C(9127));
    assert(release_calls == 1);
    assert(clear_calls == 1);
    assert(released_generation == UINT64_C(9127));
    assert(vckss_last_native_code == VCKSS_ERROR_INTERNAL_INVARIANT_FAILED);
    assert(strcmp(vckss_last_native_status, "INTERNAL_INVARIANT_FAILED") == 0);
    assert(strstr(vckss_last_native_detail, "primary export failure") != NULL);

    reset_transport();
    vckss_store_error_transport(22, "INVALID_WEIGHT", "INVALID_WEIGHT [engine_ingest]: primary");
    fail_scalar = 1;
    assert(vckss_export_last_error() == VCKSS_STATA_MEMORY_ERROR);
    assert(vckss_last_native_code == 22);
    assert(strcmp(vckss_last_native_status, "INVALID_WEIGHT") == 0);
    assert(strstr(vckss_last_native_detail, "primary") != NULL);
    return 0;
}
