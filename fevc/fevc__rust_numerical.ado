*! Additive numerical V1 fetch; no old receipt layout is repurposed.
program define fevc__rust_numerical, rclass
    version 18.0
    capture noisily _fevc_rust_nmc_fetch `0'
    local rc = _rc
    if !`rc' return add
    foreach name in status executed error point_rhs {
        capture scalar drop __vckss_nmc_`name'
    }
    exit `rc'
end

program define _fevc_rust_nmc_fetch, rclass
    version 18.0
    gettoken plugin 0 : 0
    gettoken handle 0 : 0, parse(" ,")
    syntax, PROBES(integer)
    capture confirm integer number `handle'
    if _rc | real("`handle'")<=0 | `probes'<2 exit 198
    tempname cond lev raw usable se meta rhs
    foreach matrix in cond lev raw usable {
        matrix ``matrix''=J(3,3,.)
    }
    matrix `se'=J(1,4,.)
    matrix `meta'=J(1,15,.)
    matrix `rhs'=J(`probes',3,.)
    foreach name in status executed error point_rhs {
        capture scalar drop __vckss_nmc_`name'
    }
    fevc__rust_plugin_call `plugin', numericalresultv1 `handle' ///
        `cond' `lev' `raw' `usable' `se' `meta' `rhs'
    local code=scalar(__vckss_nmc_status)
    local statuses exact_zero ok_local ok_local_psd_adjusted unstable_nonpsd ///
        nonsmooth_adjustment nonfinite_derivative replay_failed
    if !inrange(`code',1,7) | `code'!=floor(`code') exit 498
    local status : word `code' of `statuses'
    local count=el(`meta',1,5)
    if missing(`count') | `count'<0 | `count'>`probes' | `count'!=floor(`count') exit 498
    if `count'>0 matrix `rhs'=`rhs'[1..`count',1..3]
    else mata: st_matrix("`rhs'",J(0,3,.))
    return matrix conditional=`cond'
    return matrix leverage=`lev'
    return matrix raw=`raw'
    return matrix usable=`usable'
    return matrix mcse=`se'
    return matrix metadata=`meta'
    if `count'>0 return matrix replay_rhs=`rhs'
    return scalar replay_rhs_count=`count'
    return scalar executed_replay_rhs=scalar(__vckss_nmc_executed)
    return scalar replay_error_code=scalar(__vckss_nmc_error)
    return scalar point_rhs=scalar(__vckss_nmc_point_rhs)
    return local status "`status'"
    foreach name in status executed error point_rhs {
        capture scalar drop __vckss_nmc_`name'
    }
end
