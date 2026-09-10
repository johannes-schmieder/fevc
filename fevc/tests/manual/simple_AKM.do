
clear all
set seed 12345

* Size
local scale = 1
local Nworkers = `scale'*10000
local Nfirms   = `scale'*500
local T        = 10

* Worker-year panel
set obs `Nworkers'
gen workerid = _n
expand `T'
bys workerid: gen year = 2000 + _n - 1

* Firm assignment with worker mobility
bys workerid (year): gen firmid = ceil(runiform()*`Nfirms') if _n == 1
bys workerid (year): replace firmid = ///
    cond(runiform() < .15, ceil(runiform()*`Nfirms'), firmid[_n-1]) if _n > 1

* Worker and firm fixed effects
bys workerid: gen worker_fe = rnormal(0,.4) if _n == 1
bys workerid: replace worker_fe = worker_fe[1]

bys firmid: gen firm_fe = rnormal(0,.2) if _n == 1
bys firmid: replace firm_fe = firm_fe[1]

* Controls
gen experience = year - 2000 + runiform()*5
gen x = rnormal()

* Log wages
gen lnwage = 2 + worker_fe + firm_fe ///
    + .03*experience - .0005*experience^2 + .10*x + rnormal(0,.2)

order workerid firmid year lnwage experience x
sort workerid year

g exp2 = experience^2 

reghdfe lnwage experience exp2, absorb(workerid firmid)

fevc lnwage experience exp2, worker(workerid) firm(firmid) 
