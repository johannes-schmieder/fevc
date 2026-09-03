/* SPDX-License-Identifier: GPL-3.0-only */

#include "stplugin.h"
#include "vckss_rust.h"
#include "stata_interrupt.h"

#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define VCKSS_STATA_USAGE_ERROR 198
#define VCKSS_STATA_MEMORY_ERROR 909
#define VCKSS_PREPARE_FIXED_VARIABLES 8
#define VCKSS_NUMERIC_COLUMNS_BASE 6u
#define VCKSS_STAYER_NUMERIC_COLUMNS_BASE 5u
#define VCKSS_INGEST_POLL_INTERVAL UINT64_C(4096)
#define VCKSS_STATA_SCALAR_NAME_LIMIT 32
#define VCKSS_MAX_EXACT_STATA_INTEGER UINT64_C(9007199254740992)
#define VCKSS_ERROR_STATUS_CAPACITY 64
#define VCKSS_ERROR_DETAIL_CAPACITY 1024
#define VCKSS_ERROR_ALLOCATION_FAILED 41
#define VCKSS_ERROR_INTERNAL_INVARIANT_FAILED 90
#define VCKSS_FULL_CMG_BACKEND_IDENTITY 2u
#define VCKSS_FULL_CMG_SOURCE_COMMIT "92a12f2d572ca56b30a035220953f9dd4bced999"
#define VCKSS_RUST_RUNTIME_BUILD_ID "0.4.0-alpha.1"
#define VCKSS_SCALAR_LITERAL(name) \
    ((void)sizeof(char[((sizeof(name) - 1u) <= VCKSS_STATA_SCALAR_NAME_LIMIT) ? 1 : -1]), (name))

static int vckss_last_native_code = 0;
static char vckss_last_native_status[VCKSS_ERROR_STATUS_CAPACITY] = "OK";
static char vckss_last_native_detail[VCKSS_ERROR_DETAIL_CAPACITY] = "";

#ifdef VCKSS_CSHIM_TEST
static int vckss_test_fail_allocation = 0;
#endif

static void *vckss_calloc(size_t count, size_t size)
{
#ifdef VCKSS_CSHIM_TEST
    if (vckss_test_fail_allocation != 0) {
        vckss_test_fail_allocation = 0;
        return NULL;
    }
#endif
    return calloc(count, size);
}

#ifndef VCKSS_CLEANUP_RELEASE
#define VCKSS_CLEANUP_RELEASE(generation) vckss_rust_engine_release_v1((generation))
#endif
#ifndef VCKSS_CLEANUP_CLEAR
#define VCKSS_CLEANUP_CLEAR() vckss_rust_engine_clear_abandoned_v1()
#endif

static void vckss_clear_error_transport(void)
{
    vckss_last_native_code = 0;
    (void)snprintf(vckss_last_native_status, sizeof(vckss_last_native_status), "OK");
    vckss_last_native_detail[0] = '\0';
}

static void vckss_store_error_transport(int code, const char *status, const char *detail)
{
    vckss_last_native_code = code;
    (void)snprintf(
        vckss_last_native_status,
        sizeof(vckss_last_native_status),
        "%s",
        status == NULL || *status == '\0' ? "INTERNAL_INVARIANT_FAILED" : status
    );
    (void)snprintf(
        vckss_last_native_detail,
        sizeof(vckss_last_native_detail),
        "%s",
        detail == NULL || *detail == '\0' ? "Rust backend failed without an error message" : detail
    );
}

static void vckss_store_rust_error(int code, const char *message)
{
    char status[VCKSS_ERROR_STATUS_CAPACITY];
    size_t length = 0;
    const char *cursor = message;

    if (cursor != NULL) {
        while (*cursor != '\0' && *cursor != ' ' && *cursor != '[' &&
               length + 1u < sizeof(status)) {
            status[length++] = *cursor++;
        }
    }
    status[length] = '\0';
    vckss_store_error_transport(code, status, message);
}

static int vckss_c_failure(
    int code,
    const char *status,
    const char *detail,
    int stata_status
)
{
    vckss_store_error_transport(code, status, detail);
    SF_error((char *)detail);
    return stata_status;
}

static void vckss_cleanup_preserving_primary(uint64_t generation)
{
    const int primary_code = vckss_last_native_code;
    char primary_status[VCKSS_ERROR_STATUS_CAPACITY];
    char primary_detail[VCKSS_ERROR_DETAIL_CAPACITY];

    (void)snprintf(primary_status, sizeof(primary_status), "%s", vckss_last_native_status);
    (void)snprintf(primary_detail, sizeof(primary_detail), "%s", vckss_last_native_detail);
    (void)VCKSS_CLEANUP_RELEASE(generation);
    (void)VCKSS_CLEANUP_CLEAR();
    if (primary_code == 0) {
        vckss_clear_error_transport();
    } else {
        vckss_store_error_transport(primary_code, primary_status, primary_detail);
    }
}

static int vckss_usage(const char *message)
{
    char detail[VCKSS_ERROR_DETAIL_CAPACITY];
    (void)snprintf(detail, sizeof(detail), "INVALID_INPUT [stata_spi]: %s", message);
    vckss_store_error_transport(20, "INVALID_INPUT", detail);
    SF_error((char *)message);
    return VCKSS_STATA_USAGE_ERROR;
}

static int vckss_rust_failure_with_message(int status, const char *message)
{
    if (status == VCKSS_INTERRUPT_USER_BREAK) {
        vckss_clear_error_transport();
        return 1;
    }
    vckss_store_rust_error(status, message);
    (void)SF_scal_save(
        (char *)VCKSS_SCALAR_LITERAL("__vckss_rust_error_code"),
        (double)status
    );
    if (message == NULL) {
        message = "Rust backend failed without an error message";
    }
    SF_error((char *)message);
    if (status == 11 || (status >= 20 && status <= 23)) {
        return VCKSS_STATA_USAGE_ERROR;
    }
    if (status == 30) {
        return 2000;
    }
    if (status == 40 || status == 41) {
        return VCKSS_STATA_MEMORY_ERROR;
    }
    return 498;
}

static int vckss_rust_failure(int status)
{
    return vckss_rust_failure_with_message(status, vckss_rust_engine_last_error());
}

static int vckss_run_selftest(void)
{
    const int status = vckss_rust_selftest();
    if (status == 0) {
        return 0;
    }
    /* The standalone self-test has its own top-level error slot.  It never
     * creates an engine generation, so consulting the engine registry here
     * would return an unrelated or stale message. */
    return vckss_rust_failure_with_message(status, vckss_rust_last_error());
}

static int vckss_export_last_error(void)
{
    if (SF_scal_save(
            (char *)VCKSS_SCALAR_LITERAL("__vckss_rust_error_code"),
            (double)vckss_last_native_code
        ) != 0 ||
        SF_macro_save("__vckss_rust_error_status", vckss_last_native_status) != 0 ||
        SF_macro_save("__vckss_rust_error_detail", vckss_last_native_detail) != 0) {
        if (vckss_last_native_code == 0) {
            vckss_store_error_transport(
                VCKSS_ERROR_ALLOCATION_FAILED,
                "ALLOCATION_FAILED",
                "ALLOCATION_FAILED [stata_spi]: could not export the Rust native error transport"
            );
        }
        SF_error("could not export the Rust native error transport");
        return VCKSS_STATA_MEMORY_ERROR;
    }
    return 0;
}

static int vckss_save_double(const char *name, double value)
{
    if (name == NULL || strlen(name) > VCKSS_STATA_SCALAR_NAME_LIMIT) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust backend scalar name exceeds Stata's 32-character limit",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    if (SF_scal_save((char *)name, value) != 0) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not save a Rust backend scalar",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    return 0;
}

static int vckss_save_u64(const char *name, uint64_t value)
{
    if (value > VCKSS_MAX_EXACT_STATA_INTEGER) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust receipt integer is not exactly representable by Stata",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    const double converted = (double)value;
    if ((uint64_t)converted != value) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust receipt integer lost precision during Stata conversion",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    return vckss_save_double(name, converted);
}

static int vckss_save_u64_parts(
    const char *high_name,
    const char *low_name,
    uint64_t value
)
{
    int status = vckss_save_u64(high_name, value >> 32);
    if (status != 0) return status;
    return vckss_save_u64(low_name, value & UINT64_C(0xffffffff));
}

static int vckss_full_cmg_receipt(uint64_t generation)
{
    VckssFullCmgReceiptV1 receipt;
    char source_commit[41];
    uint64_t strategy_batches;
    uint64_t retained_floor;
    uint64_t actual_cmg_peak;
    uint64_t actual_command_peak;
    uint64_t expected_admitted_peak;
    double complete_tolerance;
    int status;

    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_full_cmg_receipt_v1(
        generation, &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) return vckss_rust_failure(status);
    memcpy(source_commit, receipt.cmg_source_commit, 40);
    source_commit[40] = '\0';
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.schema_version != 1u || receipt.generation != generation ||
        receipt.backend_identity != VCKSS_FULL_CMG_BACKEND_IDENTITY ||
        memcmp(receipt.cmg_source_commit, VCKSS_FULL_CMG_SOURCE_COMMIT, 40) != 0 ||
        (receipt.platform_os != 1u && receipt.platform_os != 2u) ||
        (receipt.platform_arch != 1u && receipt.platform_arch != 2u) ||
        receipt.batch_strategy_mask == 0u ||
        (receipt.batch_strategy_mask & ~UINT32_C(7)) != 0u ||
        receipt.threads_requested == 0u ||
        receipt.threads_used != receipt.threads_requested ||
        receipt.maximum_concurrency == 0u ||
        receipt.maximum_concurrency > receipt.threads_used ||
        receipt.vertices == 0u || receipt.edges == 0u ||
        receipt.hierarchy_levels == 0u || receipt.terminal_vertices == 0u ||
        receipt.terminal_vertices > receipt.vertices ||
        receipt.workspace_pool_bytes < receipt.workspace_bytes_each ||
        receipt.maximum_batch_rhs == 0u || receipt.workspace_count == 0u ||
        receipt.workspace_count > receipt.threads_used ||
        receipt.workspace_count > receipt.maximum_batch_rhs ||
        receipt.maximum_concurrency > receipt.workspace_count ||
        receipt.prepared_persistent_bytes > receipt.non_cmg_command_peak_bytes ||
        !isfinite(receipt.fit_effective_tolerance) ||
        !isfinite(receipt.probe_effective_tolerance) ||
        !isfinite(receipt.fit_initial_inner_tolerance) ||
        !isfinite(receipt.probe_initial_inner_tolerance) ||
        receipt.fit_effective_tolerance <= 0.0 ||
        receipt.probe_effective_tolerance <= 0.0 ||
        receipt.fit_initial_inner_tolerance <= 0.0 ||
        receipt.probe_initial_inner_tolerance <= 0.0 ||
        receipt.fit_initial_inner_tolerance > receipt.fit_effective_tolerance ||
        receipt.probe_initial_inner_tolerance > receipt.probe_effective_tolerance ||
        receipt.batch_calls == 0u || receipt.rhs_count == 0u ||
        !isfinite(receipt.maximum_reduced_residual) ||
        !isfinite(receipt.maximum_complete_residual) ||
        receipt.maximum_reduced_residual < 0.0 ||
        receipt.maximum_complete_residual < 0.0) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: full-CMG receipt identity or structural fields are inconsistent",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    if (receipt.serial_batches > UINT64_MAX - receipt.planned_batches ||
        receipt.serial_batches + receipt.planned_batches > UINT64_MAX - receipt.across_rhs_batches) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: full-CMG batch receipt overflowed",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    strategy_batches = receipt.serial_batches + receipt.planned_batches +
        receipt.across_rhs_batches;
    if (strategy_batches != receipt.batch_calls ||
        receipt.graph_copy_bytes > UINT64_MAX - receipt.hierarchy_bytes ||
        receipt.graph_copy_bytes + receipt.hierarchy_bytes > UINT64_MAX - receipt.plan_bytes ||
        receipt.graph_copy_bytes + receipt.hierarchy_bytes + receipt.plan_bytes >
            UINT64_MAX - receipt.workspace_pool_bytes ||
        receipt.actual_retained_bytes > UINT64_MAX - receipt.allocator_allowance_bytes) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: full-CMG batch or memory receipt is inconsistent",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    retained_floor = receipt.graph_copy_bytes + receipt.hierarchy_bytes +
        receipt.plan_bytes + receipt.workspace_pool_bytes;
    actual_cmg_peak = receipt.actual_retained_bytes + receipt.allocator_allowance_bytes;
    if (receipt.non_cmg_command_peak_bytes > UINT64_MAX - actual_cmg_peak) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: full-CMG command-memory receipt overflowed",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    actual_command_peak = receipt.non_cmg_command_peak_bytes + actual_cmg_peak;
    expected_admitted_peak = receipt.preparation_peak_bytes > actual_command_peak
        ? receipt.preparation_peak_bytes : actual_command_peak;
    complete_tolerance = fmax(
        1e-11,
        10.0 * fmax(receipt.fit_effective_tolerance, receipt.probe_effective_tolerance)
    );
    if (receipt.actual_retained_bytes < retained_floor ||
        receipt.allocator_allowance_bytes != receipt.actual_retained_bytes / UINT64_C(5) ||
        receipt.admitted_peak_bytes != expected_admitted_peak ||
        receipt.pre_rng_forecast_bytes < receipt.admitted_peak_bytes ||
        receipt.maximum_complete_residual > complete_tolerance) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: full-CMG memory or complete-residual receipt did not reconcile",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    if (SF_macro_save("__vckss_cmg_backend", "CMG_FULL_V2") != 0 ||
        SF_macro_save("__vckss_cmg_source", source_commit) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_struct", receipt.struct_size)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_schema", receipt.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_generation", receipt.generation)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_backend_id", receipt.backend_identity)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_platform_os", receipt.platform_os)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_platform_arch", receipt.platform_arch)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_batch_mask", receipt.batch_strategy_mask)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_threads_req", receipt.threads_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_threads_used", receipt.threads_used)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_max_concurrency", receipt.maximum_concurrency)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_vertices", receipt.vertices)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_edges", receipt.edges)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_levels", receipt.hierarchy_levels)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_terminal", receipt.terminal_vertices)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_graph_bytes", receipt.graph_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_hierarchy_bytes", receipt.hierarchy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_plan_bytes", receipt.plan_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_ws_each", receipt.workspace_bytes_each)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_ws_pool", receipt.workspace_pool_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_admitted_peak", receipt.admitted_peak_bytes)) != 0 ||
        (status = vckss_save_double("__vckss_cmg_fit_tol", receipt.fit_effective_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_cmg_probe_tol", receipt.probe_effective_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_cmg_fit_inner", receipt.fit_initial_inner_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_cmg_probe_inner", receipt.probe_initial_inner_tolerance)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_refine_attempts", receipt.refinement_attempts)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_refined_cols", receipt.refined_columns)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_batch_calls", receipt.batch_calls)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_rhs_count", receipt.rhs_count)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_serial_batches", receipt.serial_batches)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_planned_batches", receipt.planned_batches)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_across_batches", receipt.across_rhs_batches)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_iterations", receipt.total_iterations)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_operator_apps", receipt.total_operator_applications)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_precond_apps", receipt.total_preconditioner_applications)) != 0 ||
        (status = vckss_save_double("__vckss_cmg_max_reduced", receipt.maximum_reduced_residual)) != 0 ||
        (status = vckss_save_double("__vckss_cmg_max_complete", receipt.maximum_complete_residual)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_graph_ns", receipt.graph_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_hierarchy_ns", receipt.hierarchy_plan_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_rhs_ns", receipt.rhs_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_solve_ns", receipt.solve_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_extract_ns", receipt.extraction_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_prep_peak", receipt.preparation_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_prepared_bytes", receipt.prepared_persistent_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_non_cmg_peak", receipt.non_cmg_command_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_pre_rng_forecast", receipt.pre_rng_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_actual_retained", receipt.actual_retained_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_allocator_allowance", receipt.allocator_allowance_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_max_batch_rhs", receipt.maximum_batch_rhs)) != 0 ||
        (status = vckss_save_u64("__vckss_cmg_workspace_count", receipt.workspace_count)) != 0) {
        if (status == 0) {
            status = vckss_c_failure(
                VCKSS_ERROR_ALLOCATION_FAILED,
                "ALLOCATION_FAILED",
                "ALLOCATION_FAILED [stata_spi]: could not save the full-CMG identity locals",
                VCKSS_STATA_MEMORY_ERROR
            );
        }
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    return 0;
}

static int vckss_export_counter_phase(
    const char *phase,
    const VckssCounterPhaseExecutionReceiptV1 *receipt
)
{
    static const char *suffix[8] = {"pla", "ala", "puw", "auw", "ppt", "apt", "pgw", "agw"};
    const uint64_t value[8] = {
        receipt->planned_logical_atoms,
        receipt->actual_logical_atoms,
        receipt->planned_unique_packed_words,
        receipt->actual_unique_packed_words,
        receipt->planned_physical_trials,
        receipt->actual_physical_trials,
        receipt->planned_generator_work,
        receipt->actual_generator_work
    };
    char high[33];
    char low[33];
    int index;
    int status;
    for (index = 0; index < 8; ++index) {
        if (snprintf(high, sizeof(high), "__vckss_ctr_%s_%s_hi", phase, suffix[index]) < 0 ||
            snprintf(low, sizeof(low), "__vckss_ctr_%s_%s_lo", phase, suffix[index]) < 0) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: could not construct a Counter receipt scalar name",
                VCKSS_STATA_MEMORY_ERROR
            );
        }
        status = vckss_save_u64_parts(high, low, value[index]);
        if (status != 0) return status;
    }
    return 0;
}

