version 18.0
// estat sample separates original one-unit stayer histories (e(N_stayers),
// e(N_stayer_rows)) from the stayers actually included in the hybrid target.
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
quietly fevc, simulate_data(ex2) clear
// Three two-row stayers at an isolated firm cannot attach to the mover graph;
// two two-row stayers at the first firm can.
quietly summarize worker_id
local next = r(max)
quietly summarize firm_id
local isolated = r(max)+1
local attached = r(min)
local rows = _N
quietly set obs `=`rows'+10'
forvalues k = 1/10 {
    local row = `rows'+`k'
    quietly replace worker_id = `next'+ceil(`k'/2) in `row'
    quietly replace firm_id = cond(`k'<=6,`isolated',`attached') in `row'
    quietly replace log_wage = .1*`k' in `row'
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) algorithm(exact) ///
    backend(mata) nodisplay
assert "`e(deletion)'"=="match" & "`e(stayers)'"=="both"
assert e(stayer_hybrid_N_unattached)>=3 & e(stayer_hybrid_N_stayers)>=2
assert e(N_stayers)==e(stayer_hybrid_N_stayers)+e(stayer_hybrid_N_unattached) ///
    +e(stayer_hybrid_N_singleton_drop)
assert e(stayer_hybrid_N_stayer_rows)<e(N_stayer_rows)
tempfile out
quietly log using `"`out'"', text replace name(stayers)
estat sample
quietly log close stayers
tempname fh
file open `fh' using `"`out'"', read text
local original 0
local included 0
local unattached 0
file read `fh' line
while r(eof)==0 {
    local text `"`macval(line)'"'
    if strpos(`"`text'"',"Original stayer workers") {
        local value = real(subinstr(word(`"`text'"',wordcount(`"`text'"')),",","",.))
        assert `value'==e(N_stayers)
        local original 1
    }
    if strpos(`"`text'"',"Included stayer workers") {
        local value = real(subinstr(word(`"`text'"',wordcount(`"`text'"')),",","",.))
        assert `value'==e(stayer_hybrid_N_stayers)
        local included 1
    }
    if strpos(`"`text'"',"Stayers outside retained firms") {
        local value = real(subinstr(word(`"`text'"',wordcount(`"`text'"')),",","",.))
        assert `value'==e(stayer_hybrid_N_unattached)
        local unattached 1
    }
    file read `fh' line
}
file close `fh'
assert `original' & `included' & `unattached'
di as result "PASS test_estat_sample_stayers.do"
