// SPDX-License-Identifier: GPL-3.0-only
// Existing-outcome diagnosis, separate from the frozen confirmation executable.
fn main() {
    let args=std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(),7,"CELL K MASTER NUMSEED CHECK_REP FAILED_REPS");
    let k=args[2].parse::<usize>().unwrap();
    let c=cells(Profile::Confirmation).into_iter().find(|c|c.name==args[1]&&c.k==k).unwrap();
    let master=args[3].parse::<u64>().unwrap();let numseed=args[4].parse::<u64>().unwrap();
    let check=args[5].parse::<usize>().unwrap();
    let d=make_design(k,c.controls,c.dominant,c.beta_zero);
    let s=make_variance(&d,c.variance);
    let mu=matrix_vector(&d.x,d.rows,d.parameters,&d.beta);
    let (_,y0)=follow_y(&d,c,&s,&mu,master,check);
    let p=native_problem(&d,c,&y0,&(0..d.rows).collect::<Vec<_>>());
    let a=native_once(&p,c,numseed,16).unwrap().component_inference.unwrap();
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
            CounterRng::new(numseed^0x941a_0765),r as u64,&ids,&vec![0;d.rows],&vec![1.0;d.rows],
            &mut gaussian[r*d.rows..(r+1)*d.rows],&mut NeverInterrupt).unwrap();
    }
    println!("{{\"kind\":\"geometry\",\"n\":{},\"p\":{},\"x\":{:?},\"inverse\":{:?},\"factors\":{:?},\"exact_ratios\":{:?},\"gram\":{:?},\"terms\":{},\"z\":{:?},\"h\":{:?},\"b\":{:?},\"maker\":{:?},\"variance_true\":{:?},\"fit0\":{:?},\"y0\":{:?},\"covariance0\":{:?},\"gaussian\":{:?}}}",d.rows,d.parameters,d.x,d.information_inverse,d.kernel_factor,d.ratio,fit.gram,terms,z,h,b,maker,s,fit0,y0,a.covariance,gaussian);
    for rep in args[6].split(',').map(|r|r.parse::<usize>().unwrap()) {
        let (seed,y)=follow_y(&d,c,&s,&mu,master,rep);
        println!("{{\"kind\":\"outcome\",\"replication\":{rep},\"seed\":{seed},\"y\":{:?}}}",y);
    }
}
