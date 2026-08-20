*! varcomp_kss_run 0.3.0-dev 20aug2026
*! Run marked examples embedded in varcomp_kss.sthlp.
*! The marker convention follows Robert Picard's geo2xy pattern as adapted
*! by Johannes F. Schmieder's cellgraph_run.ado.

program define varcomp_kss_run
    version 18.0

    syntax anything(name=example_name id="example name") using/

    preserve
    capture noisily _vckss_run_example `example_name' using `"`using'"'
    local example_rc = _rc
    capture restore
    local restore_rc = _rc
    if `restore_rc' {
        di as error "varcomp_kss_run could not restore the caller's data"
        exit 498
    }
    exit `example_rc'
end

program define _vckss_run_example
    version 18.0

    syntax anything(name=example_name id="example name") using/

    quietly {
        capture findfile `"`using'"'
        if _rc {
            noisily di as error `"help file `using' was not found on the Stata adopath"'
            exit 601
        }
        local help_file `"`r(fn)'"'

        infix str244 source_line 1-244 using `"`help_file'"', clear
        generate long source_number = _n

        summarize source_number if strpos(source_line,             ///
            "{* example_start - `example_name'}{...}"), meanonly
        if missing(r(min)) {
            noisily di as error `"example `example_name' was not found in `using'"'
            exit 111
        }
        local first_line = r(min)+1

        summarize source_number if source_number >= `first_line' & ///
            strpos(source_line,"{* example_end}{...}"), meanonly
        if missing(r(min)) {
            noisily di as error `"example `example_name' has no closing marker in `using'"'
            exit 111
        }
        local last_line = r(min)-1
        if `last_line' < `first_line' {
            noisily di as error `"example `example_name' contains no executable code"'
            exit 111
        }

        keep in `first_line'/`last_line'
    }

    tempfile example_do
    outfile source_line using `"`example_do'"', noquote
    do `"`example_do'"'
end