static int vckss_export_batch_phase(
    const char *phase,
    const VckssBatchPhasePlanReceiptV1 *receipt
)
{
    static const char *suffix[13] = {
        "mode", "reason", "app", "req", "sel", "probe", "threads", "threadcap",
        "routecap", "effcap", "hard", "onebytes", "selbytes"
    };
    const uint64_t value[13] = {
        receipt->request_mode,
        receipt->selection_reason,
        receipt->applicability,
        receipt->requested_width,
        receipt->selected_width,
        receipt->probe_width_cap,
        receipt->declared_threads,
        receipt->thread_width_cap,
        receipt->route_width_cap,
        receipt->effective_width_cap,
        receipt->hard_memory_bytes,
        receipt->width_one_forecast_bytes,
        receipt->selected_forecast_bytes
    };
    char name[33];
    int index;
    int status;
    for (index = 0; index < 13; ++index) {
        if (snprintf(name, sizeof(name), "__vckss_batch_%s_%s", phase, suffix[index]) < 0) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: could not construct a batch receipt scalar name",
                VCKSS_STATA_MEMORY_ERROR
            );
        }
        status = vckss_save_u64(name, value[index]);
        if (status != 0) return status;
    }
    return 0;
}

static int vckss_export_execution_plan(const VckssExecutionPlanReceiptV1 *plan)
{
    int status;
    if ((status = vckss_save_u64("__vckss_plan_struct", plan->struct_size)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_schema", plan->schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_alg_schema", plan->resolution.algorithm_schema)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_alg_req", plan->resolution.algorithm_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_alg_sel", plan->resolution.algorithm_selected)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_alg_reason", plan->resolution.algorithm_reason)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_eng_schema", plan->resolution.engine_schema)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_eng_req", plan->resolution.engine_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_eng_sel", plan->resolution.engine_selected)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_eng_reason", plan->resolution.engine_reason)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_comp_elig", plan->resolution.compressed_eligibility)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_resolved", plan->resolution.resolved_before_rng)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_eng_fallback", plan->resolution.opportunistic_engine_fallback_allowed)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_complexity", plan->resolution.identified_complexity)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_exact_limit", plan->resolution.exact_limit)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_route_schema", plan->solver.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_route_req", plan->solver.requested_route)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_route_sel", plan->solver.selected_route)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_route_fallback", plan->solver.fallback_used)) != 0 ||
        (status = vckss_save_double("__vckss_plan_route_error", (double)plan->solver.fallback_error)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_full_setup", plan->solver.full_setup_complete)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_fe_setup", plan->solver.fe_setup_complete)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_fe_reuse", plan->solver.fe_hierarchy_reused)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_frozen", plan->solver.plan_frozen_before_rng)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_route_contract", plan->solver.auto_route_contract)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_threads_req", plan->solver.threads_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_threads_used", plan->solver.threads_used)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_parallel", plan->solver.parallel_regions)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_applicability", plan->solver.applicability)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_rhs", plan->solver.planned_rhs)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_auto_firms", plan->solver.auto_firm_threshold)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_auto_rhs", plan->solver.auto_rhs_threshold)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_full_dim", plan->solver.full_solver_dimension)) != 0 ||
        (status = vckss_save_u64("__vckss_plan_fe_dim", plan->solver.fe_solver_dimension)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_schema", plan->batch.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_determ", plan->batch.deterministic)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_invariant", plan->batch.width_invariance_required)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_arithmetic", plan->batch.arithmetic_contract)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_admitted", plan->batch.whole_command_admitted)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_app", plan->batch.applicability)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_nonbatched", plan->batch.non_batched_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_batch_command", plan->batch.selected_command_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_schema", plan->wall.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_model", plan->wall.model_code)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_status", plan->wall.status)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_routing", plan->wall.routing_effect)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_req_app", plan->wall.requested_applicable)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_fcst_app", plan->wall.forecast_applicable)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_adv_app", plan->wall.advisory_applicable)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_margin_app", plan->wall.margin_applicable)) != 0 ||
        (status = vckss_save_double("__vckss_wall_requested", plan->wall.requested_seconds)) != 0 ||
        (status = vckss_save_double("__vckss_wall_forecast", plan->wall.forecast_seconds)) != 0 ||
        (status = vckss_save_double("__vckss_wall_advisory", plan->wall.advisory_seconds)) != 0 ||
        (status = vckss_save_double("__vckss_wall_margin", plan->wall.advisory_margin_fraction)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_prepare", plan->wall.preparation_work)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_setup", plan->wall.engine_setup_work)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_fit", plan->wall.full_fit_work)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_leverage", plan->wall.leverage_work)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_target", plan->wall.target_work)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_export", plan->wall.result_export_work)) != 0 ||
        (status = vckss_save_u64("__vckss_wall_total", plan->wall.total_work)) != 0 ||
        (status = vckss_save_u64("__vckss_ctr_schema", plan->counter.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_ctr_rng", plan->counter.rng_contract)) != 0 ||
        (status = vckss_save_u64("__vckss_ctr_work_app", plan->counter.generator_work_applicable)) != 0 ||
        (status = vckss_save_u64("__vckss_ctr_complete", plan->counter.completed)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_schema", plan->memory.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_app", plan->memory.applicability)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_phase", plan->memory.peak_phase)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_hard", plan->memory.hard_limit_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_prepared", plan->memory.prepared_persistent_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_setup", plan->memory.setup_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_fit", plan->memory.fit_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_correction", plan->memory.correction_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_leverage", plan->memory.leverage_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_target", plan->memory.target_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_result", plan->memory.result_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_nonbatched", plan->memory.non_batched_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_command", plan->memory.command_peak_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_shared_cmg", plan->memory.shared_cmg_persistent_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_control", plan->memory.full_control_block_persistent_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_transient", plan->memory.setup_transient_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_cmg_workspace", plan->memory.cmg_preconditioner_workspace_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_cmg_cells", plan->memory.cmg_aggregated_cell_capacity_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_cmg_groups", plan->memory.cmg_group_index_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_cmg_graph", plan->memory.cmg_hybrid_graph_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_nq", plan->memory.retained_nq_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_mem_q2", plan->memory.retained_q2_bytes)) != 0) {
        return status;
    }
    if ((status = vckss_save_u64_parts("__vckss_plan_app_hi", "__vckss_plan_app_lo", plan->applicability_flags)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_contract_hi", "__vckss_plan_contract_lo", plan->contract_flags)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_sig_hi", "__vckss_plan_sig_lo", plan->request_signature)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_res_rng_hi", "__vckss_plan_res_rng_lo", plan->resolution.rng_draws_before_resolution)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_res_ctr_hi", "__vckss_plan_res_ctr_lo", plan->resolution.counter_atoms_before_resolution)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_pre_atom_hi", "__vckss_plan_pre_atom_lo", plan->solver.logical_atoms_before_plan_freeze)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_pre_word_hi", "__vckss_plan_pre_word_lo", plan->solver.unique_words_before_plan_freeze)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_plan_pre_trial_hi", "__vckss_plan_pre_trial_lo", plan->solver.physical_trials_before_plan_freeze)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_ctr_pre_atom_hi", "__vckss_ctr_pre_atom_lo", plan->counter.logical_atoms_before_plan_freeze)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_ctr_pre_word_hi", "__vckss_ctr_pre_word_lo", plan->counter.unique_words_before_plan_freeze)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_ctr_pre_trial_hi", "__vckss_ctr_pre_trial_lo", plan->counter.physical_trials_before_plan_freeze)) != 0 ||
        (status = vckss_export_batch_phase("lev", &plan->batch.leverage)) != 0 ||
        (status = vckss_export_batch_phase("tgt", &plan->batch.target)) != 0 ||
        (status = vckss_export_counter_phase("lev", &plan->counter.leverage)) != 0 ||
        (status = vckss_export_counter_phase("tgt", &plan->counter.target)) != 0 ||
        (status = vckss_export_counter_phase("all", &plan->counter.total)) != 0) {
        return status;
    }
    return 0;
}

/* Every later scalar write uses these wrappers. String literals that exceed
 * Stata's identifier limit fail compilation; generated names also pass the
 * runtime guard in vckss_save_double. Macro self-expansion is intentionally
 * suppressed by the C preprocessor while rescanning each replacement. */
#define vckss_save_double(name, value) \
    vckss_save_double(VCKSS_SCALAR_LITERAL(name), (value))
#define vckss_save_u64(name, value) \
    vckss_save_u64(VCKSS_SCALAR_LITERAL(name), (value))

