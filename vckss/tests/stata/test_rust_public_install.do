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
quietly net install vckss, from(`"`source_dir'"') replace

local installed_dir `"`install_root'/v"'
foreach required in vckss.ado vckss_rust.ado ///
    _vckss_rust_plugin_call.ado _vckss_rust_solve_v4.ado ///
    _vckss_rust_plan_receipt.ado                         ///
    _vckss_rust_reconcile_comp_v7.ado                    ///
    _vckss_rust_reconcile_exact_v7.ado                   ///
    _vckss_rust_post_comp_v7.ado                         ///
    _vckss_rust_post_exact_v7.ado _vckss_rust_macos.ado  ///
    _vckss_rust_windows.ado _vckss_rust_linux.ado        ///
    _vckss_rust_public_call.ado {
    local install_subdir = cond(substr("`required'",1,1)=="_","_","v")
    confirm file `"`install_root'/`install_subdir'/`required'"'
}

if `"`install_mode'"' == "qualified" {
    foreach required in vckss_rust_macos_arm64.plugin ///
        vckss_rust_macos_x86_64.plugin {
        confirm file `"`installed_dir'/`required'"'
    }
    foreach route_test in test_rust_public.do                   ///
        test_rust_exact_controls.do test_rust_generic_jla.do    ///
        test_rust_planned_v4.do test_rust_planned_compressed.do ///
        test_rust_planned_compressed_post.do                    ///
        test_rust_public_exact.do test_rust_public_generic.do {
        confirm file `"`test_root'/`route_test'"'
        do `"`test_root'/`route_test'"' `"`installed_dir'"'
    }
}
else {
    foreach absent in vckss_rust_macos_arm64.plugin ///
        vckss_rust_macos_x86_64.plugin {
        capture confirm file `"`installed_dir'/`absent'"'
        assert _rc == 601
    }
    set obs 16
    generate long worker = ceil(_n/2)
    generate byte firm = 1 + mod(_n,2)
    generate long match = _n
    generate double y = worker-firm
    generate byte frequency = 1
    generate double target = 1
    capture quietly vckss y [fw=frequency], worker(worker) ///
        firm(firm) deletion(match) targetweight(target)          ///
        algorithm(jla) engine(compressed)                        ///
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
