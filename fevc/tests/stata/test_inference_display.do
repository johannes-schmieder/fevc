* The display of the standard errors of the components: one table with the
* same columns in every mode (no p-values; the lead share; the rule or the
* request in the heading) and a few lines under it; the stored results of
* inference(highrank), which runs the leading-direction search as the other
* modes do and keeps the normal interval of every component; the one-line
* numerical MCSE and its warning; estat inference; and the default output
* without live progress. Every displayed line fits in 80 columns. Runs on
* Mata and, when the loaded plugin carries the method, natively.
version 15.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
local caller_linesize = c(linesize)
set linesize 80

local native 0
capture quietly fevc_rust probe
if !_rc {
    if r(component_repeat_api)==1 & r(component_mixed_api)==1 local native 1
}
global DISP_FILE `"`c(tmpdir)'/fevc_inference_display_test.log"'

* Runs a command with its display in a text log (programs echo no commands).
program define _disp_log
    quietly log using `"$DISP_FILE"', text replace name(_disp_test)
    capture noisily `0'
    local rc = _rc
    quietly log close _disp_test
    exit `rc'
end

* Whether the logged display shows a text (expected 1) or not (0).
program define _disp_has
    args expected text
    tempname fh
    local found 0
    file open `fh' using `"$DISP_FILE"', read text
    file read `fh' line
    while r(eof)==0 {
        if strpos(`"`macval(line)'"',`"`text'"') local found 1
        file read `fh' line
    }
    file close `fh'
    if `found'!=`expected' {
        di as error `"display check failed: expected `expected' for [`text']"'
        exit 9
    }
end

* No logged line exceeds 80 columns or is a wrapped continuation.
program define _disp_width
    tempname fh
    file open `fh' using `"$DISP_FILE"', read text
    file read `fh' line
    while r(eof)==0 {
        if strlen(`"`macval(line)'"')>80 | substr(`"`macval(line)'"',1,2)=="> " {
            di as error `"line too wide: [`macval(line)']"'
            file close `fh'
            exit 9
        }
        file read `fh' line
    }
    file close `fh'
end

* The table row of a component against e(inference_intervals): the interval
* column (62-71) names the regime, with * for a requested one, the share
* column (72-78) the lead share, and a q = 1 row has no standard error
* (columns 29-39).
program define _disp_rows
    args request
    tempname table fh
    matrix `table' = e(inference_intervals)
    forvalues t = 1/4 {
        local label : word `t' of "Worker variance" "Firm variance" ///
            "Worker-firm cov." "Total worker-firm"
        local regime = `table'[`t',7]
        local kind = cond(`regime'==1,"q = 1",cond(`regime'==0,"normal","none"))
        if ("`request'"=="highrank" & `regime'==0) | ///
            ("`request'"=="q1" & `regime'==1) local kind "`kind'*"
        local share = `table'[`t',10]
        local text "."
        if `t'<4 & !missing(`share') {
            if `share'>1 local text ">100%"
            else if `share'<.0005 local text "<0.1%"
            else local text = strtrim(string(100*`share',"%5.1f"))+"%"
        }
        local found 0
        file open `fh' using `"$DISP_FILE"', read text
        file read `fh' line
        while r(eof)==0 {
            // The decomposition table has rows of the same names; only the
            // inference table names an interval in columns 62-71.
            local cell = strtrim(substr(`"`macval(line)'"',62,10))
            if substr(`"`macval(line)'"',1,strlen("`label'"))=="`label'" & ///
                inlist("`cell'","normal","normal*","q = 1","q = 1*","none") {
                local ++found
                assert "`cell'"=="`kind'"
                assert strtrim(substr(`"`macval(line)'"',72,7))=="`text'"
                if `regime'==1 assert strtrim(substr(`"`macval(line)'"',29,11))==""
                else assert strtrim(substr(`"`macval(line)'"',29,11))!=""
            }
            file read `fh' line
        }
        file close `fh'
        assert `found'==1
    }
end

* Texts the 0.6.1 display no longer shows under the table.
program define _disp_old_texts_absent
    foreach text in "P>|z|" "Shares (worker, firm, covariance)" ///
        "Econometric component inference" "Approximate numerical MCSE" ///
        "Effective rank" "Deletion units that share" "Interval by component" ///
        "Total elapsed" "pseudo-noise draws" {
        _disp_has 0 "`text'"
    }
end

* The numerical MCSE line: the four values and, with standard errors, the
* largest MCSE as a share of a standard error, rounded up.
program define _disp_mcse
    args with_se
    if "`e(algorithm)'"=="exact" {
        _disp_has 1 "Exact leverages: no numerical MCSE from projections."
        _disp_has 0 "Numerical MCSE ("
        exit
    }
    local probes = strtrim(string(e(probes),"%12.0fc"))
    _disp_has 1 "Numerical MCSE (`probes' projections): worker "
    if `with_se' {
        tempname table mcse
        matrix `table' = e(inference_intervals)
        matrix `mcse' = e(mcse)
        local largest 0
        forvalues t = 1/4 {
            if `table'[`t',2]>0 & !missing(`table'[`t',2]) {
                local largest = max(`largest',`mcse'[1,`t']/`table'[`t',2])
            }
        }
        _disp_has 1 "at most `=max(1,ceil(100*`largest'))'% of a standard error."
        _disp_has `=(`largest'>.1)' "Warning: numerical MCSE over 10% of the standard error"
    }
    else _disp_has 0 "of a standard error"
