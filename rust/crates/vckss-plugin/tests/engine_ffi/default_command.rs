// SPDX-License-Identifier: MIT

//! The default command's native requests on a caller-supplied panel.
//!
//! These runs reproduce what `fevc y, worker() firm()` sends through the
//! plugin's C interface: match deletion, movers and stayers pooled, engine,
//! route and batches automatic, 200 probes, the numerical replay, and (by
//! default) leave-out residual standard errors. They are ignored because each
//! needs a data file or a large synthetic panel:
//!
//! ```text
//! FEVC_PANEL_CSV=/path/to/panel.csv cargo test --release -p vckss-plugin \
//!     --test engine_ffi default_command_on_a_panel_file -- --ignored --nocapture
//! FEVC_SYNTHETIC_WORKERS=1100000 FEVC_SYNTHETIC_FIRMS=87000 cargo test --release \
//!     -p vckss-plugin --test engine_ffi default_command_on_a_synthetic_panel \
//!     -- --ignored --nocapture
//! ```
//!
//! The file has no header and holds worker, firm, year, region and outcome in
//! its first five columns; `FEVC_PANEL_REGION` and `FEVC_PANEL_TWO_PERIOD`
//! apply the sample rule of the public KSS Matlab package example. No row of
//! the file is written anywhere.
use super::generic_execution::leaveout_residual_diagnostic;
use super::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Instant;
use vckss_plugin::ffi_engine::*;

const SEED: u64 = 8_675_309;
const PROBES: u32 = 200;
const DRAWS: u32 = 8;

