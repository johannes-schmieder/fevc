version 18
clear all
set more off
set processors 4
args package output
if "`output'"!="" log using "`output'",text replace
adopath ++ "`package'"
// Parser consumes the public native diagnostic schema, without private builds.
ereturn clear
fevc__control_failure_post "AMBIGUOUS_CONTROL_BASIS [control_span_deletion]: FEVC_CONTROL_CERT_V1 population=combined n=359402 q=13 units=42276 selected=span_residual bound=2e-10 transform_norm=4 conditioning=1e-4 propagated=1.6e-7 ceiling=1e-8 original_residual=2e-12 canonical_bound=4e-12 accumulated_bound=1e-9"
assert "`e(native_failure_phase)'"=="control_span_deletion"
assert "`e(control_certificate_schema)'"=="FEVC_CONTROL_CERT_V1"
assert "`e(control_certificate_population)'"=="combined"
assert "`e(control_certificate_selected)'"=="span_residual"
assert e(control_certificate_failure)[1,"N"]==359402
assert e(control_certificate_failure)[1,"controls"]==13
assert e(control_certificate_failure)[1,"units"]==42276
assert e(control_certificate_failure)[1,"bound"]==2e-10
assert e(control_certificate_failure)[1,"propagated"]==1.6e-7
assert e(control_certificate_failure)[1,"ceiling"]==1e-8
assert e(control_certificate_failure)[1,"original_residual"]==2e-12
assert e(control_certificate_failure)[1,"inconclusive"]==1
set obs 8
gen byte w=1+mod(floor((_n-1)/4),2)
gen byte f=1+mod(floor((_n-1)/2),2)
gen double d=2*mod(_n-1,2)-1
gen double x1=sqrt(1-.0002^2)*(2*w-3)+.0002*d*(2*f-3)
gen double x2=sqrt(1-.0002^2)*(2*f-3)-.0002*d*(2*w-3)
gen double y=sin(_n*1.7)+.1*_n
quietly _datasignature
local signature `"`r(datasignature)'"'
local rng `"`c(rngstate)'"'
foreach report in "nolog" "" "verbose" {
 quietly regress y x1
 capture noisily fevc y x1 x2, worker(w) firm(f) deletion(match) stayers(movers) ///
  algorithm(jla) backend(rust) rng(counter_v1) nativethreads(4) `report'
 assert _rc==498
 assert "`e(status)'"=="WITHHELD"
 assert "`e(withholding_status)'"=="FULL_RESIDUAL_FAILED"
 assert "`e(native_failure_phase)'"!=""
 assert "`e(withholding_detail)'"!=""
 capture confirm matrix e(results)
 assert _rc!=0
 capture confirm matrix e(b)
 assert _rc!=0
 capture confirm matrix e(control_certificate_failure)
 assert _rc!=0
 assert e(native_threads_selected)==4
 assert "$VCKSS_NATIVE_SELECTED"==""
 assert `"`c(rngstate)'"'==`"`rng'"'
 quietly _datasignature
 assert `"`r(datasignature)'"'==`"`signature'"'
}
noi di "PASS test_control_failure.do"
if "`output'"!="" log close