static int vckss_verify_abi(VckssBackendCapabilitiesV1 *output)
{
    VckssBackendCapabilitiesV1 capabilities;
    const uint32_t runtime_abi = vckss_rust_abi_version();
    const char *runtime_build_id = vckss_rust_backend_version();
    int status;

    memset(&capabilities, 0, sizeof(capabilities));
    status = vckss_rust_backend_capabilities_v1(
        &capabilities, (uint32_t)sizeof(capabilities)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (runtime_build_id == NULL ||
        strcmp(runtime_build_id, VCKSS_RUST_RUNTIME_BUILD_ID) != 0 ||
        runtime_abi != VCKSS_RUST_ABI_VERSION_V1 ||
        capabilities.abi_version != VCKSS_RUST_ABI_VERSION_V1 ||
        capabilities.struct_size != sizeof(capabilities) ||
        capabilities.reserved != 0) {
        vckss_store_error_transport(
            10,
            "ABI_MISMATCH",
            "ABI_MISMATCH [stata_spi]: Rust plugin build identity or ABI does not match the compiled Stata shim"
        );
        SF_error("Rust plugin build identity or ABI does not match the compiled Stata shim");
        return 498;
    }
    if (output != NULL) {
        *output = capabilities;
    }
    return 0;
}

static int vckss_probe(void)
{
    VckssBackendCapabilitiesV1 capabilities;
    int status = vckss_verify_abi(&capabilities);

    if (status != 0) {
        return status;
    }
    if ((status = vckss_save_u64("__vckss_rust_abi_compiled", VCKSS_RUST_ABI_VERSION_V1)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_abi_runtime", capabilities.abi_version)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_core_flags", capabilities.core_ready_flags)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_support_flags", capabilities.support_flags)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_deterministic", capabilities.deterministic_parallelism)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_parse_u64(const char *text, uint64_t *output)
{
    char *end = NULL;
    unsigned long long value;

    if (text == NULL || output == NULL || *text == '\0' || *text == '-') {
        return VCKSS_STATA_USAGE_ERROR;
    }
    errno = 0;
    value = strtoull(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0') {
        return VCKSS_STATA_USAGE_ERROR;
    }
    *output = (uint64_t)value;
    return 0;
}

static int vckss_parse_u32(const char *text, uint32_t *output)
{
    uint64_t value;
    if (vckss_parse_u64(text, &value) != 0 || value > UINT32_MAX) {
        return VCKSS_STATA_USAGE_ERROR;
    }
    *output = (uint32_t)value;
    return 0;
}

static int vckss_parse_double(const char *text, double *output)
{
    char *end = NULL;
    double value;

    if (text == NULL || output == NULL || *text == '\0') {
        return VCKSS_STATA_USAGE_ERROR;
    }
    errno = 0;
    value = strtod(text, &end);
    if (errno != 0 || end == text || *end != '\0') {
        return VCKSS_STATA_USAGE_ERROR;
    }
    *output = value;
    return 0;
}

static int vckss_parse_route(const char *text, uint32_t *output)
{
    if (strcmp(text, "auto") == 0) {
        *output = VCKSS_ROUTE_AUTO;
    } else if (strcmp(text, "exact") == 0) {
        *output = VCKSS_ROUTE_EXACT;
    } else if (strcmp(text, "diagonal") == 0) {
        *output = VCKSS_ROUTE_DIAGONAL_PCG;
    } else if (strcmp(text, "cmg") == 0) {
        *output = VCKSS_ROUTE_CMG_PCG;
    } else {
        return VCKSS_STATA_USAGE_ERROR;
    }
    return 0;
}

static int vckss_parse_algorithm(const char *text, uint32_t *output)
{
    if (strcmp(text, "auto") == 0) {
        *output = VCKSS_ALGORITHM_AUTO;
    } else if (strcmp(text, "exact") == 0) {
        *output = VCKSS_ALGORITHM_EXACT;
    } else if (strcmp(text, "jla") == 0) {
        *output = VCKSS_ALGORITHM_JLA;
    } else {
        return VCKSS_STATA_USAGE_ERROR;
    }
    return 0;
}

static int vckss_parse_deletion(const char *text, uint32_t *output)
{
    if (strcmp(text, "match") == 0) {
        *output = VCKSS_DELETION_MATCH;
    } else if (strcmp(text, "observation") == 0) {
        *output = VCKSS_DELETION_OBSERVATION;
    } else {
        return VCKSS_STATA_USAGE_ERROR;
    }
    return 0;
}

static int vckss_parse_nuisance(const char *text, uint32_t *output)
{
    if (strcmp(text, "joint") == 0) {
        *output = VCKSS_NUISANCE_JOINT;
    } else if (strcmp(text, "fixedoffset") == 0) {
        *output = VCKSS_NUISANCE_FIXED_OFFSET;
    } else {
        return VCKSS_STATA_USAGE_ERROR;
    }
    return 0;
}

static int vckss_parse_rng_contract(const char *text, uint32_t *output)
{
    if (strcmp(text, "none") == 0) {
        *output = VCKSS_RNG_NONE;
    } else if (strcmp(text, "counter_v1") == 0) {
        *output = VCKSS_RNG_COUNTER_V1;
    } else {
        return VCKSS_STATA_USAGE_ERROR;
    }
    return 0;
}

static int vckss_parse_frequency_use(const char *text, uint32_t *output)
{
    if (strcmp(text, "unit") == 0) {
        *output = VCKSS_REQUEST_FREQUENCY_UNIT;
    } else if (strcmp(text, "literal") == 0) {
        *output = VCKSS_REQUEST_FREQUENCY_LITERAL;
    } else {
        return VCKSS_STATA_USAGE_ERROR;
    }
    return 0;
}

static int vckss_parse_engine(const char *text, uint32_t *output)
{
    if (strcmp(text, "auto") == 0) *output = VCKSS_ENGINE_AUTO_OR_UNSPECIFIED;
    else if (strcmp(text, "compressed") == 0) *output = VCKSS_ENGINE_COMPRESSED;
    else if (strcmp(text, "generic") == 0) *output = VCKSS_ENGINE_GENERIC;
    else return VCKSS_STATA_USAGE_ERROR;
    return 0;
}

static int vckss_parse_batch_mode(const char *text, uint32_t *output)
{
    if (strcmp(text, "auto") == 0) *output = VCKSS_BATCH_MODE_AUTO;
    else if (strcmp(text, "explicit") == 0) *output = VCKSS_BATCH_MODE_EXPLICIT;
    else if (strcmp(text, "independent") == 0) *output = VCKSS_BATCH_MODE_INDEPENDENT;
    else return VCKSS_STATA_USAGE_ERROR;
    return 0;
}

static int vckss_parse_stayers_mode(const char *text, uint32_t *output)
{
    if (strcmp(text, "movers") == 0) *output = VCKSS_STAYERS_MOVERS;
    else if (strcmp(text, "all") == 0) *output = VCKSS_STAYERS_ALL;
    else return VCKSS_STATA_USAGE_ERROR;
    return 0;
}

static int vckss_parse_target_weight_mode(const char *text, uint32_t *output)
{
    if (strcmp(text, "frequency") == 0) *output = VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT;
    else if (strcmp(text, "explicit") == 0) *output = VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT;
    else return VCKSS_STATA_USAGE_ERROR;
    return 0;
}

static int vckss_parse_deletion_source(const char *text, uint32_t *output)
{
    if (strcmp(text, "cell") == 0) *output = VCKSS_DELETION_SOURCE_CELL_DEFAULT;
    else if (strcmp(text, "matchid") == 0) *output = VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT;
    else if (strcmp(text, "observation") == 0) *output = VCKSS_DELETION_SOURCE_OBSERVATION_ROW;
    else return VCKSS_STATA_USAGE_ERROR;
    return 0;
}

static int vckss_parse_projection_effect(const char *text, uint32_t *output)
{
    if (strcmp(text, "worker") == 0) {
        *output = VCKSS_PROJECTION_EFFECT_WORKER;
        return 0;
    }
    if (strcmp(text, "firm") == 0) {
        *output = VCKSS_PROJECTION_EFFECT_FIRM;
        return 0;
    }
    return -1;
}

static int vckss_parse_projection_weight(const char *text, uint32_t *output)
{
    if (strcmp(text, "frequency") == 0) {
        *output = VCKSS_PROJECTION_WEIGHT_FREQUENCY;
        return 0;
    }
    if (strcmp(text, "target") == 0) {
        *output = VCKSS_PROJECTION_WEIGHT_TARGET;
        return 0;
    }
    return -1;
}

static int vckss_parse_component_variance(const char *text, uint32_t *output)
{
    if (strcmp(text, "structured_common") == 0) {
        *output = VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON;
        return 0;
    }
    if (strcmp(text, "structured_leverage") == 0) {
        *output = VCKSS_COMPONENT_VARIANCE_STRUCTURED_LEVERAGE;
        return 0;
    }
    return -1;
}

static int vckss_parse_component_reference(const char *text, uint32_t *output)
{
    if (strcmp(text, "q0") == 0) {
        *output = VCKSS_COMPONENT_REFERENCE_Q0;
        return 0;
    }
    if (strcmp(text, "q1") == 0) {
        *output = VCKSS_COMPONENT_REFERENCE_Q1;
        return 0;
    }
    return -1;
}

static int vckss_request_capability(int argc, char *argv[])
{
    VckssBackendRequestCapabilityRequestV1 request;
    VckssBackendRequestCapabilityReceiptV1 receipt;
    int status;

    if (argc == 20) {
        VckssBackendRequestCapabilityRequestV3 request_v3;
        VckssBackendRequestCapabilityReceiptV3 receipt_v3;
        memset(&request_v3, 0, sizeof(request_v3));
        memset(&receipt_v3, 0, sizeof(receipt_v3));
        request_v3.v2.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
        request_v3.v2.v1.struct_size = (uint32_t)sizeof(request_v3);
        request_v3.v2.v1.request_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V3;
        if (vckss_parse_algorithm(argv[1], &request_v3.v2.v1.algorithm) != 0 ||
            vckss_parse_deletion(argv[2], &request_v3.v2.v1.deletion_mode) != 0 ||
            vckss_parse_nuisance(argv[3], &request_v3.v2.v1.nuisance_mode) != 0 ||
            vckss_parse_route(argv[4], &request_v3.v2.v1.solver_route) != 0 ||
            vckss_parse_rng_contract(argv[5], &request_v3.v2.v1.rng_contract) != 0 ||
            vckss_parse_u32(argv[6], &request_v3.v2.v1.controls_count) != 0 ||
            vckss_parse_frequency_use(argv[7], &request_v3.v2.v1.frequency_use) != 0 ||
            vckss_parse_engine(argv[8], &request_v3.v2.engine) != 0 ||
            vckss_parse_batch_mode(argv[9], &request_v3.v2.batch_mode) != 0 ||
            vckss_parse_stayers_mode(argv[10], &request_v3.v2.stayers_mode) != 0 ||
            vckss_parse_target_weight_mode(argv[11], &request_v3.v2.target_weight_mode) != 0 ||
            vckss_parse_deletion_source(argv[12], &request_v3.v2.deletion_unit_source) != 0 ||
            vckss_parse_u32(argv[13], &request_v3.v2.probeorder_supplied) != 0 ||
            vckss_parse_u32(argv[14], &request_v3.v2.wallseconds_supplied) != 0 ||
            vckss_parse_u64(argv[15], &request_v3.v2.physical_limit) != 0 ||
            vckss_parse_batch_mode(argv[16], &request_v3.leverage_batch_mode) != 0 ||
            vckss_parse_batch_mode(argv[17], &request_v3.target_batch_mode) != 0 ||
            vckss_parse_u32(argv[18], &request_v3.allow_automatic_cmg_setup_fallback) != 0 ||
            vckss_parse_double(argv[19], &request_v3.wallseconds) != 0) {
            return vckss_usage("invalid Rust V3 request-capability tuple");
        }
        status = vckss_rust_backend_request_capability_v3(
            &request_v3, &receipt_v3, (uint32_t)sizeof(receipt_v3)
        );
        if (status != 0) return vckss_rust_failure(status);
        if (receipt_v3.v2.v1.struct_size != sizeof(receipt_v3) ||
            receipt_v3.v2.v1.abi_version != VCKSS_RUST_ABI_VERSION_V1 ||
            receipt_v3.v2.v1.request_schema != request_v3.v2.v1.request_schema ||
            receipt_v3.v2.v1.algorithm != request_v3.v2.v1.algorithm ||
            receipt_v3.v2.v1.deletion_mode != request_v3.v2.v1.deletion_mode ||
            receipt_v3.v2.v1.nuisance_mode != request_v3.v2.v1.nuisance_mode ||
            receipt_v3.v2.v1.solver_route != request_v3.v2.v1.solver_route ||
            receipt_v3.v2.v1.rng_contract != request_v3.v2.v1.rng_contract ||
            receipt_v3.v2.v1.controls_count != request_v3.v2.v1.controls_count ||
            receipt_v3.v2.v1.frequency_use != request_v3.v2.v1.frequency_use ||
            receipt_v3.v2.engine != request_v3.v2.engine ||
            receipt_v3.v2.batch_mode != request_v3.v2.batch_mode ||
            receipt_v3.v2.stayers_mode != request_v3.v2.stayers_mode ||
            receipt_v3.v2.target_weight_mode != request_v3.v2.target_weight_mode ||
            receipt_v3.v2.deletion_unit_source != request_v3.v2.deletion_unit_source ||
            receipt_v3.v2.probeorder_supplied != request_v3.v2.probeorder_supplied ||
            receipt_v3.v2.wallseconds_supplied != request_v3.v2.wallseconds_supplied ||
            receipt_v3.v2.physical_limit != request_v3.v2.physical_limit ||
            receipt_v3.leverage_batch_mode != request_v3.leverage_batch_mode ||
            receipt_v3.target_batch_mode != request_v3.target_batch_mode ||
            receipt_v3.allow_automatic_cmg_setup_fallback !=
                request_v3.allow_automatic_cmg_setup_fallback ||
            receipt_v3.wallseconds != request_v3.wallseconds ||
            receipt_v3.v2.v1.reserved != 0 || receipt_v3.v2.reserved_2 != 0 ||
            receipt_v3.reserved_3 != 0 || receipt_v3.reserved_4 != 0 ||
            receipt_v3.v2.v1.supported > 1 || receipt_v3.wall_advisory_only != 1 ||
            (receipt_v3.v2.v1.supported == 1 &&
             receipt_v3.v2.v1.reason_code != VCKSS_REQUEST_REASON_SUPPORTED) ||
            (receipt_v3.v2.v1.supported == 0 &&
             receipt_v3.v2.v1.reason_code == VCKSS_REQUEST_REASON_SUPPORTED)) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust V3 request-capability receipt did not reconcile with the submitted tuple",
                498
            );
        }
        if ((status = vckss_save_u64("__vckss_rust_cap_struct", receipt_v3.v2.v1.struct_size)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_abi", receipt_v3.v2.v1.abi_version)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_schema", receipt_v3.v2.v1.request_schema)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_supported", receipt_v3.v2.v1.supported)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_reason", receipt_v3.v2.v1.reason_code)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_profile", receipt_v3.v2.v1.profile_code)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_algorithm", receipt_v3.v2.v1.algorithm)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_deletion", receipt_v3.v2.v1.deletion_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_nuisance", receipt_v3.v2.v1.nuisance_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_route", receipt_v3.v2.v1.solver_route)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_rng", receipt_v3.v2.v1.rng_contract)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_controls", receipt_v3.v2.v1.controls_count)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_frequency", receipt_v3.v2.v1.frequency_use)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_signature_hi", receipt_v3.v2.v1.request_signature >> 32)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_signature_lo", receipt_v3.v2.v1.request_signature & UINT64_C(0xffffffff))) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_engine", receipt_v3.v2.engine)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_batch", receipt_v3.v2.batch_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_stayers", receipt_v3.v2.stayers_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_target", receipt_v3.v2.target_weight_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_delsource", receipt_v3.v2.deletion_unit_source)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_probeorder", receipt_v3.v2.probeorder_supplied)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_wall", receipt_v3.v2.wallseconds_supplied)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_physlimit", receipt_v3.v2.physical_limit)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_levmode", receipt_v3.leverage_batch_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_tgtmode", receipt_v3.target_batch_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_autofallback", receipt_v3.allow_automatic_cmg_setup_fallback)) != 0 ||
            (status = vckss_save_double("__vckss_rust_cap_wallseconds", receipt_v3.wallseconds)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_alg_deferred", receipt_v3.algorithm_resolution_deferred)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_eng_deferred", receipt_v3.engine_resolution_deferred)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_route_deferred", receipt_v3.route_resolution_deferred)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_lev_deferred", receipt_v3.leverage_batch_resolution_deferred)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_tgt_deferred", receipt_v3.target_batch_resolution_deferred)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_wall_advisory", receipt_v3.wall_advisory_only)) != 0) {
            return status;
        }
        return 0;
    }

    if (argc == 16) {
        VckssBackendRequestCapabilityRequestV2 request_v2;
        VckssBackendRequestCapabilityReceiptV2 receipt_v2;
        memset(&request_v2, 0, sizeof(request_v2));
        memset(&receipt_v2, 0, sizeof(receipt_v2));
        request_v2.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
        request_v2.v1.struct_size = (uint32_t)sizeof(request_v2);
        request_v2.v1.request_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V2;
        if (vckss_parse_algorithm(argv[1], &request_v2.v1.algorithm) != 0 ||
            vckss_parse_deletion(argv[2], &request_v2.v1.deletion_mode) != 0 ||
            vckss_parse_nuisance(argv[3], &request_v2.v1.nuisance_mode) != 0 ||
            vckss_parse_route(argv[4], &request_v2.v1.solver_route) != 0 ||
            vckss_parse_rng_contract(argv[5], &request_v2.v1.rng_contract) != 0 ||
            vckss_parse_u32(argv[6], &request_v2.v1.controls_count) != 0 ||
            vckss_parse_frequency_use(argv[7], &request_v2.v1.frequency_use) != 0 ||
            vckss_parse_engine(argv[8], &request_v2.engine) != 0 ||
            vckss_parse_batch_mode(argv[9], &request_v2.batch_mode) != 0 ||
            vckss_parse_stayers_mode(argv[10], &request_v2.stayers_mode) != 0 ||
            vckss_parse_target_weight_mode(argv[11], &request_v2.target_weight_mode) != 0 ||
            vckss_parse_deletion_source(argv[12], &request_v2.deletion_unit_source) != 0 ||
            vckss_parse_u32(argv[13], &request_v2.probeorder_supplied) != 0 ||
            vckss_parse_u32(argv[14], &request_v2.wallseconds_supplied) != 0 ||
            vckss_parse_u64(argv[15], &request_v2.physical_limit) != 0) {
            return vckss_usage("invalid Rust V2 request-capability tuple");
        }
        status = vckss_rust_backend_request_capability_v2(
            &request_v2, &receipt_v2, (uint32_t)sizeof(receipt_v2)
        );
        if (status != 0) return vckss_rust_failure(status);
        if (receipt_v2.v1.struct_size != sizeof(receipt_v2) ||
            receipt_v2.v1.abi_version != VCKSS_RUST_ABI_VERSION_V1 ||
            receipt_v2.v1.request_schema != request_v2.v1.request_schema ||
            receipt_v2.v1.algorithm != request_v2.v1.algorithm ||
            receipt_v2.v1.deletion_mode != request_v2.v1.deletion_mode ||
            receipt_v2.v1.nuisance_mode != request_v2.v1.nuisance_mode ||
            receipt_v2.v1.solver_route != request_v2.v1.solver_route ||
            receipt_v2.v1.rng_contract != request_v2.v1.rng_contract ||
            receipt_v2.v1.controls_count != request_v2.v1.controls_count ||
            receipt_v2.v1.frequency_use != request_v2.v1.frequency_use ||
            receipt_v2.engine != request_v2.engine ||
            receipt_v2.batch_mode != request_v2.batch_mode ||
            receipt_v2.stayers_mode != request_v2.stayers_mode ||
            receipt_v2.target_weight_mode != request_v2.target_weight_mode ||
            receipt_v2.deletion_unit_source != request_v2.deletion_unit_source ||
            receipt_v2.probeorder_supplied != request_v2.probeorder_supplied ||
            receipt_v2.wallseconds_supplied != request_v2.wallseconds_supplied ||
            receipt_v2.physical_limit != request_v2.physical_limit ||
            receipt_v2.v1.reserved != 0 || receipt_v2.reserved_2 != 0 ||
            receipt_v2.v1.supported > 1 ||
            (receipt_v2.v1.supported == 1 && receipt_v2.v1.reason_code != VCKSS_REQUEST_REASON_SUPPORTED) ||
            (receipt_v2.v1.supported == 0 && receipt_v2.v1.reason_code == VCKSS_REQUEST_REASON_SUPPORTED)) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust V2 request-capability receipt did not reconcile with the submitted tuple",
                498
            );
        }
        if ((status = vckss_save_u64("__vckss_rust_cap_struct", receipt_v2.v1.struct_size)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_abi", receipt_v2.v1.abi_version)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_schema", receipt_v2.v1.request_schema)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_supported", receipt_v2.v1.supported)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_reason", receipt_v2.v1.reason_code)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_profile", receipt_v2.v1.profile_code)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_algorithm", receipt_v2.v1.algorithm)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_deletion", receipt_v2.v1.deletion_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_nuisance", receipt_v2.v1.nuisance_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_route", receipt_v2.v1.solver_route)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_rng", receipt_v2.v1.rng_contract)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_controls", receipt_v2.v1.controls_count)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_frequency", receipt_v2.v1.frequency_use)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_signature_hi", receipt_v2.v1.request_signature >> 32)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_signature_lo", receipt_v2.v1.request_signature & UINT64_C(0xffffffff))) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_engine", receipt_v2.engine)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_batch", receipt_v2.batch_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_stayers", receipt_v2.stayers_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_target", receipt_v2.target_weight_mode)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_delsource", receipt_v2.deletion_unit_source)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_probeorder", receipt_v2.probeorder_supplied)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_wall", receipt_v2.wallseconds_supplied)) != 0 ||
            (status = vckss_save_u64("__vckss_rust_cap_physlimit", receipt_v2.physical_limit)) != 0) {
            return status;
        }
        return 0;
    }

    if (argc != 8) {
        return vckss_usage(
            "Rust requestcapability requires algorithm, deletion, nuisance, route, RNG contract, control count, and frequency use"
        );
    }
    memset(&request, 0, sizeof(request));
    memset(&receipt, 0, sizeof(receipt));
    request.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.struct_size = (uint32_t)sizeof(request);
    request.request_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V1;
    if (vckss_parse_algorithm(argv[1], &request.algorithm) != 0) {
        return vckss_usage("invalid Rust request-capability algorithm");
    }
    if (vckss_parse_deletion(argv[2], &request.deletion_mode) != 0) {
        return vckss_usage("invalid Rust request-capability deletion mode");
    }
    if (vckss_parse_nuisance(argv[3], &request.nuisance_mode) != 0) {
        return vckss_usage("invalid Rust request-capability nuisance mode");
    }
    if (vckss_parse_route(argv[4], &request.solver_route) != 0) {
        return vckss_usage("invalid Rust request-capability solver route");
    }
    if (vckss_parse_rng_contract(argv[5], &request.rng_contract) != 0) {
        return vckss_usage("invalid Rust request-capability RNG contract");
    }
    if (vckss_parse_u32(argv[6], &request.controls_count) != 0) {
        return vckss_usage("invalid Rust request-capability control count");
    }
    if (vckss_parse_frequency_use(argv[7], &request.frequency_use) != 0) {
        return vckss_usage("invalid Rust request-capability frequency use");
    }
    status = vckss_rust_backend_request_capability_v1(
        &request, &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.abi_version != VCKSS_RUST_ABI_VERSION_V1 ||
        receipt.request_schema != request.request_schema ||
        receipt.algorithm != request.algorithm ||
        receipt.deletion_mode != request.deletion_mode ||
        receipt.nuisance_mode != request.nuisance_mode ||
        receipt.solver_route != request.solver_route ||
        receipt.rng_contract != request.rng_contract ||
        receipt.controls_count != request.controls_count ||
        receipt.frequency_use != request.frequency_use ||
        receipt.reserved != 0 || receipt.supported > 1 ||
        (receipt.supported == 1 && receipt.reason_code != VCKSS_REQUEST_REASON_SUPPORTED) ||
        (receipt.supported == 0 && receipt.reason_code == VCKSS_REQUEST_REASON_SUPPORTED)) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust request-capability receipt did not reconcile with the submitted tuple",
            498
        );
    }
    if ((status = vckss_save_u64("__vckss_rust_cap_struct", receipt.struct_size)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_abi", receipt.abi_version)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_schema", receipt.request_schema)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_supported", receipt.supported)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_reason", receipt.reason_code)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_profile", receipt.profile_code)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_algorithm", receipt.algorithm)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_deletion", receipt.deletion_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_nuisance", receipt.nuisance_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_route", receipt.solver_route)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_rng", receipt.rng_contract)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_controls", receipt.controls_count)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_frequency", receipt.frequency_use)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_signature_hi", receipt.request_signature >> 32)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_signature_lo", receipt.request_signature & UINT64_C(0xffffffff))) != 0) {
        return status;
    }
    return 0;
}

static int vckss_count_selected_observations(uint64_t *selected, int allow_empty)
{
    ST_int observation;
    uint64_t count = 0;
    ST_double touse = 0.0;
    uint64_t visited = 0;

    for (observation = SF_in1(); observation <= SF_in2(); ++observation) {
        if (visited % VCKSS_INGEST_POLL_INTERVAL == 0 &&
            vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
            return 1;
        }
        ++visited;
        if (!SF_ifobs(observation)) {
            continue;
        }
        if (SF_vdata(1, observation, &touse) != 0) {
            return vckss_usage("could not read the marked-sample variable");
        }
        if (touse != 0.0) {
            if (count == UINT64_MAX) {
                return vckss_usage("marked-sample row count overflow");
            }
            ++count;
        }
    }
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        return 1;
    }
    if (count == 0 && allow_empty == 0) {
        return vckss_usage("the marked sample is empty");
    }
    *selected = count;
    return 0;
}

static int vckss_selected_observations(uint64_t *selected)
{
    return vckss_count_selected_observations(selected, 0);
}

static int vckss_copy_marked_columns(
    uint64_t rows,
    uint32_t numeric_columns,
    double **storage
)
{
    ST_int observation;
    uint64_t row = 0;
    ST_double value = 0.0;
    double *buffer;
    size_t entries;
    uint32_t variable;
    uint64_t visited = 0;

    if (numeric_columns == 0 ||
        rows > (uint64_t)SIZE_MAX) {
        return vckss_usage("marked-sample column allocation overflow");
    }
#if SIZE_MAX <= UINT32_MAX
    if ((size_t)numeric_columns > SIZE_MAX / sizeof(double)) {
        return vckss_usage("marked-sample column allocation overflow");
    }
#endif
    if ((size_t)rows >
        SIZE_MAX / ((size_t)numeric_columns * sizeof(double))) {
        return vckss_usage("marked-sample column allocation overflow");
    }
    entries = (size_t)rows * (size_t)numeric_columns;
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        return 1;
    }
    buffer = (double *)vckss_calloc(entries, sizeof(double));
    if (buffer == NULL) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate marked-sample Rust input columns",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        free(buffer);
        return 1;
    }

    for (observation = SF_in1(); observation <= SF_in2(); ++observation) {
        if (visited % VCKSS_INGEST_POLL_INTERVAL == 0 &&
            vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
            free(buffer);
            return 1;
        }
        ++visited;
        if (!SF_ifobs(observation)) {
            continue;
        }
        if (SF_vdata(1, observation, &value) != 0) {
            free(buffer);
            return vckss_usage("could not read the marked-sample variable");
        }
        if (value == 0.0) {
            continue;
        }
        for (variable = 0; variable < numeric_columns; ++variable) {
            if (SF_vdata((ST_int)variable + 2, observation, &value) != 0) {
                free(buffer);
                return vckss_usage("could not read a marked-sample numeric column");
            }
            buffer[(size_t)variable * (size_t)rows + (size_t)row] = value;
        }
        ++row;
    }
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        free(buffer);
        return 1;
    }
    if (row != rows) {
        free(buffer);
        return vckss_usage("marked-sample count changed during ingestion");
    }
    *storage = buffer;
    return 0;
}

