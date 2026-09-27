version 18
clear all
set more off
args pkgroot native
if `"`pkgroot'"'=="" local pkgroot "`c(pwd)'/fevc"
adopath ++ `"`pkgroot'"'
quietly do `"`pkgroot'/fevc.mata"'
mata:
void test_posterior_oracle()
{
    real scalar fh,q,n,b,absolute,relative,weighted
    real matrix x,t,c,w
    string scalar line,name
    string rowvector part
    fh=fopen(st_local("pkgroot")+"/tests/fixtures/control_posterior.txt","r")
    line=fget(fh)
    while ((line=fget(fh))!=J(0,0,"")) {
        part=tokens(line,";")
        name=part[1]; q=strtoreal(part[3]); n=strtoreal(part[5])
        x=rowshape(strtoreal(tokens(subinstr(fget(fh),","," "))),n)
        t=rowshape(strtoreal(tokens(subinstr(fget(fh),","," "))),q)
        c=rowshape(strtoreal(tokens(subinstr(fget(fh),","," "))),n)
        part=tokens(fget(fh),";")
        absolute=strtoreal(part[1]); relative=strtoreal(part[3]); weighted=strtoreal(part[5])
        b=vckss__control_posterior(x,t,c,(1::q),J(n,1,1))
        assert(!missing(b))
        assert(b>=absolute & b>=relative)
        w=1:+mod((0::n-1):*977,10000)
        assert(vckss__control_posterior(x,t,c,(1::q),w)>=weighted)
        assert(missing(vckss__control_posterior(x,J(q,q,0),c,(1::q),J(n,1,1))))
        if (strpos(name,"p0.0_weakFalse")) assert(b<1e-9)
    }
    fclose(fh)
}
test_posterior_oracle()
end
noi di "PASS test_control_posterior.do"
