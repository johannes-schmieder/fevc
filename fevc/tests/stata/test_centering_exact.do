version 18
clear all
set more off
args pkgroot backend
if "`backend'"=="" local backend mata
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
set obs 72
gen long w=ceil(_n/6)
gen long f=1+mod(floor((_n-1)/2),3)
replace f=1 if w>=11
gen double x=sin(_n*.27)+cos(_n*.81)
gen double y=5+sin(w*.5)+f*.2+.4*x+sin(_n*.93)
gen int copies=1+mod(_n,3)
gen double mass=copies*(1+.1*mod(_n,4))
quietly fevc y, worker(w) firm(f) backend(mata) algorithm(exact) stayers(movers) mcse(off) nodisplay
mata:
real rowvector independent_center_exact(string scalar deletion,string scalar nuisance,string scalar controls)
{
 real matrix data,X,S,A,H,M,Z,D,T,B,W,F,C,Xfull
 real colvector y,z,e,r,xi,t,a,b,kappa,chat,idx,groups,weights,m,source,beta,freq
 real rowvector result
 real scalar n,p,w,f,i,j,g,start,finish,q,h,offset
 data=st_data(.,("w","f","x","y","copies","mass"),"oracle_sample")
 n=sum(data[.,5]);freq=data[.,5];X=J(n,max(data[.,1])+max(data[.,2])-1,0)
 y=weights=groups=source=J(n,1,0);start=1
 for(i=1;i<=rows(data);i++) {
  finish=start+freq[i]-1
  X[|start,data[i,1]\finish,data[i,1]|]=J(freq[i],1,1)
  if(data[i,2]<max(data[.,2]))X[|start,max(data[.,1])+data[i,2]\finish,max(data[.,1])+data[i,2]|]=J(freq[i],1,1)
  y[start..finish]=J(freq[i],1,data[i,4]);weights[start..finish]=J(freq[i],1,data[i,6]/freq[i])
  groups[start..finish]=J(freq[i],1,100*data[i,1]+data[i,2])
  idx=selectindex(data[.,1]:==data[i,1])
  if(min(data[idx,2])==max(data[idx,2]))source[start..finish]=J(freq[i],1,1)
  start=finish+1
 }
 if(controls!="no") {
  C=J(n,1,0);start=1
  for(i=1;i<=rows(data);i++){finish=start+freq[i]-1;C[start..finish]=J(freq[i],1,data[i,3]);start=finish+1; }
  Xfull=(X,C);beta=qrsolve(Xfull,y)
  if(nuisance=="fixedoffset")y=y-C*beta[rows(beta)]
  else X=Xfull
 }
 p=cols(X);A=invsym(quadcross(X,X));beta=A*quadcross(X,y)
 H=X*A*X';M=I(n)-H;e=M*y;z=y:-mean(y)
 if(deletion=="observation")groups=(1..n)'
 else for(i=1;i<=n;i++)if(source[i])groups[i]=-i
 // Construct deletion indicators without relying on production grouping helpers.
 groups=J(n,1,0);start=1
 for(i=1;i<=rows(data);i++) {
  finish=start+freq[i]-1
  for(j=start;j<=finish;j++)groups[j]=deletion=="observation" | source[j] ? -j : 100*data[i,1]+data[i,2]
  start=finish+1
 }
 idx=uniqrows(sort(groups,1));Z=J(n,rows(idx),0);a=m=J(rows(idx),1,0)
 r=xi=t=J(n,1,0)
 for(g=1;g<=rows(idx);g++) {
  source=selectindex(groups:==idx[g]);Z[source,g]=J(rows(source),1,1)
  D=M[source,source];r[source]=qrsolve(D,e[source]);m[g]=rows(source)
  t[source]=z[source]*sum(r[source]);xi[source]=(t[source]-r[source]*sum(z[source]))/m[g]
  a[g]=sum(D)/m[g]
 }
 b=Z'*M*t
 for(g=1;g<=cols(Z);g++) {source=selectindex(Z[.,g]);b[g]=b[g]-n*sum(M[source,source]*xi[source]); }
 kappa=qrsolve(n*diag(a)-Z'*M*Z,b);chat=xi+Z*(kappa:/m)
 W=J(n,p,0);W[.,1..max(data[.,1])]=X[.,1..max(data[.,1])]
 F=J(n,p,0);F[.,(max(data[.,1])+1)..(max(data[.,1])+max(data[.,2])-1)]=X[.,(max(data[.,1])+1)..(max(data[.,1])+max(data[.,2])-1)]
 weights=weights:/sum(weights);C=diag(weights)-weights*weights'
 result=J(1,4,0)
 for(h=1;h<=3;h++) {
  if(h==1)T=W'*C*W
  else if(h==2)T=F'*C*F
  else T=(W'*C*F+F'*C*W)/2
  B=X*A*T*A*X';result[h]=beta'*T*beta
  for(g=1;g<=cols(Z);g++) {source=selectindex(Z[.,g]);result[h]=result[h]-z[source]'*B[source,source]*r[source]-sum(B[source,source]*chat[source]); }
 }
 result[4]=result[1]+result[2]+2*result[3]
 return(result)
}
end
local cells=0
foreach deletion in match observation {
 foreach stay in movers both {
  foreach controls in no joint fixedoffset {
   local covars
   local nuisance joint
   if "`controls'"!="no" local covars x
   if "`controls'"=="fixedoffset" local nuisance fixedoffset
   di "CELL `deletion' `stay' `controls'"
   quietly fevc y `covars' [fw=copies], worker(w) firm(f) backend(`backend') algorithm(exact) deletion(`deletion') stayers(`stay') nuisance(`nuisance') targetweight(mass) centering(corrected) mcse(off) nodisplay
   capture drop oracle_sample
   gen byte oracle_sample=e(sample)
   matrix actual=e(kss)
   mata: st_matrix("expected",independent_center_exact("`deletion'","`nuisance'","`controls'"))
   di "ORACLE_DIFF " mreldif(actual,expected)
   assert mreldif(actual,expected)<1e-8
   local ++cells
  }
 }
}
di "SIMPLE_EXACT_ORACLE_PASS cells=`cells' backend=`backend'"
