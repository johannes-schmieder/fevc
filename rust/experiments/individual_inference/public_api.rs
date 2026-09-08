// SPDX-License-Identifier: GPL-3.0-only
// Campaign calls use the actual additive public native lifecycle, not a
// modified estimator or an oracle variance attachment.
use std::{ffi::CStr, mem::size_of, ptr};
use vckss_core::types::InputColumns;
use vckss_plugin::ffi_engine::*;

fn bytes<T>() -> u32 { size_of::<T>() as u32 }
fn checked(status: i32, phase: &str) -> Result<(), String> {
    if status == 0 { Ok(()) } else {
        let detail = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy();
        Err(format!("{phase}: status {status}: {detail}"))
    }
}
struct Handle(u64);
impl Drop for Handle {
    fn drop(&mut self) { assert_eq!(vckss_rust_engine_release_v1(self.0), 0, "native cleanup"); }
}
pub struct Export {
    pub point: [f64;4], pub mcse: [f64;4],
    pub targets: [f64;8], pub q1: [f64;80], pub spectrum: [f64;60],
    pub primitive: [f64;9], pub covariance: [f64;16],
    pub receipt: VckssComponentInferenceResultReceiptV5,
    pub units: VckssComponentInferenceUnitReceiptV1,
}
pub fn run(input: &InputColumns, observation: bool, q1: bool, common: bool, cmg: bool) -> Result<Export,String> {
    assert_eq!(vckss_rust_component_inference_interface_version(), 2);
    let n=input.outcome.len();
    // Frozen core fixtures are zero-based; the public column ABI requires
    // positive exact identifiers. This bijection changes no partition.
    let worker:Vec<_>=input.worker.iter().map(|&x|(x+1) as f64).collect();
    let firm:Vec<_>=input.firm.iter().map(|&x|(x+1) as f64).collect();
    let deletion:Vec<_>=input.deletion.iter().map(|&x|(x+1) as f64).collect();
    let frequency:Vec<_>=input.frequency.iter().map(|&x|x as f64).collect();
    if let Ok(path)=std::env::var("FEVC_INDIVIDUAL_FIXTURE") {
        use std::io::Write;
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).expect("new parity fixture");
        writeln!(file,"worker,firm,deletion,outcome,frequency,target{}",(0..input.controls.len()).map(|j|format!(",control{j}")).collect::<String>()).unwrap();
        for i in 0..n {
            writeln!(file,"{},{},{},{:.17e},{},{:.17e}{}",worker[i],firm[i],deletion[i],input.outcome[i],frequency[i],input.target_weight[i],input.controls.iter().map(|c|format!(",{:.17e}",c[i])).collect::<String>()).unwrap();
        }
    }
    let controls:Vec<_>=input.controls.iter().map(Vec::as_ptr).collect();
    let deletion_mode=if observation {VCKSS_DELETION_OBSERVATION} else {VCKSS_DELETION_MATCH};
    let descriptor=VckssEngineColumnsV2 {
        v1:VckssEngineColumnsV1 {struct_size:bytes::<VckssEngineColumnsV2>(),reserved:0,rows:n as u64,
            worker:worker.as_ptr(),firm:firm.as_ptr(),deletion:deletion.as_ptr(),outcome:input.outcome.as_ptr(),
            frequency:frequency.as_ptr(),target_weight:input.target_weight.as_ptr()},
        controls:if controls.is_empty(){ptr::null()}else{controls.as_ptr()},controls_count:controls.len() as u32,reserved_2:0,
    };
    let mut request=VckssEnginePrepareRequestV3::default();
    request.v2.rows=n as u64;
    request.v2.memory_limit_bytes=4_u64<<30;
    request.v2.caller_copy_bytes=(n*(6+controls.len())*8) as u64;
    request.deletion_mode=deletion_mode;request.controls_count=controls.len() as u32;
    let mut generation=0;
    checked(vckss_rust_engine_prepare_v3(&request,&descriptor,&mut generation,bytes::<u64>()),"prepare")?;
    let handle=Handle(generation);
    let mut attachment=VckssComponentInferenceAugmentationRequestInterruptV1::default();
    checked(vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(&mut attachment,bytes::<VckssComponentInferenceAugmentationRequestInterruptV1>()),"default attachment")?;
    attachment.options.variance_source=if common {1}else{2};
    attachment.options.reference_distribution=u32::from(q1);
    attachment.options.seed=8_675_309;
    attachment.options.fold_seed=8_675_309;
    attachment.options.probes=1000;attachment.options.batch_width=16;
    attachment.options.spectrum_probes=128;attachment.options.spectrum_iterations=if observation{512}else{128};
    attachment.options.spectrum_tolerance=0.002;
    attachment.options.critical_simulations=100000;
    let status=if observation {vckss_rust_engine_augment_component_inference_interrupt_v2(generation,&attachment)}
        else {vckss_rust_engine_augment_match_component_inference_interrupt_v2(generation,&attachment)};
    checked(status,"augment")?;
    let capability_request=VckssBackendRequestCapabilityRequestV3 {
        v2:VckssBackendRequestCapabilityRequestV2 {
            v1:VckssBackendRequestCapabilityRequestV1 {
                struct_size:bytes::<VckssBackendRequestCapabilityRequestV3>(),request_schema:VCKSS_REQUEST_CAPABILITY_SCHEMA_V3,
                algorithm:VCKSS_ALGORITHM_JLA,deletion_mode,
                nuisance_mode:if observation{VCKSS_NUISANCE_JOINT}else{VCKSS_NUISANCE_FIXED_OFFSET},
                solver_route:if cmg{VCKSS_ROUTE_CMG_PCG}else{VCKSS_ROUTE_DIAGONAL_PCG},
                rng_contract:VCKSS_RNG_COUNTER_V1,controls_count:controls.len() as u32,
                frequency_use:VCKSS_REQUEST_FREQUENCY_LITERAL,..Default::default()
            },engine:VCKSS_ENGINE_GENERIC,batch_mode:VCKSS_BATCH_MODE_EXPLICIT,stayers_mode:VCKSS_STAYERS_MOVERS,
            target_weight_mode:VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT,
            deletion_unit_source:if observation{VCKSS_DELETION_SOURCE_OBSERVATION_ROW}else{VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT},
            physical_limit:50_000_000,..Default::default()
        },leverage_batch_mode:VCKSS_BATCH_MODE_EXPLICIT,target_batch_mode:VCKSS_BATCH_MODE_EXPLICIT,
        allow_automatic_cmg_setup_fallback:0,..Default::default()
    };
    let mut capability=VckssBackendRequestCapabilityReceiptV3::default();
    checked(vckss_rust_backend_request_capability_v3(&capability_request,&mut capability,bytes::<VckssBackendRequestCapabilityReceiptV3>()),"capability")?;
    if capability.v2.v1.supported!=1 {return Err("unsupported frozen request".into());}
    let mut solve=VckssEngineSolveRequestInterruptV5::default();
    checked(vckss_rust_engine_default_solve_request_interrupt_v5(&mut solve,bytes::<VckssEngineSolveRequestInterruptV5>()),"default solve")?;
    let r=&mut solve.options.v4;
    r.v3.v2.v1.seed=8_675_309;r.v3.v2.v1.probes=200;
    r.v3.v2.v1.leverage_batch_width=16;r.v3.v2.v1.target_batch_width=16;
    r.v3.v2.v1.deletion_mode=deletion_mode;r.v3.v2.v1.rng_contract=VCKSS_RNG_COUNTER_V1;
    r.v3.v2.v1.solver_route=capability.v2.v1.solver_route;r.v3.v2.v1.allow_automatic_cmg_setup_fallback=0;
    r.v3.v2.algorithm=capability.v2.v1.algorithm;r.v3.v2.nuisance_mode=capability.v2.v1.nuisance_mode;
    r.v3.engine=capability.v2.engine;r.v3.batch_mode=capability.v2.batch_mode;r.v3.stayers_mode=capability.v2.stayers_mode;
    r.v3.target_weight_mode=capability.v2.target_weight_mode;r.v3.deletion_unit_source=capability.v2.deletion_unit_source;
    r.v3.probeorder_supplied=0;r.v3.wallseconds_supplied=0;
    r.v3.capability_schema=capability.v2.v1.request_schema;r.v3.capability_profile=capability.v2.v1.profile_code;
    r.v3.frequency_use=capability.v2.v1.frequency_use;r.v3.physical_limit=capability.v2.physical_limit;
    r.v3.request_signature=capability.v2.v1.request_signature;
    r.leverage_batch_mode=capability.leverage_batch_mode;r.target_batch_mode=capability.target_batch_mode;
    solve.options.tolerance_supplied=0;solve.options.threads=1;
    checked(vckss_rust_engine_solve_interrupt_v5(generation,&solve),"solve")?;
    let mut point=VckssEngineResultV1::default();
    checked(vckss_rust_engine_result_v1(generation,&mut point,bytes::<VckssEngineResultV1>()),"point export")?;
    let values=|v:VckssComponentVectorV1|[v.worker,v.firm,v.covariance,v.total];
    let mut export=Export {point:values(point.corrected),mcse:values(point.numerical_mcse),targets:[0.;8],q1:[f64::NAN;80],spectrum:[0.;60],primitive:[0.;9],covariance:[0.;16],receipt:Default::default(),units:Default::default()};
    let (mut mcse,mut summaries,mut folds,mut cv)=([0.;9],[0.;24],[0.;150],[0.;490]);
    checked(vckss_rust_engine_component_inference_result_v5(generation,
        export.primitive.as_mut_ptr(),9,export.covariance.as_mut_ptr(),16,mcse.as_mut_ptr(),9,export.spectrum.as_mut_ptr(),60,
        if q1{export.q1.as_mut_ptr()}else{ptr::null_mut()},if q1{80}else{0},
        summaries.as_mut_ptr(),24,folds.as_mut_ptr(),150,cv.as_mut_ptr(),490,export.targets.as_mut_ptr(),8,
        &mut export.receipt,bytes::<VckssComponentInferenceResultReceiptV5>()),"V5 export")?;
    checked(vckss_rust_engine_component_inference_unit_receipt_v1(generation,&mut export.units,bytes::<VckssComponentInferenceUnitReceiptV1>()),"unit export")?;
    drop(handle);
    Ok(export)
}

