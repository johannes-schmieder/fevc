/* SPDX-License-Identifier: GPL-3.0-only
 * Load the exact qualified thin/universal plugin, without Stata host services,
 * and exercise V6 through the compiled C ABI. Synthetic data only. */
#include "vckss_rust.h"
#include <assert.h>
#include <dlfcn.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>

#define LOAD(name) __typeof__(&name) api_##name = (__typeof__(&name))dlsym(library, #name); assert(api_##name)
#define OK(call) do { int rc = (call); if (rc) { fprintf(stderr, "%s: %d %s\n", #call, rc, api_vckss_rust_engine_last_error()); return 1; } } while (0)

struct poll_state { pthread_t caller; unsigned calls, stop; int wrong_thread; };
static int32_t poll_caller(void *context) {
    struct poll_state *state = context;
    state->wrong_thread |= !pthread_equal(state->caller, pthread_self());
    return ++state->calls >= state->stop ? VCKSS_INTERRUPT_USER_BREAK : VCKSS_INTERRUPT_CONTINUE;
}

int main(int argc, char **argv) {
    assert(argc == 2);
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) { fprintf(stderr, "%s\n", dlerror()); return 1; }
    LOAD(vckss_rust_engine_last_error);
    LOAD(vckss_rust_backend_capabilities_v1);
    LOAD(vckss_rust_backend_request_capability_v3);
    LOAD(vckss_rust_engine_prepare_v3);
    LOAD(vckss_rust_engine_default_solve_request_interrupt_v6);
    LOAD(vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1);
    LOAD(vckss_rust_engine_augment_component_inference_interrupt_v4);
    LOAD(vckss_rust_engine_augment_match_component_inference_interrupt_v4);
    LOAD(vckss_rust_engine_solve_interrupt_v6);
    LOAD(vckss_rust_engine_generic_execution_receipt_v1);
    LOAD(vckss_rust_engine_result_v1);
    LOAD(vckss_rust_engine_release_v1);
    LOAD(vckss_rust_engine_snapshot_v1);
    VckssBackendCapabilitiesV1 readiness;
    OK(api_vckss_rust_backend_capabilities_v1(&readiness, sizeof(readiness)));
    assert(readiness.core_ready_flags & VCKSS_CORE_GENERIC_EXECUTION_V1_READY);
    enum { N = 162 };
    double worker[N], firm[N], deletion[N], outcome[N], weight[N], target[N];
    double reference[2][4] = {{0}};
    for (unsigned grouped = 0; grouped < 2; ++grouped) {
        for (unsigned row = 0; row < N; ++row) {
            unsigned w = row / 18, f = (row / 2) % 9;
            worker[row] = w + 1; firm[row] = f + 1;
            deletion[row] = grouped ? row / 2 + 1 : row + 1;
            double scale = .04 + .008 * w + .005 * f;
            outcome[row] = .31 * w - .23 * f + .09 * (row % 2)
                + (((row * 37 + 11) % 101) / 50. - 1.) * scale;
            weight[row] = 1.; target[row] = .75 + ((row * 13) % 29) / 31.;
        }
        for (unsigned executor = 1; executor <= 2; ++executor) {
            for (unsigned attempt = 0; attempt < 2; ++attempt) {
                VckssEnginePrepareRequestV3 preparation = {0};
                preparation.v2.abi_version = VCKSS_RUST_ABI_VERSION_V1;
                preparation.v2.struct_size = sizeof(preparation);
                preparation.v2.rows = N;
                preparation.v2.memory_limit_bytes = UINT64_C(1) << 30;
                preparation.v2.caller_copy_bytes = N * 48;
                preparation.deletion_mode = grouped ? VCKSS_DELETION_MATCH : VCKSS_DELETION_OBSERVATION;
                VckssEngineColumnsV2 columns = {0};
                columns.v1 = (VckssEngineColumnsV1){sizeof(columns), 0, N, worker, firm, deletion, outcome, weight, target};
                uint64_t generation = 0;
                OK(api_vckss_rust_engine_prepare_v3(&preparation, &columns, &generation, sizeof(generation)));
                VckssComponentInferenceAugmentationRequestInterruptV1 attachment;
                OK(api_vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(&attachment, sizeof(attachment)));
                attachment.options.probes = 33; attachment.options.batch_width = 7;
                attachment.options.spectrum_probes = 17; attachment.options.spectrum_iterations = 64;
                attachment.options.seed = 8675309;
                OK((grouped ? api_vckss_rust_engine_augment_match_component_inference_interrupt_v4
                    : api_vckss_rust_engine_augment_component_inference_interrupt_v4)(generation, &attachment, 513));
                VckssEngineSolveRequestInterruptV6 solve;
                OK(api_vckss_rust_engine_default_solve_request_interrupt_v6(&solve, sizeof(solve)));
                VckssEngineSolveRequestV4 *v4 = &solve.options.v4;
                solve.options.execution_mode = executor; solve.options.threads = 7;
                v4->v3.v2.v1.seed = 8675309; v4->v3.v2.v1.probes = 200;
                v4->v3.v2.v1.pcg_tolerance = 1e-12;
                v4->v3.v2.v1.deletion_mode = preparation.deletion_mode;
                v4->v3.v2.v1.solver_route = executor == 1 ? VCKSS_ROUTE_DIAGONAL_PCG : VCKSS_ROUTE_CMG_PCG;
                v4->v3.v2.nuisance_mode = grouped ? VCKSS_NUISANCE_FIXED_OFFSET : VCKSS_NUISANCE_JOINT;
                v4->v3.target_weight_mode = VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT;
                v4->v3.frequency_use = VCKSS_REQUEST_FREQUENCY_UNIT;
                v4->v3.deletion_unit_source = grouped ? VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT : VCKSS_DELETION_SOURCE_OBSERVATION_ROW;
                VckssBackendRequestCapabilityRequestV3 capability = {0};
                capability.v2.v1 = (VckssBackendRequestCapabilityRequestV1){
                    VCKSS_RUST_ABI_VERSION_V1, sizeof(capability), VCKSS_REQUEST_CAPABILITY_SCHEMA_V3,
                    VCKSS_ALGORITHM_JLA, preparation.deletion_mode, v4->v3.v2.nuisance_mode,
                    v4->v3.v2.v1.solver_route, VCKSS_RNG_COUNTER_V1, 0, VCKSS_REQUEST_FREQUENCY_UNIT, 0};
                capability.v2.engine = VCKSS_ENGINE_GENERIC;
                capability.v2.batch_mode = VCKSS_BATCH_MODE_AUTO;
                capability.v2.stayers_mode = VCKSS_STAYERS_MOVERS;
                capability.v2.target_weight_mode = v4->v3.target_weight_mode;
                capability.v2.deletion_unit_source = v4->v3.deletion_unit_source;
                capability.v2.physical_limit = v4->v3.physical_limit;
                capability.leverage_batch_mode = capability.target_batch_mode = VCKSS_BATCH_MODE_AUTO;
                VckssBackendRequestCapabilityReceiptV3 supported;
                OK(api_vckss_rust_backend_request_capability_v3(&capability, &supported, sizeof(supported)));
                assert(supported.v2.v1.supported == 1);
                v4->v3.request_signature = supported.v2.v1.request_signature;
                struct poll_state poll = {pthread_self(), 0, attempt ? UINT32_MAX : 1, 0};
                solve.interrupt_poll = poll_caller; solve.interrupt_context = &poll; solve.checkpoint_interval = 1;
                int status = api_vckss_rust_engine_solve_interrupt_v6(generation, &solve);
                assert(!poll.wrong_thread && poll.calls > 0);
                if (!attempt) { assert(status == 1); /* ErrorCode::UserBreak */ }
                else {
                    OK(status);
                    VckssGenericExecutionReceiptV1 work;
                    OK(api_vckss_rust_engine_generic_execution_receipt_v1(generation, &work, sizeof(work)));
                    assert(work.generation == generation && work.gram_rhs_count == 513 && work.point_probe_rhs_count == 600);
                    assert(work.maximum_complete_residual <= 1e-11 && work.component_rhs_count > 0);
                    assert(work.logical_rhs_count == work.fit_rhs_count + work.control_projection_rhs_count + work.point_probe_rhs_count
                        + work.projection_rhs_count + work.component_rhs_count + work.gram_rhs_count);
                    assert(executor == 1 ? work.queued_rhs_count + work.fit_rhs_count == work.logical_rhs_count
                        : work.cmg_rhs_count == work.logical_rhs_count + work.control_refinement_rhs_count);
                    VckssEngineResultV1 result;
                    OK(api_vckss_rust_engine_result_v1(generation, &result, sizeof(result)));
                    double values[4] = {result.corrected.worker, result.corrected.firm, result.corrected.covariance, result.corrected.total};
                    for (unsigned i = 0; i < 4; ++i) {
                        assert(isfinite(values[i]));
                        if (executor == 1) reference[grouped][i] = values[i];
                        else assert(fabs(values[i] - reference[grouped][i]) < 1e-8 * fmax(1., fabs(reference[grouped][i])));
                    }
                    printf("PASS deletion=%u executor=%u rhs=%llu residual=%.6g\n", preparation.deletion_mode, executor,
                        (unsigned long long)work.logical_rhs_count, work.maximum_complete_residual);
                }
                OK(api_vckss_rust_engine_release_v1(generation));
                OK(api_vckss_rust_engine_release_v1(generation));
                VckssEngineSnapshotV1 state;
                OK(api_vckss_rust_engine_snapshot_v1(&state, sizeof(state)));
                assert(state.state == 0 && state.generation == 0);
            }
        }
    }
    assert(dlclose(library) == 0);
    puts("NATIVE_V6_SHARED_LIBRARY_SMOKE_PASS");
    return 0;
}
