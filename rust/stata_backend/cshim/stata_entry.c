/* SPDX-License-Identifier: GPL-3.0-only */

#include "stplugin.h"
#include "vckss_rust.h"

#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define VCKSS_STATA_USAGE_ERROR 198
#define VCKSS_STATA_MEMORY_ERROR 909
#define VCKSS_PREPARE_VARIABLES 7
#define VCKSS_NUMERIC_COLUMNS 6

static int vckss_usage(const char *message)
{
    SF_error(message);
    return VCKSS_STATA_USAGE_ERROR;
}

static int vckss_rust_failure(int status, int session_error)
{
    const char *message = session_error != 0
        ? vckss_rust_session_last_error()
        : vckss_rust_last_error();
    if (message == NULL) {
        message = "Rust backend failed without an error message";
    }
    SF_error(message);
    return status;
}

static int vckss_save_scalar(const char *name, uint64_t value)
{
    const double converted = (double)value;
    if ((uint64_t)converted != value) {
        SF_error("Rust receipt integer is not exactly representable by Stata");
        return VCKSS_STATA_MEMORY_ERROR;
    }
    if (SF_scal_save(name, converted) != 0) {
        SF_error("could not save a Rust backend receipt scalar");
        return VCKSS_STATA_MEMORY_ERROR;
    }
    return 0;
}

static int vckss_parse_generation(const char *text, uint64_t *output)
{
    char *end = NULL;
    unsigned long long value;

    if (text == NULL || output == NULL || *text == '\0' || *text == '-') {
        return VCKSS_STATA_USAGE_ERROR;
    }
    errno = 0;
    value = strtoull(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value == 0ULL) {
        return VCKSS_STATA_USAGE_ERROR;
    }
    *output = (uint64_t)value;
    return 0;
}

static int vckss_selected_observations(uint64_t *selected)
{
    ST_int observation;
    uint64_t count = 0;
    ST_double touse = 0.0;

    if (selected == NULL) {
        return VCKSS_STATA_USAGE_ERROR;
    }
    for (observation = SF_in1(); observation <= SF_in2(); ++observation) {
        if (!SF_ifobs(observation)) {
            continue;
        }
        if (SF_vdata(1, observation, &touse) != 0) {
            SF_error("could not read the marked-sample variable");
            return VCKSS_STATA_USAGE_ERROR;
        }
        if (touse != 0.0) {
            if (count == UINT64_MAX) {
                SF_error("marked-sample row count overflow");
                return VCKSS_STATA_MEMORY_ERROR;
            }
            ++count;
        }
    }
    if (count == 0) {
        SF_error("the marked sample is empty");
        return VCKSS_STATA_USAGE_ERROR;
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

    if (storage == NULL || rows > (uint64_t)SIZE_MAX) {
        return VCKSS_STATA_MEMORY_ERROR;
    }
    if ((size_t)rows > SIZE_MAX / (VCKSS_NUMERIC_COLUMNS * sizeof(double))) {
        SF_error("marked-sample column allocation overflow");
        return VCKSS_STATA_MEMORY_ERROR;
    }
    entries = (size_t)rows * VCKSS_NUMERIC_COLUMNS;
    buffer = (double *)calloc(entries, sizeof(double));
    if (buffer == NULL) {
        SF_error("could not allocate marked-sample Rust input columns");
        return VCKSS_STATA_MEMORY_ERROR;
    }

    for (observation = SF_in1(); observation <= SF_in2(); ++observation) {
        if (!SF_ifobs(observation)) {
            continue;
        }
        if (SF_vdata(1, observation, &value) != 0) {
            free(buffer);
            SF_error("could not read the marked-sample variable");
            return VCKSS_STATA_USAGE_ERROR;
        }
        if (value == 0.0) {
            continue;
        }
        if (row >= rows) {
            free(buffer);
            SF_error("marked-sample count changed during column ingestion");
            return VCKSS_STATA_USAGE_ERROR;
        }
        for (variable = 0; variable < VCKSS_NUMERIC_COLUMNS; ++variable) {
            if (SF_vdata(variable + 2, observation, &value) != 0) {
                free(buffer);
                SF_error("could not read a marked-sample numeric column");
                return VCKSS_STATA_USAGE_ERROR;
            }
            buffer[(size_t)variable * (size_t)rows + (size_t)row] = value;
        }
        ++row;
    }
    if (row != rows) {
        free(buffer);
        SF_error("marked-sample count changed during column ingestion");
        return VCKSS_STATA_USAGE_ERROR;
    }
    *storage = buffer;
    return 0;
}

static int vckss_export_preparation_receipt(uint64_t generation)
{
    VckssPreparationReceiptV1 receipt;
    int status;

    memset(&receipt, 0, sizeof(receipt));
    status = vckss_rust_session_preparation_receipt_v1(generation, &receipt);
    if (status != 0) {
        return vckss_rust_failure(status, 1);
    }
    if ((status = vckss_save_scalar("__vckss_rust_handle", receipt.generation)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_input_rows", receipt.input_rows)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_retained_rows", receipt.retained_rows)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_workers", receipt.workers)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_firms", receipt.firms)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_cells", receipt.cells)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_deletion_units", receipt.deletion_units)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_target_strata", receipt.target_strata)) != 0) {
        return status;
    }
    return 0;
}