static int vckss_export_preparation(
    uint64_t generation,
    uint64_t rows,
    ST_int retained_variable
)
{
    VckssEnginePreparationReceiptV4 receipt_v4;
    VckssEnginePreparationReceiptV3 receipt_v3;
    VckssEnginePreparationReceiptV2 receipt;
    uint8_t *mask;
    ST_int observation;
    ST_double touse;
    uint64_t row = 0;
    int status;
    uint64_t visited = 0;

    memset(&receipt_v4, 0, sizeof(receipt_v4));
    status = vckss_rust_engine_preparation_receipt_v4(
        generation, &receipt_v4, (uint32_t)sizeof(receipt_v4)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    receipt_v3 = receipt_v4.v3;
    receipt = receipt_v3.v2;
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        return 1;
    }
    mask = (uint8_t *)vckss_calloc((size_t)rows, sizeof(uint8_t));
    if (mask == NULL) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate the Rust retained-sample mask",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        free(mask);
        return 1;
    }
    status = vckss_rust_engine_retained_mask_v1(generation, mask, rows);
    if (status != 0) {
        free(mask);
        return vckss_rust_failure(status);
    }
    for (observation = SF_in1(); observation <= SF_in2(); ++observation) {
        if (visited % VCKSS_INGEST_POLL_INTERVAL == 0 &&
            vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
            free(mask);
            return 1;
        }
        ++visited;
        if (!SF_ifobs(observation)) {
            continue;
        }
        if (SF_vdata(1, observation, &touse) != 0) {
            free(mask);
            return vckss_usage("could not reread the marked-sample variable");
        }
        if (touse == 0.0) {
            continue;
        }
        if (SF_vstore(retained_variable, observation, (ST_double)mask[row]) != 0) {
            free(mask);
            return vckss_usage("could not store the Rust retained-sample mask");
        }
        ++row;
    }
    if (vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK) {
        free(mask);
        return 1;
    }
    free(mask);
    if (row != rows) {
        return vckss_usage("retained-mask row alignment failed");
    }

    if ((status = vckss_save_u64("__vckss_rust_handle", receipt.generation)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_input_rows", receipt.input_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_retained_rows", receipt.retained_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_workers", receipt.workers)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_firms", receipt.firms)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cells", receipt.cells)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_deletion_units", receipt.deletion_units)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_target_strata", receipt.target_strata)) != 0 ||
        (status = vckss_save_double("__vckss_rust_target_sum", receipt_v3.target_weight_sum)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_controls_count", receipt_v4.controls_count)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_prep_deletion", receipt_v4.deletion_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_memory_limit", receipt.memory_limit_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_caller_copy", receipt.caller_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_prepare_peak", receipt.preparation_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_prepared_resident", receipt.prepared_resident_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_input_rows", receipt.graph_input_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_retained_rows", receipt.graph_retained_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_input_mass", receipt.graph_input_physical_mass)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_retained_mass", receipt.graph_retained_physical_mass)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_init_comp", receipt.graph_initial_components)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_max_comp", receipt.graph_maximum_components)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_initial_rows", receipt.graph_initial_component_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_mover_rows", receipt.graph_mover_input_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_initial_edges", receipt.graph_initial_deletion_edges)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_keep_edges", receipt.graph_retained_deletion_edges)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_deg_removed", receipt.graph_insufficient_workers_removed)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_art_removed", receipt.graph_articulation_workers_removed)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_bridge_units", receipt.graph_bridge_units_removed)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_bridge_rows", receipt.graph_bridge_rows_removed)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_deg_iters", receipt.graph_degree_iterations)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_art_iters", receipt.graph_articulation_iterations)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_bridge_iters", receipt.graph_bridge_iterations)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_graph_fixed_iters", receipt.graph_fixed_point_iterations)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_prepare(int argc, char *argv[])
{
    VckssEnginePrepareRequestInterruptV3 request;
    VckssEngineColumnsV3 columns;
    uint64_t rows = 0;
    uint64_t caller_copy_bytes = 0;
    uint64_t generation = 0;
    uint32_t controls_count = 0;
    uint32_t probeorder_supplied = 0;
    uint32_t implicit_match = 0;
    uint32_t numeric_columns = VCKSS_NUMERIC_COLUMNS_BASE;
    uint32_t control;
    double *storage = NULL;
    const double **control_pointers = NULL;
    ST_int retained_variable;
    int status;

    if (argc != 3 && argc != 5 && argc != 6 && argc != 7) {
        return vckss_usage(
            "Rust prepare requires cleanup, memory, and optionally deletion mode, control count, probe-order flag, and implicit-match flag"
        );
    }
    if (strcmp(argv[1], "cleanup") != 0 && strcmp(argv[1], "nocleanup") != 0) {
        return vckss_usage("Rust prepare cleanup flag must be cleanup or nocleanup");
    }
    if (argc >= 5) {
        if (vckss_parse_u32(argv[4], &controls_count) != 0 ||
            controls_count > UINT32_MAX - VCKSS_NUMERIC_COLUMNS_BASE) {
            return vckss_usage("invalid Rust prepare control count");
        }
        numeric_columns = VCKSS_NUMERIC_COLUMNS_BASE + controls_count;
    }
    if (argc >= 6) {
        if (strcmp(argv[5], "probeorder") == 0) {
            probeorder_supplied = 1u;
            if (numeric_columns == UINT32_MAX) {
                return vckss_usage("Rust prepare numeric-column count overflow");
            }
            ++numeric_columns;
        } else if (strcmp(argv[5], "noprobeorder") != 0) {
            return vckss_usage("Rust prepare probe-order flag must be probeorder or noprobeorder");
        }
    }
    if (argc == 7) {
        if (strcmp(argv[6], "implicitmatch") == 0) {
            implicit_match = 1u;
        } else if (strcmp(argv[6], "explicitdeletion") != 0) {
            return vckss_usage(
                "Rust prepare implicit-match flag must be implicitmatch or explicitdeletion"
            );
        }
    }
    if (SF_nvars() != (ST_int)numeric_columns + 2) {
        return vckss_usage(
            "Rust prepare varlist must contain touse, six base inputs, every declared control, and retained output"
        );
    }
    retained_variable = (ST_int)numeric_columns + 2;
    if ((status = vckss_selected_observations(&rows)) != 0) {
        return status;
    }
    if (numeric_columns == 0 ||
        rows > UINT64_MAX / ((uint64_t)numeric_columns * sizeof(double))) {
        return vckss_usage("marked-sample caller-copy byte count overflow");
    }
    caller_copy_bytes = rows * (uint64_t)numeric_columns * sizeof(double);

    status = vckss_rust_engine_default_prepare_request_interrupt_v3(
        &request, (uint32_t)sizeof(request)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    request.options.v3.v2.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.options.v3.v2.rows = rows;
    request.options.v3.v2.cleanup_abandoned = strcmp(argv[1], "cleanup") == 0 ? 1u : 0u;
    if (vckss_parse_u64(argv[2], &request.options.v3.v2.memory_limit_bytes) != 0 ||
        request.options.v3.v2.memory_limit_bytes == 0) {
        return vckss_usage("invalid Rust whole-command byte memory limit");
    }
    request.options.v3.v2.caller_copy_bytes = caller_copy_bytes;
    request.options.v3.controls_count = controls_count;
    request.options.implicit_match = implicit_match;
    if (argc >= 5 && vckss_parse_deletion(argv[3], &request.options.v3.deletion_mode) != 0) {
        return vckss_usage("invalid Rust deletion mode");
    }
    request.interrupt_poll = vckss_stata_interrupt_poll;
    request.interrupt_context = NULL;
    request.checkpoint_interval = 1u;
    status = vckss_rust_engine_admit_prepare_v4(
        &request.options, probeorder_supplied
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (request.options.v3.v2.cleanup_abandoned == 1u) {
        status = vckss_rust_engine_clear_abandoned_v1();
        if (status != 0) {
            return vckss_rust_failure(status);
        }
        request.options.v3.v2.cleanup_abandoned = 0u;
    }
    status = vckss_copy_marked_columns(rows, numeric_columns, &storage);
    if (status != 0) {
        return status;
    }
    if (controls_count > 0) {
        control_pointers = (const double **)vckss_calloc(
            (size_t)controls_count, sizeof(*control_pointers)
        );
        if (control_pointers == NULL) {
            free(storage);
            return vckss_c_failure(
                VCKSS_ERROR_ALLOCATION_FAILED,
                "ALLOCATION_FAILED",
                "ALLOCATION_FAILED [stata_spi]: could not allocate Rust control-column pointers",
                VCKSS_STATA_MEMORY_ERROR
            );
        }
        for (control = 0; control < controls_count; ++control) {
            control_pointers[control] =
                storage + ((size_t)VCKSS_NUMERIC_COLUMNS_BASE + control) * (size_t)rows;
        }
    }

    memset(&columns, 0, sizeof(columns));
    columns.v2.v1.struct_size = (uint32_t)sizeof(columns);
    columns.v2.v1.rows = rows;
    columns.v2.v1.worker = storage;
    columns.v2.v1.firm = storage + rows;
    columns.v2.v1.deletion = storage + 2 * rows;
    columns.v2.v1.outcome = storage + 3 * rows;
    columns.v2.v1.frequency = storage + 4 * rows;
    columns.v2.v1.target_weight = storage + 5 * rows;
    columns.v2.controls = control_pointers;
    columns.v2.controls_count = controls_count;
    columns.probeorder_supplied = probeorder_supplied;
    if (probeorder_supplied != 0u) {
        columns.probe_order =
            storage + ((size_t)VCKSS_NUMERIC_COLUMNS_BASE + controls_count) * (size_t)rows;
    }

    status = vckss_rust_engine_prepare_interrupt_v4(
        &request, &columns, &generation, (uint32_t)sizeof(generation)
    );
    free(control_pointers);
    free(storage);
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    status = vckss_export_preparation(generation, rows, retained_variable);
    if (status != 0) {
        vckss_cleanup_preserving_primary(generation);
    }
    return status;
}

static int vckss_export_stayer_augmentation(uint64_t generation)
{
    VckssStayerAugmentationReceiptV1 receipt;
    int status;

    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_stayer_augmentation_receipt_v1(
        generation, &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.schema_version != 1u || receipt.generation != generation ||
        receipt.mover_stored_rows + receipt.stayer_stored_rows !=
            receipt.combined_stored_rows ||
        receipt.mover_physical_mass + receipt.stayer_physical_mass !=
            receipt.combined_physical_mass ||
        receipt.mover_workers + receipt.stayer_workers !=
            receipt.combined_workers ||
        receipt.mover_deletion_units + receipt.stayer_deletion_units !=
            receipt.combined_deletion_units ||
        receipt.augmentation_peak_forecast_bytes > receipt.memory_limit_bytes ||
        receipt.total_prepared_resident_bytes > receipt.memory_limit_bytes ||
        receipt.augmented_resident_bytes > receipt.total_prepared_resident_bytes) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust stayer augmentation receipt did not reconcile",
            498
        );
    }
    if ((status = vckss_save_u64("__vckss_hyb_aug_struct", receipt.struct_size)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_aug_schema", receipt.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_mover_rows", receipt.mover_stored_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_stayer_rows", receipt.stayer_stored_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_total_rows", receipt.combined_stored_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_mover_mass", receipt.mover_physical_mass)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_stayer_mass", receipt.stayer_physical_mass)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_total_mass", receipt.combined_physical_mass)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_mover_workers", receipt.mover_workers)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_stayer_workers", receipt.stayer_workers)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_total_workers", receipt.combined_workers)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_firms", receipt.firms)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_mover_del", receipt.mover_deletion_units)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_stayer_del", receipt.stayer_deletion_units)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_total_del", receipt.combined_deletion_units)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_mover_target", receipt.mover_target_mass)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_stayer_target", receipt.stayer_target_mass)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_total_target", receipt.combined_target_mass)) != 0 ||
        (status = vckss_save_u64_parts("__vckss_hyb_topology_hi", "__vckss_hyb_topology_lo", receipt.topology_checksum)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_mem_limit", receipt.memory_limit_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_caller_copy", receipt.caller_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_aug_peak", receipt.augmentation_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_aug_resident", receipt.augmented_resident_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_prepared_bytes", receipt.total_prepared_resident_bytes)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_augment_stayers(int argc, char *argv[])
{
    VckssStayerAugmentationRequestInterruptV1 request;
    VckssStayerAugmentationColumnsV1 columns;
    uint64_t generation = 0;
    uint64_t rows = 0;
    uint32_t controls_count = 0;
    uint32_t numeric_columns;
    uint32_t control;
    double *storage = NULL;
    const double **control_pointers = NULL;
    int status;

    if (argc != 3 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0 ||
        vckss_parse_u32(argv[2], &controls_count) != 0 ||
        controls_count > UINT32_MAX - VCKSS_STAYER_NUMERIC_COLUMNS_BASE) {
        return vckss_usage(
            "Rust augmentstayers requires a positive generation and a control count"
        );
    }
    numeric_columns = VCKSS_STAYER_NUMERIC_COLUMNS_BASE + controls_count;
    if (SF_nvars() != (ST_int)numeric_columns + 1) {
        return vckss_usage(
            "Rust augmentstayers varlist must contain touse, firm, worker, outcome, frequency, target, and every declared control"
        );
    }
    status = vckss_count_selected_observations(&rows, 1);
    if (status != 0) return status;
    if (numeric_columns == 0 ||
        rows > UINT64_MAX / ((uint64_t)numeric_columns * sizeof(double))) {
        return vckss_usage("stayer caller-copy byte count overflow");
    }
    status = vckss_rust_engine_default_stayer_augmentation_request_interrupt_v1(
        &request, (uint32_t)sizeof(request)
    );
    if (status != 0) return vckss_rust_failure(status);
    request.options.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.options.rows = rows;
    request.options.controls_count = controls_count;
    request.options.caller_copy_bytes =
        rows * (uint64_t)numeric_columns * sizeof(double);
    request.interrupt_poll = vckss_stata_interrupt_poll;
    request.interrupt_context = NULL;
    request.checkpoint_interval = 1u;

    if (rows != 0) {
        status = vckss_copy_marked_columns(rows, numeric_columns, &storage);
        if (status != 0) return status;
        if (controls_count > 0) {
            control_pointers = (const double **)vckss_calloc(
                (size_t)controls_count, sizeof(*control_pointers)
            );
            if (control_pointers == NULL) {
                free(storage);
                return vckss_c_failure(
                    VCKSS_ERROR_ALLOCATION_FAILED,
                    "ALLOCATION_FAILED",
                    "ALLOCATION_FAILED [stata_spi]: could not allocate stayer control-column pointers",
                    VCKSS_STATA_MEMORY_ERROR
                );
            }
            for (control = 0; control < controls_count; ++control) {
                control_pointers[control] = storage +
                    ((size_t)VCKSS_STAYER_NUMERIC_COLUMNS_BASE + control) * (size_t)rows;
            }
        }
    }

    memset(&columns, 0, sizeof(columns));
    columns.struct_size = (uint32_t)sizeof(columns);
    columns.rows = rows;
    columns.controls_count = controls_count;
    if (rows != 0) {
        columns.firm = storage;
        columns.worker = storage + rows;
        columns.outcome = storage + 2 * rows;
        columns.frequency = storage + 3 * rows;
        columns.target_weight = storage + 4 * rows;
        columns.controls = control_pointers;
    }
    status = vckss_rust_engine_augment_stayers_interrupt_v1(
        generation, &request, &columns
    );
    free(control_pointers);
    free(storage);
    if (status != 0) return vckss_rust_failure(status);
    return vckss_export_stayer_augmentation(generation);
}

static int vckss_export_projection_augmentation(uint64_t generation)
{
    VckssProjectionAugmentationReceiptV1 receipt;
    int status;

    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_projection_augmentation_receipt_v1(
        generation, &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) return vckss_rust_failure(status);
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.schema_version != VCKSS_PROJECTION_SCHEMA_V1 ||
        receipt.generation != generation || receipt.rows == 0 ||
        receipt.columns < 2 ||
        (receipt.effect != VCKSS_PROJECTION_EFFECT_WORKER &&
         receipt.effect != VCKSS_PROJECTION_EFFECT_FIRM) ||
        (receipt.weight != VCKSS_PROJECTION_WEIGHT_FREQUENCY &&
         receipt.weight != VCKSS_PROJECTION_WEIGHT_TARGET) ||
        !isfinite(receipt.gram_rcond) || receipt.gram_rcond <= 0.0 ||
        !isfinite(receipt.gram_relres) || receipt.gram_relres < 0.0 ||
        !isfinite(receipt.gram_original_relres) ||
        receipt.gram_original_relres < 0.0 ||
        receipt.total_prepared_resident_bytes < receipt.projection_persistent_bytes) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust projection augmentation receipt did not reconcile",
            498
        );
    }
    if ((status = vckss_save_u64("__vckss_proj_aug_schema", receipt.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_rows", receipt.rows)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_columns", receipt.columns)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_effect", receipt.effect)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_weight", receipt.weight)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_copy", receipt.caller_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_aug_peak", receipt.augmentation_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_persistent", receipt.projection_persistent_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_prepared", receipt.total_prepared_resident_bytes)) != 0 ||
        (status = vckss_save_double("__vckss_proj_gram_rcond", receipt.gram_rcond)) != 0 ||
        (status = vckss_save_double("__vckss_proj_gram_relres", receipt.gram_relres)) != 0 ||
        (status = vckss_save_double("__vckss_proj_gram_orig", receipt.gram_original_relres)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_augment_projection(int argc, char *argv[])
{
    VckssProjectionAugmentationRequestInterruptV1 request;
    VckssProjectionColumnsV1 columns;
    uint64_t generation = 0;
    uint64_t rows = 0;
    uint32_t project_count = 0;
    uint32_t effect = 0;
    uint32_t weight = 0;
    uint32_t project;
    double rank_tolerance = 0.0;
    double *storage = NULL;
    const double **project_pointers = NULL;
    int status;

    if (argc != 6) {
        return vckss_usage(
            "Rust augmentprojection requires exactly five arguments"
        );
    }
    if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
        return vckss_usage("Rust augmentprojection requires a positive generation");
    }
    if (vckss_parse_u32(argv[2], &project_count) != 0 || project_count == 0) {
        return vckss_usage("Rust augmentprojection requires a positive project count");
    }
    if (vckss_parse_projection_effect(argv[3], &effect) != 0) {
        return vckss_usage("Rust augmentprojection effect must be worker or firm");
    }
    if (vckss_parse_projection_weight(argv[4], &weight) != 0) {
        return vckss_usage("Rust augmentprojection weight must be frequency or target");
    }
    if (vckss_parse_double(argv[5], &rank_tolerance) != 0) {
        return vckss_usage("Rust augmentprojection rank tolerance must be numeric");
    }
    if (SF_nvars() != (ST_int)project_count + 1) {
        return vckss_usage(
            "Rust augmentprojection varlist must contain touse followed by every declared project column"
        );
    }
    status = vckss_selected_observations(&rows);
    if (status != 0) return status;
    if (rows > UINT64_MAX / ((uint64_t)project_count * sizeof(double))) {
        return vckss_usage("projection caller-copy byte count overflow");
    }
    status = vckss_rust_engine_default_projection_augmentation_request_interrupt_v1(
        &request, (uint32_t)sizeof(request)
    );
    if (status != 0) return vckss_rust_failure(status);
    request.options.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.options.rows = rows;
    request.options.project_count = project_count;
    request.options.effect = effect;
    request.options.weight = weight;
    request.options.rank_tolerance = rank_tolerance;
    request.options.caller_copy_bytes =
        rows * (uint64_t)project_count * sizeof(double);
    request.interrupt_poll = vckss_stata_interrupt_poll;
    request.interrupt_context = NULL;
    request.checkpoint_interval = 1u;

    status = vckss_copy_marked_columns(rows, project_count, &storage);
    if (status != 0) return status;
    project_pointers = (const double **)vckss_calloc(
        (size_t)project_count, sizeof(*project_pointers)
    );
    if (project_pointers == NULL) {
        free(storage);
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate projection-column pointers",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    for (project = 0; project < project_count; ++project) {
        project_pointers[project] = storage + (size_t)project * (size_t)rows;
    }
    memset(&columns, 0, sizeof(columns));
    columns.struct_size = (uint32_t)sizeof(columns);
    columns.rows = rows;
    columns.project = project_pointers;
    columns.project_count = project_count;
    status = vckss_rust_engine_augment_projection_interrupt_v1(
        generation, &request, &columns
    );
    free(project_pointers);
    free(storage);
    if (status != 0) return vckss_rust_failure(status);
    return vckss_export_projection_augmentation(generation);
}

