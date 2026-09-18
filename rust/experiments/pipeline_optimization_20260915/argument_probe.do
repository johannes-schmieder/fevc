version 18.0
clear all
set more off
set linesize 255
args binary
program argprobe, plugin using(`"`binary'"')
plugin call argprobe, solve 1 81227 7 0 0 auto 1e-12 10000 exact observation joint ///
    500 5000 1e-10 1e-10 auto auto movers explicit observation 0 0 50000000 3 4 1 ///
    2290599403 1137066349 auto auto 1 0
display as result "PASS argument_probe.do"
