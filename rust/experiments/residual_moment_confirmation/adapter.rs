// SPDX-License-Identifier: GPL-3.0-only
// New orchestration only: native_campaign and all estimator functions are unchanged.
const NATIVE_CONFIRMATION_SEED:u64=326417;
const NATIVE_CONFIRMATION_MASTER:u64=1956048372195603;
const NATIVE_PIPELINE_MASTER:u64=1658390174602853;

fn native_confirmation_preflight() {
    for c in cells(Profile::Confirmation) {
        let d=make_design(c.k,c.controls,c.dominant,c.beta_zero);
        let s=make_variance(&d,c.variance);
        // An artificial deterministic response tests numerical preparation only.
        // Its strong mean avoids conflating variance-estimate PSD with geometry.
        let beta=(0..d.parameters).map(|j|100.0*((j as f64+1.0)*1.618).sin()).collect::<Vec<_>>();
        let y=matrix_vector(&d.x,d.rows,d.parameters,&beta).iter().enumerate()
            .map(|(i,y)|y+((i as f64+1.0)*0.618).sin()).collect::<Vec<_>>();
        let p=native_problem(&d,c,&y,&(0..d.rows).collect::<Vec<_>>());
        let mut preflight_cell=c;preflight_cell.reference=Reference::Q0;
        let a=native_once(&p,preflight_cell,NATIVE_CONFIRMATION_SEED,16).expect("outcome-free numerical preflight").component_inference.unwrap();
        let fit=a.residual_moments.as_ref().unwrap();
        let (terms,exact_z)=follow_basis(&d.leverage,&d.target_diagonal,c.model);
        let mut h=vec![0.0;d.rows];let mut b=std::array::from_fn(|_|vec![0.0;d.rows]);
        for (i,&j) in p.retained_rows.iter().enumerate(){h[j]=a.leverage[i];for t in 0..3{b[t][j]=a.target_diagonal[t][i];}}
        let (_,z)=follow_basis(&h,&b,c.model);
        println!("{{\"kind\":\"preflight\",\"cell\":\"{}\",\"k\":{},\"n\":{},\"p\":{},\"terms\":{},\"truth\":{:?},\"leading_share\":{:?},\"remainder_share\":{:?},\"maker_minimum\":{},\"variance\":{:?},\"h\":{:?},\"b\":{:?},\"gram\":{:?},\"exact_h\":{:?},\"exact_z\":{:?},\"z\":{:?},\"rcond\":{},\"key_contract\":\"{}\"}}",c.name,c.k,d.rows,d.parameters,terms,d.truth,d.leading_share,d.remainder_share,d.maker_inverse.iter().map(|v|1.0/v).fold(f64::INFINITY,f64::min),s,a.leverage,a.target_diagonal,fit.gram,d.leverage,exact_z,z,fit.preparation.gram_rcond,fit.ordering_contract);
    }
}

fn main() {
    let a=std::env::args().collect::<Vec<_>>();
    if a.len()==2 && a[1]=="preflight"{native_confirmation_preflight();return;}
    assert_eq!(a.len(),6,"PROFILE CELL K START REPS");
    let master=match a[1].as_str(){"tiny"=>NATIVE_PIPELINE_MASTER,"confirmation"=>NATIVE_CONFIRMATION_MASTER,_=>panic!("invalid profile")};
    let k=a[3].parse::<usize>().unwrap();let start=a[4].parse::<usize>().unwrap();let reps=a[5].parse::<usize>().unwrap();
    let c=cells(Profile::Confirmation).into_iter().find(|c|c.name==a[2]&&c.k==k).expect("registered cell/dimension");
    assert!(reps>0&&start.checked_add(reps).is_some_and(|n|n<=2500));
    println!("{{\"kind\":\"task\",\"schema\":\"FEVC-NATIVE-RESIDUAL-CONFIRMATION-TASK-V1\",\"profile\":\"{}\",\"cell\":\"{}\",\"k\":{},\"start\":{},\"reps\":{},\"master\":{},\"numseed\":{}}}",a[1],c.name,k,start,reps,master,NATIVE_CONFIRMATION_SEED);
    native_campaign(c,start,reps,master,NATIVE_CONFIRMATION_SEED);
}
