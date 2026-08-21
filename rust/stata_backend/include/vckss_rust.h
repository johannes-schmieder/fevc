/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef VCKSS_RUST_H
#define VCKSS_RUST_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

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

uint32_t vckss_rust_abi_version(void);
const char *vckss_rust_backend_version(void);
const char *vckss_rust_capabilities_json(void);
const char *vckss_rust_last_error(void);
int32_t vckss_rust_selftest(void);

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

#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(VckssPrepareRequestV1) == 24, "unexpected prepare request ABI size");
_Static_assert(sizeof(VckssColumnsV1) == 64, "unexpected column descriptor ABI size");
_Static_assert(sizeof(VckssPreparationReceiptV1) == 72, "unexpected preparation receipt ABI size");
_Static_assert(sizeof(VckssSessionSnapshotV1) == 24, "unexpected snapshot ABI size");
#endif

#ifdef __cplusplus
}
#endif

#endif /* VCKSS_RUST_H */
