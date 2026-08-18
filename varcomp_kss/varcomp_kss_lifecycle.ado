*! varcomp_kss lifecycle 0.3.0-dev 18aug2026

// This is an internal KSS-SCALE-1 prototype.  It deliberately uses only
// Stata's preserve/restore and tempfile save/clear/use mechanisms.  The
// numerical callback runs while the Stata dataset is empty; compressed Mata
// state, named Stata matrices, and scalars survive clear and may carry the
// numerical inputs and outputs.
program define varcomp_kss_lifecycle, rclass
    version 18.0
    syntax, METHOD(string) SAMPLE(varname numeric) CALLBACK(name) [ ///
        CALLBACKOptions(string asis) CERTIFY FORCEDISK ]

    local method = lower(strtrim("`method'"))
    if !inlist("`method'", "preserve", "tempfile") {
        di as error "method() must be preserve or tempfile"
        exit 198
    }
    if "`forcedisk'" != "" & "`method'" != "preserve" {
        di as error "forcedisk is allowed only with method(preserve)"
        exit 198
    }

    quietly count if missing(`sample') | !inlist(`sample', 0, 1)
    if r(N) {
        di as error "sample() must contain only zero and one"
        exit 459
    }

    local caller_N = _N
    local caller_k = c(k)
    local caller_changed = c(changed)
    local caller_filename `"`c(filename)'"'
    local caller_filedate `"`c(filedate)'"'
    local caller_sortedby : sortedby
    quietly count if `sample'
    local caller_sample_N = r(N)
    quietly _datasignature `sample', nonames
    local caller_sample_signature `"`r(datasignature)'"'
    local caller_data_signature
    if "`certify'" != "" {
        quietly _datasignature
        local caller_data_signature `"`r(datasignature)'"'
    }

    local stata_tmpdir `"`c(tmpdir)'"'
    local stata_version `"`c(stata_version)'"'
    local env_statatmp : environment STATATMP
    local env_tmpdir : environment TMPDIR
    local caller_max_preservemem = c(max_preservemem)

    quietly _vckss_lifecycle_memory, stage(before)
    foreach quantity in data_used data_alloc mata_alloc total_alloc {
        local mem_before_`quantity' = r(`quantity'_bytes)
    }
    local memory_schema `"`r(memory_schema)'"'

    quietly _vckss_lifecycle_timer_ids
    local transition_timer = r(transition_timer)
    local work_timer = r(work_timer)
    local restore_timer = r(restore_timer)
    foreach timer in `transition_timer' `work_timer' `restore_timer' {
        quietly timer clear `timer'
    }

    local preserve_forced_disk = 0
    local transition_rc = 0
    local restore_rc = 0
    local changed_guard_active = 0
    tempfile lifecycle_data

    quietly timer on `transition_timer'
    if "`method'" == "preserve" {
        local reset_preservemem = 0
        if "`forcedisk'" != "" & !missing(c(max_preservemem)) {
            local old_preservemem_text : display %21.0f c(max_preservemem)
            local old_preservemem_text = strtrim("`old_preservemem_text'")
            capture quietly set max_preservemem 0
            if _rc {
                local transition_rc = _rc
            }
            else {
                local reset_preservemem = 1
                local preserve_forced_disk = 1
            }
        }
        if !`transition_rc' {
            capture quietly preserve
            local transition_rc = _rc
        }
        if `reset_preservemem' {
            capture quietly set max_preservemem `old_preservemem_text'
            if _rc & !`transition_rc' local transition_rc = _rc
        }
        if !`transition_rc' {
            capture quietly clear
            local transition_rc = _rc
        }
    }
    else {
        // preserve, changed is Stata's supported guard for the caller's
        // modified-since-save flag.  The data themselves use a Stata DTA in
        // c(tmpdir); no package-specific serialization is involved.
        capture quietly preserve, changed
        local transition_rc = _rc
        if !`transition_rc' local changed_guard_active = 1
        if !`transition_rc' {
            capture quietly save `"`lifecycle_data'"', replace
            local transition_rc = _rc
        }
        if !`transition_rc' {
            capture quietly clear
            local transition_rc = _rc
        }
    }
    quietly timer off `transition_timer'

    if `transition_rc' {
        // A full preserve restores automatically at program termination.
        // The changed-only guard never cleared data unless save succeeded.
        if `changed_guard_active' capture quietly restore
        quietly _vckss_lifecycle_release_timers,                 ///
            timers(`transition_timer' `work_timer' `restore_timer')
        di as error "dataset lifecycle transition failed"
        exit `transition_rc'
    }

    quietly _vckss_lifecycle_memory, stage(cleared)
    foreach quantity in data_used data_alloc mata_alloc total_alloc {
        local mem_cleared_`quantity' = r(`quantity'_bytes)
    }

    quietly timer on `work_timer'
    if strtrim(`"`callbackoptions'"') == "" {
        capture noisily `callback'
    }
    else {
        capture noisily `callback', `callbackoptions'
    }
    local work_rc = _rc
    quietly timer off `work_timer'

    quietly _vckss_lifecycle_memory, stage(after_work)
    foreach quantity in data_used data_alloc mata_alloc total_alloc {
        local mem_afterwork_`quantity' = r(`quantity'_bytes)
    }

    // The callback contract leaves no Stata dataset behind.  Clear anyway
    // so restoration can never overlap a callback-created dataset.
    capture quietly clear

    quietly timer on `restore_timer'
    if "`method'" == "preserve" {
        capture quietly restore
        local restore_rc = _rc
    }
    else {
        capture quietly use `"`lifecycle_data'"', clear
        local restore_rc = _rc
        if `changed_guard_active' {
            // For preserve, changed, restore changes only the caller's
            // modified-since-save flag.  It does not replace the DTA loaded
            // above.
            capture quietly restore
            if _rc & !`restore_rc' local restore_rc = _rc
        }
    }
    quietly timer off `restore_timer'

    if `restore_rc' {
        quietly _vckss_lifecycle_release_timers,                 ///
            timers(`transition_timer' `work_timer' `restore_timer')
        di as error "caller dataset restoration failed"
        exit `restore_rc'
    }

    local sample_restored = 1
    capture confirm numeric variable `sample'
    if _rc local sample_restored = 0
    if `sample_restored' {
        quietly count if missing(`sample') | !inlist(`sample', 0, 1)
        if r(N) local sample_restored = 0
    }
    if `sample_restored' {
        quietly count if `sample'
        if r(N) != `caller_sample_N' local sample_restored = 0
    }
    local restored_sample_signature
    if `sample_restored' {
        quietly _datasignature `sample', nonames
        local restored_sample_signature `"`r(datasignature)'"'
        if `"`restored_sample_signature'"' !=                     ///
            `"`caller_sample_signature'"' local sample_restored = 0
    }

    local data_restored = (_N == `caller_N' & c(k) == `caller_k')
    local restored_data_signature
    if "`certify'" != "" & `data_restored' {
        quietly _datasignature
        local restored_data_signature `"`r(datasignature)'"'
        if `"`restored_data_signature'"' !=                       ///
            `"`caller_data_signature'"' local data_restored = 0
    }
    local restored_sortedby : sortedby
    if `"`restored_sortedby'"' != `"`caller_sortedby'"'          ///
        local data_restored = 0

    local filename_restored =                                     ///
        (`"`c(filename)'"' == `"`caller_filename'"')
    local filedate_restored =                                     ///
        (`"`c(filedate)'"' == `"`caller_filedate'"')
    local changed_restored = (c(changed) == `caller_changed')

    quietly _vckss_lifecycle_memory, stage(restored)
    foreach quantity in data_used data_alloc mata_alloc total_alloc {
        local mem_restored_`quantity' = r(`quantity'_bytes)
    }

    foreach label in transition work restore {
        local timer = ``label'_timer'
        quietly timer list `timer'
        local `label'_seconds = r(t`timer')
    }
    quietly _vckss_lifecycle_release_timers,                     ///
        timers(`transition_timer' `work_timer' `restore_timer')

    if !`sample_restored' | !`data_restored' {
        di as error "caller dataset or sample marker failed restoration certification"
        exit 498
    }

    return scalar lifecycle_work_rc = `work_rc'
    return scalar sample_restored = `sample_restored'
    return scalar data_restored = `data_restored'
    return scalar filename_restored = `filename_restored'
    return scalar filedate_restored = `filedate_restored'
    return scalar changed_restored = `changed_restored'
    return scalar preserve_forced_disk = `preserve_forced_disk'
    return scalar caller_max_preservemem_bytes = `caller_max_preservemem'
    return scalar transition_seconds = `transition_seconds'
    return scalar work_seconds = `work_seconds'
    return scalar restore_seconds = `restore_seconds'
    foreach stage in before cleared afterwork restored {
        foreach quantity in data_used data_alloc mata_alloc total_alloc {
            return scalar mem_`stage'_`quantity'_bytes =                 ///
                `mem_`stage'_`quantity''
        }
    }
    return local method "`method'"
    return local memory_schema `"`memory_schema'"'
    return local stata_tmpdir `"`stata_tmpdir'"'
    return local env_statatmp `"`env_statatmp'"'
    return local env_tmpdir `"`env_tmpdir'"'
    return local stata_version `"`stata_version'"'
    return local caller_sample_signature `"`caller_sample_signature'"'
    return local restored_sample_signature `"`restored_sample_signature'"'
    if "`certify'" != "" {
        return local caller_data_signature `"`caller_data_signature'"'
        return local restored_data_signature `"`restored_data_signature'"'
    }

    if `work_rc' {
        di as error "compressed numerical callback failed after caller-data restoration"
        exit `work_rc'
    }
