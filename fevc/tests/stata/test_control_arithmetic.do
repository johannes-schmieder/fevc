version 18.0
clear all
set more off
args pkgroot
if `"`pkgroot'"' == "" {
    findfile fevc.mata
    local pkgroot = subinstr(`"`r(fn)'"',"/fevc.mata","",.)
}
quietly do `"`pkgroot'/fevc.mata"'
mata:
void vckss_test_control_arithmetic()
{
    struct vckss_control_product scalar product, p
    struct vckss_control_basis_result scalar basis
    struct vckss_inverse_result scalar inverse
    real scalar fh, q, i, j, lo, hi, pivot, uncertainty, chosen, n
    real matrix left, right, f, truth, oracle, anchors, u, residualized, scores, anchor
    real rowvector counts
    string scalar line
    string rowvector part
fh=fopen(st_local("pkgroot")+"/tests/fixtures/control_crossproducts.txt","r")
line=fget(fh)
product=vckss__control_cross(J(1,1,1),J(1,1,1),J(1,1,1))
while ((line=fget(fh))!=J(0,0,"")) {
    part=tokens(subinstr(line,";"," "))
    left=strtoreal(tokens(subinstr(part[2],","," ")))'
    right=strtoreal(tokens(subinstr(part[3],","," ")))'
    f=strtoreal(tokens(subinstr(part[4],","," ")))'
    product=vckss__control_cross(left,right,f)
    if (part[5]=="FAIL") assert(hasmissing(product.values))
    else {
        lo=strtoreal(part[5]); hi=strtoreal(part[6])
        assert(product.values[1,1]-product.errors[1,1]<=lo)
        assert(product.values[1,1]+product.errors[1,1]>=hi)
    }
}
fclose(fh)
p=vckss__control_cross(J(3,1,1),(1e16\1\-1e16),J(3,1,1))
assert(p.values[1,1]==1)
p=vckss__control_cross(J(385,1,1),((J(128,1,1)#(1e16\1\-1e16))\.5),J(385,1,1))
assert(p.values[1,1]==128.5)
p=vckss__control_cross(J(1537,1,1),((J(512,1,1)#(1e16\1\-1e16))\.5),J(1537,1,1))
assert(p.values[1,1]==512.5)
right=J(769,1,0); right[1]=1e16; right[257]=1; right[513]=-1e16
p=vckss__control_cross(J(769,1,1),right,J(769,1,1))
assert(p.values[1,1]==1)
assert(vckss__control_anchor((1\.9),.9,1e-12)==1)
assert(vckss__control_anchor((.8\1\.9),.9,1e-12)==2)
assert(vckss__control_anchor((.9\1),.9,1e-12)==0)
assert(vckss__control_anchor((.9000000000001\1),.9,1e-12)==0)
assert(vckss__control_anchor((.9001\1),.9,.001)==0)
counts=(63,64,65,255,256,257,511,512,513,769)
for (pivot=1;pivot<=cols(counts);pivot++) {
  n=counts[pivot]
  for (q=1;q<=32;q++) {
    left=right=J(n,q,0)
    f=1:+mod((0..n-1)',17)
    for (i=1;i<=n;i++) for (j=1;j<=q;j++) {
        left[i,j]=mod(((i-1)*q+j-1)*37,101)-50
        right[i,j]=mod(((i-1)*q+j-1)*71,103)-51
    }
    // These integer products and sums are exact in binary64, independently
    // of reduction order. The Decimal fixtures cover larger exact sums.
    truth=left'*(f:*right)
    product=vckss__control_cross(left,right,f)
    assert(all(abs(product.values-truth):<=product.errors))
  }
}
fh=fopen(st_local("pkgroot")+"/tests/fixtures/control_scores.txt","r")
line=fget(fh)
anchors=strtoreal(tokens(subinstr(substr(line,10,.),","," ")))
line=fget(fh); oracle=J(0,11,.)
while ((line=fget(fh))!=J(0,0,"")) oracle=oracle\strtoreal(tokens(subinstr(line,","," ")))
fclose(fh)
left=oracle[.,1..2]; f=oracle[.,3]
basis=vckss__canonical_controls(left,f,1e-10)
assert(basis.status=="CONVERGED")
product=vckss__control_cross(left,left,f)
inverse=vckss__inverse(product.values,1e-10)
u=left*cholesky(inverse.inverse); residualized=u
uncertainty=max((1e-12,4*(2*basis.forward_error+vckss__rounding_gamma(4)*(1+basis.forward_error)^2)))
for (pivot=1;pivot<=2;pivot++) {
    scores=rowsum(residualized:^2)
    chosen=vckss__control_anchor(scores,max(scores)-1e-7*max((1,max(scores))),uncertainty)
    assert(chosen==anchors[pivot])
    assert(all(scores:-uncertainty:<=oracle[.,2+2*pivot]))
    assert(all(scores:+uncertainty:>=oracle[.,3+2*pivot]))
    anchor=u[chosen,.]
    residualized=u-u*anchor'*anchor/(anchor*anchor')
}
for (j=1;j<=2;j++) {
    assert(all(basis.controls[.,j]:-basis.forward_error:*(1:+abs(basis.controls[.,j])):<=oracle[.,6+2*j]))
    assert(all(basis.controls[.,j]:+basis.forward_error:*(1:+abs(basis.controls[.,j])):>=oracle[.,7+2*j]))
}

}
vckss_test_control_arithmetic()
end
display "PASS test_control_arithmetic.do"
