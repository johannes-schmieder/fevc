version 18.0
clear all
set more off
set varabbrev off

capture confirm file "kss_bc/kss_bc_resource.mata"
if _rc {
    di as error "run the scale-resource test from the repository root"
    exit 601
}
adopath ++ "`c(pwd)'/kss_bc"

quietly do "kss_bc/kss_bc_resource.mata"
quietly do "kss_bc/kss_bc.mata"
quietly do "kss_bc/kss_bc_cmg.mata"
quietly do "kss_bc/kss_bc_solver.mata"

mata:
void test_scale_resource()
{
    real scalar gib
    struct kssbc_resource_components scalar components, large, generic_parts
    struct kssbc_resource_forecast scalar compressed, generic, pass25, fail30
    struct kssbc_resource_forecast scalar wall_pass, wall_fail, generic_fail
    struct kssbc_resource_forecast scalar route_pass, route_fail
    struct kssbc_resource_model scalar model1, model4, model16
    struct kssbc_resource_model scalar physical4, chunk_heavy, cal_cz24
    struct kssbc_resource_model scalar cal_cz25, cal_cz18
    struct kssbc_resource_selection scalar selected
    struct kssbc_resource_reconciliation scalar reconciled, understated
    real rowvector solver_gate

    assert(kssbc_resource__api_level() == 5)
    assert(kssbc_resource__build_id() ==
        "kss-bc-resource-api5-transition-highwater")
    assert(kssbc_solver__api_level() == 22)
    assert(kssbc_solver__build_id() ==
        "kss-bc-solver-api22-runtime-residency-receipt")
    assert(kssbc_resource__hard_mem_bytes() == 56*1024^3)
    assert(kssbc_resource__hard_wall_secs() == 12*60*60)
    assert(kssbc_resource__wall_margin() == 0.50)
    assert(kssbc_resource__rng_call_upper() == 0.001)
    assert(cols(kssbc_resource__component_names()) == 11)
    assert(kssbc_resource__runtime_rss() == 96*1024^2)

    components = kssbc_resource__empty_components()
    components.raw_stata_bytes = 10
    components.cell_bytes = 1
    components.deletion_unit_bytes = 2
    components.target_stratum_bytes = 3
    components.cmg_hierarchy_bytes = 4
    components.phase_scratch_bytes = 5
    components.sorting_compression_bytes = 6
    components.solve_ahead_bytes = 7
    components.output_certificate_bytes = 8
    components.preservation_transition_bytes = 9

    compressed = kssbc_resource__forecast(
        "compressed",components,100,0.30)
    assert(compressed.status == "ADMITTED")
    assert(compressed.before_rng == 1)
    assert(compressed.persistent_compressed_bytes == 6)
    assert(compressed.selection_peak_bytes == 24)
    assert(compressed.transition_peak_bytes == 39)
    assert(compressed.numerical_peak_bytes == 30)
    assert(compressed.restoration_peak_bytes == 27)
    assert(compressed.peak_bytes == 39)
    assert(compressed.peak_phase == "transition")
    assert(compressed.memory_admission_bytes == 51)
    assert(compressed.wall_admission_seconds == 150)

    // The generic route retains the raw dataset at numerical peak.
    generic = kssbc_resource__forecast("generic",components,100,0.30)
    assert(generic.status == "ADMITTED")
    assert(generic.numerical_peak_bytes == 40)
    assert(generic.peak_bytes == 40)
    assert(generic.peak_phase == "numerical")
    assert(kssbc_resource__nonsolver_peak("compressed",components) == 26)
    assert(kssbc_resource__nonsolver_peak("generic",components) == 36)

    // Twenty-five and thirty percent are both supported, but the selected
    // headroom is part of the admission decision rather than a report-only
    // annotation.
    gib = kssbc_resource__gib()
    large = kssbc_resource__empty_components()
    large.raw_stata_bytes = 44*gib
    pass25 = kssbc_resource__forecast("compressed",large,100,0.25)
    fail30 = kssbc_resource__forecast("compressed",large,100,0.30)
    assert(pass25.status == "ADMITTED")
    assert(pass25.memory_admission_bytes == 55*gib)
    assert(fail30.status == "RESOURCE_ADMISSION_FAILED")
    assert(fail30.memory_admitted == 0)
    assert(fail30.before_rng == 1)

    // A 50% wall margin is mandatory.  The central/upper forecast may use
    // at most eight hours to fit within a twelve-hour hard limit.
    wall_pass = kssbc_resource__forecast(
        "compressed",components,8*60*60,0.25)
    wall_fail = kssbc_resource__forecast(
        "compressed",components,8*60*60+1,0.25)
    assert(wall_pass.status == "ADMITTED")
    assert(wall_pass.wall_admission_seconds == 12*60*60)
    assert(wall_fail.status == "RESOURCE_ADMISSION_FAILED")
    assert(wall_fail.wall_admitted == 0)

    // The registered KSS-PROD-1 anchored dimension model exposes every
    // allocation family.  Its initial 4x compressed projection is admitted,
    // while the row-resident generic route is rejected on memory.  A 16x
    // command remains fail closed on the conservative wall envelope.
    model1 = kssbc_resource__model(
        8201888,8201888,311730,311730,311730,117529,10603,128131,
        200,32,16,1,1,kssbc_resource__rng_call_upper(),
        1*gib,56*gib,12*60*60)
    assert(model1.status == "MODELED")
    assert(model1.row_scale == 1 & model1.structure_scale == 1)
    assert(model1.physical_scale == 1)
    assert(model1.rng_total_calls == 400)
    assert(model1.rng_wall_upper_seconds == 0.4)
    assert(model1.compressed_components.runtime_resident_bytes == 96*1024^2)
    assert(model1.generic_components.runtime_resident_bytes == 96*1024^2)
    assert(cols(kssbc_resource__forecast_vector(model1.compressed)) == 15)

    // Final-source CZ24/CZ25 P200 jobs measured a maximum omitted residency
    // charge of 63,906,719 bytes.  The calibrated 96-MiB component makes both
    // preserved selection-phase RSS peaks fit the central upper forecast.
    cal_cz24 = kssbc_resource__model(
        256472,256472,10343,10343,10343,4063,1285,5347,
        200,32,32,1,1,kssbc_resource__rng_call_upper(),
        106808536,56*gib,480)
    cal_cz25 = kssbc_resource__model(
        390128,390128,15097,15097,15097,5825,1780,7604,
        200,32,32,1,1,kssbc_resource__rng_call_upper(),
        243123745,56*gib,780)
    assert(cal_cz24.compressed.selection_peak_bytes == 254883048)
    assert(cal_cz25.compressed.selection_peak_bytes == 415746657)
    assert(cal_cz24.compressed.selection_peak_bytes >= 154001408)
    assert(cal_cz25.compressed.selection_peak_bytes >= 366505984)

    // Final-source CZ18 P40 job 7203882 exposed a row-scaled allocator
    // high-water omission during compression transition.  The observed
    // process peak exceeded the API4 forecast by 146,872,938 bytes, or
    // 17.91 bytes per retained row.  Charge a conservative 32 bytes per
    // retained row to the sorting/compression family, separately from the
    // fixed 96-MiB runtime residency family and the 30% admission margin.
    cal_cz18 = kssbc_resource__model(
        8201888,8201888,311730,311730,311730,117529,10603,128131,
        40,32,32,1,1,kssbc_resource__rng_call_upper(),
        3943540425,56*gib,5280)
    assert(cal_cz18.compressed_components.sorting_compression_bytes ==
        1478446048)
    assert(cal_cz18.compressed.selection_peak_bytes == 5554633033)
    assert(cal_cz18.compressed.transition_peak_bytes == 5854808470.25)
    assert(cal_cz18.compressed.transition_peak_bytes >= 5707063296)
    assert(cal_cz18.compressed.peak_bytes >= 5739220992)
    model4 = kssbc_resource__model(
        4*8201888,4*8201888,4*311730,4*311730,4*311730,
        4*117529,4*10603,4*128131,200,32,16,
        1,1,kssbc_resource__rng_call_upper(),
        4*gib,56*gib,12*60*60)
    assert(model4.status == "MODELED")
    assert(model4.compressed.status == "ADMITTED")
    assert(model4.generic.status == "GENERIC_RESOURCE_ADMISSION_FAILED")
    assert(model4.compressed_components.solve_ahead_bytes == 0)
    model16 = kssbc_resource__model(
        16*8201888,16*8201888,16*311730,16*311730,16*311730,
        16*117529,16*10603,16*128131,200,16,8,
        1,1,kssbc_resource__rng_call_upper(),
        16*gib,56*gib,12*60*60)
    assert(model16.status == "MODELED")
    assert(model16.compressed.wall_admitted == 0)

    // Literal physical mass affects the row-resident generic route even when
    // the stored-row and coefficient dimensions do not change.
    physical4 = kssbc_resource__model(
        8201888,4*8201888,311730,311730,311730,117529,10603,128131,
        200,32,16,1,1,kssbc_resource__rng_call_upper(),
        1*gib,56*gib,12*60*60)
    assert(physical4.status == "MODELED")
    assert(physical4.physical_scale == 4)
    assert(physical4.generic_components.phase_scratch_bytes >
        model1.generic_components.phase_scratch_bytes)
    assert(physical4.generic_wall_upper_seconds >
        model1.generic_wall_upper_seconds)

    // An exact chunk contract with a very large registered call count is
    // rejected on the explicit per-call wall charge before estimator RNG.
    chunk_heavy = kssbc_resource__model(
        8201888,8201888,311730,311730,311730,117529,10603,128131,
        200,32,16,200000,200000,kssbc_resource__rng_call_upper(),
        1*gib,56*gib,12*60*60)
    assert(chunk_heavy.status == "MODELED")
    assert(chunk_heavy.rng_total_calls == 80000000)
    assert(chunk_heavy.compressed.status == "RESOURCE_ADMISSION_FAILED")
    assert(chunk_heavy.compressed.wall_admitted == 0)

    // The final route gate replaces the provisional hierarchy term with the
    // actual solver/base/hierarchy peak and always applies 30% memory margin.
    route_pass = kssbc_resource__forecast(
        "compressed",components,100,0.25,200,1000)
    route_pass = kssbc_resource__route_reconcile(route_pass,100)
    assert(route_pass.status == "ADMITTED")
    assert(route_pass.route_reconciled == 1)
    assert(route_pass.non_solver_numerical_bytes == 26)
    assert(route_pass.routed_solver_peak_bytes == 100)
    assert(route_pass.components.cmg_hierarchy_bytes == 100)
    assert(route_pass.component_bytes[5] == 100)
    assert(route_pass.numerical_peak_bytes == 126)
    assert(route_pass.memory_headroom_fraction == 0.30)
    assert(route_pass.memory_admission_bytes == 164)
    route_fail = kssbc_resource__forecast(
        "compressed",components,100,0.25,160,1000)
    assert(route_fail.status == "ADMITTED")
    route_fail = kssbc_resource__route_reconcile(route_fail,100)
    assert(route_fail.status == "RESOURCE_ADMISSION_FAILED")
    assert(route_fail.before_rng == 1)
    assert(route_fail.memory_admitted == 0)
    route_fail = kssbc_resource__forecast(
        "generic",components,100,0.25,170,1000)
    assert(route_fail.status == "ADMITTED")
    route_fail = kssbc_resource__route_reconcile(route_fail,100)
    assert(route_fail.status == "GENERIC_RESOURCE_ADMISSION_FAILED")
    assert(route_fail.non_solver_numerical_bytes == 36)
    assert(route_fail.memory_admission_bytes == 177)

    // Direct/test calls preserve the legacy 65% solver cap.  A configured
    // command instead derives its exact solver budget from the whole-command
    // 30% envelope, then produces the same final forecast as the independent
    // resource module.
    kssbc_solver__resource_clear()
    assert(kssbc_solver__resource_budget(1000) == 650)
    assert(kssbc_solver__resource_config(
        "compressed",26,24,39,27,100,200,1000) == 0)
    assert(kssbc_solver__resource_budget(1000) ==
        floor(200/1.30)-26)
    assert(kssbc_solver__resource_apply(100) == 1)
    assert(kssbc_solver__resource_status() == "ADMITTED")
    solver_gate = kssbc_solver__resource_vector()
    route_pass = kssbc_resource__forecast(
        "compressed",components,100,0.25,200,1000)
    route_pass = kssbc_resource__route_reconcile(route_pass,100)
    assert(solver_gate == kssbc_resource__forecast_vector(route_pass))
    kssbc_solver__resource_clear()
    assert(kssbc_solver__resource_budget(1000) == 650)

    assert(kssbc_solver__resource_config(
        "generic",36,24,39,27,100,170,1000) == 0)
    assert(kssbc_solver__resource_apply(100) == 0)
    assert(kssbc_solver__resource_status() ==
        "GENERIC_RESOURCE_ADMISSION_FAILED")
    solver_gate = kssbc_solver__resource_vector()
    assert(solver_gate[7] == 177)
    assert(solver_gate[15] == 0)
    kssbc_solver__resource_clear()

    // Ineligible compression may select the generic route only after the
    // raw-resident generic peak has independently passed admission.
    selected = kssbc_resource__select_auto(0,compressed,generic)
    assert(selected.status == "ADMITTED")
    assert(selected.route == "generic")
    assert(selected.before_rng == 1)

    generic_parts = kssbc_resource__empty_components()
    generic_parts.raw_stata_bytes = 20*gib
    generic_parts.phase_scratch_bytes = 25*gib
    generic_fail = kssbc_resource__forecast_default(
        "generic",generic_parts,100)
    assert(generic_fail.status == "GENERIC_RESOURCE_ADMISSION_FAILED")
    assert(generic_fail.numerical_peak_bytes == 45*gib)
    selected = kssbc_resource__select_auto(0,compressed,generic_fail)
    assert(selected.status == "GENERIC_RESOURCE_ADMISSION_FAILED")
    assert(selected.route == "generic")
    assert(selected.admitted == 0)
    assert(selected.before_rng == 1)

    // If compression is eligible, a failed compressed forecast is itself a
    // typed rejection; auto does not launch the more expensive generic path.
    selected = kssbc_resource__select_auto(1,fail30,generic)
    assert(selected.status == "RESOURCE_ADMISSION_FAILED")
    assert(selected.route == "compressed")
    assert(selected.admitted == 0)

    // Scale progression is fail-closed when a phase, process peak, qacct
    // maximum, or wall time exceeds its registered upper forecast.
    reconciled = kssbc_resource__reconcile(
        compressed,(20,35,28,25),38,39,90)
    assert(reconciled.status == "RECONCILED")
    assert(reconciled.comparison_peak_bytes == 39)
    assert(reconciled.next_scale_allowed == 1)
    understated = kssbc_resource__reconcile(
        compressed,(20,35,28,25),38,40,90)
    assert(understated.status == "FORECAST_UNDERESTIMATED")
    assert(understated.within_memory_forecast == 0)
    assert(understated.next_scale_allowed == 0)

    // Invalid components and invalid headroom fail before any RNG entry.
    components.cell_bytes = -1
    compressed = kssbc_resource__forecast(
        "compressed",components,100,0.30)
    assert(compressed.status == "INVALID_RESOURCE_FORECAST")
    assert(compressed.before_rng == 1)
    components.cell_bytes = 1
    compressed = kssbc_resource__forecast(
        "compressed",components,100,0.31)
    assert(compressed.status == "INVALID_RESOURCE_FORECAST")
}
test_scale_resource()
end

