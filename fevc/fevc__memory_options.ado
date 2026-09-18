*! Command-local memory policy
program define fevc__memory_options, rclass
    version 18.0
    args memory_gib memorycheck
    local memory_present = strtrim("`memory_gib'") != ""
    if `memory_present' {
        capture confirm number `memory_gib'
        if _rc {
            di as error "memory_gib() must be finite and positive"
            exit 198
        }
        local memory_gib = real("`memory_gib'")
        if missing(`memory_gib') | `memory_gib' <= 0 | floor(`memory_gib'*1024^3)<1 | `memory_gib'*1024^3>2^53 {
            di as error "memory_gib() must specify between 1 and 2^53 bytes"
            exit 198
        }
    }
    else local memory_gib = 0
    local memorycheck = lower(strtrim("`memorycheck'"))
    if "`memorycheck'" == "" local memorycheck warn
    if !inlist("`memorycheck'", "warn", "error", "off") {
        di as error "memorycheck() must be warn, error, or off"
        exit 198
    }
    global VCKSS_MEMORY_ACTIVE 1
    global VCKSS_MEMORY_PRESENT `memory_present'
    global VCKSS_MEMORY_BYTES = floor(`memory_gib'*1024^3)
    global VCKSS_MEMORY_FORECAST 0
    global VCKSS_MEMORY_CHECK `memorycheck'
    global VCKSS_MEMORY_MODE = cond("`memorycheck'"=="error",1,cond("`memorycheck'"=="warn",2,3))
    global VCKSS_MEMORY_ADVISORY = !`memory_present' | "`memorycheck'"!="error"
    return scalar memory_gib = `memory_gib'
    return scalar budget_present = `memory_present'
end
