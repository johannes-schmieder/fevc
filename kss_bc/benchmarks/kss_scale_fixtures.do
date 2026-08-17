*! KSS-SCALE-1 deterministic scaling fixture wrapper 16aug2026

version 18.0

capture program drop kssbc_scale_fixture
program define kssbc_scale_fixture, rclass
    version 18.0
    syntax , DESIGN(string) COPIES(integer) WORKER(varname numeric) ///
        FIRM(varname numeric) DELETIONid(varname numeric) ///
        OUTCOME(varname numeric) [FREQuency(varname numeric) ///
        TARGET(varname numeric) COPYVAR(name) CONNECTORVAR(name) ///
        ROWKEY(name)]

    capture mata: assert(kssbc_scale_fixture__api_level() == 1 & ///
        kssbc_scale_fixture__build_id() == "kss-scale-fixtures-api1")
    if _rc {
        di as error "kss_scale_fixtures.mata API 1 is not loaded"
        exit 3000
    }

    local design = lower(strtrim("`design'"))
    local design = subinstr("`design'","-","_",.)
    if !inlist("`design'","well_connected","ring") {
        di as error "design() must be well_connected or ring"
        exit 198
    }
    if `copies' < 2 {
        di as error "copies() must be an integer of at least two"
        exit 198
    }
    if "`copyvar'" == "" local copyvar "scale_copy"
    if "`connectorvar'" == "" local connectorvar "scale_connector"
    if "`rowkey'" == "" local rowkey "scale_row"
    local required_names "`worker' `firm' `deletionid' `outcome'"
    local unique_required : list uniq required_names
    local required_count : word count `required_names'
    local unique_count : word count `unique_required'
    if `required_count' != `unique_count' {
        di as error "worker(), firm(), deletionid(), and outcome() must be distinct"
        exit 198
    }
    if "`frequency'" != "" {
        local reserved "`worker' `firm' `deletionid' `outcome'"
        local frequency_reserved : list frequency in reserved
        if `frequency_reserved' {
            di as error "frequency() must be distinct from IDs and outcome()"
            exit 198
        }
    }
    if "`target'" != "" {
        local reserved "`worker' `firm' `deletionid' `outcome'"
        local target_reserved : list target in reserved
        if `target_reserved' {
            di as error "target() must be distinct from IDs and outcome()"
            exit 198
        }
    }
    confirm new variable `copyvar'
    confirm new variable `connectorvar'
    if inlist("`rowkey'","`worker'","`firm'","`deletionid'", ///
        "`outcome'","`frequency'","`target'","`copyvar'", ///
        "`connectorvar'") {
        di as error "rowkey() must not overwrite a fixture input or marker"
        exit 198
    }
    capture confirm variable `rowkey'
    if !_rc confirm numeric variable `rowkey'

    quietly count
    if r(N) == 0 {
        di as error "the base fixture is empty"
        exit 2000
    }
    foreach variable in `worker' `firm' `deletionid' {
        capture assert !missing(`variable') & `variable' == floor(`variable')
        if _rc {
            di as error "fixture identifiers must be finite integers"
            exit 459
        }
    }
    capture assert !missing(`outcome')
    if _rc {
        di as error "outcome() must be nonmissing on the base fixture"
        exit 459
    }
    if "`frequency'" != "" {
        capture assert !missing(`frequency') & `frequency' > 0 & ///
            `frequency' == floor(`frequency')
        if _rc {
            di as error "frequency() must contain positive integers"
            exit 459
        }
    }
    if "`target'" != "" {
        capture assert !missing(`target') & `target' >= 0
        if _rc {
            di as error "target() must be finite and nonnegative"
            exit 459
        }
    }

    tempvar worker_dense firm_dense deletion_dense base_row unit_frequency
    quietly egen double `worker_dense' = group(`worker')
    quietly egen double `firm_dense' = group(`firm')
    quietly egen double `deletion_dense' = group(`deletionid')
    local diagnostic_frequency "`frequency'"
    if "`diagnostic_frequency'" == "" {
        quietly generate double `unit_frequency' = 1
        local diagnostic_frequency "`unit_frequency'"
    }

    tempname base_diagnostics fixture_diagnostics meta_metrics
    local base_status
    local base_message
    mata: kssbc_scale__stata_diagnose( ///
        "`worker_dense'","`firm_dense'","`deletion_dense'", ///
        "`diagnostic_frequency'","`base_diagnostics'", ///
        "base_status","base_message")
    matrix colnames `base_diagnostics' = rows physical workers firms ///
        cells deletion_units components bridge_units ///
        minimum_weighted_degree maximum_weighted_degree
    if "`base_status'" != "CONVERGED" {
        di as error "base fixture diagnostic failed: `base_status': `base_message'"
        exit 459
    }
    if el(`base_diagnostics',1,7) != 1 {
        di as error "BASE_FIXTURE_DISCONNECTED: base fixture must be connected"
        exit 459
    }
    if el(`base_diagnostics',1,8) != 0 {
        di as error "BASE_FIXTURE_NOT_DELETION_SAFE: base deletion units contain bridges"
        exit 459
    }

    local base_rows = el(`base_diagnostics',1,1)
    local base_physical = el(`base_diagnostics',1,2)
    local base_workers = el(`base_diagnostics',1,3)
    local base_firms = el(`base_diagnostics',1,4)
    local base_cells = el(`base_diagnostics',1,5)
    local base_units = el(`base_diagnostics',1,6)
    local pair_count = cond("`design'" == "well_connected", ///
        `copies'*(`copies'-1)/2,cond(`copies' == 2,1,`copies'))
    local connector_workers = 2*`pair_count'
    local connector_rows = 4*`pair_count'
    local expected_rows = `copies'*`base_rows'+`connector_rows'
    local expected_physical = `copies'*`base_physical'+`connector_rows'
    local expected_workers = `copies'*`base_workers'+`connector_workers'
    local expected_firms = `copies'*`base_firms'
    local expected_cells = `copies'*`base_cells'+`connector_rows'
    local expected_units = `copies'*`base_units'+`connector_rows'
    foreach bound in expected_rows expected_physical expected_workers ///
        expected_firms expected_cells expected_units {
        if ``bound'' >= 2^53 {
            di as error "FIXTURE_INTEGER_LIMIT: `bound' is not exactly representable"
            exit 459
        }
    }

    local semantic_sort "`outcome'"
    foreach candidate in frequency target {
        local candidate_name "``candidate''"
        if "`candidate_name'" != "" {
            local candidate_position : list posof "`candidate_name'" in semantic_sort
            if `candidate_position' == 0 {
                local semantic_sort "`semantic_sort' `candidate_name'"
            }
        }
    }
    quietly recast double `worker' `firm' `deletionid'
    quietly sort `worker_dense' `firm_dense' `deletion_dense' `semantic_sort'
    quietly generate double `base_row' = _n
    quietly expand `copies'
    quietly bysort `base_row': generate int `copyvar' = _n
    quietly generate byte `connectorvar' = 0
    quietly replace `worker' = `worker_dense'+ ///
        (`copyvar'-1)*`base_workers'
    quietly replace `firm' = `firm_dense'+ ///
        (`copyvar'-1)*`base_firms'
    quietly replace `deletionid' = `deletion_dense'+ ///
        (`copyvar'-1)*`base_units'

    mata: kssbc_scale__stata_connect( ///
        "`design'",`copies',`base_workers',`base_firms',`base_units', ///
        "`worker'","`firm'","`deletionid'","`outcome'", ///
        "`diagnostic_frequency'","`target'","`copyvar'", ///
        "`connectorvar'", ///
        "`meta_metrics'")
    matrix colnames `meta_metrics' = pair_count connector_workers ///
        connector_rows copy_cut_conductance normalized_lambda2 ///
        normalized_lambda_max normalized_condition_proxy

    quietly sort `connectorvar' `copyvar' `worker' `firm' `deletionid' ///
        `semantic_sort'
    capture confirm variable `rowkey'
    if _rc quietly generate double `rowkey' = _n
    else quietly replace `rowkey' = _n
    quietly isid `rowkey'

    local fixture_status
    local fixture_message
    mata: kssbc_scale__stata_diagnose( ///
        "`worker'","`firm'","`deletionid'", ///
        "`diagnostic_frequency'","`fixture_diagnostics'", ///
        "fixture_status","fixture_message")
    matrix colnames `fixture_diagnostics' = rows physical workers firms ///
        cells deletion_units components bridge_units ///
        minimum_weighted_degree maximum_weighted_degree
    if "`fixture_status'" != "CONVERGED" {
        di as error "constructed fixture diagnostic failed: `fixture_status': `fixture_message'"
        exit 459
    }
    if el(`fixture_diagnostics',1,1) != `expected_rows' | ///
        el(`fixture_diagnostics',1,2) != `expected_physical' | ///
        el(`fixture_diagnostics',1,3) != `expected_workers' | ///
        el(`fixture_diagnostics',1,4) != `expected_firms' | ///
        el(`fixture_diagnostics',1,5) != `expected_cells' | ///
        el(`fixture_diagnostics',1,6) != `expected_units' {
        di as error "FIXTURE_DIMENSION_MISMATCH: constructed counts differ from expectations"
        exit 459
    }
    if el(`fixture_diagnostics',1,7) != 1 | ///
        el(`fixture_diagnostics',1,8) != 0 {
        di as error "FIXTURE_GRAPH_CERTIFICATE_FAILED: fixture is disconnected or has a bridge"
        exit 459
    }

    return local status "CONVERGED"
    return local design "`design'"
    return scalar copies = `copies'
    return scalar base_rows = `base_rows'
    return scalar base_physical = `base_physical'
    return scalar base_workers = `base_workers'
    return scalar base_firms = `base_firms'
    return scalar base_cells = `base_cells'
    return scalar base_deletion_units = `base_units'
    return scalar connector_pairs = `pair_count'
    return scalar connector_workers = `connector_workers'
    return scalar connector_firms = 0
    return scalar connector_rows = `connector_rows'
    return scalar connector_physical = `connector_rows'
    return scalar connector_cells = `connector_rows'
    return scalar connector_deletion_units = `connector_rows'
    return scalar expected_rows = `expected_rows'
    return scalar expected_physical = `expected_physical'
    return scalar expected_workers = `expected_workers'
    return scalar expected_firms = `expected_firms'
    return scalar expected_cells = `expected_cells'
    return scalar expected_deletion_units = `expected_units'
    return scalar copy_cut_conductance = el(`meta_metrics',1,4)
    return scalar normalized_lambda2 = el(`meta_metrics',1,5)
    return scalar normalized_lambda_max = el(`meta_metrics',1,6)
    return scalar normalized_condition_proxy = el(`meta_metrics',1,7)
    return scalar minimum_weighted_degree = ///
        el(`fixture_diagnostics',1,9)
    return scalar maximum_weighted_degree = ///
        el(`fixture_diagnostics',1,10)
    return matrix base_diagnostics = `base_diagnostics'
    return matrix fixture_diagnostics = `fixture_diagnostics'
    return matrix meta_metrics = `meta_metrics'
end
