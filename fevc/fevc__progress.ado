*! Runtime reporting transport; estimation options and receipts are unchanged.
program define fevc__progress, rclass
    version 18.0
    gettoken level 0 : 0
    gettoken plugin 0 : 0, parse(" ,")
    local comma = strpos(`"`0'"', ",")
    local data = substr(`"`0'"', 1, `comma'-1)
    local options = substr(`"`0'"', `comma'+1, .)
    gettoken operation rest : options
    local reports = inlist("`operation'", "prepare", "augmentstayers", "augmentprojection") | ///
        substr("`operation'",1,5)=="solve" | substr("`operation'",1,16)=="augmentcomponent"
    if `reports' & inlist("`level'","1","2") & c(noisily) {
        if "$VCKSS_REPORT_API" == "1" {
            if "`level'"=="2" & "`operation'"=="prepare" {
                if "$VCKSS_MEMORY_PRESENT"=="1" {
                    di as txt "  Memory budget: " as result %9.3f (real("$VCKSS_MEMORY_BYTES")/1024^3) ///
                        as txt " GiB; policy=$VCKSS_MEMORY_CHECK"
                }
                else di as txt "  Memory budget: not supplied; automatic batches are not memory-limited"
            }
            plugin call `plugin' `data', reportv1 `level' `options'
            exit
        }
        if "$VCKSS_REPORT_NOTICE" != "1" {
            di as txt "Note: this Rust plugin does not provide live progress; estimation continues."
            global VCKSS_REPORT_NOTICE 1
        }
    }
    plugin call `plugin' `0'
end
