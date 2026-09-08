// SPDX-License-Identifier: GPL-3.0-only
// Inserted inside the unchanged match DGP module in a disposable build.
pub fn individual_entry() {
    let args:Vec<_>=env::args().skip(1).collect();
    if args.first().map(String::as_str)==Some("oracle-preflight") {
        run_q1_preflight(&args[1..]);
        return;
    }
    assert_eq!(args.len(),7,"profile cell k start reps master schema");
    assert_eq!(args[6],"individual-v1");
    let k:usize=args[2].parse().unwrap();assert_eq!(k,20);
    let (c,q)=q1_cell(&args[1]).expect("registered match cell");
    let start:usize=args[3].parse().unwrap();let reps:usize=args[4].parse().unwrap();let master:u64=args[5].parse().unwrap();
    assert!(reps>0&&start+reps<=2500);
    println!("{{\"kind\":\"task\",\"schema\":\"individual-v1\",\"family\":{:?},\"profile\":{:?},\"cell\":{:?},\"k\":{k},\"start\":{start},\"reps\":{reps},\"master\":{master},\"numseed\":8675309}}",INDIVIDUAL_FAMILY,args[0],args[1]);
    for rep in start..start+reps {
        let seed=semantic_seed(master,c.name,k,rep);
        let generated=make_problem(c,k,seed);
        if rep==start {
            println!("{{\"kind\":\"design\",\"n\":{},\"truth\":{:?},\"min_variance\":{}}}",generated.input.outcome.len(),generated.truth,generated.aggregate_variance.iter().copied().fold(f64::INFINITY,f64::min));
        }
        let now=std::time::Instant::now();
        let result=crate::public_api::run(&generated.input,false,q==Reference::Q1,c.variance_source==ComponentVarianceSource::StructuredCommon,c.route==ModelSolverRoute::Cmg);
        crate::public_api::emit(result,rep,seed,now.elapsed().as_secs_f64());
    }
}