// End-to-end receipt gate.  This checks public e(resource_*) state after the
// accepted solver route has replaced the provisional fifth component.  The
// same reconstruction is consumed by the SCC evidence validator.
capture program drop _assert_resource_receipt
program define _assert_resource_receipt
    args resource_row
    tempname components forecasts reconstructed
    matrix `components' = e(resource_components)
    matrix `forecasts' = e(resource_forecasts)
    scalar `reconstructed' =                                      ///
        `components'[`resource_row',2]+                            ///
        `components'[`resource_row',3]+                            ///
        `components'[`resource_row',4]+                            ///
        `components'[`resource_row',5]+                            ///
        `components'[`resource_row',6]+                            ///
        `components'[`resource_row',8]+                            ///
        `components'[`resource_row',9]+                            ///
        `components'[`resource_row',11]
    if `resource_row' == 2 {
        scalar `reconstructed' = `reconstructed'+                 ///
            `components'[`resource_row',1]
    }
    assert `components'[`resource_row',5] ==                       ///
        e(route_forecast_peak_bytes)
    assert e(resource_cmg_hierarchy_bytes) ==                      ///
        `components'[`resource_row',5]
    assert e(resource_routed_solver_bytes) ==                      ///
        `components'[`resource_row',5]
    assert `forecasts'[`resource_row',3] == `reconstructed'
    assert e(resource_numerical_peak_bytes) == `reconstructed'
    assert e(resource_peak_bytes) == `forecasts'[`resource_row',5]
    assert e(resource_mem_admit_bytes) ==                          ///
        ceil(1.30*e(resource_peak_bytes))
    assert e(resource_memory_headroom) == .30
    assert e(resource_wall_headroom) == .50
    assert e(resource_admitted) == 1
