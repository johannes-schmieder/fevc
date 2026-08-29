version 18.0
clear all
set more off
set varabbrev off
set processors 4

args outdir pkgroot
if `"`outdir'"' == "" exit 198
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/vckss"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'

set obs 1002
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker + floor(time/2),50)
generate double z1 = sin(worker/5) + cos(firm/3) + time/20
generate double z2 = cos(worker/7) - sin(firm/4) + (time^2)/100
set seed 20260829
sort worker time
by worker: generate double alpha = rnormal() if _n == 1
by worker: replace alpha = alpha[1]
sort firm worker time
by firm: generate double psi = .7*rnormal() if _n == 1
by firm: replace psi = psi[1]
sort worker time
generate double noise = rnormal()*(.2 + .02*mod(time,3))
generate double y = 1 + alpha + psi + noise
export delimited y worker firm z1 z2 using `"`outdir'/input.csv"', replace

tempname handle
file open `handle' using `"`outdir'/vckss_results.csv"', write text replace
file write `handle' "kind,seed,row,col,value" _n

foreach seed in 101 202 303 404 505 {
    quietly vckss y, worker(worker) firm(firm)                  ///
        deletion(observation) algorithm(exact) backend(mata)   ///
        inference(highrank) inferencesimulations(1000)         ///
        inferenceseed(`seed') inferencebins(16)                ///
        project(z1 z2) projecteffect(firm) nodisplay

    matrix component_b = e(b)
    matrix component_V = e(V)
    matrix projection_b = e(projection_b)
    matrix projection_V = e(projection_V)
    matrix projection_V_naive = e(projection_V_naive)
    local component_names : colnames component_b
    local projection_names : colnames projection_b

    forvalues j = 1/4 {
        local name : word `j' of `component_names'
        file write `handle' "component_b,`seed',,`name'," %24.17e (component_b[1,`j']) _n
        forvalues k = 1/4 {
            local name2 : word `k' of `component_names'
            file write `handle' "component_V,`seed',`name',`name2'," %24.17e (component_V[`j',`k']) _n
        }
    }

    if `seed' == 101 {
        forvalues j = 1/3 {
            local name : word `j' of `projection_names'
            file write `handle' "projection_b,`seed',,`name'," %24.17e (projection_b[1,`j']) _n
            forvalues k = 1/3 {
                local name2 : word `k' of `projection_names'
                file write `handle' "projection_V,`seed',`name',`name2'," %24.17e (projection_V[`j',`k']) _n
                file write `handle' "projection_V_naive,`seed',`name',`name2'," %24.17e (projection_V_naive[`j',`k']) _n
            }
        }
        file write `handle' "meta,,N_retained,," %24.17e (e(N_retained)) _n
        file write `handle' "meta,,worker_levels,," %24.17e (e(worker_levels)) _n
        file write `handle' "meta,,firm_levels,," %24.17e (e(firm_levels)) _n
    }
}

file close `handle'
di as result "VCKSS MATLAB COMPARISON PASS"
exit 0
