version 18.0
clear all
set more off
set varabbrev off

args package_root input_csv output_csv phase_start phase_end algorithm rows_arg cores_arg probes_arg seed_arg memory_arg
local rows = real("`rows_arg'")
local cores = real("`cores_arg'")
local probes = real("`probes_arg'")
local seed = real("`seed_arg'")
local memory = real("`memory_arg'")
if !inlist(`rows',960,7680,30720,122880,491520) | ///
   !inlist(`cores',1,2,4,8,14,28) | !inlist("`algorithm'","jla","exact") | ///
   missing(`memory') | `memory'<8 {
    exit 198
}
confirm file `"`package_root'/fevc/fevc.ado"'
confirm file `"`package_root'/fevc/fevc_rust_linux_x64.plugin"'
adopath ++ `"`package_root'/fevc"'
local stata_processors = min(4,`cores')
capture set processors `stata_processors'
assert !_rc & c(processors)==`stata_processors'

timer clear 70
timer on 70
import delimited using `"`input_csv'"', clear varnames(1) asdouble bindquote(strict)
timer off 70
timer list 70
local import_seconds = r(t70)
confirm numeric variable observation_key worker firm period match y
assert _N==`rows'
isid observation_key
isid worker firm
assert `rows'==3*`=floor(`rows'/3)' & `rows'==120*`=floor(`rows'/120)'
quietly count if missing(observation_key,worker,firm,period,match,y)
assert r(N)==0
sort worker period firm

tempname marker
file open `marker' using `"`phase_start'"', write text replace
file write `marker' "START fevc" _n
file close `marker'
timer clear 80
timer on 80
if "`algorithm'"=="jla" {
    capture noisily fevc y, worker(worker) firm(firm) deletion(match) ///
        probeorder(observation_key) backend(rust) rng(counter_v1) algorithm(jla) ///
        engine(auto) preconditioner(auto) batch(auto) memory_gib(`memory') ///
        probes(`probes') seed(`seed') maxiter(20000) stayers(movers) nodisplay
}
else {
    capture noisily fevc y, worker(worker) firm(firm) deletion(match) ///
        backend(rust) algorithm(exact) ///
        memory_gib(`memory') stayers(movers) nodisplay
}
local rc = _rc
timer off 80
file open `marker' using `"`phase_end'"', write text replace
file write `marker' "END fevc rc=`rc'" _n
file close `marker'
if `rc' exit `rc'
timer list 80
local primary_seconds = r(t80)
assert e(N_stored)==`rows' & e(N_retained)==`rows'
assert e(worker_levels)==`rows'/3 & e(firm_levels)==`rows'/120
assert "`e(backend_selected)'"=="rust" & "`e(algorithm)'"=="`algorithm'"
assert e(target_identity_residual)<=1e-10
matrix values = e(results)
assert rowsof(values)==4 & colsof(values)==4

clear
set obs 1
generate str32 schema = "FEVC-FIVE-WAY-ROLE-V1"
generate str12 status = "PASS"
generate str12 role = "fevc"
generate str8 algorithm = "`algorithm'"
generate long rows = `rows'
generate byte cores = `cores'
generate int probes = `probes'
generate long seed = `seed'
generate double import_seconds = `import_seconds'
generate double primary_seconds = `primary_seconds'
generate double estimator_seconds = `primary_seconds'
generate double raw_worker = values[3,1]
generate double raw_firm = values[3,2]
generate double raw_covariance = values[3,3]
generate double raw_total = values[3,4]
generate double plugin_worker = values[1,1]
generate double plugin_firm = values[1,2]
generate double plugin_covariance = values[1,3]
generate double plugin_total = values[1,4]
generate double correction_worker = values[2,1]
generate double correction_firm = values[2,2]
generate double correction_covariance = values[2,3]
generate double correction_total = values[2,4]
generate double normalization_factor = 1
generate double normalized_worker = raw_worker
generate double normalized_firm = raw_firm
generate double normalized_covariance = raw_covariance
generate double normalized_total = raw_total
generate str40 rng_policy = cond("`algorithm'"=="jla","FEVC_COUNTER_V1_SEED","NONE_EXACT")
generate long retained_rows = `rows'
export delimited using `"`output_csv'"', replace
di as result "FEVC_FIVE_WAY_ROLE_PASS fevc `algorithm' rows=`rows' cores=`cores'"
exit 0
