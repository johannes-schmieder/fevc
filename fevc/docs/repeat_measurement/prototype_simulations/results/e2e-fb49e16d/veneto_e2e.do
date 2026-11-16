* End-to-end run of the package on a private panel: the point estimate alone and the plain default command (no
* inference option: repeat-measurement standard errors with the automatic interval), under one deletion mode. Writes aggregates only (estimates, intervals, directions,
* counts, timings); no observation is listed or exported.
* Usage: do veneto_e2e.do <pkgroot> <panel csv> <label> <deletion> <threads> <output directory>
version 18.0
clear all
set more off
set varabbrev off
args pkgroot panel label deletion threads outdir
adopath ++ "`pkgroot'"
local stem "`outdir'/`label'__`deletion'"
capture confirm file "`stem'.txt"
if !_rc {
    display as error "`stem'.txt exists; refusing to overwrite"
    exit 602
}
import delimited using "`panel'", clear asdouble
local N = _N
display "E2E `label' `deletion' person-years `N' threads `threads'"

timer clear
timer on 1
fevc y, worker(worker) firm(firm) backend(rust) deletion(`deletion') nativethreads(`threads') inference(none)
timer off 1
matrix point = e(b)
local n_point = e(N_retained)

timer on 2
fevc y, worker(worker) firm(firm) deletion(`deletion') nativethreads(`threads')
timer off 2
assert "`e(backend_selected)'"=="rust" & e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
quietly timer list
local s_point = r(t1)
local s_auto = r(t2)
ereturn list
matrix b = e(b)
matrix T = e(inference_intervals)
matrix L = e(inference_leading)
* An assignment from a missing e() matrix does not fail in Stata; only confirm tells whether e(V) is posted.
capture confirm matrix e(V)
local have_V = (_rc==0)
assert "`e(inference_model)'"=="repeat_measurement"
* Standard errors do not change the estimator, but on the native backend they select another executor, so the two
* estimates agree to solver tolerance (the package's own regression uses the same bound).
local point_reldif = mreldif(point,b)
assert `point_reldif'<1e-5

tempname out
file open `out' using "`stem'.txt.tmp", write text replace
file write `out' "schema=FEVC-E2E-RESULT-V1" _n "label=`label'" _n "deletion=`deletion'" _n "person_years=`N'" _n
file write `out' "retained_point=`n_point'" _n "retained=`=e(N_retained)'" _n "status=`e(status)'" _n "model=`e(inference_model)'" _n
file write `out' "point_reldif=`point_reldif'" _n "seconds_point=`s_point'" _n "seconds_auto=`s_auto'" _n "threads=`threads'" _n "e_V_posted=`have_V'" _n
foreach s in inference_q1_components inference_leading_steps inference_leading_unresolved inference_share_threshold ///
    inference_pair_term inference_repeat_sets inference_joint_posted inference_offset_pass_solves mc_leverage_probes N_stayers N_stayer_rows {
    file write `out' "`s'=`=e(`s')'" _n
}
forvalues j = 1/4 {
    file write `out' "b`j'=" %21.15e (b[1,`j']) _n
    file write `out' "intervals`j'="
    forvalues k = 1/16 {
        file write `out' %21.15e (T[`j',`k']) " "
    }
    file write `out' _n "leading`j'="
    forvalues k = 1/28 {
        file write `out' %21.15e (L[`j',`k']) " "
    }
    file write `out' _n
}
file close `out'
shell mv "`stem'.txt.tmp" "`stem'.txt"
display "E2E DONE `label' `deletion' point `s_point's automatic `s_auto's"