end


// Best-effort in-process memory snapshots.  Stata documents that the hidden
// r() names produced by memory may change, so these are lifecycle diagnostics,
// never substitutes for process RSS or qacct maxvmem.
program define _vckss_lifecycle_memory, rclass
    version 18.0
    syntax [, STAGE(string)]

    quietly memory
    foreach source in data_data_u data_strl_u data_etc_u data_oh_u       ///
        data_data_a data_strl_a data_etc_a data_oh_a                    ///
        stata_matrices_a stata_ado_a stata_sr_a maxvar_a other_a       ///
        mata_matrices_a mata_functions_a {
        tempname value_`source'
        scalar `value_`source'' = .
        capture scalar `value_`source'' = r(`source')
    }

    tempname data_used data_allocated mata_allocated total_allocated
    scalar `data_used' =                                                ///
        scalar(`value_data_data_u') + scalar(`value_data_strl_u') +    ///
        scalar(`value_data_etc_u') + scalar(`value_data_oh_u')
    scalar `data_allocated' =                                           ///
        scalar(`value_data_data_a') + scalar(`value_data_strl_a') +    ///
        scalar(`value_data_etc_a') + scalar(`value_data_oh_a')
    scalar `mata_allocated' =                                           ///
        scalar(`value_mata_matrices_a') +                               ///
        scalar(`value_mata_functions_a')
    scalar `total_allocated' = scalar(`data_allocated') +               ///
        scalar(`value_stata_matrices_a') + scalar(`value_stata_ado_a') + ///
        scalar(`value_stata_sr_a') + scalar(`value_maxvar_a') +         ///
        scalar(`value_other_a') + scalar(`mata_allocated')

    return scalar data_used_bytes = scalar(`data_used')
    return scalar data_alloc_bytes = scalar(`data_allocated')
    return scalar mata_alloc_bytes = scalar(`mata_allocated')
    return scalar total_alloc_bytes = scalar(`total_allocated')
    return local stage "`stage'"
    return local memory_schema "STATA_MEMORY_R18_BEST_EFFORT"
end


// Acquire three currently unused Stata timer IDs without disturbing caller
// timers.  IDs 51--69 are reserved only for the duration of this prototype.
program define _vckss_lifecycle_timer_ids, rclass
    version 18.0
    local timers
    forvalues id = 51/69 {
        quietly capture timer list `id'
        if missing(r(t`id')) local timers `timers' `id'
        local timer_count : word count `timers'
        if `timer_count' == 3 continue, break
    }
    local timer_count : word count `timers'
    if `timer_count' != 3 {
        di as error "three free lifecycle timer IDs were not available"
        exit 498
    }
    return scalar transition_timer = real(word("`timers'", 1))
    return scalar work_timer = real(word("`timers'", 2))
    return scalar restore_timer = real(word("`timers'", 3))
end


program define _vckss_lifecycle_release_timers
    version 18.0
    syntax, TIMERS(numlist integer min=3 max=3)
    foreach timer of numlist `timers' {
        capture quietly timer off `timer'
        capture quietly timer clear `timer'
    }
end


// SCC evidence hook.  Ordinary commands do not set KSS_PHASE_FILE, so this
// helper returns without validating phase() or touching the filesystem.  An
// evidence wrapper may point KSS_PHASE_FILE at a narrow absolute path below
// c(tmpdir) or STATATMP.  The helper writes the tiny marker directly with
// Stata file I/O and never consumes estimator RNG state.  Evidence samplers
// must tolerate a transient unreadable marker while replace is in progress.
program define _vckss_lifecycle_phase, rclass
    version 18.0
    syntax, PHASE(string)

    local phase_file : environment KSS_PHASE_FILE
    if strtrim(`"`phase_file'"') == "" {
        return local status "DISABLED"
        return local phase_file ""
        exit 0
    }

    quietly _vckss_lifecycle_phase_write,                      ///
        phase(`phase') phasefile(`"`phase_file'"')
    return add
end


// Explicit-path worker retained for focused tests.  Production code calls
// only _vckss_lifecycle_phase, which obtains the path from the environment.
program define _vckss_lifecycle_phase_write, rclass
    version 18.0
    syntax, PHASE(string) PHASEFILE(string asis)

    local phase = lower(strtrim("`phase'"))
    if !inlist("`phase'", "import_selection",                    ///
        "compression_transition", "numerical", "restoration") {
        di as error "KSS_PHASE_INVALID: unregistered phase token"
        exit 198
    }

    local phase_file `phasefile'
    local phase_file = strtrim(`"`phase_file'"')
    forvalues pass = 1/8 {
        local phase_file = subinstr(`"`phase_file'"', "//", "/", .)
    }
    if substr(`"`phase_file'"', 1, 1) != "/" |                  ///
        !regexm(`"`phase_file'"', "^[A-Za-z0-9_./-]+$") |       ///
        regexm(`"`phase_file'"', "(^|/)[.](/|$)") |             ///
        regexm(`"`phase_file'"', "(^|/)[.][.](/|$)") |          ///
        substr(`"`phase_file'"', -1, 1) == "/" {
        di as error "KSS_PHASE_PATH_INVALID: marker path must be a narrow absolute file path"
        exit 198
    }

    local allowed = 0
    local stata_tmpdir `"`c(tmpdir)'"'
    local env_statatmp : environment STATATMP
    foreach candidate in stata_tmpdir env_statatmp {
        local root `"``candidate''"'
        forvalues pass = 1/8 {
            local root = subinstr(`"`root'"', "//", "/", .)
        }
        while strlen(`"`root'"') > 1 &                           ///
            substr(`"`root'"', -1, 1) == "/" {
            local root = substr(`"`root'"', 1, strlen(`"`root'"')-1)
        }
        if substr(`"`root'"', 1, 1) == "/" &                    ///
            regexm(`"`root'"', "^[A-Za-z0-9_./-]+$") &          ///
            !regexm(`"`root'"', "(^|/)[.][.](/|$)") {
            local prefix `"`root'/"'
            if `"`root'"' == "/" local prefix "/"
            if substr(`"`phase_file'"', 1, strlen(`"`prefix'"')) == ///
                `"`prefix'"' local allowed = 1
        }
    }
    if !`allowed' {
        di as error "KSS_PHASE_PATH_INVALID: marker path is outside c(tmpdir) and STATATMP"
        exit 198
    }

    local last_slash = strrpos(`"`phase_file'"', "/")
    local phase_dir = substr(`"`phase_file'"', 1, `last_slash'-1)
    local phase_base = substr(`"`phase_file'"', `last_slash'+1, .)
    if strtrim(`"`phase_base'"') == "" {
        di as error "KSS_PHASE_PATH_INVALID: marker filename is empty"
        exit 198
    }

    tempname marker_handle
    capture file open `marker_handle' using `"`phase_file'"',      ///
        write text replace
    local marker_rc = _rc
    if !`marker_rc' {
        capture file write `marker_handle' "`phase'"
        local marker_rc = _rc
    }
    capture file close `marker_handle'
    if `marker_rc' {
        di as error "KSS_PHASE_IO_FAILED: could not write phase marker"
        exit 603
    }
    if fileread(`"`phase_file'"') != "`phase'" {
        di as error "KSS_PHASE_IO_FAILED: phase marker failed verification"
        exit 603
    }

    return local status "WRITTEN"
    return local phase "`phase'"
    return local phase_file `"`phase_file'"'
end