static int vckss_export_component_inference_augmentation(uint64_t generation)
{
    VckssComponentInferenceAugmentationReceiptV1 receipt;
    int status;

    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_component_inference_augmentation_receipt_v1(
        generation, &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) return vckss_rust_failure(status);
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.schema_version != VCKSS_COMPONENT_INFERENCE_SCHEMA_V1 ||
        receipt.generation != generation || receipt.rows == 0 ||
        (receipt.variance_source != VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON &&
         receipt.variance_source != VCKSS_COMPONENT_VARIANCE_STRUCTURED_LEVERAGE) ||
        (receipt.reference_distribution != VCKSS_COMPONENT_REFERENCE_Q0 &&
         receipt.reference_distribution != VCKSS_COMPONENT_REFERENCE_Q1) ||
        receipt.reserved != 0 ||
        receipt.augmentation_peak_forecast_bytes == 0 ||
        receipt.total_prepared_resident_bytes < receipt.component_persistent_bytes) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust component-inference augmentation receipt did not reconcile",
            498
        );
    }
    if ((status = vckss_save_u64("__vckss_comp_aug_schema", receipt.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_rows", receipt.rows)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_model", receipt.variance_source)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_reference", receipt.reference_distribution)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_aug_peak", receipt.augmentation_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_persistent", receipt.component_persistent_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_prepared", receipt.total_prepared_resident_bytes)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_augment_component_inference(int argc, char *argv[])
{
    VckssComponentInferenceAugmentationRequestInterruptV1 request;
    uint64_t generation = 0;
    int status;

    if (argc != 17) {
        return vckss_usage(
            "Rust augmentcomponent requires generation, model, reference, probe, spectrum, and structured-variance arguments"
        );
    }
    status = vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(
        &request, (uint32_t)sizeof(request)
    );
    if (status != 0) return vckss_rust_failure(status);
    if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0)
        return vckss_usage("Rust augmentcomponent received an invalid generation");
    if (vckss_parse_component_variance(argv[2], &request.options.variance_source) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid variance model");
    if (vckss_parse_component_reference(argv[3], &request.options.reference_distribution) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid reference distribution");
    if (vckss_parse_u32(argv[4], &request.options.probes) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid covariance-probe count");
    if (vckss_parse_u32(argv[5], &request.options.batch_width) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid batch width");
    if (vckss_parse_u32(argv[6], &request.options.spectrum_probes) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid spectrum-probe count");
    if (vckss_parse_u32(argv[7], &request.options.spectrum_iterations) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid spectrum-iteration count");
    if (vckss_parse_u64(argv[8], &request.options.seed) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid inference seed");
    if (vckss_parse_double(argv[9], &request.options.psd_tolerance) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid PSD tolerance");
    if (vckss_parse_double(argv[10], &request.options.spectrum_tolerance) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid spectrum tolerance");
    if (vckss_parse_double(argv[11], &request.options.confidence_level) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid confidence level");
    if (vckss_parse_u32(argv[12], &request.options.critical_simulations) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid critical-value simulation count");
    if (vckss_parse_u32(argv[13], &request.options.observations_per_term) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid observations-per-term value");
    if (vckss_parse_u64(argv[14], &request.options.fold_seed) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid variance-fold seed");
    if (vckss_parse_double(argv[15], &request.options.variance_rank_tolerance) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid variance-rank tolerance");
    if (vckss_parse_double(argv[16], &request.options.positivity_multiplier) != 0)
        return vckss_usage("Rust augmentcomponent received an invalid positivity multiplier");
    request.options.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.interrupt_poll = vckss_stata_interrupt_poll;
    request.interrupt_context = NULL;
    request.checkpoint_interval = 1u;
    status = vckss_rust_engine_augment_component_inference_interrupt_v1(
        generation, &request
    );
    if (status != 0) return vckss_rust_failure(status);
    return vckss_export_component_inference_augmentation(generation);
}

static int vckss_solve(int argc, char *argv[])
{
    uint64_t generation;
    int status;

    if (argc == 9) {
        VckssEngineSolveRequestInterruptV1 request;
        status = vckss_rust_engine_default_solve_request_interrupt_v1(
            &request, (uint32_t)sizeof(request)
        );
        if (status != 0) {
            return vckss_rust_failure(status);
        }
        request.options.abi_version = VCKSS_RUST_ABI_VERSION_V1;
        request.interrupt_poll = vckss_stata_interrupt_poll;
        request.interrupt_context = NULL;
        request.checkpoint_interval = 1u;
        if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("invalid Rust solve generation");
        }
        if (vckss_parse_u64(argv[2], &request.options.seed) != 0) {
            return vckss_usage("invalid Rust solve seed");
        }
        if (vckss_parse_u32(argv[3], &request.options.probes) != 0) {
            return vckss_usage("invalid Rust solve probe count");
        }
        if (vckss_parse_u32(argv[4], &request.options.leverage_batch_width) != 0) {
            return vckss_usage("invalid Rust leverage batch width");
        }
        if (vckss_parse_u32(argv[5], &request.options.target_batch_width) != 0) {
            return vckss_usage("invalid Rust target batch width");
        }
        if (vckss_parse_route(argv[6], &request.options.solver_route) != 0) {
            return vckss_usage("invalid Rust solver route");
        }
        if (vckss_parse_double(argv[7], &request.options.pcg_tolerance) != 0) {
            return vckss_usage("invalid Rust PCG tolerance");
        }
        if (vckss_parse_u32(argv[8], &request.options.maximum_iterations) != 0) {
            return vckss_usage("invalid Rust maximum iteration count");
        }
        status = vckss_rust_engine_solve_interrupt_v1(generation, &request);
        return status == 0 ? 0 : vckss_rust_failure(status);
    }
    if (argc == 33) {
        VckssEngineSolveRequestInterruptV4 request;
        uint64_t signature_hi;
        uint64_t signature_lo;
        status = vckss_rust_engine_default_solve_request_interrupt_v4(
            &request, (uint32_t)sizeof(request)
        );
        if (status != 0) return vckss_rust_failure(status);
        request.options.v3.v2.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
        request.interrupt_poll = vckss_stata_interrupt_poll;
        request.interrupt_context = NULL;
        request.checkpoint_interval = 1u;
        if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0 ||
            vckss_parse_u64(argv[2], &request.options.v3.v2.v1.seed) != 0 ||
            vckss_parse_u32(argv[3], &request.options.v3.v2.v1.probes) != 0 ||
            vckss_parse_u32(argv[4], &request.options.v3.v2.v1.leverage_batch_width) != 0 ||
            vckss_parse_u32(argv[5], &request.options.v3.v2.v1.target_batch_width) != 0 ||
            vckss_parse_route(argv[6], &request.options.v3.v2.v1.solver_route) != 0 ||
            vckss_parse_double(argv[7], &request.options.v3.v2.v1.pcg_tolerance) != 0 ||
            vckss_parse_u32(argv[8], &request.options.v3.v2.v1.maximum_iterations) != 0 ||
            vckss_parse_algorithm(argv[9], &request.options.v3.v2.algorithm) != 0 ||
            vckss_parse_deletion(argv[10], &request.options.v3.v2.v1.deletion_mode) != 0 ||
            vckss_parse_nuisance(argv[11], &request.options.v3.v2.nuisance_mode) != 0 ||
            vckss_parse_u64(argv[12], &request.options.v3.v2.exact_estimator_limit) != 0 ||
            vckss_parse_u64(argv[13], &request.options.v3.v2.blocksize_limit) != 0 ||
            vckss_parse_double(argv[14], &request.options.v3.v2.v1.rank_tolerance) != 0 ||
            vckss_parse_double(argv[15], &request.options.v3.v2.v1.block_tolerance) != 0 ||
            vckss_parse_engine(argv[16], &request.options.v3.engine) != 0 ||
            vckss_parse_batch_mode(argv[17], &request.options.v3.batch_mode) != 0 ||
            vckss_parse_stayers_mode(argv[18], &request.options.v3.stayers_mode) != 0 ||
            vckss_parse_target_weight_mode(argv[19], &request.options.v3.target_weight_mode) != 0 ||
            vckss_parse_deletion_source(argv[20], &request.options.v3.deletion_unit_source) != 0 ||
            vckss_parse_u32(argv[21], &request.options.v3.probeorder_supplied) != 0 ||
            vckss_parse_u32(argv[22], &request.options.v3.wallseconds_supplied) != 0 ||
            vckss_parse_u64(argv[23], &request.options.v3.physical_limit) != 0 ||
            vckss_parse_u32(argv[24], &request.options.v3.capability_schema) != 0 ||
            vckss_parse_u32(argv[25], &request.options.v3.capability_profile) != 0 ||
            vckss_parse_u32(argv[26], &request.options.v3.frequency_use) != 0 ||
            vckss_parse_u64(argv[27], &signature_hi) != 0 || signature_hi > UINT32_MAX ||
            vckss_parse_u64(argv[28], &signature_lo) != 0 || signature_lo > UINT32_MAX ||
            vckss_parse_batch_mode(argv[29], &request.options.leverage_batch_mode) != 0 ||
            vckss_parse_batch_mode(argv[30], &request.options.target_batch_mode) != 0 ||
            vckss_parse_u32(argv[31],
                &request.options.v3.v2.v1.allow_automatic_cmg_setup_fallback) != 0 ||
            vckss_parse_double(argv[32], &request.options.wallseconds) != 0) {
            return vckss_usage("invalid Rust V4 planned solve request");
        }
        request.options.v3.v2.v1.rng_contract =
            request.options.v3.v2.algorithm == VCKSS_ALGORITHM_EXACT
                ? VCKSS_RNG_NONE
                : VCKSS_RNG_COUNTER_V1;
        request.options.v3.request_signature =
            (signature_hi << 32) | (signature_lo & UINT64_C(0xffffffff));
        status = vckss_rust_engine_solve_interrupt_v4(generation, &request);
        return status == 0 ? 0 : vckss_rust_failure(status);
    }
    if (argc == 29) {
        VckssEngineSolveRequestInterruptV3 request;
        uint64_t signature_hi;
        uint64_t signature_lo;
        status = vckss_rust_engine_default_solve_request_interrupt_v3(
            &request, (uint32_t)sizeof(request)
        );
        if (status != 0) return vckss_rust_failure(status);
        request.options.v2.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
        request.interrupt_poll = vckss_stata_interrupt_poll;
        request.interrupt_context = NULL;
        request.checkpoint_interval = 1u;
        if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0 ||
            vckss_parse_u64(argv[2], &request.options.v2.v1.seed) != 0 ||
            vckss_parse_u32(argv[3], &request.options.v2.v1.probes) != 0 ||
            vckss_parse_u32(argv[4], &request.options.v2.v1.leverage_batch_width) != 0 ||
            vckss_parse_u32(argv[5], &request.options.v2.v1.target_batch_width) != 0 ||
            vckss_parse_route(argv[6], &request.options.v2.v1.solver_route) != 0 ||
            vckss_parse_double(argv[7], &request.options.v2.v1.pcg_tolerance) != 0 ||
            vckss_parse_u32(argv[8], &request.options.v2.v1.maximum_iterations) != 0 ||
            vckss_parse_algorithm(argv[9], &request.options.v2.algorithm) != 0 ||
            vckss_parse_deletion(argv[10], &request.options.v2.v1.deletion_mode) != 0 ||
            vckss_parse_nuisance(argv[11], &request.options.v2.nuisance_mode) != 0 ||
            vckss_parse_u64(argv[12], &request.options.v2.exact_estimator_limit) != 0 ||
            vckss_parse_u64(argv[13], &request.options.v2.blocksize_limit) != 0 ||
            vckss_parse_double(argv[14], &request.options.v2.v1.rank_tolerance) != 0 ||
            vckss_parse_double(argv[15], &request.options.v2.v1.block_tolerance) != 0 ||
            vckss_parse_engine(argv[16], &request.options.engine) != 0 ||
            vckss_parse_batch_mode(argv[17], &request.options.batch_mode) != 0 ||
            vckss_parse_stayers_mode(argv[18], &request.options.stayers_mode) != 0 ||
            vckss_parse_target_weight_mode(argv[19], &request.options.target_weight_mode) != 0 ||
            vckss_parse_deletion_source(argv[20], &request.options.deletion_unit_source) != 0 ||
            vckss_parse_u32(argv[21], &request.options.probeorder_supplied) != 0 ||
            vckss_parse_u32(argv[22], &request.options.wallseconds_supplied) != 0 ||
            vckss_parse_u64(argv[23], &request.options.physical_limit) != 0 ||
            vckss_parse_u32(argv[24], &request.options.capability_schema) != 0 ||
            vckss_parse_u32(argv[25], &request.options.capability_profile) != 0 ||
            vckss_parse_u32(argv[26], &request.options.frequency_use) != 0 ||
            vckss_parse_u64(argv[27], &signature_hi) != 0 || signature_hi > UINT32_MAX ||
            vckss_parse_u64(argv[28], &signature_lo) != 0 || signature_lo > UINT32_MAX) {
            return vckss_usage("invalid Rust V3 generic solve request");
        }
        request.options.request_signature =
            (signature_hi << 32) | (signature_lo & UINT64_C(0xffffffff));
        status = vckss_rust_engine_solve_interrupt_v3(generation, &request);
        return status == 0 ? 0 : vckss_rust_failure(status);
    }
    if (argc != 16) {
        char message[96];
        (void)snprintf(
            message,
            sizeof(message),
            "Rust solve expected 9, 16, 29, or 33 arguments but received %d",
            argc
        );
        return vckss_usage(message);
    }
    {
        VckssEngineSolveRequestInterruptV2 request;
        status = vckss_rust_engine_default_solve_request_interrupt_v2(
            &request, (uint32_t)sizeof(request)
        );
        if (status != 0) {
            return vckss_rust_failure(status);
        }
        request.options.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
        request.interrupt_poll = vckss_stata_interrupt_poll;
        request.interrupt_context = NULL;
        request.checkpoint_interval = 1u;
        if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("invalid Rust solve generation");
        }
        if (vckss_parse_u64(argv[2], &request.options.v1.seed) != 0) {
            return vckss_usage("invalid Rust solve seed");
        }
        if (vckss_parse_u32(argv[3], &request.options.v1.probes) != 0) {
            return vckss_usage("invalid Rust solve probe count");
        }
        if (vckss_parse_u32(argv[4], &request.options.v1.leverage_batch_width) != 0) {
            return vckss_usage("invalid Rust leverage batch width");
        }
        if (vckss_parse_u32(argv[5], &request.options.v1.target_batch_width) != 0) {
            return vckss_usage("invalid Rust target batch width");
        }
        if (vckss_parse_route(argv[6], &request.options.v1.solver_route) != 0) {
            return vckss_usage("invalid Rust solver route");
        }
        if (vckss_parse_double(argv[7], &request.options.v1.pcg_tolerance) != 0) {
            return vckss_usage("invalid Rust PCG tolerance");
        }
        if (vckss_parse_u32(argv[8], &request.options.v1.maximum_iterations) != 0) {
            return vckss_usage("invalid Rust maximum iteration count");
        }
        if (vckss_parse_algorithm(argv[9], &request.options.algorithm) != 0) {
            return vckss_usage("invalid Rust estimator algorithm");
        }
        if (vckss_parse_deletion(argv[10], &request.options.v1.deletion_mode) != 0) {
            return vckss_usage("invalid Rust deletion mode");
        }
        if (vckss_parse_nuisance(argv[11], &request.options.nuisance_mode) != 0) {
            return vckss_usage("invalid Rust nuisance mode");
        }
        if (vckss_parse_u64(argv[12], &request.options.exact_estimator_limit) != 0) {
            return vckss_usage("invalid Rust exact estimator limit");
        }
        if (vckss_parse_u64(argv[13], &request.options.blocksize_limit) != 0) {
            return vckss_usage("invalid Rust block-size limit");
        }
        if (vckss_parse_double(argv[14], &request.options.v1.rank_tolerance) != 0) {
            return vckss_usage("invalid Rust rank tolerance");
        }
        if (vckss_parse_double(argv[15], &request.options.v1.block_tolerance) != 0) {
            return vckss_usage("invalid Rust block tolerance");
        }
        status = vckss_rust_engine_solve_interrupt_v2(generation, &request);
        return status == 0 ? 0 : vckss_rust_failure(status);
    }
}

static int vckss_solve_full(int argc, char *argv[])
{
    VckssEngineSolveRequestInterruptV5 request;
    uint64_t generation;
    uint64_t signature_hi;
    uint64_t signature_lo;
    int status;

    if (argc != 35) {
        return vckss_usage("Rust solvefull requires the V5 planned solve arguments");
    }
    status = vckss_rust_engine_default_solve_request_interrupt_v5(
        &request, (uint32_t)sizeof(request)
    );
    if (status != 0) return vckss_rust_failure(status);
    request.options.v4.v3.v2.v1.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.interrupt_poll = vckss_stata_interrupt_poll;
    request.interrupt_context = NULL;
    request.checkpoint_interval = 1u;
    if (vckss_parse_u64(argv[1], &generation) != 0 || generation == 0 ||
        vckss_parse_u64(argv[2], &request.options.v4.v3.v2.v1.seed) != 0 ||
        vckss_parse_u32(argv[3], &request.options.v4.v3.v2.v1.probes) != 0 ||
        vckss_parse_u32(argv[4], &request.options.v4.v3.v2.v1.leverage_batch_width) != 0 ||
        vckss_parse_u32(argv[5], &request.options.v4.v3.v2.v1.target_batch_width) != 0 ||
        vckss_parse_route(argv[6], &request.options.v4.v3.v2.v1.solver_route) != 0 ||
        vckss_parse_double(argv[7], &request.options.v4.v3.v2.v1.pcg_tolerance) != 0 ||
        vckss_parse_u32(argv[8], &request.options.v4.v3.v2.v1.maximum_iterations) != 0 ||
        vckss_parse_algorithm(argv[9], &request.options.v4.v3.v2.algorithm) != 0 ||
        vckss_parse_deletion(argv[10], &request.options.v4.v3.v2.v1.deletion_mode) != 0 ||
        vckss_parse_nuisance(argv[11], &request.options.v4.v3.v2.nuisance_mode) != 0 ||
        vckss_parse_u64(argv[12], &request.options.v4.v3.v2.exact_estimator_limit) != 0 ||
        vckss_parse_u64(argv[13], &request.options.v4.v3.v2.blocksize_limit) != 0 ||
        vckss_parse_double(argv[14], &request.options.v4.v3.v2.v1.rank_tolerance) != 0 ||
        vckss_parse_double(argv[15], &request.options.v4.v3.v2.v1.block_tolerance) != 0 ||
        vckss_parse_engine(argv[16], &request.options.v4.v3.engine) != 0 ||
        vckss_parse_batch_mode(argv[17], &request.options.v4.v3.batch_mode) != 0 ||
        vckss_parse_stayers_mode(argv[18], &request.options.v4.v3.stayers_mode) != 0 ||
        vckss_parse_target_weight_mode(argv[19], &request.options.v4.v3.target_weight_mode) != 0 ||
        vckss_parse_deletion_source(argv[20], &request.options.v4.v3.deletion_unit_source) != 0 ||
        vckss_parse_u32(argv[21], &request.options.v4.v3.probeorder_supplied) != 0 ||
        vckss_parse_u32(argv[22], &request.options.v4.v3.wallseconds_supplied) != 0 ||
        vckss_parse_u64(argv[23], &request.options.v4.v3.physical_limit) != 0 ||
        vckss_parse_u32(argv[24], &request.options.v4.v3.capability_schema) != 0 ||
        vckss_parse_u32(argv[25], &request.options.v4.v3.capability_profile) != 0 ||
        vckss_parse_u32(argv[26], &request.options.v4.v3.frequency_use) != 0 ||
        vckss_parse_u64(argv[27], &signature_hi) != 0 || signature_hi > UINT32_MAX ||
        vckss_parse_u64(argv[28], &signature_lo) != 0 || signature_lo > UINT32_MAX ||
        vckss_parse_batch_mode(argv[29], &request.options.v4.leverage_batch_mode) != 0 ||
        vckss_parse_batch_mode(argv[30], &request.options.v4.target_batch_mode) != 0 ||
        vckss_parse_u32(argv[31],
            &request.options.v4.v3.v2.v1.allow_automatic_cmg_setup_fallback) != 0 ||
        vckss_parse_double(argv[32], &request.options.v4.wallseconds) != 0 ||
        vckss_parse_u32(argv[33], &request.options.threads) != 0 ||
        request.options.threads == 0 ||
        vckss_parse_u32(argv[34], &request.options.tolerance_supplied) != 0 ||
        request.options.tolerance_supplied > 1) {
        return vckss_usage("invalid Rust V5 full-CMG planned solve request");
    }
    request.options.v4.v3.v2.v1.rng_contract = VCKSS_RNG_COUNTER_V1;
    request.options.v4.v3.request_signature =
        (signature_hi << 32) | (signature_lo & UINT64_C(0xffffffff));
    request.options.full_cmg_v2 = 1u;
    status = vckss_rust_engine_solve_interrupt_v5(generation, &request);
    return status == 0 ? 0 : vckss_rust_failure(status);
}

static int vckss_save_components(const char *prefix, const VckssComponentVectorV1 *value)
{
    char name[33];
    int status;
    const char *suffixes[4] = {"worker", "firm", "cov", "total"};
    const double values[4] = {value->worker, value->firm, value->covariance, value->total};
    int index;

    for (index = 0; index < 4; ++index) {
        const int written = snprintf(name, sizeof(name), "__vckss_%s_%s", prefix, suffixes[index]);
        if (written < 0 || (size_t)written >= sizeof(name)) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: could not construct a Rust component scalar name",
                VCKSS_STATA_MEMORY_ERROR
            );
        }
        status = vckss_save_double(name, values[index]);
        if (status != 0) {
            return status;
        }
    }
    return 0;
}

static int vckss_result(uint64_t generation)
{
    VckssEngineResultV1 result;
    VckssEngineDetailedReceiptV7 receipt_v7;
    VckssEngineDetailedReceiptV6 receipt_v6;
    VckssEngineDetailedReceiptV5 receipt_v5;
    VckssEngineDetailedReceiptV4 receipt_v4;
    VckssEngineDetailedReceiptV3 receipt_v3;
    VckssEngineDetailedReceiptV2 receipt;
    VckssEnginePerformanceReceiptV1 performance;
    VckssProjectionResultReceiptV1 projection_receipt;
    uint64_t projection_columns = 0;
    int has_plan = 0;
    int status;

    memset(&result, 0, sizeof(result));
    memset(&receipt_v7, 0, sizeof(receipt_v7));
    memset(&receipt_v6, 0, sizeof(receipt_v6));
    memset(&receipt_v5, 0, sizeof(receipt_v5));
    memset(&receipt_v3, 0, sizeof(receipt_v3));
    memset(&performance, 0, sizeof(performance));
    memset(&projection_receipt, 0, sizeof(projection_receipt));
    status = vckss_rust_engine_result_v1(generation, &result, (uint32_t)sizeof(result));
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    status = vckss_rust_engine_performance_receipt_v1(
        generation, &performance, (uint32_t)sizeof(performance)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    status = vckss_rust_engine_detailed_receipt_v6(
        generation, &receipt_v6, (uint32_t)sizeof(receipt_v6)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (receipt_v6.capability_schema == VCKSS_REQUEST_CAPABILITY_SCHEMA_V3) {
        status = vckss_rust_engine_detailed_receipt_v7(
            generation, &receipt_v7, (uint32_t)sizeof(receipt_v7)
        );
        if (status != 0) {
            return vckss_rust_failure(status);
        }
        if (receipt_v7.execution.struct_size != sizeof(receipt_v7.execution) ||
            receipt_v7.execution.schema_version != 1u ||
            receipt_v7.execution.generation != generation ||
            receipt_v7.execution.request_signature != receipt_v6.request_signature ||
            receipt_v7.v6.capability_schema != receipt_v6.capability_schema ||
            receipt_v7.v6.capability_profile != receipt_v6.capability_profile ||
            receipt_v7.v6.request_signature != receipt_v6.request_signature ||
            receipt_v7.v6.engine_requested != receipt_v6.engine_requested ||
            receipt_v7.v6.engine_selected != receipt_v6.engine_selected ||
            receipt_v7.execution.resolution.reserved != 0 ||
            receipt_v7.execution.solver.reserved_1 != 0 ||
            receipt_v7.execution.solver.reserved_2 != 0 ||
            receipt_v7.execution.batch.leverage.reserved != 0 ||
            receipt_v7.execution.batch.target.reserved != 0 ||
            receipt_v7.execution.memory.reserved != 0) {
            status = vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust V7 execution-plan receipt did not reconcile with its V6 prefix",
                498
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        receipt_v6 = receipt_v7.v6;
        has_plan = 1;
    }
    receipt_v5 = receipt_v6.v5;
    receipt_v4 = receipt_v5.v4;
    receipt_v3 = receipt_v4.v3;
    receipt = receipt_v3.v2;
    if (receipt_v6.engine_selected == VCKSS_ENGINE_GENERIC &&
        receipt_v6.rhs_receipt_schema == 2) {
        uint64_t base_rows = (uint64_t)receipt_v6.controls_count + UINT64_C(1);
        if (receipt_v4.nuisance_mode == VCKSS_NUISANCE_FIXED_OFFSET &&
            receipt_v6.controls_count != 0) {
            ++base_rows;
        }
        if (receipt.probes_requested > (UINT64_MAX - base_rows) / UINT64_C(3)) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: generic RHS base-row count overflow",
                498
            );
        }
        base_rows += UINT64_C(3) * receipt.probes_requested;
        if (receipt_v3.rhs_receipt_rows < base_rows) {
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: generic RHS rows cannot contain the declared phases",
                498
            );
        }
        projection_columns = receipt_v3.rhs_receipt_rows - base_rows;
        if (projection_columns != 0) {
            status = vckss_rust_engine_projection_result_receipt_v1(
                generation, &projection_receipt, (uint32_t)sizeof(projection_receipt)
            );
            if (status != 0) return vckss_rust_failure(status);
            if (projection_receipt.struct_size != sizeof(projection_receipt) ||
                projection_receipt.schema_version != VCKSS_PROJECTION_SCHEMA_V1 ||
                projection_receipt.generation != generation ||
                projection_receipt.columns != projection_columns ||
                projection_receipt.projection_peak_forecast_bytes == 0) {
                return vckss_c_failure(
                    VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                    "INTERNAL_INVARIANT_FAILED",
                    "INTERNAL_INVARIANT_FAILED [stata_spi]: projection metadata did not reconcile with generic RHS rows",
                    498
                );
            }
        }
    }
    if (performance.struct_size != sizeof(performance) ||
        performance.schema_version != 1u || performance.generation != generation ||
        (performance.applicability_flags & UINT64_C(3)) != UINT64_C(3) ||
        performance.algorithm_selected != receipt_v4.algorithm_selected ||
        performance.engine_selected != receipt_v6.engine_selected ||
        performance.ingest_ns > performance.native_total_ns ||
        performance.canonicalize_ns > performance.native_total_ns ||
        performance.graph_ns > performance.native_total_ns ||
        performance.compress_ns > performance.native_total_ns ||
        performance.plan_ns > performance.native_total_ns ||
        performance.stayer_augmentation_ns > performance.native_total_ns ||
        performance.solve_ns > performance.native_total_ns) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust performance receipt did not reconcile with the solved result",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    if (receipt_v6.reserved_6 != 0) {
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust V6 detailed receipt reserved field is nonzero",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    switch (receipt_v6.rhs_receipt_schema) {
    case 0:
        if (receipt_v3.rhs_receipt_rows != 0 ||
            receipt_v3.caller_result_copy_bytes != 0 ||
            receipt_v6.rhs_v2_caller_copy_bytes != 0) {
            status = vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust RHS schema 0 export receipt is inconsistent",
                498
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        break;
    case 1: {
        const uint64_t per_row =
            (uint64_t)sizeof(VckssEngineRhsReceiptV1) + UINT64_C(8) * sizeof(double);
        if (receipt_v3.rhs_receipt_rows == 0 ||
            receipt_v3.rhs_receipt_rows > UINT64_MAX / per_row ||
            receipt_v3.caller_result_copy_bytes != receipt_v3.rhs_receipt_rows * per_row ||
            receipt_v6.rhs_v2_caller_copy_bytes != 0) {
            status = vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust RHS schema 1 export receipt is inconsistent",
                498
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        break;
    }
    case 2: {
        const uint64_t per_row =
            (uint64_t)sizeof(VckssEngineRhsReceiptV2) + UINT64_C(15) * sizeof(double);
        if (receipt_v3.rhs_receipt_rows == 0 ||
            receipt_v3.caller_result_copy_bytes != 0 ||
            receipt_v3.rhs_receipt_rows > UINT64_MAX / per_row ||
            receipt_v6.rhs_v2_caller_copy_bytes != receipt_v3.rhs_receipt_rows * per_row) {
            status = vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust RHS schema 2 export receipt is inconsistent",
                498
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        break;
    }
    default:
        status = vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust RHS export receipt has an unknown schema",
            498
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    if ((status = vckss_save_components("plugin", &result.plugin)) != 0 ||
        (status = vckss_save_components("correction", &result.correction)) != 0 ||
        (status = vckss_save_components("corrected", &result.corrected)) != 0 ||
        (status = vckss_save_components("mcse", &result.numerical_mcse)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_seed", receipt.seed)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_probes", receipt.probes_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_lev_accepted", receipt.leverage_probes_accepted)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_tgt_accepted", receipt.target_probes_accepted)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_route_requested", receipt.solver_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_route_selected", receipt.solver_selected)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_fallback", receipt.solver_fallback)) != 0 ||
        (status = vckss_save_double("__vckss_rust_fallback_error", (double)receipt.solver_fallback_error)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solver_dimension", receipt.solver_dimension)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_lev_batch", receipt.leverage_batch_width)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_tgt_batch", receipt.target_batch_width)) != 0 ||
        (status = vckss_save_double("__vckss_rust_rank_tolerance", receipt.rank_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_rust_block_tolerance", receipt.block_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_rust_full_tolerance", receipt.full_residual_tolerance)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_full_route", receipt.full_fit_route)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_full_iterations", receipt.full_fit_iterations)) != 0 ||
        (status = vckss_save_double("__vckss_rust_full_reduced", receipt.full_fit_reduced_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_full_complete", receipt.full_fit_complete_residual)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_full_zero", receipt.full_fit_zero_rhs)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_lev_rhs", receipt.leverage_rhs_count)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_tgt_rhs", receipt.target_rhs_count)) != 0 ||
        (status = vckss_save_double("__vckss_rust_max_reduced", receipt.max_reduced_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_max_complete", receipt.max_complete_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_max_leverage", receipt.max_leverage)) != 0 ||
        (status = vckss_save_double("__vckss_rust_max_reciprocal", receipt.max_reciprocal_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_accounting", receipt.accounting_residual)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_topology_hi", receipt.topology_checksum >> 32)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_topology_lo", receipt.topology_checksum & UINT64_C(0xffffffff))) != 0 ||
        (status = vckss_save_u64("__vckss_rust_rng_contract", receipt_v3.rng_contract)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_rhs_rows", receipt_v3.rhs_receipt_rows)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_rhs_copy", receipt_v3.caller_result_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_algorithm_req", receipt_v4.algorithm_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_algorithm_sel", receipt_v4.algorithm_selected)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_deletion_mode", receipt_v4.deletion_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_nuisance_mode", receipt_v4.nuisance_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_parameters", receipt_v4.parameters)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_full_parameters", receipt_v4.full_parameters)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_corr_parameters", receipt_v4.correction_parameters)) != 0 ||
        (status = vckss_save_double("__vckss_rust_info_rcond", receipt_v4.information_rcond)) != 0 ||
        (status = vckss_save_double("__vckss_rust_inverse_relres", receipt_v4.inverse_relres)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_exact_peak", receipt_v4.exact_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_exact_flags", receipt_v5.applicability_flags)) != 0 ||
        (status = vckss_save_double("__vckss_rust_working_fit", receipt_v5.working_fit_complete_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_inverse_sqrt", receipt_v5.inverse_sqrt_relres)) != 0 ||
        (status = vckss_save_double("__vckss_rust_maker_relres", receipt_v5.maker_relres)) != 0 ||
        (status = vckss_save_double("__vckss_rust_control_relres", receipt_v5.control_basis_relres)) != 0 ||
        (status = vckss_save_double("__vckss_rust_control_fwd_error", receipt_v5.control_basis_forward_error)) != 0 ||
        (status = vckss_save_double("__vckss_rust_deletion_rank_gap", receipt_v5.deletion_rank_gap)) != 0 ||
        (status = vckss_save_double("__vckss_rust_firm_zero_sum", receipt_v5.firm_zero_sum_residual)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_fit_peak", receipt_v5.fit_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_correction_peak", receipt_v5.correction_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_double("__vckss_rust_actual_accounting", receipt_v5.actual_accounting_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_weighted_rss", receipt.full_fit_weighted_rss)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_memory_limit", receipt.memory_limit_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_caller_copy", receipt.caller_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_prepare_peak", receipt.preparation_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_prepared_resident", receipt.prepared_resident_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solver_setup", receipt.solver_setup_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_leverage_phase", receipt.leverage_phase_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_target_phase", receipt.target_phase_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_result_bytes", receipt.result_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_peak", receipt.solve_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_command_peak", receipt.command_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_engine_requested", receipt_v6.engine_requested)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_engine_selected", receipt_v6.engine_selected)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_generic_flags", receipt_v6.generic_applicability_flags)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_generic_controls", receipt_v6.controls_count)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_rhs_schema", receipt_v6.rhs_receipt_schema)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_proj_columns", projection_columns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_proj_peak", projection_receipt.projection_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_control_rhs", receipt_v6.control_projection_rhs_count)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_rcond", receipt_v6.control_rank_rcond)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_small_lo", receipt_v6.control_rank_smallest_generalized_eigenvalue_lower)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_large_hi", receipt_v6.control_rank_largest_generalized_eigenvalue_upper)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_proj_err", receipt_v6.control_rank_projection_error_bound)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_norm_err", receipt_v6.control_rank_normalization_error_bound)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_fe_lo", receipt_v6.control_rank_fe_information_eigenvalue_lower_bound)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_max_proj", receipt_v6.control_rank_maximum_projection_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_tol", receipt_v6.control_rank_effective_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_pcg_tol", receipt_v6.control_rank_projection_pcg_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_rust_cr_resid_gate", receipt_v6.control_rank_projection_residual_gate)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_control_relres", receipt_v6.generic_control_basis_relres)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_control_fwd", receipt_v6.generic_control_basis_forward_error)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_schur_rcond", receipt_v6.generic_control_schur_rcond)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_schur_relres", receipt_v6.generic_control_schur_relres)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_delete_gap", receipt_v6.generic_deletion_rank_gap)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_full_joint", receipt_v6.full_joint_fit_complete_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_working_fit", receipt_v6.generic_working_fit_complete_residual)) != 0 ||
        (status = vckss_save_double("__vckss_rust_g_maker", receipt_v6.generic_maker_relres)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_canon_peak", receipt_v6.canonicalization_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_fit_peak", receipt_v6.generic_fit_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_geometry_peak", receipt_v6.geometry_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_lev_peak", receipt_v6.generic_leverage_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_tgt_peak", receipt_v6.generic_target_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_maker_peak", receipt_v6.maker_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_result_bytes", receipt_v6.generic_result_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_g_peak", receipt_v6.generic_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_rhs_v2_copy", receipt_v6.rhs_v2_caller_copy_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_schema_echo", receipt_v6.capability_schema)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_cap_profile_echo", receipt_v6.capability_profile)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_batch", receipt_v6.batch_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_stayers", receipt_v6.stayers_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_target", receipt_v6.target_weight_mode)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_delsource", receipt_v6.deletion_unit_source)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_probeorder", receipt_v6.probeorder_supplied)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_wall", receipt_v6.wallseconds_supplied)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_frequency", receipt_v6.frequency_use)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_physlimit", receipt_v6.physical_limit)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_signature_hi", receipt_v6.request_signature >> 32)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_solve_signature_lo", receipt_v6.request_signature & UINT64_C(0xffffffff))) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_schema", performance.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_flags", performance.applicability_flags)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_ingest_ns", performance.ingest_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_canon_ns", performance.canonicalize_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_graph_ns", performance.graph_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_compress_ns", performance.compress_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_plan_ns", performance.plan_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_stayer_ns", performance.stayer_augmentation_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_solve_ns", performance.solve_ns)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_pf_total_ns", performance.native_total_ns)) != 0) {
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    if (has_plan) {
        status = vckss_export_execution_plan(&receipt_v7.execution);
        if (status != 0) {
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
    }
    return 0;
}