end

* Everything of the request but the reported intervals equals the auto
* request's on the same data: one pass, one search.
program define _disp_same_as_auto
    args auto_prefix
    tempname table lead ci b
    matrix `table' = e(inference_intervals)
    matrix `lead' = e(inference_leading)
    matrix `ci' = e(component_inference)
    matrix `b' = e(b)
    assert mreldif(`lead',`auto_prefix'_lead)==0
    assert mreldif(`ci',`auto_prefix'_ci)==0
    assert mreldif(`b',`auto_prefix'_b)==0
    forvalues t = 1/4 {
        foreach j of numlist 1/6 10/16 {
            assert `table'[`t',`j']==`auto_prefix'_table[`t',`j'] | ///
                (missing(`table'[`t',`j']) & missing(`auto_prefix'_table[`t',`j']))
        }
    }
end

fevc, simulate_data(ex4) clear
local routes `""backend(mata)" "backend(mata) algorithm(exact)""'
if `native' local routes `"`routes' "backend(rust)""'
foreach route of local routes {
    local common worker(worker_id) firm(firm_id) `route'
    * --- the default rule --------------------------------------------------
    _disp_log fevc log_wage, `common' inference(auto)
    assert "`e(inference_leading_request)'"=="auto"
    matrix auto_table = e(inference_intervals)
    matrix auto_lead = e(inference_leading)
    matrix auto_ci = e(component_inference)
    matrix auto_b = e(b)
    local auto_steps = e(inference_leading_steps)
    local auto_columns = e(inference_solver_columns)
    _disp_width
    _disp_old_texts_absent
    _disp_has 1 "Component inference (q = 1 interval when the lead share exceeds 10%)"
    _disp_has 1 "Component           Estimate  Std. err.  [95% conf. interval]  Interval  share"
    _disp_rows auto
    _disp_has 1 "Lead share: leading direction's share of the estimated sampling variance."
    * The bottleneck of ex4: the firm variance and the covariance are q = 1.
    assert e(inference_q1_components)==2
    _disp_has 1 "q = 1: Andrews-Mikusheva interval (KSS 2020, Sec. 7.1); no standard error."
    _disp_has 1 "e(V) is not posted: a component has a q = 1 interval."
    _disp_has 1 "Details: estat inference; estat decomposition, full; estat sample;"
    _disp_mcse 1

    * --- inference(highrank): normal intervals, the search and its shares --
    _disp_log fevc log_wage, `common' inference(highrank)
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_SE"
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_leading_request)'"=="highrank"
    assert strpos("`e(inference_regime_rule)'","inference(highrank)")==1
    assert e(inference_q1_components)==0 & e(inference_joint_posted)==1
    assert e(inference_share_threshold)==.1
    confirm matrix e(V)
    tempname table lead V
    matrix `table' = e(inference_intervals)
    matrix `lead' = e(inference_leading)
    matrix `V' = e(V)
    assert rowsof(`table')==4 & colsof(`table')==16
    assert rowsof(`lead')==4 & colsof(`lead')==28
    forvalues t = 1/4 {
        assert `table'[`t',7]==0
        assert `table'[`t',8]==`table'[`t',3] & `table'[`t',9]==`table'[`t',4]
        assert reldif(`table'[`t',2],sqrt(`V'[`t',`t']))<1e-12
    }
    _disp_same_as_auto auto
    assert e(inference_leading_steps)==`auto_steps'
    if "`route'"=="backend(rust)" {
        assert e(inference_solver_columns)==`auto_columns'
        assert e(inference_joint_status)==0
        assert e(inference_leading_steps)>0
    }
    _disp_width
    _disp_old_texts_absent
    _disp_has 1 "Component inference (* normal interval requested by inference(highrank))"
    _disp_rows highrank
    _disp_has 1 "Under the default rule (lead share above 10%), 2 components would get q = 1."
    _disp_has 1 "e(V) is posted: every component has a normal interval."
    _disp_has 0 "q = 1: Andrews-Mikusheva"
    _disp_mcse 1
    * estat inference: the details, and the warning about the effective rank
    * of components that keep the normal interval.
    _disp_log estat inference
    _disp_width
    foreach text in "Standard errors of the components" ///
        "Request: inference(highrank): the normal interval of every component" ///
        "Std. err. (normal form)" "Normal interval, lower" "q = 1 interval, lower" ///
        "q = 1 curvature" "q = 1 critical value" "q = 1 status" "Reported interval" ///
        "Reason" "Lead share" "Leading eigenvalue lambda1" "Second eigenvalue lambda2" ///
        "KSS's Lindeberg max w_i1^2" "Effective rank" "Search converged" ///
        "Method and assumptions" "e(V) is posted for the four components" {
        _disp_has 1 "`text'"
    }
    if e(inference_low_rank_warning)==1 {
        _disp_has 1 "Warning: a low effective rank means few directions carry a component;"
    }
    _disp_log estat inf
    _disp_has 1 "Std. err. (normal form)"
    capture noisily estat inference, detail
    assert _rc==198

    * --- inference(q1) -------------------------------------------------------
    _disp_log fevc log_wage, `common' inference(q1)
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_Q1"
    _disp_same_as_auto auto
    _disp_width
    _disp_old_texts_absent
    _disp_has 1 "Component inference (* q = 1 interval requested by inference(q1))"
    _disp_rows q1
    _disp_has 1 "e(V) is not posted: a component has a q = 1 interval."
    _disp_mcse 1
    _disp_log estat inference
    _disp_width
    _disp_has 1 "Request: inference(q1): the q = 1 interval wherever it exists"
    _disp_has 1 "requested"

    * --- auto again, with its reasons, and another level --------------------
    _disp_log fevc log_wage, `common' inference(auto) level(90)
    _disp_has 1 "[90% conf. interval]"
    _disp_log estat inference
    _disp_width
    _disp_has 1 "share > 10%"
    _disp_has 1 "share <= 10%"
    _disp_has 1 "sum: normal"

    * --- point estimates only: the MCSE line without the ratio -------------
    _disp_log fevc log_wage, `common' inference(none)
    _disp_width
    _disp_has 0 "Component inference"
    _disp_mcse 0
    _disp_log estat inference
    _disp_has 1 "Standard errors of the components: not computed for this fit."
}

* A request the default cannot carry: estat inference says why.
_disp_log fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) project(log_firm_size) projecteffect(firm)
_disp_log estat inference
_disp_width
if e(inference_auto)==1 _disp_has 1 "The default returned point estimates only"
else _disp_has 1 "With project() the components have point estimates only"

* Few probes: the numerical MCSE exceeds 10% of a standard error, and the
* line says so and suggests more projections.
fevc, simulate_data(ex2) clear
local routes "backend(mata)"
if `native' local routes `""backend(mata)" "backend(rust)""'
foreach route of local routes {
    _disp_log fevc log_wage productivity i.period, worker(worker_id) ///
        firm(firm_id) `route' deletion(observation) probes(8) inference(auto)
    _disp_width
    _disp_mcse 1
    _disp_has 1 "Warning: numerical MCSE over 10% of the standard error"
    _disp_has 1 "probes(2000)"
    * estat diagnostics keeps the MCSE in full.
    _disp_log estat diagnostics
    _disp_width
    _disp_has 1 "Approximate numerical MCSE (all main point probes)"
    _disp_has 1 "MCSE method: crossfit_if_v1"
    _disp_has 1 "Main point estimates only; excludes projection/inference calculations."
    _disp_has 1 "Standard errors of the components: type estat inference."
    * The default output shows no live progress; verbose does natively.
    if "`route'"=="backend(rust)" {
        _disp_log fevc log_wage productivity i.period, worker(worker_id) ///
            firm(firm_id) `route' inference(auto)
        _disp_has 0 "Total elapsed"
        _disp_has 0 "Prepared graph sample"
        _disp_log fevc log_wage productivity i.period, worker(worker_id) ///
            firm(firm_id) `route' inference(auto) verbose
        _disp_has 1 "Total elapsed"
        _disp_has 1 "Prepared graph sample"
    }
}

capture erase `"$DISP_FILE"'
macro drop DISP_FILE
set linesize `caller_linesize'
display "PASS test_inference_display.do native=`native'"
