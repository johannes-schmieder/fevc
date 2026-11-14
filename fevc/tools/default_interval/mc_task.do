* Package Monte Carlo, one task: one design, one deletion mode, a block of replications. Every replication draws new
* errors on the fixed design, runs fevc with the plugin under test and the package defaults (probes, draws), and
* records the estimates, the normal and q = 1 pieces and the interval the command reports for each component.
*
* Usage: do mc_task.do <pkgroot> <design csv> <cell> <deletion> <task> <first rep> <reps> <cell seed> <errors>
*            <output directory> <native threads> <fevc options joined by +>
* Stata's batch mode splits a quoted argument at its blanks, so the options arrive as one word with + for blanks.
* Every row records the standard-error model the fit used and the threshold of its interval rule, so that a run with
* the wrong options or another rule cannot pass, and the reason when the default returned point estimates only.
* <native threads> = 0: the options name the route themselves (for the Mata routes); otherwise the native backend
* runs with that many threads. The options word "none" stands for no option at all: the plain default command.
* Random numbers: Stata's stream generator (mt64s). Replication r draws from stream r + 1 of the cell's seed, so
* its outcomes depend on the cell and on r alone, not on how replications are split into tasks or on their order.
* A design with the columns x and xb has a control: x enters the model and xb (its contribution) the outcome.
* Output: <output directory>/<cell>__<deletion>__<task>.csv, written under a temporary name and renamed.
version 18.0
clear all
set more off
set varabbrev off
* One Stata processor per task: many tasks run side by side, and idle Stata/MP threads would compete with them.
set processors 1
args pkgroot design cell deletion task first reps seedbase errors outdir threads options
if "`options'"=="" | "`13'"!="" {
    display as error "the fevc options must be one word (blanks written as +)"
    exit 198
}
local options : subinstr local options "+" " ", all
if "`options'"=="none" local options ""
local route "backend(rust) nativethreads(`threads')"
if `threads'==0 local route ""
adopath ++ "`pkgroot'"
local stem "`outdir'/`cell'__`deletion'__`task'"
capture confirm file "`stem'.csv"
if !_rc {
    display as error "`stem'.csv exists; refusing to overwrite"
    exit 602
}
if !inlist("`deletion'","match","observation") | !inlist("`errors'","normal","skew") | `reps'<1 | `first'+`reps'>32767 {
    display as error "invalid task arguments"
    exit 198
}
set rng mt64s
import delimited using "`design'", clear asdouble
confirm numeric variable worker firm alpha psi sd
* A design with a control carries it as x and its contribution to the outcome as xb; the control enters the model.
local controls ""
capture confirm numeric variable x xb
if !_rc local controls "x"
else generate double xb = 0
local N = _N
* True components: variances and covariance of the effects over person-years.
quietly correlate alpha psi, covariance
local t1 = r(Var_1)*(`N'-1)/`N'
local t2 = r(Var_2)*(`N'-1)/`N'
local t3 = r(cov_12)*(`N'-1)/`N'
local t4 = `t1'+`t2'+2*`t3'

local vars "str40 cell str12 deletion int task long rep long seed int stream int rc str40 status str24 model str120 reason long n_retained double seconds"
local vars "`vars' int q1_components int steps int unresolved byte joint_posted double threshold"
forvalues j = 1/4 {
    local vars "`vars' double truth`j' double b`j' double se`j' double nlb`j' double nub`j' double qlb`j' double qub`j'"
    local vars "`vars' double lb`j' double ub`j' double regime`j' double share`j' double kappa`j' double crit`j'"
    local vars "`vars' double eigshare`j' double maxw`j' double conv`j' double q1status`j' double lam`j'"
    local vars "`vars' double b1_`j' double vb_lo`j' double vb_fit`j' double vr`j' double vb`j'"
}
tempname post
tempfile results
postfile `post' `vars' using `results'
local last = `first'+`reps'-1
forvalues r = `first'/`last' {
    local seed = `seedbase'
    local stream = `r'+1
    set rngstream `stream'
    set seed `seed'
    capture drop y
    if "`errors'"=="skew" quietly generate double y = alpha+psi+xb+sd*(rgamma(1,1)-1)
    else quietly generate double y = alpha+psi+xb+sd*rnormal()
    timer clear 1
    timer on 1
    capture fevc y `controls', worker(worker) firm(firm) deletion(`deletion') `route' `options' nodisplay
    local rc = _rc
    timer off 1
    quietly timer list 1
    local seconds = r(t1)
    local line `"("`cell'") ("`deletion'") (`task') (`r') (`seed') (`stream') (`rc')"'
    if `rc' {
        local line `"`line' ("`e(withholding_status)'") ("") ("") (.) (`seconds') (.) (.) (.) (.) (.)"'
        forvalues j = 1/4 {
            local line `"`line' (`t`j'')"'
            forvalues k = 1/22 {
                local line `"`line' (.)"'
            }
        }
        post `post' `line'
        continue
    }
    local status "`e(status)'"
    * Why the default returned no standard errors, when it did not (empty otherwise).
    local reason = substr(`"`e(inference_auto_reason)'"',1,120)
    local reason : subinstr local reason `"""' "", all
    local line `"`line' ("`status'") ("`e(inference_model)'") ("`reason'") (`=e(N_retained)') (`seconds')"'
    tempname b T L
    matrix `b' = e(b)
    * An assignment from a missing e() matrix does not fail in Stata; confirm does.
    capture confirm matrix e(inference_intervals)
    local have = (_rc==0)
    if `have' matrix `T' = e(inference_intervals)
    if `have' {
        capture confirm matrix e(inference_leading)
        if _rc matrix `L' = J(4,28,.)
        else matrix `L' = e(inference_leading)
        local line `"`line' (`=e(inference_q1_components)') (`=e(inference_leading_steps)') (`=e(inference_leading_unresolved)') (`=e(inference_joint_posted)') (`=e(inference_share_threshold)')"'
    }
    else local line `"`line' (.) (.) (.) (.) (.)"'
    forvalues j = 1/4 {
        local line `"`line' (`t`j'') (`=`b'[1,`j']')"'
        if `have' {
            foreach k in 2 3 4 5 6 8 9 7 10 11 12 13 14 15 16 {
                local line `"`line' (`=`T'[`j',`k']')"'
            }
            * Eigenvalue, score, the two candidate variances of the score, the remainder variance,
            * and the variance of the score the fit used (with controls it has a control part).
            foreach k in 5 11 12 13 18 14 {
                local line `"`line' (`=`L'[`j',`k']')"'
            }
        }
        else {
            forvalues k = 1/21 {
                local line `"`line' (.)"'
            }
        }
    }
    post `post' `line'
}
postclose `post'
use `results', clear
assert _N==`reps'
export delimited using "`stem'.csv.tmp", replace
shell mv "`stem'.csv.tmp" "`stem'.csv"
display "wrote `stem'.csv"
