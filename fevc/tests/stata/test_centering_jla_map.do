version 18
clear all
set more off
args pkgroot
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
quietly fevc y, worker(w) firm(f) backend(mata) algorithm(exact) stayers(both) mcse(off) nodisplay
gen byte oracle_sample=e(sample)
mata:
real scalar independent_jla_map(string scalar deletion,string scalar nuisance,string scalar controls,string scalar stay)
{
 struct vckss_fe_design scalar base
 struct vckss_joint_design scalar joint
 struct vckss_center_jla__map scalar map
 real matrix data,X,A,H,M,Z,D,C,Xfe,Afe,Hfe,Hq,coef,prod,expected,pp,po,cf,panel,chat,physical_panels
 real colvector frequency,y,z,e,r,xi,t,a,b,kappa,idx,groups,weights,m,source,rowsmap,order,copies,w,f,workerfirms
 real scalar n,p,i,j,g,start,finish,q,h,offset,pool,ng,felevels,matched,physical,divisor
 data=st_data(.,("w","f","x","y","copies","mass"),"oracle_sample")
 if(stay=="movers")data=select(data,data[.,1]:<11)
 w=data[.,1];f=data[.,2];frequency=data[.,5];physical=sum(frequency)
 Xfe=vckss__design(w,f,J(rows(data),0,.),max(w),max(f))
 X=Xfe;y=data[.,4];C=controls=="no" ? J(rows(data),0,.) : data[.,3]
 if(controls!="no") {
  coef=qrsolve((sqrt(frequency):*Xfe,sqrt(frequency):*C),sqrt(frequency):*y)
  if(nuisance=="fixedoffset"){y=y-C*coef[rows(coef)];C=J(rows(data),0,.);}
 }
 base=vckss__fe_prepare(w,f,frequency,1e-12)
 joint=vckss__joint_prepare(base,C,1e-12,10000,1e-12,vckss__diagonal_backend())
 if(joint.status!="CONVERGED")_error(498,joint.message)
 if(cols(C)) {X=(Xfe,C);cf=cholesky(joint.schur_inverse);}
 else {X=Xfe;cf=J(0,0,.);}
 coef=qrsolve(sqrt(frequency):*X,sqrt(frequency):*y)
 e=y-X*coef;z=y:-quadcross(frequency,y)/physical
 // Fully expanded design and independent original projection.
 rowsmap=J(physical,1,.);physical_panels=vckss__physical_panels(frequency)
 for(i=1;i<=rows(data);i++)rowsmap[physical_panels[i,1]..physical_panels[i,2]]=J(frequency[i],1,i)
 A=invsym(quadcross(X,frequency,X));H=X[rowsmap,.]*A*X[rowsmap,.]';M=I(physical)-H
 Afe=invsym(quadcross(Xfe,frequency,Xfe));Hfe=Xfe[rowsmap,.]*Afe*Xfe[rowsmap,.]';Hq=H-Hfe
 source=J(rows(data),1,0)
 for(i=1;i<=rows(data);i++){idx=selectindex(w:==w[i]);source[i]=(min(f[idx])==max(f[idx]));}
 order=J(0,1,.);panel=J(0,2,.);copies=J(0,1,.);groups=J(physical,1,.)
 matched=0
 if(deletion=="match") {
  idx=selectindex(source:==0);order=idx[order(100*w[idx]+f[idx],1)];panel=panelsetup((100*w+f)[order],1)
  matched=rows(panel)
  for(g=1;g<=matched;g++) {
   idx=order[panel[g,1]..panel[g,2]]
   for(i=1;i<=rows(idx);i++)groups[physical_panels[idx[i],1]..physical_panels[idx[i],2]]=J(frequency[idx[i]],1,g)
  }
  idx=selectindex(source[rowsmap]:==1);copies=rowsmap[idx]
  for(i=1;i<=rows(idx);i++)groups[idx[i]]=matched+i
 }
 else {copies=rowsmap;groups=(1..physical)';}
 ng=matched+rows(copies);pp=J(matched,3,.);po=J(rows(copies),3,.)
 Z=J(physical,ng,0);m=J(ng,1,.)
 for(g=1;g<=ng;g++){idx=selectindex(groups:==g);Z[idx,g]=J(rows(idx),1,1);m[g]=rows(idx);}
 chat=J(physical,3,.)
 for(pool=1;pool<=3;pool++) {
  r=xi=t=J(physical,1,0);a=J(ng,1,0);b=J(ng,1,0)
  for(g=1;g<=ng;g++) {
   idx=selectindex(groups:==g);q=rows(idx)
   divisor=(1,1.025,.98)[pool]
   p=sum(Hfe[idx,idx])/q*divisor
   if(g<=matched)pp[g,pool]=p
   else po[g-matched,pool]=p
   D=I(q)-J(q,q,p/q)-Hq[idx,idx]
   r[idx]=qrsolve(D,e[rowsmap[idx]])
   t[idx]=z[rowsmap[idx]]*sum(r[idx]);xi[idx]=(t[idx]-r[idx]*sum(z[rowsmap[idx]]))/q
   a[g]=sum(D)/q
   b[g]=sum(t[idx])-physical*sum(D*xi[idx])
  }
  b=b-Z'*H*t
  kappa=qrsolve(physical*diag(a)-Z'*M*Z,b)
  chat[.,pool]=xi+Z*(kappa:/m)
 }
 chat=2*chat[.,1]-(chat[.,2]+chat[.,3])/2
 map=vckss_center_jla__prepare(joint,order,panel,copies,frequency,z,e,pp,po,cf,1e-12,10000,1e-12,1e-12)
 if(map.status!="CONVERGED")_error(498,map.message)
 coef=(sin((1..cols(X))'),cos((1..cols(X))'))
 prod=vckss_center_jla__contract(map,X*coef[.,1],X*coef[.,2]);expected=J(1,4,0)
 for(g=1;g<=ng;g++) {
  idx=selectindex(groups:==g);w=X[rowsmap[idx],.]*coef[.,1];f=X[rowsmap[idx],.]*coef[.,2]
  expected[1]=expected[1]+sum(w)*sum(chat[idx]:*w)
  expected[2]=expected[2]+sum(f)*sum(chat[idx]:*f)
  expected[3]=expected[3]+(sum(w)*sum(chat[idx]:*f)+sum(f)*sum(chat[idx]:*w))/2
 }
 expected[4]=expected[1]+expected[2]+2*expected[3]
 return(mreldif(prod,expected))
}
end
foreach deletion in match observation {
 foreach controls in no joint fixedoffset {
  foreach stay in movers both {
   local nuisance joint
   if "`controls'"=="fixedoffset" local nuisance fixedoffset
   mata: st_numscalar("diff",independent_jla_map("`deletion'","`nuisance'","`controls'","`stay'"))
   di "JLA_MAP_ORACLE `deletion' `controls' `stay' " scalar(diff)
   assert scalar(diff)<1e-8
  }
 }
}
di "SIMPLE_JLA_MAP_ORACLE_PASS"
