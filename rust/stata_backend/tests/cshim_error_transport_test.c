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

static int fail_scalar;
static int fail_matrix;
static int error_calls;
static int release_calls;
static int clear_calls;
static int release_status;
static int clear_status;
static uint64_t released_generation;
static int selftest_status;
static const char *selftest_error;

int32_t vckss_rust_selftest(void)
{
    return selftest_status;
}

const char *vckss_rust_last_error(void)
{
    return selftest_error;
}

static ST_int mock_error(char *message)
{
    assert(message != NULL);
    ++error_calls;
    return 0;
}

static ST_int mock_scalar_save(char *name, ST_double value)
{
    (void)name;
    (void)value;
    return fail_scalar;
}

static ST_int mock_matrix_store(char *name, ST_int row, ST_int column, ST_double value)
{
    (void)name;
    (void)row;
    (void)column;
    (void)value;
    return fail_matrix;
}

static int mock_cleanup_release(uint64_t generation)
{
    ++release_calls;
    released_generation = generation;
    return release_status;
}

static int mock_cleanup_clear(void)
{
    ++clear_calls;
    return clear_status;
}

static void reset_transport(void)
{
    fail_scalar = 0;
    fail_matrix = 0;
    error_calls = 0;
    release_calls = 0;
    clear_calls = 0;
    release_status = 0;
    clear_status = 0;
    released_generation = 0;
    selftest_status = 0;
    selftest_error = "OK";
    vckss_clear_error_transport();
}

int main(void)
{
    ST_plugin plugin = {0};
    plugin.spouterr = mock_error;
    plugin.scalsave = mock_scalar_save;
    plugin.safematstore = mock_matrix_store;
    plugin.matstore = mock_matrix_store;
    _stata_ = &plugin;

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
