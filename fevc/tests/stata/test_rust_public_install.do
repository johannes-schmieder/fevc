version 18.0
clear all
set more off
set varabbrev off

args source_dir install_root install_mode test_root
if `"`source_dir'"' == "" | `"`install_root'"' == "" | ///
    !inlist(`"`install_mode'"',"unavailable","qualified") {
    di as error "source, isolated PLUS root, and install mode required"
    exit 198
}
if `"`install_mode'"' == "qualified" & `"`test_root'"' == "" {
    di as error "qualified install mode requires the Rust route-test root"
    exit 198
}

sysdir set PLUS `"`install_root'"'
quietly net install fevc, from(`"`source_dir'"') replace

local installed_dir `"`install_root'/f"'
local installed_plugin_dir `"`install_root'/f"'
foreach required in fevc.ado fevc_rust.ado ///
    _fevc_rust_plugin_call.ado _fevc_rust_solve_v4.ado ///
    _fevc_rust_solve_v5.ado ///
    _fevc_rust_plan_receipt.ado                         ///
    _fevc_rust_reconcile_comp_v7.ado                    ///
    _fevc_rust_reconcile_exact_v7.ado                   ///
    _fevc_rust_post_comp_v7.ado                         ///
    _fevc_rust_post_exact_v7.ado                        ///
    _fevc_rust_capture_stayers.ado                      ///
    _fevc_rust_post_stayer_hybrid.ado _fevc_rust_macos.ado ///
    _fevc_rust_windows.ado _fevc_rust_linux.ado        ///
    _fevc_rust_public_call.ado                         ///
    _fevc_component_model_route.ado                    ///
    _fevc_exact_inference_model_post.ado                ///
    _fevc_rust_component_attach.ado                    ///
    _fevc_rust_component_fetch.ado                     ///
    _fevc_rust_component_post.ado                      ///
    _fevc_failure_guidance.ado {
    local install_subdir = lower(substr("`required'",1,1))
    confirm file `"`install_root'/`install_subdir'/`required'"'
}

if `"`install_mode'"' == "qualified" {
    local qualified_plugins
    local machine_type = lower(`"`c(machine_type)'"')
    if strpos(`"`machine_type'"',"mac") {
        local qualified_plugins fevc_rust_macos_arm64.plugin ///
            fevc_rust_macos_x86_64.plugin
    }
    else if `"`c(os)'"' == "Unix" {
        local qualified_plugins fevc_rust_linux_x64.plugin
    }
    else if `"`c(os)'"' == "Windows" {
        local qualified_plugins fevc_rust_windows_x64.plugin
    }
    else {
        di as error "unsupported qualified-install operating system: `c(os)'"
        exit 9
    }
    foreach required of local qualified_plugins {
        confirm file `"`installed_plugin_dir'/`required'"'
    }
    foreach route_test in test_rust_public.do                   ///
        test_rust_exact_controls.do test_rust_generic_jla.do    ///
        test_rust_planned_v4.do test_rust_planned_compressed.do ///
        test_rust_planned_compressed_post.do                    ///
        test_rust_full_cmg_v2.do                               ///
        test_rust_public_exact.do test_rust_public_generic.do   ///
        test_stayers_hybrid.do {
        confirm file `"`test_root'/`route_test'"'
        do `"`test_root'/`route_test'"' `"`installed_dir'"'
    }

    // Exercise the intended public tuple once more from the isolated net
    // install itself, independent of the source-local route-test adopath.
    clear
    set obs 8
    generate long obsid = _n
    generate long worker = cond(_n<=4,1,2)
    generate long firm = cond(inlist(_n,1,2,5,6),1,2)
    generate long deletion_id = _n
    generate double y = worker-firm+.05*obsid
    generate byte frequency = 1
    generate double target = 1
    sort obsid
    set rng kiss32
    set seed 20260824
    local install_rng `"`c(rng)'"'
    local install_stream = c(rngstream)
    local install_state `"`c(rngstate)'"'
    local install_sortedby : sortedby
    quietly _datasignature
    local install_signature `"`r(datasignature)'"'
    quietly fevc y [fw=frequency], worker(worker) firm(firm)   ///
        deletion(match) deletionid(deletion_id) targetweight(target) ///
        backend(rust) rng(counter_v1) algorithm(auto) engine(auto)   ///
        exact_limit(500) nodisplay
    assert `"`e(cmd)'"' == "fevc"
    assert `"`e(version)'"' == "0.5.0-alpha.1"
    assert `"`e(algorithm_requested)'"' == "auto"
    assert `"`e(algorithm)'"' == "exact"
    assert `"`e(engine_requested)'"' == "auto"
    assert `"`e(result_family)'"' == "exact"
    assert e(rust_selected_algorithm_code) == 1
    assert e(rust_selected_engine_code) == 3
    assert `"`e(rust_phase_profile_schema)'"' == "VCKSS-NATIVE-PHASE-PERF-V1"
    assert `"`e(rust_phase_profile_units)'"' == "seconds"
    assert rowsof(e(rust_phase_profile)) == 1
    assert colsof(e(rust_phase_profile)) == 8
    forvalues phase = 1/8 {
        assert e(rust_phase_profile)[1,`phase'] >= 0
    }
    assert e(rust_phase_profile)[1,8] >= e(rust_phase_profile)[1,7]
    assert e(rust_plan_resolved) == 1 & e(rust_plan_frozen) == 1
    assert e(probes) == 0 & e(seed) == 0 & e(batch) == 0
    assert e(rust_counter_plan_complete) == 1
    assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
    capture confirm matrix e(V)
    assert _rc != 0
    quietly count if e(sample)
    assert r(N) == e(N_retained)
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
    assert `"`c(rng)'"' == `"`install_rng'"'
    assert c(rngstream) == `install_stream'
    assert `"`c(rngstate)'"' == `"`install_state'"'
    local install_sortedby_after : sortedby
    assert `"`install_sortedby_after'"' == `"`install_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`install_signature'"'
    which fevc
    capture which varcomp_kss
    assert _rc == 111
}
else {
    foreach absent in fevc_rust_macos_arm64.plugin ///
        fevc_rust_macos_x86_64.plugin fevc_rust_linux_x64.plugin ///
        fevc_rust_windows_x64.plugin {
        capture confirm file `"`installed_plugin_dir'/`absent'"'
        assert _rc == 601
    }
    set obs 16
    generate long worker = ceil(_n/2)
    generate byte firm = 1 + mod(_n,2)
    generate long match = _n
    generate double y = worker-firm
    generate byte frequency = 1
    generate double target = 1
    capture quietly fevc y [fw=frequency], worker(worker) ///
        firm(firm) deletion(match) targetweight(target)          ///
        algorithm(jla) engine(compressed)                        ///
        stayers(movers)                                         ///
        preconditioner(diagonal) batch(2) probes(4) seed(91827) ///
        tolerance(1e-10) maxiter(10000) backend(rust)            ///
        rng(counter_v1) nodisplay
    assert _rc == 498
    assert `"`e(status)'"' == "WITHHELD"
    assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNAVAILABLE"
    assert `"`e(backend_requested)'"' == "rust"
    assert `"`e(backend_selected)'"' == ""
}
di as result "PASS test_rust_public_install.do"
