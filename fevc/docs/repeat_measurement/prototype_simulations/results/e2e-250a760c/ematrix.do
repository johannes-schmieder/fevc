version 18.0
clear all
ereturn clear
capture noisily matrix T = e(nothing)
display "rc after matrix T = e(nothing): " _rc
capture noisily matrix list T
capture noisily confirm matrix e(nothing)
display "rc after confirm matrix e(nothing): " _rc
display "rowsof: " rowsof(T) " el " T[1,1]