static int vckss_prepare(int argc, char *argv[])
{
    VckssPrepareRequestV1 request;
    VckssColumnsV1 columns;
    uint64_t rows = 0;
    uint64_t generation = 0;
    uint32_t cleanup = 0;
    double *storage = NULL;
    int status;

    if (SF_nvars() != VCKSS_PREPARE_VARIABLES) {
        return vckss_usage(
            "Rust prepare requires: touse worker firm deletion outcome frequency target_weight"
        );
    }
    if (argc > 2) {
        return vckss_usage("Rust prepare accepts at most one cleanup flag");
    }
    if (argc == 2) {
        if (strcmp(argv[1], "cleanup") == 0) {
            cleanup = 1;
        } else if (strcmp(argv[1], "nocleanup") != 0) {
            return vckss_usage("Rust prepare cleanup flag must be cleanup or nocleanup");
        }
    }
    status = vckss_selected_observations(&rows);
    if (status != 0) {
        return status;
    }
    status = vckss_copy_marked_columns(rows, &storage);
    if (status != 0) {
        return status;
    }

    memset(&request, 0, sizeof(request));
    request.abi_version = vckss_rust_abi_version();
    request.struct_size = (uint32_t)sizeof(request);
    request.rows = rows;
    request.cleanup_abandoned = cleanup;

    memset(&columns, 0, sizeof(columns));
    columns.struct_size = (uint32_t)sizeof(columns);
    columns.rows = rows;
    columns.worker = storage;
    columns.firm = storage + rows;
    columns.deletion = storage + 2 * rows;
    columns.outcome = storage + 3 * rows;
    columns.frequency = storage + 4 * rows;
    columns.target_weight = storage + 5 * rows;

    status = vckss_rust_session_prepare_v1(&request, &columns, &generation);
    free(storage);
    storage = NULL;
    if (status != 0) {
        return vckss_rust_failure(status, 1);
    }
    status = vckss_export_preparation_receipt(generation);
    if (status != 0) {
        (void)vckss_rust_session_release_v1(generation);
        return status;
    }
    return 0;
}

static int vckss_snapshot(void)
{
    VckssSessionSnapshotV1 snapshot;
    int status;

    memset(&snapshot, 0, sizeof(snapshot));
    status = vckss_rust_session_snapshot_v1(&snapshot);
    if (status != 0) {
        return vckss_rust_failure(status, 1);
    }
    if ((status = vckss_save_scalar("__vckss_rust_state", snapshot.state)) != 0 ||
        (status = vckss_save_scalar("__vckss_rust_handle", snapshot.generation)) != 0 ||
        (status = vckss_save_scalar(
             "__vckss_rust_last_released", snapshot.last_released_generation
         )) != 0) {
        return status;
    }
    return 0;
}

STDLL stata_call(int argc, char *argv[])
{
    int status;
    uint64_t generation;

    if (argc < 1 || argv == NULL || argv[0] == NULL) {
        return vckss_usage(
            "Rust plugin command required: capabilities, version, selftest, prepare, snapshot, release, or clear"
        );
    }
    if (strcmp(argv[0], "capabilities") == 0) {
        SF_display(vckss_rust_capabilities_json());
        SF_display("\n");
        return 0;
    }
    if (strcmp(argv[0], "version") == 0) {
        SF_display(vckss_rust_backend_version());
        SF_display("\n");
        return 0;
    }
    if (strcmp(argv[0], "selftest") == 0) {
        status = vckss_rust_selftest();
        return status == 0 ? 0 : vckss_rust_failure(status, 0);
    }
    if (strcmp(argv[0], "prepare") == 0) {
        return vckss_prepare(argc, argv);
    }
    if (strcmp(argv[0], "snapshot") == 0) {
        return vckss_snapshot();
    }
    if (strcmp(argv[0], "clear") == 0) {
        status = vckss_rust_session_clear_abandoned_v1();
        return status == 0 ? 0 : vckss_rust_failure(status, 1);
    }
    if (strcmp(argv[0], "release") == 0) {
        if (argc != 2 || vckss_parse_generation(argv[1], &generation) != 0) {
            return vckss_usage("Rust release requires one positive integer generation");
        }
        status = vckss_rust_session_release_v1(generation);
        return status == 0 ? 0 : vckss_rust_failure(status, 1);
    }
    return vckss_usage("unknown Rust plugin command");
}
