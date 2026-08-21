/* SPDX-License-Identifier: GPL-3.0-only */

#include "stplugin.h"
#include "vckss_rust.h"
#include "stata_interrupt.h"

#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define VCKSS_STATA_USAGE_ERROR 198
#define VCKSS_STATA_MEMORY_ERROR 909
#define VCKSS_PREPARE_VARIABLES 8
#define VCKSS_NUMERIC_COLUMNS 6
#define VCKSS_INGEST_POLL_INTERVAL UINT64_C(4096)
#define VCKSS_STATA_SCALAR_NAME_LIMIT 32
#define VCKSS_MAX_EXACT_STATA_INTEGER UINT64_C(9007199254740992)
#define VCKSS_ERROR_STATUS_CAPACITY 64
#define VCKSS_ERROR_DETAIL_CAPACITY 1024
#define VCKSS_ERROR_ALLOCATION_FAILED 41
#define VCKSS_ERROR_INTERNAL_INVARIANT_FAILED 90
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
    int status;

    memset(&capabilities, 0, sizeof(capabilities));
    status = vckss_rust_backend_capabilities_v1(
        &capabilities, (uint32_t)sizeof(capabilities)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (runtime_abi != VCKSS_RUST_ABI_VERSION_V1 ||
        capabilities.abi_version != VCKSS_RUST_ABI_VERSION_V1 ||
        capabilities.struct_size != sizeof(capabilities) ||
        capabilities.reserved != 0) {
        vckss_store_error_transport(
            10,
            "ABI_MISMATCH",
            "ABI_MISMATCH [stata_spi]: Rust plugin ABI does not match the compiled Stata shim"
        );
        SF_error("Rust plugin ABI does not match the compiled Stata shim");
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

static int vckss_selected_observations(uint64_t *selected)
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
    if (count == 0) {
        return vckss_usage("the marked sample is empty");
    }
    *selected = count;
    return 0;
}

static int vckss_copy_marked_columns(uint64_t rows, double **storage)
{
    ST_int observation;
    uint64_t row = 0;
    ST_double value = 0.0;
    double *buffer;
    size_t entries;
    int variable;
    uint64_t visited = 0;

    if (rows > (uint64_t)SIZE_MAX ||
        (size_t)rows > SIZE_MAX / (VCKSS_NUMERIC_COLUMNS * sizeof(double))) {
        return vckss_usage("marked-sample column allocation overflow");
    }
    entries = (size_t)rows * VCKSS_NUMERIC_COLUMNS;
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
        for (variable = 0; variable < VCKSS_NUMERIC_COLUMNS; ++variable) {
            if (SF_vdata(variable + 2, observation, &value) != 0) {
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

static int vckss_export_preparation(uint64_t generation, uint64_t rows)
{
    VckssEnginePreparationReceiptV3 receipt_v3;
    VckssEnginePreparationReceiptV2 receipt;
    uint8_t *mask;
    ST_int observation;
    ST_double touse;
    uint64_t row = 0;
    int status;
    uint64_t visited = 0;

    memset(&receipt_v3, 0, sizeof(receipt_v3));
    status = vckss_rust_engine_preparation_receipt_v3(
        generation, &receipt_v3, (uint32_t)sizeof(receipt_v3)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
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
        if (SF_vstore(8, observation, (ST_double)mask[row]) != 0) {
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
    VckssEnginePrepareRequestInterruptV1 request;
    VckssEngineColumnsV1 columns;
    uint64_t rows = 0;
    uint64_t caller_copy_bytes = 0;
    uint64_t generation = 0;
    double *storage = NULL;
    int status;

    if (SF_nvars() != VCKSS_PREPARE_VARIABLES || argc != 3) {
        return vckss_usage(
            "Rust prepare requires touse, six numeric inputs, retained output, a cleanup flag, and a byte memory limit"
        );
    }
    if (strcmp(argv[1], "cleanup") != 0 && strcmp(argv[1], "nocleanup") != 0) {
        return vckss_usage("Rust prepare cleanup flag must be cleanup or nocleanup");
    }
    if ((status = vckss_selected_observations(&rows)) != 0) {
        return status;
    }
    if (rows > UINT64_MAX / (VCKSS_NUMERIC_COLUMNS * sizeof(double))) {
        return vckss_usage("marked-sample caller-copy byte count overflow");
    }
    caller_copy_bytes = rows * VCKSS_NUMERIC_COLUMNS * sizeof(double);

    status = vckss_rust_engine_default_prepare_request_interrupt_v1(
        &request, (uint32_t)sizeof(request)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    request.options.abi_version = VCKSS_RUST_ABI_VERSION_V1;
    request.options.rows = rows;
    request.options.cleanup_abandoned = strcmp(argv[1], "cleanup") == 0 ? 1u : 0u;
    if (vckss_parse_u64(argv[2], &request.options.memory_limit_bytes) != 0 ||
        request.options.memory_limit_bytes == 0) {
        return vckss_usage("invalid Rust whole-command byte memory limit");
    }
    request.options.caller_copy_bytes = caller_copy_bytes;
    request.interrupt_poll = vckss_stata_interrupt_poll;
    request.interrupt_context = NULL;
    request.checkpoint_interval = 1u;
    status = vckss_rust_engine_admit_prepare_v2(&request.options);
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    if (request.options.cleanup_abandoned == 1u) {
        status = vckss_rust_engine_clear_abandoned_v1();
        if (status != 0) {
            return vckss_rust_failure(status);
        }
        request.options.cleanup_abandoned = 0u;
    }
    status = vckss_copy_marked_columns(rows, &storage);
    if (status != 0) {
        return status;
    }

    memset(&columns, 0, sizeof(columns));
    columns.struct_size = (uint32_t)sizeof(columns);
    columns.rows = rows;
    columns.worker = storage;
    columns.firm = storage + rows;
    columns.deletion = storage + 2 * rows;
    columns.outcome = storage + 3 * rows;
    columns.frequency = storage + 4 * rows;
    columns.target_weight = storage + 5 * rows;

    status = vckss_rust_engine_prepare_interrupt_v1(
        &request, &columns, &generation, (uint32_t)sizeof(generation)
    );
    free(storage);
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    status = vckss_export_preparation(generation, rows);
    if (status != 0) {
        vckss_cleanup_preserving_primary(generation);
    }
    return status;
}

static int vckss_solve(int argc, char *argv[])
{
    VckssEngineSolveRequestInterruptV1 request;
    uint64_t generation;
    int status;

    if (argc != 9) {
        char message[96];
        (void)snprintf(message, sizeof(message), "Rust solve expected 9 arguments but received %d", argc);
        return vckss_usage(message);
    }
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
        char message[160];
        (void)snprintf(
            message,
            sizeof(message),
            "invalid Rust PCG tolerance: '%s'",
            argv[7] == NULL ? "(null)" : argv[7]
        );
        return vckss_usage(message);
    }
    if (vckss_parse_u32(argv[8], &request.options.maximum_iterations) != 0) {
        return vckss_usage("invalid Rust maximum iteration count");
    }
    status = vckss_rust_engine_solve_interrupt_v1(generation, &request);
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
    VckssEngineDetailedReceiptV3 receipt_v3;
    VckssEngineDetailedReceiptV2 receipt;
    int status;

    memset(&result, 0, sizeof(result));
    memset(&receipt_v3, 0, sizeof(receipt_v3));
    status = vckss_rust_engine_result_v1(generation, &result, (uint32_t)sizeof(result));
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    status = vckss_rust_engine_detailed_receipt_v3(
        generation, &receipt_v3, (uint32_t)sizeof(receipt_v3)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    receipt = receipt_v3.v2;
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
        (status = vckss_save_u64("__vckss_rust_command_peak", receipt.command_peak_forecast_bytes)) != 0) {
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
    VckssEngineDetailedReceiptV3 receipt;
    VckssEngineRhsReceiptV1 *rows = NULL;
    uint64_t row_count;
    uint64_t row;
    int status;

    if (matrix_name == NULL || *matrix_name == '\0') {
        return vckss_usage("Rust rhsresult requires a Stata matrix name");
    }
    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_engine_detailed_receipt_v3(
        generation, &receipt, (uint32_t)sizeof(receipt)
    );
    if (status != 0) {
        return vckss_rust_failure(status);
    }
    row_count = receipt.rhs_receipt_rows;
    if (row_count == 0 || row_count > (uint64_t)SIZE_MAX / sizeof(*rows)) {
        return vckss_usage("Rust RHS receipt row count is not allocatable");
    }
    if (SF_row((char *)matrix_name) != (ST_int)row_count ||
        SF_col((char *)matrix_name) != 8) {
        return vckss_usage("Rust rhsresult matrix must have the exact reported row count and eight columns");
    }
    rows = (VckssEngineRhsReceiptV1 *)vckss_calloc((size_t)row_count, sizeof(*rows));
    if (rows == NULL) {
        return vckss_c_failure(
            VCKSS_ERROR_ALLOCATION_FAILED,
            "ALLOCATION_FAILED",
            "ALLOCATION_FAILED [stata_spi]: could not allocate the Rust RHS receipt caller copy",
            VCKSS_STATA_MEMORY_ERROR
        );
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
            if (vckss_store_rhs_matrix_cell(
                    matrix_name,
                    (ST_int)(row + 1),
                    (ST_int)(column + 1),
                    values[column]
                ) != 0) {
                free(rows);
                return VCKSS_STATA_MEMORY_ERROR;
            }
        }
    }
    free(rows);
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
    if (strcmp(argv[0], "prepare") == 0) {
        return vckss_prepare(argc, argv);
    }
    if (strcmp(argv[0], "solve") == 0) {
        return vckss_solve(argc, argv);
    }
    if (strcmp(argv[0], "result") == 0) {
        if (argc != 2 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust result requires one positive integer generation");
        }
        return vckss_result(generation);
    }
    if (strcmp(argv[0], "rhsresult") == 0) {
        if (argc != 3 || vckss_parse_u64(argv[1], &generation) != 0 || generation == 0) {
            return vckss_usage("Rust rhsresult requires a positive generation and a matrix name");
        }
        return vckss_rhsresult(generation, argv[2]);
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
