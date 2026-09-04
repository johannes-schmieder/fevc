
clear
set seed 20260821
local workers 4000
local firms 200
local spells 2
local periods 3
set obs `=`workers'*`spells'*`periods''
generate long worker_id = ceil(_n/(`spells'*`periods'))
bysort worker_id: generate byte within_worker = _n
generate byte spell = ceil(within_worker/`periods')
generate byte period = mod(within_worker-1,`periods')+1
generate long firm_id = mod(worker_id-1+(spell-1)*7,`firms')+1
generate long match_id = worker_id*10+spell
generate double worker_fe = rnormal() if within_worker==1
bysort worker_id: replace worker_fe = worker_fe[1]
bysort firm_id: generate double firm_fe = rnormal() if _n==1
bysort firm_id: replace firm_fe = firm_fe[1]
generate double productivity = rnormal()
generate double log_wage = 2+worker_fe+firm_fe+.30*productivity+.15*(period==2)+.50*rnormal()
display as text _newline "True DGP worker-firm components (population):"
display as text "  Var(worker effect)       = " as result %6.2f 1
display as text "  Var(firm effect)         = " as result %6.2f 1
display as text "  Cov(worker, firm)        = " as result %6.2f 0
display as text "  Var(worker + firm)       = " as result %6.2f 2

timer clear 
timer on 1
fevc log_wage productivity i.period, worker(worker_id) firm(firm_id) ///
  deletion(match) deletionid(match_id) nuisance(joint) 
timer off 1

timer on 2
fevc log_wage productivity i.period, worker(worker_id) firm(firm_id) ///
  deletion(match) deletionid(match_id) nuisance(fixedoffset) 
timer off 2 

timer list
