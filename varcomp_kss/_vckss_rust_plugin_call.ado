program define _vckss_rust_plugin_call, rclass
    version 18.0
    gettoken plugin 0 : 0
    local parsed = strtrim(subinstr(`"`0'"', ",", "", 1))
    gettoken subcommand rest : parsed
    plugin call `plugin' `0'
    if lower(strtrim("`subcommand'")) == "lasterror" {
        return local native_error_status `"$__vckss_rust_error_status"'
        return local native_error_detail `"$__vckss_rust_error_detail"'
        capture macro drop __vckss_rust_error_status
        capture macro drop __vckss_rust_error_detail
    }
end