pub fn numbers(values: &[f64]) -> String {
    format!("[{}]",values.iter().map(|x|if x.is_finite(){format!("{x:.17e}")}else{"null".into()}).collect::<Vec<_>>().join(","))
}
pub fn emit(result: Result<Export,String>, rep:usize, seed:u64, seconds:f64) {
    match result {
        Err(detail)=>println!("{{\"kind\":\"call\",\"replication\":{rep},\"seed\":{seed},\"seconds\":{seconds},\"status\":\"shared_failure\",\"detail\":{detail:?}}}"),
        Ok(r)=>println!(concat!("{{\"kind\":\"call\",\"replication\":{},\"seed\":{},\"seconds\":{},\"status\":\"success\",",
            "\"point\":{},\"point_mcse\":{},\"targets\":{},\"q1\":{},\"spectrum\":{},\"primitive\":{},\"covariance\":{},",
            "\"joint\":{},\"fit\":{},\"computed\":{},\"gram_probes\":{},\"gram_rcond\":{},\"floored\":{},\"units\":{},",
            "\"solver_columns\":{},\"max_residual\":{},\"residual_gate\":{},\"counter_atoms\":{},\"counter_words\":{},\"peak\":{},\"critical_draws\":{}}}"),
            rep,seed,seconds,numbers(&r.point),numbers(&r.mcse),numbers(&r.targets),numbers(&r.q1),numbers(&r.spectrum),numbers(&r.primitive),numbers(&r.covariance),
            r.receipt.joint_status,r.receipt.variance_fit,r.receipt.v4.computed_targets,r.receipt.gram_probes,numbers(&[r.receipt.gram_rcond]),r.receipt.floored_predictions,r.units.independent_units,
            r.receipt.v4.solver_columns,r.receipt.v4.v3.v2.maximum_complete_residual,r.receipt.v4.v3.v2.full_residual_tolerance,r.receipt.v4.v3.v2.counter_atoms,r.receipt.v4.v3.v2.counter_words,r.receipt.v4.v3.v2.peak_forecast_bytes,r.receipt.v4.critical_draws),
    }
}