fn ok(status: i32) {
    assert_eq!(
        status,
        0,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
}

struct Panel {
    worker: Vec<u64>,
    firm: Vec<u64>,
    outcome: Vec<f64>,
    control: Vec<f64>,
}

struct Report {
    input_rows: usize,
    mover_rows: usize,
    stayer_rows: usize,
    workers: usize,
    firms: usize,
    outcome_variance: f64,
    plugin: [f64; 4],
    correction: [f64; 4],
    corrected: [f64; 4],
    mcse: [f64; 4],
    prepare_seconds: f64,
    solve_seconds: f64,
    engine: u32,
    inference: Option<Inference>,
}

struct Inference {
    covariance: [f64; 16],
    spectrum: [f64; 60],
    units: VckssComponentInferenceUnitReceiptV2,
    diagnostic: VckssLeaveoutResidualDiagnosticV1,
    solver_columns: u32,
    peak_forecast_bytes: u64,
}

fn components(value: VckssComponentVectorV1) -> [f64; 4] {
    [value.worker, value.firm, value.covariance, value.total]
}

/// One run of the default request. `controls` adds the panel's control under
/// joint nuisance; `attach` adds leave-out residual standard errors.
fn run(panel: &Panel, controls: bool, attach: bool, threads: u32) -> Report {
    let rows = panel.worker.len();
    let mut match_id = HashMap::new();
    let mut firms_of = HashMap::<u64, BTreeSet<u64>>::new();
    let mut rows_of = HashMap::<u64, usize>::new();
    for row in 0..rows {
        let next = match_id.len() as f64 + 1.0;
        match_id
            .entry((panel.worker[row], panel.firm[row]))
            .or_insert(next);
        firms_of
            .entry(panel.worker[row])
            .or_default()
            .insert(panel.firm[row]);
        *rows_of.entry(panel.worker[row]).or_default() += 1;
    }
    let columns = OwnedColumns {
        worker: panel.worker.iter().map(|&value| value as f64).collect(),
        firm: panel.firm.iter().map(|&value| value as f64).collect(),
        deletion: (0..rows)
            .map(|row| match_id[&(panel.worker[row], panel.firm[row])])
            .collect(),
        outcome: panel.outcome.clone(),
        frequency: vec![1.0; rows],
        target_weight: vec![1.0; rows],
    };
    let control_columns: Vec<Vec<f64>> = if controls {
        vec![panel.control.clone()]
    } else {
        Vec::new()
    };
    let q = control_columns.len() as u32;

    let started = Instant::now();
    let generation = prepare_with_controls_memory(
        &columns,
        &control_columns,
        VCKSS_DELETION_MATCH,
        12_u64 << 30,
    );
    let mut mask = vec![0_u8; rows];
    ok(vckss_rust_engine_retained_mask_v1(
        generation,
        mask.as_mut_ptr(),
        rows as u64,
    ));
    // Stayers are workers with one firm in the input, at a firm the movers
    // retain, with at least two observations. Their identifiers are dense in
    // sorted order, as the command's `egen group()` makes them.
    let retained_firms: BTreeSet<u64> = (0..rows)
        .filter(|&row| mask[row] == 1)
        .map(|row| panel.firm[row])
        .collect();
    let dense_firm: BTreeMap<u64, f64> = retained_firms
        .iter()
        .enumerate()
        .map(|(index, &firm)| (firm, index as f64 + 1.0))
        .collect();
    let eligible = |row: usize| {
        let worker = panel.worker[row];
        firms_of[&worker].len() == 1
            && rows_of[&worker] >= 2
            && retained_firms.contains(&panel.firm[row])
    };
    let stayer_workers: BTreeSet<u64> = (0..rows)
        .filter(|&row| eligible(row))
        .map(|row| panel.worker[row])
        .collect();
    let dense_worker: BTreeMap<u64, f64> = stayer_workers
        .iter()
        .enumerate()
        .map(|(index, &worker)| (worker, index as f64 + 1.0))
        .collect();
    let mut order: Vec<usize> = (0..rows).filter(|&row| eligible(row)).collect();
    order.sort_by(|&a, &b| {
        dense_worker[&panel.worker[a]]
            .total_cmp(&dense_worker[&panel.worker[b]])
            .then(panel.outcome[a].total_cmp(&panel.outcome[b]))
            .then(panel.control[a].total_cmp(&panel.control[b]))
    });
    let stayer_rows = order.len();
    let stayer_firm: Vec<f64> = order
        .iter()
        .map(|&row| dense_firm[&panel.firm[row]])
        .collect();
    let stayer_worker: Vec<f64> = order
        .iter()
        .map(|&row| dense_worker[&panel.worker[row]])
        .collect();
    let stayer_outcome: Vec<f64> = order.iter().map(|&row| panel.outcome[row]).collect();
    let stayer_control: Vec<f64> = order.iter().map(|&row| panel.control[row]).collect();
    let ones = vec![1.0; stayer_rows];
    let control_pointer = [stayer_control.as_ptr()];
    let descriptor = VckssStayerAugmentationColumnsV1 {
        struct_size: bytes::<VckssStayerAugmentationColumnsV1>(),
        reserved: 0,
        rows: stayer_rows as u64,
        firm: stayer_firm.as_ptr(),
        worker: stayer_worker.as_ptr(),
        outcome: stayer_outcome.as_ptr(),
        frequency: ones.as_ptr(),
        target_weight: ones.as_ptr(),
        controls: if controls {
            control_pointer.as_ptr()
        } else {
            ptr::null()
        },
        controls_count: q,
        reserved_2: 0,
    };
    let stayers = VckssStayerAugmentationRequestV1 {
        rows: stayer_rows as u64,
        controls_count: q,
        caller_copy_bytes: stayer_rows as u64 * u64::from(5 + q) * 8,
        ..VckssStayerAugmentationRequestV1::default()
    };
    ok(vckss_rust_engine_augment_stayers_v1(
        generation,
        &stayers,
        &descriptor,
    ));
    if attach {
        let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
        ok(
            vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(
                &mut augmentation,
                bytes::<VckssComponentInferenceAugmentationRequestInterruptV1>(),
            ),
        );
        augmentation.options.variance_source = VCKSS_COMPONENT_VARIANCE_LEAVEOUT_RESIDUAL;
        augmentation.options.seed = SEED;
        augmentation.options.probes = 4 * DRAWS;
        augmentation.options.batch_width = 8;
        ok(
            vckss_rust_engine_augment_match_component_inference_interrupt_v4(
                generation,
                &augmentation,
                512,
            ),
        );
    }
    // centering(mean), the command's default.
    ok(vckss_rust_engine_centering_v1(generation, 1));
    let prepare_seconds = started.elapsed().as_secs_f64();

    let mut capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        q,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    capability.v2.stayers_mode = VCKSS_STAYERS_ALL;
    let mut point = planned_solve_request(capability, 0, 0);
    point.v3.v2.v1.seed = SEED;
    point.v3.v2.v1.probes = PROBES;
    point.v3.v2.v1.pcg_tolerance = 1.0e-10;
    // Without standard errors the command solves on the full CMG executor;
    // with them it lets the resolved executor carry the attachment.
    let mut request = VckssNumericalRequestV2 {
        v1: VckssNumericalRequestV1 {
            point,
            controls_count: q,
            threads,
            ..VckssNumericalRequestV1::default()
        },
        execution_mode: if attach {
            VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO
        } else {
            0
        },
        full_cmg: u32::from(!attach),
        ..VckssNumericalRequestV2::default()
    };
    request.v1.struct_size = bytes::<VckssNumericalRequestV2>();
    request.v1.schema_version = 2;
    request.v1.component_requested = u32::from(attach);
    let started = Instant::now();
    ok(vckss_rust_engine_solve_numerical_interrupt_v2(
        generation,
        &request,
        None,
        ptr::null_mut(),
        0,
    ));
    let solve_seconds = started.elapsed().as_secs_f64();

    // The pooled mover-stayer estimate is the generation's ordinary result.
    let mut hybrid = VckssEngineResultV1::default();
    ok(vckss_rust_engine_result_v1(
        generation,
        &mut hybrid,
        bytes::<VckssEngineResultV1>(),
    ));
    let mut numerical = VckssNumericalResultV1::default();
    ok(vckss_rust_engine_numerical_result_v1(
        generation,
        &mut numerical,
        bytes::<VckssNumericalResultV1>(),
    ));
    let mut plan = VckssExecutionPlanReceiptV1::default();
    ok(vckss_rust_engine_execution_plan_receipt_v1(
        generation,
        &mut plan,
        bytes::<VckssExecutionPlanReceiptV1>(),
    ));
    let inference = attach.then(|| {
        let (mut primitive, mut covariance, mut mcse) = ([0.0; 9], [0.0; 16], [0.0; 9]);
        let (mut spectrum, mut summaries) = ([0.0; 60], [0.0; 24]);
        let (mut folds, mut cv, mut targets) = ([0.0; 150], [0.0; 490], [0.0; 8]);
        let mut receipt = VckssComponentInferenceResultReceiptV5::default();
        ok(vckss_rust_engine_component_inference_result_v5(
            generation,
            primitive.as_mut_ptr(),
            9,
            covariance.as_mut_ptr(),
            16,
            mcse.as_mut_ptr(),
            9,
            spectrum.as_mut_ptr(),
            60,
            ptr::null_mut(),
            0,
            summaries.as_mut_ptr(),
            24,
            folds.as_mut_ptr(),
            150,
            cv.as_mut_ptr(),
            490,
            targets.as_mut_ptr(),
            8,
            &mut receipt,
            288,
        ));
        assert_eq!(receipt.variance_fit, 3);
        assert_eq!(receipt.joint_status, 0);
        let mut units = VckssComponentInferenceUnitReceiptV2::default();
        ok(vckss_rust_engine_component_inference_unit_receipt_v2(
            generation,
            &mut units,
            bytes::<VckssComponentInferenceUnitReceiptV2>(),
        ));
        Inference {
            covariance,
            spectrum,
            units,
            diagnostic: leaveout_residual_diagnostic(generation),
            solver_columns: receipt.v4.solver_columns,
            peak_forecast_bytes: receipt.v4.v3.v2.peak_forecast_bytes,
        }
    });
    ok(vckss_rust_engine_release_v1(generation));

    // Outcome variance of the pooled retained sample, with divisor N.
    let kept: Vec<f64> = (0..rows)
        .filter(|&row| mask[row] == 1 || eligible(row))
        .map(|row| panel.outcome[row])
        .collect();
    let mean = kept.iter().sum::<f64>() / kept.len() as f64;
    let outcome_variance =
        kept.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / kept.len() as f64;
    let mover_rows = mask.iter().filter(|&&value| value == 1).count();
    let mover_workers: BTreeSet<u64> = (0..rows)
        .filter(|&row| mask[row] == 1)
        .map(|row| panel.worker[row])
        .collect();
    Report {
        input_rows: rows,
        mover_rows,
        stayer_rows,
        workers: mover_workers.len() + stayer_workers.len(),
        firms: retained_firms.len(),
        outcome_variance,
        plugin: components(hybrid.plugin),
        correction: components(hybrid.correction),
        corrected: components(hybrid.corrected),
        mcse: numerical.mcse,
        prepare_seconds,
        solve_seconds,
        engine: plan.resolution.engine_selected,
        inference,
    }
}

fn print(title: &str, report: &Report) {
    const NAMES: [&str; 4] = [
        "Worker variance",
        "Firm variance",
        "Covariance",
        "Variance of sum",
    ];
    println!("\n{title}");
    println!(
        "  input rows {}; retained {} ({} mover rows, {} stayer rows); {} workers; {} firms",
        report.input_rows,
        report.mover_rows + report.stayer_rows,
        report.mover_rows,
        report.stayer_rows,
        report.workers,
        report.firms
    );
    println!(
        "  engine code {}; probes {PROBES}; seed {SEED}; Var(Y) {:.7}",
        report.engine, report.outcome_variance
    );
    println!(
        "  preparation {:.2} s; solve {:.2} s",
        report.prepare_seconds, report.solve_seconds
    );
    let z = 1.959_963_984_540_054_f64;
    match &report.inference {
        None => {
            println!(
                "  {:<16} {:>11} {:>11} {:>11} {:>11}",
                "", "plug-in", "bias", "KSS", "num. MCSE"
            );
            for index in 0..4 {
                println!(
                    "  {:<16} {:>11.7} {:>11.7} {:>11.7} {:>11.2e}",
                    NAMES[index],
                    report.plugin[index],
                    report.correction[index],
                    report.corrected[index],
                    report.mcse[index]
                );
            }
        }
        Some(inference) => {
            println!(
                "  {:<16} {:>11} {:>10} {:>23} {:>11} {:>9} {:>10}",
                "", "KSS", "std. err.", "[95% conf. interval]", "num. MCSE", "eff.rank", "model SE"
            );
            for index in 0..4 {
                let se = inference.covariance[5 * index].sqrt();
                println!(
                    "  {:<16} {:>11.7} {:>10.7} {:>11.7} {:>11.7} {:>11.2e} {:>9.0} {:>10.7}",
                    NAMES[index],
                    report.corrected[index],
                    se,
                    report.corrected[index] - z * se,
                    report.corrected[index] + z * se,
                    report.mcse[index],
                    inference.diagnostic.effective_rank[index],
                    inference.diagnostic.model_covariance[5 * index].sqrt()
                );
            }
            let share = (0..4)
                .map(|target| inference.spectrum[15 * target + 14])
                .fold(0.0_f64, f64::max);
            println!(
                "  units {} ({} mover deletion units, {} stayer observations); largest influence share {:.2e}",
                inference.units.independent_units,
                inference.units.mover_units,
                inference.units.stayer_observations,
                share
            );
            println!(
                "  draws {}; solves {} (offset pass {}); working-variance terms {} mover, {} stayer; floored {}; signal truncation {:.2e}",
                inference.diagnostic.draws,
                inference.solver_columns,
                inference.diagnostic.offset_pass_solves,
                inference.diagnostic.mover_terms,
                inference.diagnostic.stayer_terms,
                inference.diagnostic.working_floored,
                inference.diagnostic.signal_truncation
            );
            println!(
                "  attachment memory forecast {:.1} MiB",
                inference.peak_forecast_bytes as f64 / (1_u64 << 20) as f64
            );
        }
    }
}

fn threads() -> u32 {
    std::env::var("FEVC_THREADS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4)
}

fn compare(panel: &Panel, label: &str) {
    let threads = threads();
    println!("\n==== {label}; {threads} threads ====");
    let point = run(panel, false, false, threads);
    print(
        "Default request without standard errors (inference(none))",
        &point,
    );
    let inference = run(panel, false, true, threads);
    print(
        "Default request with leave-out residual standard errors",
        &inference,
    );
    // The two requests use different executors, so the estimates agree to
    // solver tolerance, judged on the scale of the outcome variance.
    for index in 0..4 {
        let (a, b) = (point.corrected[index], inference.corrected[index]);
        assert!(
            (a - b).abs() <= 1e-6 * point.outcome_variance,
            "{a} versus {b}"
        );
    }
    println!(
        "\n  runtime: solve {:.2} s -> {:.2} s ({:+.0}%); with preparation {:.2} s -> {:.2} s ({:+.0}%)",
        point.solve_seconds,
        inference.solve_seconds,
        100.0 * (inference.solve_seconds / point.solve_seconds - 1.0),
        point.prepare_seconds + point.solve_seconds,
        inference.prepare_seconds + inference.solve_seconds,
        100.0
            * ((inference.prepare_seconds + inference.solve_seconds)
                / (point.prepare_seconds + point.solve_seconds)
                - 1.0)
    );
    if std::env::var("FEVC_PANEL_CONTROL").is_ok() {
        let point = run(panel, true, false, threads);
        print("With one control, without standard errors", &point);
        let inference = run(panel, true, true, threads);
        print(
            "With one control, with standard errors (offset pass)",
            &inference,
        );
        println!(
            "\n  runtime with one control: solve {:.2} s -> {:.2} s ({:+.0}%)",
            point.solve_seconds,
            inference.solve_seconds,
            100.0 * (inference.solve_seconds / point.solve_seconds - 1.0)
        );
    }
}

#[test]
#[ignore = "needs FEVC_PANEL_CSV"]
fn default_command_on_a_panel_file() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let path = std::env::var("FEVC_PANEL_CSV").expect("FEVC_PANEL_CSV names the panel file");
    let region = std::env::var("FEVC_PANEL_REGION")
        .ok()
        .map(|value| value.parse::<f64>().expect("numeric region"));
    let two_period = std::env::var("FEVC_PANEL_TWO_PERIOD").is_ok();
    let text = std::fs::read_to_string(path).expect("panel file");
    let mut records = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<f64> = line
            .split(',')
            .take(5)
            .map(|field| field.trim().parse().expect("numeric field"))
            .collect();
        assert_eq!(fields.len(), 5, "worker, firm, year, region, outcome");
        if region.is_none_or(|value| fields[3] == value) {
            records.push((fields[0] as u64, fields[1] as u64, fields[2], fields[4]));
        }
    }
    let mut count = HashMap::<u64, usize>::new();
    for record in &records {
        *count.entry(record.0).or_default() += 1;
    }
    if two_period {
        records.retain(|record| count[&record.0] == 2);
    }
    let last_year = records
        .iter()
        .map(|record| record.2)
        .fold(f64::MIN, f64::max);
    let panel = Panel {
        worker: records.iter().map(|record| record.0).collect(),
        firm: records.iter().map(|record| record.1).collect(),
        outcome: records.iter().map(|record| record.3).collect(),
        // A last-year indicator: the one control these files always support.
        control: records
            .iter()
            .map(|record| f64::from(record.2 == last_year))
            .collect(),
    };
    compare(&panel, "panel file");
}

