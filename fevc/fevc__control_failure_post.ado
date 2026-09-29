*! Stable, failure-only control-certificate diagnostics.
program define fevc__control_failure_post, eclass
    version 18.0
    args detail
    if regexm(`"`detail'"', "^[A-Z_]+ \[([^]]+)\]") {
        ereturn local native_failure_phase = regexs(1)
    }
    if strpos(`"`detail'"',"FEVC_CONTROL_CERT_V1 ")==0 exit
    ereturn local control_certificate_schema "FEVC_CONTROL_CERT_V1"
    foreach key in population selected {
        if regexm(`"`detail'"', "(^|[ ;])`key'=([^ ;]+)") {
            ereturn local control_certificate_`key' = regexs(2)
        }
    }
    tempname diagnostic
    matrix `diagnostic'=J(1,12,.)
    local column=0
    foreach key in n q units bound transform_norm conditioning propagated ceiling original_residual canonical_bound accumulated_bound {
        local ++column
        if regexm(`"`detail'"', "(^|[ ;])`key'=([^ ;]+)") {
            matrix `diagnostic'[1,`column']=real(regexs(2))
        }
    }
    // A missing/infinite propagated bound is an explicit inconclusive failure.
    matrix `diagnostic'[1,12]=1
    matrix colnames `diagnostic'=N controls units bound transform_norm conditioning propagated ceiling original_residual canonical_bound accumulated_bound inconclusive
    ereturn matrix control_certificate_failure=`diagnostic'
end
