*! Reconcile additive native readiness and independent C transport versions.
program define _fevc_rust_core_ready, rclass
    version 18.0
    args flags platform fullcmg execution componentauto resolved project autoexact exactstayer ///
        algorithm stayers executionapi
    local ok = mod(floor(`flags'/1),2)==1
    if `platform' local ok = `ok' & mod(floor(`flags'/256),2)==1
    if `fullcmg' local ok = `ok' & mod(floor(`flags'/1024),2)==1 & ///
        mod(floor(`flags'/2048),2)==1
    if `execution' local ok = `ok' & mod(floor(`flags'/4096),2)==1 & ///
        `executionapi'>=1
    if `componentauto' local ok = `ok' & mod(floor(`flags'/8192),2)==1 & ///
        `executionapi'>=2
    if `resolved' local ok = `ok' & mod(floor(`flags'/16384),2)==1 & ///
        `executionapi'>=3
    if `project' local ok = `ok' & mod(floor(`flags'/512),2)==1
    if "`algorithm'"=="exact" & "`stayers'"=="movers" {
        local ok = `ok' & mod(floor(`flags'/2),2)==1
    }
    else {
        foreach bit in 4 8 32 64 128 {
            local ok = `ok' & mod(floor(`flags'/`bit'),2)==1
        }
        if `autoexact' | `exactstayer' ///
            local ok = `ok' & mod(floor(`flags'/2),2)==1
    }
    return scalar ready = `ok'
end