/// A two-period panel with the proportions of a matched employer-employee
/// extract: `mover_share` of workers change firm, firm sizes are skewed, and
/// workers sort mildly on firm effects.
#[test]
#[ignore = "large synthetic panel"]
fn default_command_on_a_synthetic_panel() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let read = |name: &str, default: u64| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    };
    let workers = read("FEVC_SYNTHETIC_WORKERS", 200_000);
    let firms = read("FEVC_SYNTHETIC_FIRMS", 16_000);
    let periods = read("FEVC_SYNTHETIC_PERIODS", 2);
    let mover_percent = read("FEVC_SYNTHETIC_MOVER_PERCENT", 20);
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut uniform = move || (next() >> 11) as f64 / (1_u64 << 53) as f64;
    let firm_effect: Vec<f64> = (0..firms)
        .map(|_| 0.25 * (uniform() + uniform() + uniform() - 1.5))
        .collect();
    let mut panel = Panel {
        worker: Vec::new(),
        firm: Vec::new(),
        outcome: Vec::new(),
        control: Vec::new(),
    };
    // Squaring a uniform draw skews firm sizes toward low indices.
    let draw_firm = |uniform: &mut dyn FnMut() -> f64| {
        let value = uniform();
        ((value * value * firms as f64) as u64).min(firms - 1)
    };
    for worker in 0..workers {
        let ability = 0.6 * (uniform() + uniform() + uniform() - 1.5);
        let mover = uniform() * 100.0 < mover_percent as f64;
        let first = draw_firm(&mut uniform);
        let mut current = first;
        let change_at = 1 + (uniform() * (periods - 1) as f64) as u64;
        for period in 0..periods {
            if mover && period == change_at {
                current = draw_firm(&mut uniform);
            }
            let noise = 0.35 * (uniform() + uniform() + uniform() - 1.5);
            panel.worker.push(worker + 1);
            panel.firm.push(current + 1);
            panel.control.push(period as f64);
            panel
                .outcome
                .push(ability + firm_effect[current as usize] + 0.02 * period as f64 + noise);
        }
    }
    compare(
        &panel,
        &format!("synthetic panel: {workers} workers, {firms} firms, {periods} periods"),
    );
}
