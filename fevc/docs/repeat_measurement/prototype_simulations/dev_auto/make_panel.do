version 18.0
clear all
args pkgroot
adopath ++ "`pkgroot'"
fevc, simulate_data(ex2) clear
keep log_wage worker_id firm_id
local N = _N
quietly set obs `=_N+600'
quietly replace worker_id = 200+ceil((_n-`N')/4) if _n>`N'
quietly replace firm_id = 1+mod(worker_id,61) if _n>`N'
quietly replace log_wage = sin(worker_id*1.7)+0.01*firm_id+(0.2+0.3*mod(worker_id,3))*cos(_n*2.3+worker_id) if _n>`N'
generate double x1 = sin(_n*.37)+.2*firm_id/61
* Observation-deletion panel: matches of one, two and three observations.
bysort worker_id (firm_id log_wage): generate long seq = _n
drop if worker_id<=200 & mod(worker_id,3)==0 & seq==2
expand 2 if worker_id<=200 & mod(worker_id,3)==1 & seq==1, generate(extra)
replace log_wage = log_wage+0.1*cos(worker_id) if extra==1
drop seq extra
sort worker_id firm_id log_wage
format log_wage x1 %21.0g
export delimited using panel_obs.csv, replace
