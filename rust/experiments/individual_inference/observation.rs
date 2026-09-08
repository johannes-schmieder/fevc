// SPDX-License-Identifier: GPL-3.0-only
// Appended to the frozen independent observation fixture/oracle module.
fn main() {
    let args:Vec<_>=env::args().skip(1).collect();
    assert_eq!(args.len(),7,"profile cell k start reps master schema");
    assert_eq!(args[6],"individual-v1");
    let k:usize=args[2].parse().unwrap();
    let c=cells(Profile::Confirmation).into_iter().find(|c|c.name==args[1]&&c.k==k).expect("registered cell and dimension");
    let start:usize=args[3].parse().unwrap();let reps:usize=args[4].parse().unwrap();let master:u64=args[5].parse().unwrap();
    assert!(reps>0&&start+reps<=2500);
    println!("{{\"kind\":\"task\",\"schema\":\"individual-v1\",\"family\":\"observation\",\"profile\":{:?},\"cell\":{:?},\"k\":{k},\"start\":{start},\"reps\":{reps},\"master\":{master},\"numseed\":8675309}}",args[0],args[1]);
    let d=make_design(c.k,c.controls,c.dominant,c.beta_zero);
    let variance=make_variance(&d,c.variance);
    let mean=matrix_vector(&d.x,d.rows,d.parameters,&d.beta);
    let worker:Vec<_>=d.sparse_x.iter().map(|r|r.iter().find(|(j,_)|*j<k).unwrap().0 as u64).collect();
    let firm:Vec<_>=d.sparse_x.iter().map(|r|r.iter().find(|(j,_)|*j>=k&&*j<2*k-1).map_or((k-1) as u64,|(j,_)|(*j-k)as u64)).collect();
    let controls:Vec<Vec<f64>>=(2*k-1..d.parameters).map(|j|(0..d.rows).map(|r|d.x[r*d.parameters+j]).collect()).collect();
    println!("{{\"kind\":\"design\",\"n\":{},\"p\":{},\"truth\":{:?},\"leading_share\":{:?},\"remainder_share\":{:?},\"max_h\":{},\"min_variance\":{}}}",d.rows,d.parameters,d.truth,d.leading_share,d.remainder_share,d.leverage.iter().copied().fold(0.,f64::max),variance.iter().copied().fold(f64::INFINITY,f64::min));
    for rep in start..start+reps {
        let seed=semantic_seed(master,c.name,k,rep);
        let mut rng=IndependentRng::new(seed);
        let outcome=mean.iter().zip(&variance).map(|(mu,s)|mu+s.sqrt()*rng.standardized_error(c.error)).collect();
        let input=vckss_core::types::InputColumns {worker:worker.clone(),firm:firm.clone(),deletion:(0..d.rows as u64).collect(),outcome,frequency:vec![1;d.rows],target_weight:vec![1.;d.rows],controls:controls.clone()};
        let now=std::time::Instant::now();
        let result=public_api::run(&input,true,c.reference==Reference::Q1,c.model==StructuredVarianceModel::Common,false);
        public_api::emit(result,rep,seed,now.elapsed().as_secs_f64());
    }
}