static double vckss_component_accounting_residual(
    const VckssComponentVectorV1 *value
)
{
    return fabs(value->total - value->worker - value->firm - 2.0 * value->covariance);
}

static int vckss_stayer_result(uint64_t generation)
{
    VckssStayerHybridResultV1 result;
    VckssStayerAugmentationReceiptV1 augmentation;
    double source_residual = 0.0;
    int status;

    memset(&result, 0, sizeof(result));
    memset(&augmentation, 0, sizeof(augmentation));
    status = vckss_rust_engine_stayer_hybrid_result_v1(
        generation, &result, (uint32_t)sizeof(result)
    );
    if (status != 0) return vckss_rust_failure(status);
    status = vckss_rust_engine_stayer_augmentation_receipt_v1(
        generation, &augmentation, (uint32_t)sizeof(augmentation)
    );
    if (status != 0) return vckss_rust_failure(status);

#define VCKSS_SOURCE_RESIDUAL(field) \
    fabs(result.correction.field - result.mover_correction.field - \
         result.stayer_correction.field)
    {
        double value = VCKSS_SOURCE_RESIDUAL(worker);
        if (value > source_residual) source_residual = value;
        value = VCKSS_SOURCE_RESIDUAL(firm);
        if (value > source_residual) source_residual = value;
        value = VCKSS_SOURCE_RESIDUAL(covariance);
        if (value > source_residual) source_residual = value;
        value = VCKSS_SOURCE_RESIDUAL(total);
        if (value > source_residual) source_residual = value;
    }
#undef VCKSS_SOURCE_RESIDUAL
    if (result.struct_size != sizeof(result) || result.schema_version != 1u ||
        result.generation != generation ||
        augmentation.struct_size != sizeof(augmentation) ||
        augmentation.schema_version != 1u || augmentation.generation != generation ||
        result.deletion_units != augmentation.combined_deletion_units ||
        result.topology_checksum != augmentation.topology_checksum ||
        result.peak_forecast_bytes > augmentation.memory_limit_bytes ||
        result.fit_peak_forecast_bytes > result.peak_forecast_bytes ||
        result.correction_peak_forecast_bytes > result.peak_forecast_bytes ||
        !isfinite(result.accounting_residual) || result.accounting_residual > 1.0e-10 ||
        !isfinite(source_residual) || source_residual > 1.0e-10 ||
        vckss_component_accounting_residual(&result.plugin) > 1.0e-10 ||
        vckss_component_accounting_residual(&result.correction) > 1.0e-10 ||
        vckss_component_accounting_residual(&result.corrected) > 1.0e-10 ||
        vckss_component_accounting_residual(&result.mover_correction) > 1.0e-10 ||
        vckss_component_accounting_residual(&result.stayer_correction) > 1.0e-10) {
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust stayer-hybrid result did not reconcile",
            498
        );
    }
    if ((status = vckss_save_components("hyb_plugin", &result.plugin)) != 0 ||
        (status = vckss_save_components("hyb_correction", &result.correction)) != 0 ||
        (status = vckss_save_components("hyb_corrected", &result.corrected)) != 0 ||
        (status = vckss_save_components("hyb_mover", &result.mover_correction)) != 0 ||
        (status = vckss_save_components("hyb_stayer", &result.stayer_correction)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_result_schema", result.schema_version)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_weighted_rss", result.weighted_rss)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_parameters", result.parameters)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_full_parameters", result.full_parameters)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_corr_parameters", result.correction_parameters)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_result_del", result.deletion_units)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_max_leverage", result.max_leverage)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_info_rcond", result.information_rcond)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_inverse", result.inverse_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_inverse_original", result.inverse_original_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_inverse_sqrt", result.inverse_sqrt_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_maker", result.maker_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_full_fit", result.full_fit_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_working_fit", result.working_fit_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_fit_tolerance", result.fit_residual_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_control_relres", result.control_basis_relres)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_control_fwd", result.control_basis_forward_error)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_delete_gap", result.deletion_rank_gap)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_firm_zero_sum", result.firm_zero_sum_residual)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_result_peak", result.peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_fit_peak", result.fit_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_hyb_corr_peak", result.correction_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_accounting", result.accounting_residual)) != 0 ||
        (status = vckss_save_double("__vckss_hyb_source_resid", source_residual)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_store_rhs_matrix_cell(
    const char *matrix_name,
    ST_int row,
    ST_int column,
    double value
)
{
    if (SF_mat_store((char *)matrix_name, row, column, value) != 0) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not store the Rust RHS receipt matrix",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    return 0;
}

static int vckss_rhsresult(uint64_t generation, const char *matrix_name)
{
    VckssEngineDetailedReceiptV6 receipt_v6;
    VckssEngineDetailedReceiptV3 receipt;
    VckssEngineRhsReceiptV1 *rows = NULL;
    uint64_t row_count;
    uint64_t row;
    int status;

    if (matrix_name == NULL || *matrix_name == '\0') {
        return vckss_usage("Rust rhsresult requires a Stata matrix name");
    }
    memset(&receipt_v6, 0, sizeof(receipt_v6));
    status = vckss_rust_engine_detailed_receipt_v6(
        generation, &receipt_v6, (uint32_t)sizeof(receipt_v6)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    receipt = receipt_v6.v5.v4.v3;
    row_count = receipt.rhs_receipt_rows;
    if (receipt_v6.rhs_receipt_schema == 2) {
        VckssEngineRhsReceiptV2 *rows_v2;
        uint64_t per_row = (uint64_t)sizeof(*rows_v2) + UINT64_C(15) * sizeof(double);
        if (row_count == 0 || row_count > (uint64_t)SIZE_MAX / sizeof(*rows_v2)) {
            status = vckss_usage("Rust RHS V2 receipt row count is not allocatable");
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        if (receipt.caller_result_copy_bytes != 0 ||
            row_count > UINT64_MAX / per_row ||
            receipt_v6.rhs_v2_caller_copy_bytes != row_count * per_row) {
            status = vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust RHS V2 export memory receipt is inconsistent",
                498
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        if (SF_row((char *)matrix_name) != (ST_int)row_count ||
            SF_col((char *)matrix_name) != 15) {
            status = vckss_usage(
                "Rust rhsresult V2 matrix must have the exact reported row count and fifteen columns"
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        rows_v2 = (VckssEngineRhsReceiptV2 *)vckss_calloc((size_t)row_count, sizeof(*rows_v2));
        if (rows_v2 == NULL) {
            status = vckss_c_failure(
                VCKSS_ERROR_ALLOCATION_FAILED,
                "ALLOCATION_FAILED",
                "ALLOCATION_FAILED [stata_spi]: could not allocate the Rust RHS V2 receipt caller copy",
                VCKSS_STATA_MEMORY_ERROR
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
        status = vckss_rust_engine_rhs_receipts_v2(generation, rows_v2, row_count);
        if (status != 0) {
            free(rows_v2);
            return vckss_rust_failure(status);
        }
        for (row = 0; row < row_count; ++row) {
            const double values[15] = {
                (double)rows_v2[row].v1.phase,
                (double)rows_v2[row].v1.probe,
                (double)rows_v2[row].v1.side,
                (double)rows_v2[row].v1.route,
                (double)rows_v2[row].v1.iterations,
                rows_v2[row].v1.reduced_residual,
                rows_v2[row].v1.complete_residual,
                (double)rows_v2[row].v1.zero_rhs,
                (double)rows_v2[row].status,
                (double)rows_v2[row].residual_replacements,
                (double)rows_v2[row].operator_applications,
                (double)rows_v2[row].preconditioner_applications,
                rows_v2[row].full_residual_tolerance,
                (double)rows_v2[row].residual_space,
                (double)rows_v2[row].solver_dimension
            };
            int column;
            for (column = 0; column < 15; ++column) {
                status = vckss_store_rhs_matrix_cell(
                    matrix_name,
                    (ST_int)(row + 1),
                    (ST_int)(column + 1),
                    values[column]
                );
                if (status != 0) {
                    free(rows_v2);
                    vckss_cleanup_preserving_primary(generation);
                    return status;
                }
            }
        }
        free(rows_v2);
        return 0;
    }
    if (receipt_v6.rhs_receipt_schema != 1) {
        status = vckss_usage("Rust result does not expose an RHS receipt schema");
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    {
        const uint64_t per_row =
            (uint64_t)sizeof(*rows) + UINT64_C(8) * sizeof(double);
        if (receipt_v6.rhs_v2_caller_copy_bytes != 0 ||
            row_count > UINT64_MAX / per_row ||
            receipt.caller_result_copy_bytes != row_count * per_row) {
            status = vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust RHS V1 export memory receipt is inconsistent",
                498
            );
            vckss_cleanup_preserving_primary(generation);
            return status;
        }
    }
    if (row_count == 0 || row_count > (uint64_t)SIZE_MAX / sizeof(*rows)) {
        status = vckss_usage("Rust RHS receipt row count is not allocatable");
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    if (SF_row((char *)matrix_name) != (ST_int)row_count ||
        SF_col((char *)matrix_name) != 8) {
        status = vckss_usage(
            "Rust rhsresult matrix must have the exact reported row count and eight columns"
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    rows = (VckssEngineRhsReceiptV1 *)vckss_calloc((size_t)row_count, sizeof(*rows));
    if (rows == NULL) {
        status = vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate the Rust RHS receipt caller copy",
            VCKSS_STATA_MEMORY_ERROR
        );
        vckss_cleanup_preserving_primary(generation);
        return status;
    }
    status = vckss_rust_engine_rhs_receipts_v1(generation, rows, row_count);
    if (status != 0) {
        free(rows);
        return vckss_rust_failure(status);
    }
    for (row = 0; row < row_count; ++row) {
        const double values[8] = {
            (double)rows[row].phase,
            (double)rows[row].probe,
            (double)rows[row].side,
            (double)rows[row].route,
            (double)rows[row].iterations,
            rows[row].reduced_residual,
            rows[row].complete_residual,
            (double)rows[row].zero_rhs
        };
        int column;
        for (column = 0; column < 8; ++column) {
            status = vckss_store_rhs_matrix_cell(
                matrix_name,
                (ST_int)(row + 1),
                (ST_int)(column + 1),
                values[column]
            );
            if (status != 0) {
                free(rows);
                vckss_cleanup_preserving_primary(generation);
                return status;
            }
        }
    }
    free(rows);
    return 0;
}

static int vckss_projectionresult(
    uint64_t generation,
    uint64_t columns,
    const char *coefficient_matrix,
    const char *covariance_matrix,
    const char *naive_matrix
)
{
    VckssProjectionResultReceiptV1 receipt;
    double *coefficients = NULL;
    double *covariance = NULL;
    double *naive = NULL;
    uint64_t entries;
    uint64_t expected_result_bytes;
    uint64_t row;
    uint64_t column;
    int status;

    if (columns == 0 || columns > (uint64_t)SIZE_MAX / sizeof(double) ||
        columns > UINT64_MAX / columns ||
        columns * columns > (uint64_t)SIZE_MAX / sizeof(double)) {
        return vckss_usage("Rust projection result dimensions are not allocatable");
    }
    if (coefficient_matrix == NULL || covariance_matrix == NULL || naive_matrix == NULL ||
        *coefficient_matrix == '\0' || *covariance_matrix == '\0' || *naive_matrix == '\0' ||
        SF_row((char *)coefficient_matrix) != 1 ||
        SF_col((char *)coefficient_matrix) != (ST_int)columns ||
        SF_row((char *)covariance_matrix) != (ST_int)columns ||
        SF_col((char *)covariance_matrix) != (ST_int)columns ||
        SF_row((char *)naive_matrix) != (ST_int)columns ||
        SF_col((char *)naive_matrix) != (ST_int)columns) {
        return vckss_usage(
            "Rust projection result matrices must have exact dimensions 1 x q, q x q, and q x q"
        );
    }
    entries = columns * columns;
    coefficients = (double *)vckss_calloc((size_t)columns, sizeof(double));
    covariance = (double *)vckss_calloc((size_t)entries, sizeof(double));
    naive = (double *)vckss_calloc((size_t)entries, sizeof(double));
    if (coefficients == NULL || covariance == NULL || naive == NULL) {
        free(coefficients);
        free(covariance);
        free(naive);
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate projection result buffers",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_projection_result_v1(
        generation,
        coefficients,
        columns,
        covariance,
        entries,
        naive,
        entries,
        &receipt,
        (uint32_t)sizeof(receipt)
    );
    if (status != 0) {
        free(coefficients);
        free(covariance);
        free(naive);
        return vckss_rust_failure(status);
    }
    if (entries > (UINT64_MAX - columns) / 2 ||
        columns + 2 * entries > UINT64_MAX / sizeof(double)) {
        free(coefficients);
        free(covariance);
        free(naive);
        return vckss_usage("projection result byte count overflow");
    }
    expected_result_bytes = (columns + 2 * entries) * sizeof(double);
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.schema_version != VCKSS_PROJECTION_SCHEMA_V1 ||
        receipt.generation != generation || receipt.columns != columns ||
        receipt.reserved != 0 ||
        (receipt.effect != VCKSS_PROJECTION_EFFECT_WORKER &&
         receipt.effect != VCKSS_PROJECTION_EFFECT_FIRM) ||
        (receipt.weight != VCKSS_PROJECTION_WEIGHT_FREQUENCY &&
         receipt.weight != VCKSS_PROJECTION_WEIGHT_TARGET) ||
        !isfinite(receipt.gram_rcond) || receipt.gram_rcond <= 0.0 ||
        !isfinite(receipt.gram_relres) || receipt.gram_relres < 0.0 ||
        !isfinite(receipt.gram_original_relres) || receipt.gram_original_relres < 0.0 ||
        !isfinite(receipt.covariance_smallest_eigenvalue) ||
        !isfinite(receipt.covariance_largest_eigenvalue) ||
        !isfinite(receipt.psd_cleanup) || receipt.psd_cleanup < 0.0 ||
        !isfinite(receipt.proxy_minimum) || !isfinite(receipt.proxy_maximum) ||
        receipt.proxy_minimum > receipt.proxy_maximum ||
        !isfinite(receipt.maximum_reduced_residual) ||
        !isfinite(receipt.maximum_complete_residual) ||
        !isfinite(receipt.full_residual_tolerance) ||
        receipt.maximum_complete_residual > receipt.full_residual_tolerance ||
        receipt.projection_peak_forecast_bytes == 0 ||
        receipt.result_bytes != expected_result_bytes) {
        free(coefficients);
        free(covariance);
        free(naive);
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust projection result receipt did not reconcile",
            498
        );
    }
    for (column = 0; column < columns; ++column) {
        if (SF_mat_store((char *)coefficient_matrix, 1, (ST_int)(column + 1),
                         coefficients[column]) != 0) {
            status = VCKSS_STATA_MEMORY_ERROR;
            goto projection_store_failure;
        }
    }
    for (row = 0; row < columns; ++row) {
        for (column = 0; column < columns; ++column) {
            const size_t index = (size_t)row * (size_t)columns + (size_t)column;
            if (SF_mat_store((char *)covariance_matrix, (ST_int)(row + 1),
                             (ST_int)(column + 1), covariance[index]) != 0 ||
                SF_mat_store((char *)naive_matrix, (ST_int)(row + 1),
                             (ST_int)(column + 1), naive[index]) != 0) {
                status = VCKSS_STATA_MEMORY_ERROR;
                goto projection_store_failure;
            }
        }
    }
    free(coefficients);
    free(covariance);
    free(naive);
    if ((status = vckss_save_u64("__vckss_proj_result_schema", receipt.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_result_columns", receipt.columns)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_result_effect", receipt.effect)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_result_weight", receipt.weight)) != 0 ||
        (status = vckss_save_double("__vckss_proj_cov_min", receipt.covariance_smallest_eigenvalue)) != 0 ||
        (status = vckss_save_double("__vckss_proj_cov_max", receipt.covariance_largest_eigenvalue)) != 0 ||
        (status = vckss_save_double("__vckss_proj_psd_cleanup", receipt.psd_cleanup)) != 0 ||
        (status = vckss_save_double("__vckss_proj_proxy_min", receipt.proxy_minimum)) != 0 ||
        (status = vckss_save_double("__vckss_proj_proxy_max", receipt.proxy_maximum)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_max_iter", receipt.maximum_iterations)) != 0 ||
        (status = vckss_save_double("__vckss_proj_max_reduced", receipt.maximum_reduced_residual)) != 0 ||
        (status = vckss_save_double("__vckss_proj_max_complete", receipt.maximum_complete_residual)) != 0 ||
        (status = vckss_save_double("__vckss_proj_full_tol", receipt.full_residual_tolerance)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_peak", receipt.projection_peak_forecast_bytes)) != 0 ||
        (status = vckss_save_u64("__vckss_proj_result_bytes", receipt.result_bytes)) != 0) {
        return status;
    }
    return 0;

projection_store_failure:
    free(coefficients);
    free(covariance);
    free(naive);
    return vckss_c_failure(
        VCKSS_ERROR_ALLOCATION_FAILED,
        "ALLOCATION_FAILED",
        "ALLOCATION_FAILED [stata_spi]: could not store a Rust projection result matrix",
        status
    );
}

static int vckss_store_result_matrix(
    const char *name,
    uint64_t rows,
    uint64_t columns,
    const double *values
)
{
    uint64_t row;
    uint64_t column;
    for (row = 0; row < rows; ++row) {
        for (column = 0; column < columns; ++column) {
            const size_t index = (size_t)row * (size_t)columns + (size_t)column;
            if (SF_mat_store((char *)name, (ST_int)(row + 1),
                             (ST_int)(column + 1), values[index]) != 0) {
                return VCKSS_STATA_MEMORY_ERROR;
            }
        }
    }
    return 0;
}

static int vckss_componentresult(int argc, char *argv[])
{
    VckssComponentInferenceResultReceiptV2 receipt;
    uint64_t generation = 0;
    uint32_t reference = 0;
    const int q1_present = argc == 11;
    const uint64_t q1_entries = q1_present ? 56u : 0u;
    const uint64_t total_entries = 9u + 16u + 9u + 60u + q1_entries + 24u + 150u + 490u;
    double *storage = NULL;
    double *primitive;
    double *covariance;
    double *mcse;
    double *spectrum;
    double *q1;
    double *summaries;
    double *folds;
    double *cv;
    uint32_t target;
    int status;

    if ((argc != 10 && argc != 11) ||
        vckss_parse_u64(argv[1], &generation) != 0 || generation == 0 ||
        vckss_parse_component_reference(argv[2], &reference) != 0 ||
        (q1_present != (reference == VCKSS_COMPONENT_REFERENCE_Q1))) {
        return vckss_usage(
            "Rust componentresult requires generation, q0 plus seven matrices, or q1 plus eight matrices"
        );
    }
    if (SF_row(argv[3]) != 3 || SF_col(argv[3]) != 3 ||
        SF_row(argv[4]) != 4 || SF_col(argv[4]) != 4 ||
        SF_row(argv[5]) != 3 || SF_col(argv[5]) != 3 ||
        SF_row(argv[6]) != 4 || SF_col(argv[6]) != 15 ||
        SF_row(argv[q1_present ? 8 : 7]) != 2 || SF_col(argv[q1_present ? 8 : 7]) != 12 ||
        SF_row(argv[q1_present ? 9 : 8]) != 10 || SF_col(argv[q1_present ? 9 : 8]) != 15 ||
        SF_row(argv[q1_present ? 10 : 9]) != 70 || SF_col(argv[q1_present ? 10 : 9]) != 7 ||
        (q1_present && (SF_row(argv[7]) != 4 || SF_col(argv[7]) != 14))) {
        return vckss_usage("Rust componentresult matrices have invalid dimensions");
    }
    storage = (double *)vckss_calloc((size_t)total_entries, sizeof(double));
    if (storage == NULL) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate component-inference result buffers",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    primitive = storage;
    covariance = primitive + 9;
    mcse = covariance + 16;
    spectrum = mcse + 9;
    q1 = q1_present ? spectrum + 60 : NULL;
    summaries = spectrum + 60 + q1_entries;
    folds = summaries + 24;
    cv = folds + 150;
    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_component_inference_result_v2(
        generation,
        primitive, 9,
        covariance, 16,
        mcse, 9,
        spectrum, 60,
        q1, q1_entries,
        summaries, 24,
        folds, 150,
        cv, 490,
        &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) {
        free(storage);
        return vckss_rust_failure(status);
    }
    if (receipt.struct_size != sizeof(receipt) ||
        receipt.schema_version != VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V2 ||
        receipt.generation != generation ||
        (receipt.variance_source != VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON &&
         receipt.variance_source != VCKSS_COMPONENT_VARIANCE_STRUCTURED_LEVERAGE) ||
        receipt.reference_distribution != reference ||
        receipt.q1_present != (uint32_t)q1_present ||
        receipt.probes < 2 || receipt.counter_atoms == 0 || receipt.counter_words == 0 ||
        receipt.peak_forecast_bytes == 0 ||
        !isfinite(receipt.psd_cleanup) || receipt.psd_cleanup < 0.0 ||
        !isfinite(receipt.smallest_eigenvalue_before_cleanup) ||
        !isfinite(receipt.largest_eigenvalue_before_cleanup) ||
        !isfinite(receipt.point_correction_identity_error) ||
        !isfinite(receipt.maximum_reduced_residual) ||
        !isfinite(receipt.maximum_complete_residual) ||
        !isfinite(receipt.full_residual_tolerance) ||
        receipt.maximum_complete_residual > receipt.full_residual_tolerance ||
        receipt.structured_schema_version == 0 ||
        receipt.fold_rows != 10 || receipt.cv_rows != 70 ||
        !isfinite(receipt.median_absolute_log_ratio) ||
        !isfinite(receipt.p90_absolute_log_ratio) ||
        !isfinite(receipt.maximum_absolute_log_ratio) ||
        !isfinite(receipt.log_variance_correlation)) {
        free(storage);
        return vckss_c_failure(
            VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
            "INTERNAL_INVARIANT_FAILED",
            "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust component-inference result receipt did not reconcile",
            498
        );
    }
    for (target = 0; target < 4u; ++target) {
        const double concentration = spectrum[(size_t)target * 15u + 14u];
        if (!isfinite(concentration) || concentration <= 0.0 || concentration > 1.0) {
            free(storage);
            return vckss_c_failure(
                VCKSS_ERROR_INTERNAL_INVARIANT_FAILED,
                "INTERNAL_INVARIANT_FAILED",
                "INTERNAL_INVARIANT_FAILED [stata_spi]: Rust component-inference influence concentration did not reconcile",
                498
            );
        }
    }
    if ((status = vckss_store_result_matrix(argv[3], 3, 3, primitive)) != 0 ||
        (status = vckss_store_result_matrix(argv[4], 4, 4, covariance)) != 0 ||
        (status = vckss_store_result_matrix(argv[5], 3, 3, mcse)) != 0 ||
        (status = vckss_store_result_matrix(argv[6], 4, 15, spectrum)) != 0 ||
        (q1_present && (status = vckss_store_result_matrix(argv[7], 4, 14, q1)) != 0) ||
        (status = vckss_store_result_matrix(argv[q1_present ? 8 : 7], 2, 12, summaries)) != 0 ||
        (status = vckss_store_result_matrix(argv[q1_present ? 9 : 8], 10, 15, folds)) != 0 ||
        (status = vckss_store_result_matrix(argv[q1_present ? 10 : 9], 70, 7, cv)) != 0) {
        free(storage);
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not store a Rust component-inference result matrix",
            status
        );
    }
    free(storage);
    if ((status = vckss_save_u64("__vckss_comp_schema", receipt.schema_version)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_model", receipt.variance_source)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_reference", receipt.reference_distribution)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_probes", receipt.probes)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_atoms", receipt.counter_atoms)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_words", receipt.counter_words)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_peak", receipt.peak_forecast_bytes)) != 0 ||
        (status = vckss_save_double("__vckss_comp_psd", receipt.psd_cleanup)) != 0 ||
        (status = vckss_save_double("__vckss_comp_eig_min", receipt.smallest_eigenvalue_before_cleanup)) != 0 ||
        (status = vckss_save_double("__vckss_comp_eig_max", receipt.largest_eigenvalue_before_cleanup)) != 0 ||
        (status = vckss_save_double("__vckss_comp_point_err", receipt.point_correction_identity_error)) != 0 ||
        (status = vckss_save_u64("__vckss_comp_max_iter", receipt.maximum_iterations)) != 0 ||
        (status = vckss_save_double("__vckss_comp_max_reduced", receipt.maximum_reduced_residual)) != 0 ||
        (status = vckss_save_double("__vckss_comp_max_complete", receipt.maximum_complete_residual)) != 0 ||
        (status = vckss_save_double("__vckss_comp_full_tol", receipt.full_residual_tolerance)) != 0 ||
        (status = vckss_save_double("__vckss_comp_logratio_med", receipt.median_absolute_log_ratio)) != 0 ||
        (status = vckss_save_double("__vckss_comp_logratio_p90", receipt.p90_absolute_log_ratio)) != 0 ||
        (status = vckss_save_double("__vckss_comp_logratio_max", receipt.maximum_absolute_log_ratio)) != 0 ||
        (status = vckss_save_double("__vckss_comp_logvar_corr", receipt.log_variance_correlation)) != 0) {
        return status;
    }
    return 0;
}

#ifdef VCKSS_CSHIM_TEST
int vckss_cshim_test_allocation_failure(void)
{
    void *allocation;
    vckss_test_fail_allocation = 1;
    allocation = vckss_calloc(1u, 1u);
    if (allocation == NULL) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: injected C allocation failure",
            VCKSS_STATA_MEMORY_ERROR
        );
    }
    free(allocation);
    return 0;
}
#endif

static int vckss_snapshot(void)
{
    VckssEngineSnapshotV1 snapshot;
    int status;

    memset(&snapshot, 0, sizeof(snapshot));
    status = vckss_rust_engine_snapshot_v1(&snapshot, (uint32_t)sizeof(snapshot));
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if ((status = vckss_save_u64("__vckss_rust_state", snapshot.state)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_handle", snapshot.generation)) != 0 ||
        (status = vckss_save_u64("__vckss_rust_last_released", snapshot.last_released_generation)) != 0) {
        return status;
    }
    return 0;
}

ST_retcode vckss_stata_call_impl(int argc, char *argv[])
{
    uint64_t generation;
    int status;

    if (argc < 1 || argv == NULL || argv[0] == NULL) {
        vckss_clear_error_transport();
        return vckss_usage("Rust plugin command required");
    }
    if (strcmp(argv[0], "lasterror") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust lasterror does not accept arguments");
        }
        return vckss_export_last_error();
    }
    /* Every operation starts with an empty transport. A successful call
     * cannot leak a prior failure; lasterror above is the sole nonclearing
     * command and never calls the Rust ABI last-error accessor. */
    vckss_clear_error_transport();
    if (strcmp(argv[0], "capabilities") != 0 &&
        strcmp(argv[0], "version") != 0 &&
        strcmp(argv[0], "probe") != 0) {
        status = vckss_verify_abi(NULL);
        if (status != 0) {
            return status;
        }
    }
    if (strcmp(argv[0], "capabilities") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust capabilities does not accept arguments");
        }
        SF_display((char *)vckss_rust_capabilities_json());
        SF_display("\n");
        return 0;
    }
    if (strcmp(argv[0], "version") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust version does not accept arguments");
        }
        SF_display((char *)vckss_rust_backend_version());
        SF_display("\n");
        return 0;
    }
    if (strcmp(argv[0], "selftest") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust selftest does not accept arguments");
        }
        return vckss_run_selftest();
    }
    if (strcmp(argv[0], "probe") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust probe does not accept arguments");
        }
        return vckss_probe();
    }
    if (strcmp(argv[0], "requestcapability") == 0) {
        return vckss_request_capability(argc, argv);
    }
    if (strcmp(argv[0], "prepare") == 0) {
        return vckss_prepare(argc, argv);
    }
    if (strcmp(argv[0], "augmentstayers") == 0) {
        return vckss_augment_stayers(argc, argv);
    }
    if (strcmp(argv[0], "augmentprojection") == 0) {
        return vckss_augment_projection(argc, argv);
    }
    if (strcmp(argv[0], "augmentcomponent") == 0) {
        return vckss_augment_component_inference(argc, argv);
    }
    if (strcmp(argv[0], "solve") == 0) {
        return vckss_solve(argc, argv);
    }
    if (strcmp(argv[0], "solvefull") == 0) {
        return vckss_solve_full(argc, argv);
    }
    if (strcmp(argv[0], "result") == 0) {
        if (argc != 2 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust result requires one positive integer generation");
        }
        return vckss_result(generation);
    }
    if (strcmp(argv[0], "fullcmgreceipt") == 0) {
        if (argc != 2 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust fullcmgreceipt requires one positive integer generation");
        }
        return vckss_full_cmg_receipt(generation);
    }
    if (strcmp(argv[0], "stayerresult") == 0) {
        if (argc != 2 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust stayerresult requires one positive integer generation");
        }
        return vckss_stayer_result(generation);
    }
    if (strcmp(argv[0], "rhsresult") == 0) {
        if (argc != 3 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust rhsresult requires a positive generation and a matrix name");
        }
        return vckss_rhsresult(generation, argv[2]);
    }
    if (strcmp(argv[0], "projectionresult") == 0) {
        uint64_t columns;
        if (argc != 6 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0 ||
            vckss_parse_u64(argv[2], &columns) != 0 || columns == 0) {
            return vckss_usage(
                "Rust projectionresult requires generation, q, coefficient, covariance, and naive-covariance matrices"
            );
        }
        return vckss_projectionresult(generation, columns, argv[3], argv[4], argv[5]);
    }
    if (strcmp(argv[0], "componentresult") == 0) {
        return vckss_componentresult(argc, argv);
    }
    if (strcmp(argv[0], "snapshot") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust snapshot does not accept arguments");
        }
        return vckss_snapshot();
    }
    if (strcmp(argv[0], "clear") == 0) {
        if (argc != 1) {
            return vckss_usage("Rust clear does not accept arguments");
        }
        status = vckss_rust_engine_clear_abandoned_v1();
        return status == 0 ? 0 : vckss_rust_failure(status);
    }
    if (strcmp(argv[0], "release") == 0) {
        if (argc != 2 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust release requires one positive integer generation");
        }
        status = vckss_rust_engine_release_v1(generation);
        return status == 0 ? 0 : vckss_rust_failure(status);
    }
    return vckss_usage("unknown Rust plugin command");
}
