// Post-development diagnosis only; not part of the frozen 800-call campaign.
fn main() {
    let c=follow_cell("dominant_common",16);
    let d=make_design(c.k,false,true,false);
    let s=make_variance(&d,c.variance);
    let mu=matrix_vector(&d.x,d.rows,d.parameters,&d.beta);
    let (_,y0)=follow_y(&d,c,&s,&mu,10382619457023651,0);
    let (_,y75)=follow_y(&d,c,&s,&mu,10382619457023651,75);
    let order=(0..d.rows).collect::<Vec<_>>();
    let p=native_problem(&d,c,&y0,&order);
    let a=native_once(&p,c,791503,16).unwrap().component_inference.unwrap();
    let fit=a.residual_moments.as_ref().unwrap();
    let mut h=vec![0.0;d.rows];let mut b=std::array::from_fn(|_|vec![0.0;d.rows]);
    let mut maker=vec![0.0;d.rows];let mut fit0=vec![0.0;d.rows];
    for (i,&j) in p.retained_rows.iter().enumerate() {
        h[j]=a.leverage[i];maker[j]=a.maker_inverse[i];fit0[j]=fit.fit.positive_variance[i];
        for t in 0..3{b[t][j]=a.target_diagonal[t][i];}
    }
    let (terms,z)=follow_basis(&h,&b,c.model);
    let ids=(1..=d.rows as u64).collect::<Vec<_>>();
    let mut gaussian=vec![0.0;d.rows*1000];
    for r in 0..1000 {
        vckss_core::component_inference::fill_gaussian_pseudo_outcome(
            CounterRng::new(791503^0x941a_0765),r as u64,&ids,&vec![0;d.rows],&vec![1.0;d.rows],
            &mut gaussian[r*d.rows..(r+1)*d.rows],&mut NeverInterrupt).unwrap();
    }
    println!("{{\"n\":{},\"p\":{},\"x\":{:?},\"inverse\":{:?},\"factors\":{:?},\"exact_ratios\":{:?},\"gram\":{:?},\"terms\":{},\"z\":{:?},\"h\":{:?},\"b\":{:?},\"maker\":{:?},\"variance_true\":{:?},\"fit0\":{:?},\"y0\":{:?},\"y75\":{:?},\"gaussian\":{:?}}}",d.rows,d.parameters,d.x,d.information_inverse,d.kernel_factor,d.ratio,fit.gram,terms,z,h,b,maker,s,fit0,y0,y75,gaussian);
}
