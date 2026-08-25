version 18.0
clear all
set more off
set varabbrev off

capture confirm file "vckss/vckss_resource.mata"
if _rc {
    di as error "run the scale-resource test from the repository root"
    exit 601
}
adopath ++ "`c(pwd)'/vckss"

quietly do "vckss/vckss_resource.mata"
quietly do "vckss/vckss.mata"
quietly do "vckss/vckss_cmg.mata"
quietly do "vckss/vckss_solver.mata"

mata:
void test_scale_resource()
{
    real scalar gib
    real rowvector solver_gate
    struct vckss_resource_components scalar components, large, generic_parts
    struct vckss_resource_forecast scalar compressed, generic, advisory
    struct vckss_resource_forecast scalar wall_miss, hard_fail, routed
    struct vckss_resource_model scalar model
    struct vckss_resource_selection scalar selected
    struct vckss_resource_reconciliation scalar reconciled, unsafe

    assert(vckss_resource__api_level() == 10)
    assert(vckss_resource__build_id() ==
        "vckss-resource-api10-fe-buf1-buffered")
    assert(vckss_solver__api_level() == 26)
    assert(vckss_solver__build_id() ==
        "vckss-solver-api26-gpl-mata-cmg")
    // Compatibility defaults are not package-wide ceilings.
    assert(vckss_resource__hard_mem_bytes() == 56*1024^3)
    assert(vckss_resource__hard_wall_secs() == 12*60*60)
    assert(vckss_resource__wall_margin() == 0.50)
    assert(vckss_resource__rng_call_upper() == 0)
    assert(cols(vckss_resource__component_names()) == 11)

    components = vckss_resource__empty_components()
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

    compressed = vckss_resource__forecast(
        "compressed",components,100,0.30)
    assert(compressed.status == "ADMITTED")
    assert(compressed.before_rng == 1)
    assert(compressed.selection_peak_bytes == 24)
    assert(compressed.transition_peak_bytes == 39)
    assert(compressed.non_solver_numerical_bytes == 51)
    assert(compressed.numerical_peak_bytes == 55)
    assert(compressed.restoration_peak_bytes == 27)
    assert(compressed.peak_bytes == 55)
    assert(compressed.memory_admission_bytes == 72)
    assert(compressed.wall_admission_seconds == 150)

    generic = vckss_resource__forecast("generic",components,100,0.30)
    assert(generic.status == "ADMITTED")
    assert(generic.numerical_peak_bytes == 40)
    assert(vckss_resource__nonsolver_peak("compressed",components) == 26)
    assert(vckss_resource__nonsolver_peak("generic",components) == 36)

    gib = vckss_resource__gib()
    large = vckss_resource__empty_components()
    large.raw_stata_bytes = 44*gib
    advisory = vckss_resource__forecast(
        "compressed",large,100,0.30,56*gib,120)
    assert(advisory.status == "ADMITTED")
    assert(advisory.memory_admitted == 1)
    assert(advisory.memory_admission_bytes > advisory.hard_memory_bytes)
    assert(advisory.admitted == 1)

    wall_miss = vckss_resource__forecast(
        "compressed",components,101,0.75,1000,150)
    assert(wall_miss.status == "ADMITTED")
    assert(wall_miss.wall_admitted == 0)
    assert(wall_miss.memory_admitted == 1)
    assert(wall_miss.admitted == 1)

    large.raw_stata_bytes = 57*gib
    hard_fail = vckss_resource__forecast(
        "compressed",large,100,0,56*gib,.)
    assert(hard_fail.status == "RESOURCE_ADMISSION_FAILED")
    assert(hard_fail.memory_admitted == 0)

    // A hypothetical CMG allocation can exceed the envelope while the
    // selected diagonal base fits.  Route reconciliation, not that planning
    // component, decides admission.
    components.cmg_hierarchy_bytes = 120
    routed = vckss_resource__forecast(
        "compressed",components,100,0.30,160,1000)
    assert(routed.status == "RESOURCE_ADMISSION_FAILED")
    routed = vckss_resource__route_reconcile(routed,100)
    assert(routed.status == "ADMITTED")
    assert(routed.numerical_peak_bytes == 151)
    assert(routed.memory_admission_bytes == 197)
    assert(routed.memory_admission_bytes > routed.hard_memory_bytes)
    assert(routed.memory_admitted == 1)
    components.cmg_hierarchy_bytes = 4
    routed = vckss_resource__forecast(
        "compressed",components,100,0.30,150,1000)
    routed = vckss_resource__route_reconcile(routed,100)
    assert(routed.status == "RESOURCE_ADMISSION_FAILED")

    // The solver-owned final gate uses the same direct peak rule.
    vckss_solver__resource_clear()
    assert(vckss_solver__resource_budget(1000) == 1000)
    assert(vckss_solver__resource_config(
        "compressed",51,24,39,27,100,160,1000) == 0)
    assert(vckss_solver__resource_budget(1000) == 109)
    assert(vckss_solver__resource_apply(100) == 1)
    assert(vckss_solver__resource_status() == "ADMITTED")
    solver_gate = vckss_solver__resource_vector()
    assert(solver_gate[5] == 151)
    assert(solver_gate[7] == 197)
    assert(solver_gate[15] == 1)
    vckss_solver__resource_clear()

    assert(vckss_solver__resource_config(
        "compressed",51,24,39,27,100,150,1000) == 0)
    assert(vckss_solver__resource_apply(100) == 0)
    assert(vckss_solver__resource_status() == "RESOURCE_ADMISSION_FAILED")
    vckss_solver__resource_clear()

    // Forecast misses calibrate the model; they never unlock later work.
    reconciled = vckss_resource__reconcile(
        compressed,(20,35,28,25),38,56,160)
    assert(reconciled.status == "RECONCILED")
    assert(reconciled.within_memory_forecast == 0)
    assert(reconciled.within_wall_forecast == 0)
    assert(reconciled.next_scale_allowed == 0)
    unsafe = vckss_resource__reconcile(
        compressed,(20,35,28,25),38,57*gib,90)
    assert(unsafe.status == "ACTUAL_MEMORY_LIMIT_EXCEEDED")
    assert(unsafe.next_scale_allowed == 0)

    selected = vckss_resource__select_auto(0,compressed,generic)
    assert(selected.status == "ADMITTED" & selected.route == "generic")
    selected = vckss_resource__select_auto(1,compressed,generic)
    assert(selected.status == "ADMITTED" & selected.route == "compressed")
    generic_parts = vckss_resource__empty_components()
    generic_parts.raw_stata_bytes = 30*gib
    generic_parts.phase_scratch_bytes = 30*gib
    generic = vckss_resource__forecast(
        "generic",generic_parts,100,0.30,56*gib,.)
    selected = vckss_resource__select_auto(0,compressed,generic)
    assert(selected.status == "GENERIC_RESOURCE_ADMISSION_FAILED")
    assert(selected.admitted == 0)

    model = vckss_resource__model(
        1000,1000,100,100,100,50,20,70,200,8,8,
        1,1,vckss_resource__rng_call_upper(),1*gib,20*gib,.)
    assert(model.status == "MODELED")
    assert(model.rng_total_calls == 400)
    assert(model.rng_wall_upper_seconds == 0)
}
test_scale_resource()
end

// End-to-end receipt: the routed solver replaces the provisional component,
// and optional wallseconds() is not needed for a valid user command.
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

quietly vckss y [fw=frequency], worker(worker) firm(firm)         ///
    deletion(match) deletionid(match) targetweight(target)         ///
    algorithm(jla) engine(compressed) preconditioner(diagonal)     ///
    memory_gib(4) probes(4) batch(2) seed(8675309)                 ///
    tolerance(1e-10) backend(mata) rng(stata) nodisplay
assert "`e(preconditioner_selected)'" == "DIAGONAL"
assert "`e(route_api)'" == "KSS-ROUTE-STRUCTURAL-V1"
assert e(route_code) == 1
assert e(resource_routed_solver_bytes) > 0
assert e(resource_peak_bytes) <= e(resource_hard_mem_bytes)
assert e(resource_admitted) == 1
assert e(resource_mem_admit_bytes) == ceil(1.30*e(resource_peak_bytes))

di as result "PASS test_scale_resource.do"
exit 0
