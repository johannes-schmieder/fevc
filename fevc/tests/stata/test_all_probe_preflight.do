version 18.0
clear all
// Native preflight must validate with the small module and leave the FE runtime unloaded.
local oldstate `"`c(rngstate)'"'
quietly fevc__numerical request all none ""
mata: assert(vckss_nmc__module_api()==4)
capture mata: vckss__api_level()
assert _rc
assert `"`c(rngstate)'"'==`"`oldstate'"'
quietly fevc__numerical clear
// A previously loaded monolithic schema cannot be overwritten with new types.
mata: mata clear
mata:
real scalar vckss_nmc__schema()
{
    return(1)
}
end
capture quietly fevc__numerical request all none ""
assert _rc==498
assert "$VCKSS_NMC_ERR"=="STALE_NUMERICAL_RUNTIME"
assert `"`c(rngstate)'"'==`"`oldstate'"'
quietly fevc__numerical clear
mata: mata clear
// The earlier extracted module has incompatible private state, too.
mata:
real scalar vckss_nmc__module_api()
{
    return(2)
}
string scalar vckss_nmc__build_id()
{
    return("vckss-numerical-api2-bounded-replay")
}
real scalar vckss_nmc__schema()
{
    return(1)
}
end
capture quietly fevc__numerical request all none ""
assert _rc==498
assert "$VCKSS_NMC_ERR"=="STALE_NUMERICAL_RUNTIME"
assert `"`c(rngstate)'"'==`"`oldstate'"'
quietly fevc__numerical clear
mata: mata clear
di as result "PASS test_all_probe_preflight.do"
