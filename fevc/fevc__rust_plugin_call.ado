program define fevc__rust_plugin_call, rclass
    version 18.0
    gettoken plugin 0 : 0, parse(" ,")
    local parsed = strtrim(subinstr(`"`0'"', ",", "", 1))
    gettoken subcommand rest : parsed
    if inlist("$VCKSS_REPORT_LEVEL","1","2") {
        fevc__progress $VCKSS_REPORT_LEVEL `plugin' `0'
    }
    else plugin call `plugin' `0'
    if lower(strtrim("`subcommand'")) == "lasterror" {
        return local native_error_status `"$__vckss_rust_error_status"'
        return local native_error_detail `"$__vckss_rust_error_detail"'
        capture macro drop __vckss_rust_error_status
        capture macro drop __vckss_rust_error_detail
    }
end
