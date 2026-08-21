/* SPDX-License-Identifier: GPL-3.0-only */

#include "vckss_rust.h"

_Static_assert(sizeof(struct VckssPrepareRequestV1) == 24, "legacy request tag changed");
_Static_assert(sizeof(struct VckssColumnsV1) == 64, "legacy columns tag changed");
_Static_assert(sizeof(struct VckssPreparationReceiptV1) == 72, "legacy receipt tag changed");
_Static_assert(sizeof(struct VckssSessionSnapshotV1) == 24, "legacy snapshot tag changed");

void vckss_legacy_header_signatures_compile(void)
{
    int32_t (*prepare)(
        const VckssPrepareRequestV1 *,
        const VckssColumnsV1 *,
        uint64_t *
    ) = vckss_rust_session_prepare_v1;
    int32_t (*receipt)(uint64_t, VckssPreparationReceiptV1 *) =
        vckss_rust_session_preparation_receipt_v1;
    int32_t (*release)(uint64_t) = vckss_rust_session_release_v1;
    int32_t (*clear)(void) = vckss_rust_session_clear_abandoned_v1;
    int32_t (*snapshot)(VckssSessionSnapshotV1 *) = vckss_rust_session_snapshot_v1;
    const char *(*last_error)(void) = vckss_rust_session_last_error;

    (void)prepare;
    (void)receipt;
    (void)release;
    (void)clear;
    (void)snapshot;
    (void)last_error;
}
