/* SPDX-License-Identifier: GPL-3.0-only */

#include "vckss_rust.h"

_Static_assert(sizeof(VckssComponentInferenceResultReceiptV5) == 288, "component V5 layout changed");
_Static_assert(offsetof(VckssComponentInferenceResultReceiptV5, joint_status) == 208, "component V5 prefix changed");

_Static_assert(sizeof(struct VckssPrepareRequestV1) == 24, "legacy request tag changed");
_Static_assert(sizeof(struct VckssColumnsV1) == 64, "legacy columns tag changed");
_Static_assert(sizeof(struct VckssPreparationReceiptV1) == 72, "legacy receipt tag changed");
_Static_assert(sizeof(struct VckssSessionSnapshotV1) == 24, "legacy snapshot tag changed");
_Static_assert(offsetof(VckssEngineDetailedReceiptV5, v4) == 0, "V5 lost V4 prefix");
_Static_assert(offsetof(VckssEnginePreparationReceiptV4, v3) == 0, "preparation V4 lost V3 prefix");
_Static_assert(sizeof(VckssBackendRequestCapabilityRequestV2) == 88, "capability request V2 size changed");
_Static_assert(sizeof(VckssBackendRequestCapabilityReceiptV2) == 104, "capability receipt V2 size changed");
_Static_assert(offsetof(VckssBackendRequestCapabilityRequestV2, v1) == 0, "capability request V2 lost V1 prefix");
_Static_assert(offsetof(VckssBackendRequestCapabilityReceiptV2, v1) == 0, "capability receipt V2 lost V1 prefix");
_Static_assert(offsetof(VckssBackendRequestCapabilityRequestV2, engine) == 48, "capability request V2 engine offset changed");
_Static_assert(offsetof(VckssBackendRequestCapabilityReceiptV2, engine) == 64, "capability receipt V2 engine offset changed");
_Static_assert(sizeof(VckssEngineSolveRequestV3) == 264, "solve request V3 size changed");
_Static_assert(sizeof(VckssEngineSolveRequestInterruptV3) == 288, "interrupt solve request V3 size changed");
_Static_assert(offsetof(VckssEngineSolveRequestV3, v2) == 0, "solve request V3 lost V2 prefix");
_Static_assert(offsetof(VckssEngineSolveRequestV3, engine) == 200, "solve request V3 engine offset changed");
_Static_assert(offsetof(VckssEngineSolveRequestInterruptV3, interrupt_poll) == 264, "interrupt solve request V3 callback offset changed");
_Static_assert(sizeof(VckssEngineDetailedReceiptV6) == 840, "detailed receipt V6 size changed");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, v5) == 0, "V6 lost V5 prefix");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, engine_requested) == 536, "V6 engine offset changed");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, rhs_v2_caller_copy_bytes) == 776, "V6 RHS copy offset changed");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, capability_schema) == 784, "V6 capability echo offset changed");
_Static_assert(offsetof(VckssEngineDetailedReceiptV6, request_signature) == 832, "V6 tail offset changed");
_Static_assert(sizeof(VckssEngineRhsReceiptV2) == 96, "RHS receipt V2 size changed");
_Static_assert(offsetof(VckssEngineRhsReceiptV2, v1) == 0, "RHS receipt V2 lost V1 prefix");
_Static_assert(offsetof(VckssEngineRhsReceiptV2, status) == 48, "RHS receipt V2 status offset changed");
_Static_assert(sizeof(VckssStayerAugmentationRequestV1) == 32, "stayer augmentation request size changed");
_Static_assert(sizeof(VckssStayerAugmentationRequestInterruptV1) == 56, "interrupt stayer augmentation request size changed");
_Static_assert(sizeof(VckssStayerAugmentationColumnsV1) == 72, "stayer augmentation columns size changed");
_Static_assert(sizeof(VckssStayerAugmentationReceiptV1) == 192, "stayer augmentation receipt size changed");
_Static_assert(sizeof(VckssStayerHybridResultV1) == 360, "stayer hybrid result size changed");
_Static_assert(sizeof(VckssEnginePerformanceReceiptV1) == 96, "performance receipt size changed");
_Static_assert(offsetof(VckssEnginePerformanceReceiptV1, ingest_ns) == 32, "performance timing offset changed");
_Static_assert(sizeof(VckssComponentInferenceUnitReceiptV1) == 64, "component unit receipt size changed");
_Static_assert(offsetof(VckssComponentInferenceUnitReceiptV1, independent_units) == 24, "component independent unit offset changed");
_Static_assert(offsetof(VckssComponentInferenceUnitReceiptV1, effective_match_count) == 32, "component match mass offset changed");

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
    int32_t (*request_capability)(
        const VckssBackendRequestCapabilityRequestV1 *,
        VckssBackendRequestCapabilityReceiptV1 *,
        uint32_t
    ) = vckss_rust_backend_request_capability_v1;
    int32_t (*request_capability_v2)(
        const VckssBackendRequestCapabilityRequestV2 *,
        VckssBackendRequestCapabilityReceiptV2 *,
        uint32_t
    ) = vckss_rust_backend_request_capability_v2;
    int32_t (*preparation_v4)(
        uint64_t,
        VckssEnginePreparationReceiptV4 *,
        uint32_t
    ) = vckss_rust_engine_preparation_receipt_v4;
    int32_t (*detailed_v5)(
        uint64_t,
        VckssEngineDetailedReceiptV5 *,
        uint32_t
    ) = vckss_rust_engine_detailed_receipt_v5;
    int32_t (*solve_v3)(uint64_t, const VckssEngineSolveRequestV3 *) =
        vckss_rust_engine_solve_v3;
    int32_t (*solve_interrupt_v3)(
        uint64_t,
        const VckssEngineSolveRequestInterruptV3 *
    ) = vckss_rust_engine_solve_interrupt_v3;
    int32_t (*detailed_v6)(
        uint64_t,
        VckssEngineDetailedReceiptV6 *,
        uint32_t
    ) = vckss_rust_engine_detailed_receipt_v6;
    int32_t (*rhs_v2)(uint64_t, VckssEngineRhsReceiptV2 *, uint64_t) =
        vckss_rust_engine_rhs_receipts_v2;
    int32_t (*augment_stayers)(
        uint64_t,
        const VckssStayerAugmentationRequestV1 *,
        const VckssStayerAugmentationColumnsV1 *
    ) = vckss_rust_engine_augment_stayers_v1;
    int32_t (*stayer_augmentation_receipt)(
        uint64_t,
        VckssStayerAugmentationReceiptV1 *,
        uint32_t
    ) = vckss_rust_engine_stayer_augmentation_receipt_v1;
    int32_t (*stayer_hybrid_result)(
        uint64_t,
        VckssStayerHybridResultV1 *,
        uint32_t
    ) = vckss_rust_engine_stayer_hybrid_result_v1;
    int32_t (*performance_receipt)(
        uint64_t,
        VckssEnginePerformanceReceiptV1 *,
        uint32_t
    ) = vckss_rust_engine_performance_receipt_v1;

    (void)prepare;
    (void)receipt;
    (void)release;
    (void)clear;
    (void)snapshot;
    (void)last_error;
    (void)request_capability;
    (void)request_capability_v2;
    (void)preparation_v4;
    (void)detailed_v5;
    (void)solve_v3;
    (void)solve_interrupt_v3;
    (void)detailed_v6;
    (void)rhs_v2;
    (void)augment_stayers;
    (void)stayer_augmentation_receipt;
    (void)stayer_hybrid_result;
    (void)performance_receipt;
    int32_t (*augment_match_component)(uint64_t,
        const VckssComponentInferenceAugmentationRequestInterruptV1 *) =
        vckss_rust_engine_augment_match_component_inference_interrupt_v1;
    int32_t (*component_units)(uint64_t, VckssComponentInferenceUnitReceiptV1 *, uint32_t) =
        vckss_rust_engine_component_inference_unit_receipt_v1;
    int32_t (*augment_component_v3)(uint64_t,
        const VckssComponentInferenceAugmentationRequestInterruptV1 *) =
        vckss_rust_engine_augment_component_inference_interrupt_v3;
    int32_t (*augment_match_component_v3)(uint64_t,
        const VckssComponentInferenceAugmentationRequestInterruptV1 *) =
        vckss_rust_engine_augment_match_component_inference_interrupt_v3;
    (void)augment_match_component;
    (void)component_units;
    (void)augment_component_v3;
    (void)augment_match_component_v3;
    int32_t (*augment_component_v4)(uint64_t,
        const VckssComponentInferenceAugmentationRequestInterruptV1 *, uint32_t) =
        vckss_rust_engine_augment_component_inference_interrupt_v4;
    int32_t (*augment_match_component_v4)(uint64_t,
        const VckssComponentInferenceAugmentationRequestInterruptV1 *, uint32_t) =
        vckss_rust_engine_augment_match_component_inference_interrupt_v4;
    (void)augment_component_v4;
    (void)augment_match_component_v4;
}
