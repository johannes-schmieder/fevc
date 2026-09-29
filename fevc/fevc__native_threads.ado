program define fevc__native_threads, rclass
    version 18.0
    args requested context
    if "`requested'"=="post" & "`context'"=="" {
        fevc__native_threads_post
        exit
    }
    capture noisily fevc__native_threads_compute `"`requested'"'
    if _rc exit 198
    if "`context'"=="context" {
        foreach key in requested selected allocation supplied {
            local upper = upper("`key'")
            global VCKSS_NATIVE_`upper' = r(`key')
        }
        global VCKSS_NATIVE_SOURCE `"`r(source)'"'
    }
    return add
end

*! Native execution budget; independent of the Stata/MP license.
program define fevc__native_threads_compute, rclass
    version 18.0
    args requested
    local supplied = (strtrim(`"`requested'"')!="")
    local threads = c(processors)
    if `supplied' local threads = real(strtrim(`"`requested'"'))
    if missing(`threads') | `threads'!=floor(`threads') | !inrange(`threads',1,64) {
        di as error "nativethreads() must be an integer from 1 through 64"
        exit 198
    }
    local allocation = min(64,c(processors_mach))
    local source "host"
    local slots : environment NSLOTS
    if `"`slots'"'!="" {
        local scheduler = real(`"`slots'"')
        if missing(`scheduler') | `scheduler'!=floor(`scheduler') | `scheduler'<1 {
            di as error "NSLOTS must contain a positive integer allocation"
            exit 198
        }
        if c(processors)>`scheduler' {
            di as error "Stata processors exceed NSLOTS; set processors within the allocation"
            exit 198
        }
        local allocation = min(`allocation',`scheduler')
        local source "NSLOTS"
    }
    if `supplied' & `threads'>`allocation' {
        di as error "nativethreads() exceeds the available allocation (`allocation')"
        exit 198
    }
    return scalar requested = `threads'
    return scalar selected = min(`threads',`allocation')
    return scalar allocation = `allocation'
    return scalar supplied = `supplied'
    return local source "`source'"
end

program define fevc__native_threads_post, eclass
    version 18.0
    foreach key in requested selected supplied {
        local upper = upper("`key'")
        ereturn scalar native_threads_`key' = real("${VCKSS_NATIVE_`upper'}")
    }
    ereturn scalar native_thread_allocation = real("$VCKSS_NATIVE_ALLOCATION")
    ereturn local native_thread_allocation_source "$VCKSS_NATIVE_SOURCE"
    local capacity = .
    local active = .
    if "`e(status)'"!="WITHHELD" & "`e(backend_selected)'"=="rust" {
        local capacity = 1
        local active = 1
        capture confirm matrix e(rust_exact_execution)
        if !_rc {
            local capacity = e(rust_exact_execution)[1,5]
            local active = .
        }
        capture confirm scalar e(cmg_threads_used)
        if !_rc {
            local capacity = e(cmg_threads_used)
            local active = .
            capture confirm matrix e(full_cmg_receipt)
            if !_rc {
                if e(full_cmg_receipt)[1,7]>0 local active=e(full_cmg_receipt)[1,7]
            }
        }
        capture confirm scalar e(rust_execution_workers)
        if !_rc {
            local capacity = e(rust_execution_workers)
            local observed = max(e(rust_execution_active_workers),e(rust_execution_cmg_concurrency))
            local active = cond(`observed'>0,`observed',.)
        }
    }
    ereturn scalar native_threads_capacity = `capacity'
    ereturn scalar native_threads_active = `active'
end