end

clear
set obs 27
generate long obsid = _n
generate byte cell = ceil(_n/3)
generate byte within_cell = mod(_n-1,3)+1
generate byte worker_index = floor((cell-1)/3)+1
generate byte firm_index = mod(cell-1,3)+1
generate long worker = 10*worker_index
generate long firm = 100*firm_index
generate long match = 1000+cell
replace match = 101 in 1/2
replace match = 102 in 3
generate long frequency = 1+mod(obsid,3)
generate double target = frequency*(1+mod(obsid,3)/4)
generate double y = 2+.7*worker_index-.3*firm_index +              ///
    .11*within_cell+.03*worker_index*firm_index+.007*obsid^2
generate long atom_key = obsid

quietly kss_bc y [fw=frequency], worker(worker) firm(firm)         ///
    deletion(match) deletionid(match) targetweight(target)         ///
    probeorder(atom_key) algorithm(jla) engine(compressed)         ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(4) batch(2) seed(8675309) tolerance(1e-10) nodisplay
assert "`e(preconditioner_selected)'" == "DIAGONAL"
assert e(route_code) == 1
assert e(route_hierarchy_levels) == 0
assert e(resource_routed_solver_bytes) > 0
_assert_resource_receipt 1

quietly kss_bc y [fw=frequency], worker(worker) firm(firm)         ///
    deletion(match) deletionid(match) targetweight(target)         ///
    probeorder(atom_key) algorithm(jla) engine(generic)            ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(4) batch(2) seed(8675309) tolerance(1e-10) nodisplay
assert "`e(preconditioner_selected)'" == "DIAGONAL"
assert e(route_code) == 1
assert e(route_hierarchy_levels) == 0
assert e(resource_routed_solver_bytes) > 0
_assert_resource_receipt 2

quietly kss_bc y [fw=frequency], worker(worker) firm(firm)         ///
    deletion(match) deletionid(match) targetweight(target)         ///
    probeorder(atom_key) algorithm(jla) engine(generic)            ///
    preconditioner(cmg) memory_gib(4) wallseconds(3600)            ///
    probes(4) batch(2) seed(8675309) tolerance(1e-10) nodisplay
assert "`e(preconditioner_selected)'" == "CMG"
assert e(route_code) == 2
assert e(route_hierarchy_levels) >= 1
assert e(resource_routed_solver_bytes) > 0
_assert_resource_receipt 2

di as result "PASS test_scale_resource.do"
exit 0
