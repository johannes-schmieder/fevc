version 18.0
clear all
set more off
set varabbrev off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set obs 288
generate long key = _n
generate long worker = ceil(key/8)
generate byte slot = mod(key-1,8)
generate int firm = floor(slot/2)+1
generate double x = (worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x
sort key
local caller_rng `"`c(rngstate)'"'
local caller_sort_rng `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
local routes mata_exact mata_generic mata_compressed
capture quietly fevc_rust probe
if !_rc local routes `routes' rust_exact rust_generic

foreach route of local routes {
    local backend mata
    if strpos("`route'","rust_")==1 local backend rust
    local options algorithm(exact) deletion(observation)
    local controls x
    if strpos("`route'","generic") local options algorithm(jla) deletion(observation) engine(generic) preconditioner(diagonal)
    if "`route'"=="mata_compressed" {
        local controls
        local options algorithm(jla) deletion(match) engine(compressed) preconditioner(diagonal)
    }
    local command fevc y `controls', worker(worker) firm(firm) backend(`backend') stayers(movers) `options' probes(24) seed(1731) nodisplay
    quietly `command'
    matrix reference = e(results)
    foreach occupancy in high partial full {
        timer clear
        local ids 85/99
        if "`occupancy'"=="partial" local ids 2/100
        if "`occupancy'"=="full" local ids 1/100
        foreach id of numlist `ids' {
            forvalues repeat=1/2 {
                timer on `id'
                timer off `id'
            }
            if mod(`id',2) timer on `id'
        }
        mata: caller_timers = J(100,2,0); for (i=1;i<=100;i++) caller_timers[i,.] = timer_value(i)
        if "`occupancy'"=="full" & "`backend'"=="rust" noisily `command'
        else quietly `command'
        mata: assert(max(abs(st_matrix("reference")-st_matrix("e(results)")))<=1e-8)
        mata: for (i=1;i<=100;i++) assert(timer_value(i)==caller_timers[i,.])
        mata: assert(__fevc_timer_depth==0 & all(__fevc_timer_state:==0))
        if "`occupancy'"=="full" {
            assert missing(e(sample_selection_seconds))
            assert missing(e(validation_seconds))
            if "`backend'"=="mata" assert missing(e(fit_seconds))
        }
        // A caller timer still running must include time after the command.
        sleep 25
        timer off 99
        mata: assert(timer_value(99)[1]>=.02 & timer_value(99)[2]==3)
        assert `"`c(rngstate)'"'==`"`caller_rng'"'
        assert `"`c(sortrngstate)'"'==`"`caller_sort_rng'"'
        local sortedby : sortedby
        assert "`sortedby'"=="key"
        quietly _datasignature
        assert `"`r(datasignature)'"'==`"`signature'"'
    }
}

// Nested scopes share ownership, but only the outer scope releases timers.
timer clear
quietly fevc__timer begin
quietly fevc__timer on 1
quietly fevc__timer begin
quietly fevc__timer on 2
quietly fevc__timer end
mata: assert(__fevc_timer_depth==1 & sum(__fevc_timer_state[.,1]:>0)==2)
quietly fevc__timer end
mata: assert(__fevc_timer_depth==0 & all(__fevc_timer_state:==0))
mata: for (i=1;i<=100;i++) assert(timer_value(i)==(0,0))

timer on 96
capture noisily fevc y, worker(worker) firm(firm) backend(mata) algorithm(invalid)
assert _rc!=0
mata: assert(timer_value(96)[2]==1 & __fevc_timer_depth==0)
timer off 96
timer clear

// Inject interruption after profiling starts, then verify release and reuse.
quietly fevc y x, worker(worker) firm(firm) backend(mata) algorithm(exact) nodisplay
mata: mata drop vckss__control_sum_step()
mata:
void vckss__control_sum_step(real matrix total, real matrix correction,
    real matrix term)
{
    _error(1)
}
end
timer on 96
capture noisily fevc y x, worker(worker) firm(firm) backend(mata) algorithm(exact) nodisplay
assert _rc==1 & "`e(cmd)'"==""
mata: assert(__fevc_timer_depth==0 & all(__fevc_timer_state:==0))
mata: assert(timer_value(96)[2]==1)
timer off 96
timer clear
mata: mata clear
quietly fevc y x, worker(worker) firm(firm) backend(mata) algorithm(exact) nodisplay
assert "`e(cmd)'"=="fevc"
display "PASS test_timer_ownership.do"
