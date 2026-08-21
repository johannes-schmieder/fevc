version 18.0
clear all
set more off
set varabbrev off

args source_dir install_root install_mode public_test
if `"`source_dir'"' == "" | `"`install_root'"' == "" | ///
    !inlist(`"`install_mode'"',"unavailable","qualified") {
    di as error "source, isolated PLUS root, and install mode required"
    exit 198
}
if `"`install_mode'"' == "qualified" & `"`public_test'"' == "" {
    di as error "qualified install mode requires the public test path"
    exit 198
}

sysdir set PLUS `"`install_root'"'
quietly net install varcomp_kss, from(`"`source_dir'"') replace

local installed_dir `"`install_root'/v"'
foreach required in varcomp_kss.ado varcomp_kss_rust.ado ///
    _vckss_rust_public_call.ado {
    local install_subdir = cond(substr("`required'",1,1)=="_","_","v")
    confirm file `"`install_root'/`install_subdir'/`required'"'
}

if `"`install_mode'"' == "qualified" {
    foreach required in varcomp_kss_rust_macos_arm64.plugin ///
        varcomp_kss_rust_macos_x86_64.plugin {
        confirm file `"`installed_dir'/`required'"'
    }
    do `"`public_test'"' `"`installed_dir'"'
}
else {
    foreach absent in varcomp_kss_rust_macos_arm64.plugin ///
        varcomp_kss_rust_macos_x86_64.plugin {
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
    capture quietly varcomp_kss y [fw=frequency], worker(worker) ///
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
