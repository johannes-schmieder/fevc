*! fevc example data 0.5.0-rc.1 22sep2026
program define fevc__simulate_data, rclass
    version 18.0
    syntax , SIMULATE_data(string) [SEED(string) CLEAR]
    local example = lower(strtrim("`simulate_data'"))
    if !inlist("`example'", "ex1", "ex2", "ex3", "ex4", "ex5") {
        di as error "simulate_data() must be ex1, ex2, ex3, ex4, or ex5; see help fevc"
        exit 198
    }
    if "`seed'" != "" {
        capture confirm integer number `seed'
        if _rc {
            di as error "seed() must be an integer from 0 through 2147483647"
            exit 198
        }
        if missing(real("`seed'")) | real("`seed'")<0 | real("`seed'")>2147483647 {
            di as error "seed() must be an integer from 0 through 2147483647"
            exit 198
        }
        if "`example'" == "ex5" {
            di as error "ex5 is a fixed inference illustration and does not accept seed()"
            exit 198
        }
    }
    else {
        if "`example'" == "ex1" local seed 20260821
        if "`example'" == "ex2" local seed 20260820
        if "`example'" == "ex3" local seed 20260822
        if "`example'" == "ex4" local seed 20260824
        if "`example'" == "ex5" local seed 0
    }
    if "`clear'" == "" & (c(N)>0 | c(k)>0) {
        di as error "data are already in memory; specify clear to replace them"
        exit 4
    }

    // Preserve the old data on any failed generation, including a user break.
    // This nests safely inside fevc_run's preserve/restore scope.
    preserve
    local caller_rng = c(rng)
    local caller_state = c(rngstate)
    local caller_sortstate = c(sortrngstate)
    quietly set rng mt64
    local mt64_state = c(rngstate)
    capture noisily fevc__simulate_build, example(`example') seed(`seed')
    local rc = _rc
    // Also restore the inactive mt64 generator when the caller uses another RNG.
    quietly set rngstate `mt64_state'
    quietly set rng `caller_rng'
    quietly set rngstate `caller_state'
    quietly set sortrngstate `caller_sortstate'
    if `rc' exit `rc'
    return add
    restore, not
end

program define fevc__simulate_build, rclass
    version 18.0
    syntax , EXAMPLE(string) SEED(integer)
    quietly {
        clear
        set seed `seed'
        set sortseed 20260821

        if inlist("`example'", "ex1", "ex2") {
            local description "Positive sorting with controls"
            local workers = cond("`example'"=="ex1",10000,200)
            local firms = cond("`example'"=="ex1",3001,61)
            local spells 3
            local periods 2
            set obs `=`workers'*`spells'*`periods''
            generate long worker_id = ceil(_n/(`spells'*`periods'))
            bysort worker_id: generate byte within_worker = _n
            generate byte spell = ceil(within_worker/`periods')
            generate byte period = mod(within_worker-1,`periods')+1
            by worker_id: generate int step = runiformint(1,`firms'-1) if _n==1
            by worker_id: replace step = step[1]
            generate long firm_id = mod(worker_id-1+(spell-1)*step,`firms')+1
            generate long match_id = worker_id*10+spell
            bysort firm_id (worker_id within_worker): generate double firm_fe = rnormal() if _n==1
            by firm_id: replace firm_fe = firm_fe[1]
            bysort worker_id (within_worker): egen double mean_firm_fe = mean(firm_fe)
            // Workers at higher-premium firms have higher effects on average.
            by worker_id: generate double worker_fe = .5*mean_firm_fe+rnormal() if _n==1
            by worker_id: replace worker_fe = worker_fe[1]
            generate double productivity = rnormal()
            generate double error = 3*rnormal()
            generate double log_wage = 2+worker_fe+firm_fe+.30*productivity+.15*(period==2)+error
            local design "Three jobs per worker, two observations per job; positive sorting."
        }
        else if "`example'" == "ex3" {
            local description "Frequency weights and a separate target population"
            local workers 60
            local firms 15
            local spells 3
            set obs `=`workers'*`spells''
            generate long worker_id = ceil(_n/`spells')
            bysort worker_id: generate byte spell = _n
            generate long firm_id = mod(worker_id-1+cond(spell==1,0,cond(spell==2,1,7)),`firms')+1
            generate long match_id = worker_id*10+spell
            generate int frequency = 1+mod(worker_id+spell,3)
            generate double target_mass = 1+spell/2
            generate double worker_fe = rnormal() if spell==1
            bysort worker_id: replace worker_fe = worker_fe[1]
            bysort firm_id: generate double firm_fe = .6*rnormal() if _n==1
            bysort firm_id: replace firm_fe = firm_fe[1]
            generate double productivity = rnormal()
            generate double error = .45*rnormal()
            generate double log_wage = 2+worker_fe+firm_fe+.35*productivity+error
            local design "Three jobs per worker; frequency counts and separate stored-row target mass."
        }
        else if "`example'" == "ex4" {
            local description "Project firm effects on firm size"
            set obs 10
            generate byte origin_firm = _n
            expand 2*origin_firm
            generate long worker_id = _n
            generate double worker_fe = .4*rnormal()
            expand 4
            bysort worker_id: generate byte year = _n
            generate byte firm_id = cond(year<=2,origin_firm,mod(origin_firm,10)+1)
            // Average annual employment over the four-year panel.
            bysort firm_id: generate double firm_size = _N/4
            generate double log_firm_size = ln(firm_size)
            generate double firm_fe = .5*log_firm_size
            sort worker_id year
            generate double error = .15*rnormal()
            generate double log_wage = 2+worker_fe+firm_fe+error
            local design "Four years per worker, one move; true slope on log firm size = 0.5."
        }
        else {
            local description "Small fixed illustration of component inference"
            // Fixed values make this a reproducible syntax example, not a coverage experiment.
            set obs 24
            generate long worker_id = floor((_n-1)/4)
            generate byte period = mod(_n-1,4)
            generate double productivity = period-1.5
            generate double policy = period==2
            generate byte firm_id = .
            generate double error = .
            local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
            local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
            forvalues row = 1/24 {
                local value : word `row' of `firms'
                replace firm_id = `value' in `row'
                local value : word `row' of `noises'
                replace error = `value' in `row'
            }
            generate double worker_fe = .3*worker_id
            generate double firm_fe = -.2*firm_id
            generate double log_wage = 1.5+worker_fe+firm_fe+.4*productivity-.15*policy+error
            local design "Four observations per worker; fixed effects and disturbances (no random draws)."
        }

        label data "fevc `example': `description'"
        label variable worker_id "Worker identifier"
        label variable firm_id "Firm identifier"
        label variable worker_fe "True simulated worker effect"
        label variable firm_fe "True simulated firm effect"
        label variable error "Simulated disturbance"
        label variable log_wage "Simulated log wage"
        if inlist("`example'","ex1","ex2","ex3","ex5") {
            label variable productivity "Time-varying control"
        }
        if "`example'" == "ex3" {
            label variable frequency "Positive integer frequency weight"
            label variable target_mass "Stored-row mass for variance targets"
        }
        if "`example'" == "ex4" {
            label variable firm_size "Average annual employment"
            label variable log_firm_size "Log average annual employment"
        }

        tempvar worker_tag firm_tag mass term
        egen byte `worker_tag' = tag(worker_id)
        count if `worker_tag'
        local nworkers = r(N)
        egen byte `firm_tag' = tag(firm_id)
        count if `firm_tag'
        local nfirms = r(N)
        generate double `mass' = 1
        local weighting "observation-weighted"
        if "`example'" == "ex3" {
            replace `mass' = target_mass
            local weighting "target_mass-weighted (not multiplied by frequency)"
        }
        summarize `mass', meanonly
        tempname total mean_worker mean_firm truth
        scalar `total' = r(sum)
        generate double `term' = `mass'*worker_fe
        summarize `term', meanonly
        scalar `mean_worker' = r(sum)/`total'
        replace `term' = `mass'*firm_fe
        summarize `term', meanonly
        scalar `mean_firm' = r(sum)/`total'
        // Population moments of the realized effects, using the example's target mass.
        matrix `truth' = J(1,4,.)
        replace `term' = `mass'*(worker_fe-`mean_worker')^2
        summarize `term', meanonly
        matrix `truth'[1,1] = r(sum)/`total'
        replace `term' = `mass'*(firm_fe-`mean_firm')^2
        summarize `term', meanonly
        matrix `truth'[1,2] = r(sum)/`total'
        replace `term' = `mass'*(worker_fe-`mean_worker')*(firm_fe-`mean_firm')
        summarize `term', meanonly
        matrix `truth'[1,3] = r(sum)/`total'
        matrix `truth'[1,4] = `truth'[1,1]+`truth'[1,2]+2*`truth'[1,3]
        matrix colnames `truth' = worker_variance firm_variance worker_firm_covariance total_variance
        matrix rownames `truth' = true
    }
    di as text _newline "fevc `example': `description'"
    di as text "  Observations: " as result %9.0fc _N ///
        as text "    Workers: " as result %9.0fc `nworkers' ///
        as text "    Firms: " as result %9.0fc `nfirms'
    di as text "  `design'"
    if "`example'" != "ex5" di as text "  Simulation seed: " as result `seed'
    di as text _newline "True components in the generated sample (`weighting'):"
    di as text "  Var(worker effect)       = " as result %9.5f el(`truth',1,1)
    di as text "  Var(firm effect)         = " as result %9.5f el(`truth',1,2)
    di as text "  Cov(worker, firm)        = " as result %9.5f el(`truth',1,3)
    di as text "  Var(worker + firm)       = " as result %9.5f el(`truth',1,4)
    di as text "  True effects and disturbances: worker_fe, firm_fe, error."
    di as text "  Source: {stata viewsource fevc__simulate_data.ado:view data-generation code}"
    return scalar N = _N
    return scalar workers = `nworkers'
    return scalar firms = `nfirms'
    return scalar seed = cond("`example'"=="ex5",.,`seed')
    return matrix truth = `truth'
    return local example "`example'"
    return local weighting "`weighting'"
end
