// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic structured observation-variance regression.
//!
//! This is an FEVC variance-model approximation, not the unrestricted
//! heteroskedastic variance-product estimator in Kline--Saggio--Solvsten.
//! Folds exclude the validation response from the small variance regression;
//! they do not refit the worker--firm model and do not create the independent
//! split predictions used by the unrestricted KSS construction.

use core::cmp::Ordering;

use crate::component_inference::PRIMITIVE_TARGETS;
use crate::dense::invert_scaled_spd;
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck};
use crate::rng::{CounterRng, ProbeDomain};

pub const STRUCTURED_VARIANCE_SCHEMA_VERSION: u32 = 1;
pub const STRUCTURED_PRIMARY_TERMS: usize = 15;
pub const GROUPED_STRUCTURED_PRIMARY_TERMS: usize = 21;
pub const STRUCTURED_LEVERAGE_TERMS: usize = 3;
pub const STRUCTURED_OUTER_FOLDS: usize = 5;
pub const STRUCTURED_INNER_FOLDS: usize = 4;
pub const STRUCTURED_RIDGE_GRID: [f64; 7] = [0.0, 1.0e-8, 1.0e-6, 1.0e-4, 1.0e-2, 1.0, 1.0e2];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StructuredVarianceModel {
    Common = 1,
    LeverageOnly = 2,
}

#[derive(Clone, Copy, Debug)]
pub struct StructuredVarianceOptions {
    pub seed: u64,
    pub rank_tolerance: f64,
    pub positivity_multiplier: f64,
    pub observations_per_term: usize,
}

impl Default for StructuredVarianceOptions {
    fn default() -> Self {
        Self {
            seed: 1,
            rank_tolerance: 1.0e-10,
            positivity_multiplier: 1.0e-8,
            observations_per_term: 5,
        }
    }
}

impl StructuredVarianceOptions {
    pub fn validate(self) -> Result<Self> {
        if !self.rank_tolerance.is_finite()
            || self.rank_tolerance < 1.0e-14
            || self.rank_tolerance >= 0.1
            || !self.positivity_multiplier.is_finite()
            || self.positivity_multiplier <= 0.0
            || self.positivity_multiplier >= 1.0
            || self.observations_per_term == 0
        {
            return Err(BackendError::invalid(
                "structured_variance_prepare",
                "structured variance tuning is invalid",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StructuredVarianceFoldDiagnostic {
    pub model: u32,
    pub outer_fold: u32,
    pub training_observations: u64,
    pub validation_observations: u64,
    pub active_terms: u32,
    pub selected_lambda: f64,
    pub selected_cv_mse: f64,
    pub variance_scale: f64,
    pub positivity_floor: f64,
    pub floored_predictions: u64,
    pub boundary_predictions: u64,
    pub maximum_boundary_excess: f64,
    pub maximum_prediction_leverage: f64,
    pub fitted_rcond: f64,
    pub fitted_relres: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StructuredVarianceCvDiagnostic {
    pub model: u32,
    pub outer_fold: u32,
    pub grid_index: u32,
    pub lambda: f64,
    pub validation_observations: u64,
    pub mse: f64,
    pub available: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StructuredVarianceSummary {
    pub model: u32,
    pub observations: u64,
    pub minimum: f64,
    pub median: f64,
    pub maximum: f64,
    pub floor_count: u64,
    pub floor_share: f64,
    pub boundary_count: u64,
    pub boundary_share: f64,
    pub maximum_boundary_excess: f64,
    pub maximum_prediction_leverage: f64,
    pub minimum_fitted_rcond: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StructuredVarianceSensitivity {
    pub median_absolute_log_ratio: f64,
    pub p90_absolute_log_ratio: f64,
    pub maximum_absolute_log_ratio: f64,
    pub log_variance_correlation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuredVarianceResult {
    pub schema_version: u32,
    pub common: Vec<f64>,
    pub leverage_only: Vec<f64>,
    pub folds: Vec<StructuredVarianceFoldDiagnostic>,
    pub cv: Vec<StructuredVarianceCvDiagnostic>,
    pub summary: [StructuredVarianceSummary; 2],
    pub sensitivity: StructuredVarianceSensitivity,
    pub outer_fold: Vec<u8>,
    pub counter_atoms: u64,
    pub counter_words: u64,
}

impl StructuredVarianceResult {
    #[must_use]
    pub fn selected(&self, model: StructuredVarianceModel) -> &[f64] {
        match model {
            StructuredVarianceModel::Common => &self.common,
            StructuredVarianceModel::LeverageOnly => &self.leverage_only,
        }
    }
}

#[derive(Clone, Debug)]
struct FoldClass {
    entity: u64,
    rows: Vec<usize>,
}

#[derive(Clone, Debug)]
struct ModelFit {
    model: StructuredVarianceModel,
    diagnostics: usize,
    means: Vec<f64>,
    scales: Vec<f64>,
    active: Vec<usize>,
    coefficients: Vec<f64>,
    inverse: Vec<f64>,
    rcond: f64,
    relres: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        if self.sum.abs() >= value.abs() {
            self.correction += (self.sum - next) + value;
        } else {
            self.correction += (value - next) + self.sum;
        }
        self.sum = next;
    }

    fn finish(self) -> f64 {
        self.sum + self.correction
    }
}

/// Fit the registered primary and leverage-only observation-variance models.
/// `fold_entity` must be an outcome-free canonical design-class identifier;
/// every row in one exact design class is assigned to the same fold.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn fit_structured_variance_with_interrupt(
    proxy: &[f64],
    residual: &[f64],
    maker_inverse: &[f64],
    leverage: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    fold_entity: &[u64],
    options: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<StructuredVarianceResult> {
    let rows = proxy.len();
    if rows == 0
        || residual.len() != rows
        || maker_inverse.len() != rows
        || leverage.len() != rows
        || fold_entity.len() != rows
        || target_diagonal.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invalid(
            "structured_variance_prepare",
            "structured variance inputs have incompatible dimensions",
        ));
    }
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "structured_variance_validate")?;
        if !proxy[row].is_finite()
            || !residual[row].is_finite()
            || !maker_inverse[row].is_finite()
            || maker_inverse[row] <= 0.0
            || !leverage[row].is_finite()
            || target_diagonal
                .iter()
                .any(|column| !column[row].is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "structured_variance_validate",
                "structured variance inputs must be finite with positive maker inverses",
            ));
        }
    }

    let mut raw = vec![vec![0.0; rows]; 4];
    raw[0].copy_from_slice(leverage);
    for target in 0..PRIMITIVE_TARGETS {
        raw[target + 1].copy_from_slice(&target_diagonal[target]);
    }
    fit_structured_variance_from_raw(
        proxy,
        residual,
        maker_inverse,
        &raw,
        fold_entity,
        options,
        interrupt,
    )
}

/// Fit the fixed-offset collapsed-match variance models.  Each input row is
/// one independent declared match. `match_mass` is an algebraic regression
/// mass and enters only as a fifth outcome-free diagnostic; it never expands
/// the response or the fold counts into independent copies.
#[allow(clippy::too_many_arguments)]
pub fn fit_grouped_structured_variance_with_interrupt(
    proxy: &[f64],
    residual: &[f64],
    maker_inverse: &[f64],
    leverage: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    match_mass: &[f64],
    fold_entity: &[u64],
    options: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<StructuredVarianceResult> {
    let rows = proxy.len();
    if rows == 0
        || residual.len() != rows
        || maker_inverse.len() != rows
        || leverage.len() != rows
        || match_mass.len() != rows
        || fold_entity.len() != rows
        || target_diagonal.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invalid(
            "structured_variance_prepare",
            "grouped structured variance inputs have incompatible dimensions",
        ));
    }
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "structured_variance_validate")?;
        if !proxy[row].is_finite()
            || !residual[row].is_finite()
            || !maker_inverse[row].is_finite()
            || maker_inverse[row] <= 0.0
            || !leverage[row].is_finite()
            || !match_mass[row].is_finite()
            || match_mass[row] <= 0.0
            || target_diagonal
                .iter()
                .any(|column| !column[row].is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "structured_variance_validate",
                "grouped structured variance inputs must be finite with positive maker inverses and match masses",
            ));
        }
    }
    let mut raw = vec![vec![0.0; rows]; 5];
    raw[0].copy_from_slice(leverage);
    for target in 0..PRIMITIVE_TARGETS {
        raw[target + 1].copy_from_slice(&target_diagonal[target]);
    }
    raw[4].copy_from_slice(match_mass);
    fit_structured_variance_from_raw(
        proxy,
        residual,
        maker_inverse,
        &raw,
        fold_entity,
        options,
        interrupt,
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn fit_structured_variance_from_raw(
    proxy: &[f64],
    residual: &[f64],
    maker_inverse: &[f64],
    raw: &[Vec<f64>],
    fold_entity: &[u64],
    options: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<StructuredVarianceResult> {
    let options = options.validate()?;
    let rows = proxy.len();
    if !matches!(raw.len(), 4 | 5) || raw.iter().any(|column| column.len() != rows) {
        return Err(BackendError::invariant(
            "structured_variance_prepare",
            "structured variance diagnostic dimensions are invalid",
        ));
    }
    let mut ranks = vec![vec![0.0; rows]; raw.len()];
    for column in 0..raw.len() {
        ranks[column] = normalized_midranks(&raw[column], interrupt)?;
    }

    let classes = fold_classes(fold_entity, interrupt)?;
    if classes.len() < STRUCTURED_OUTER_FOLDS {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "structured_variance_folds",
            "fewer than five outcome-free design classes cannot support five-fold variance regression",
        ));
    }
    let rng = CounterRng::new(options.seed);
    let outer_fold = assign_classes_to_folds(
        rows,
        &classes,
        None,
        STRUCTURED_OUTER_FOLDS,
        rng,
        0,
        interrupt,
    )?;
    let mut inner_fold = Vec::with_capacity(STRUCTURED_OUTER_FOLDS);
    for outer in 0..STRUCTURED_OUTER_FOLDS {
        let eligible = outer_fold
            .iter()
            .map(|&fold| usize::from(fold) != outer)
            .collect::<Vec<_>>();
        inner_fold.push(assign_classes_to_folds(
            rows,
            &classes,
            Some(&eligible),
            STRUCTURED_INNER_FOLDS,
            rng,
            1 + outer as u64,
            interrupt,
        )?);
    }
    let canonical_order = canonical_response_order(fold_entity, proxy, interrupt)?;

    let mut all_folds = Vec::with_capacity(2 * STRUCTURED_OUTER_FOLDS);
    let mut all_cv = Vec::with_capacity(2 * STRUCTURED_OUTER_FOLDS * STRUCTURED_RIDGE_GRID.len());
    let (common, common_summary) = fit_cross_fitted_model(
        StructuredVarianceModel::Common,
        proxy,
        residual,
        maker_inverse,
        &ranks,
        &outer_fold,
        &inner_fold,
        &canonical_order,
        options,
        &mut all_folds,
        &mut all_cv,
        interrupt,
    )?;
    let (leverage_only, leverage_summary) = fit_cross_fitted_model(
        StructuredVarianceModel::LeverageOnly,
        proxy,
        residual,
        maker_inverse,
        &ranks,
        &outer_fold,
        &inner_fold,
        &canonical_order,
        options,
        &mut all_folds,
        &mut all_cv,
        interrupt,
    )?;
    let sensitivity = variance_sensitivity(&common, &leverage_only, interrupt)?;
    let classes_u64 = u64::try_from(classes.len()).map_err(|_| resource("fold class count"))?;
    // One outer-fold key plus four inner-fold keys for every class: a class
    // is excluded from the inner split belonging to its own outer fold.
    let counter_atoms = classes_u64
        .checked_mul(STRUCTURED_OUTER_FOLDS as u64)
        .ok_or_else(|| resource("structured fold Counter atom count"))?;
    Ok(StructuredVarianceResult {
        schema_version: STRUCTURED_VARIANCE_SCHEMA_VERSION,
        common,
        leverage_only,
        folds: all_folds,
        cv: all_cv,
        summary: [common_summary, leverage_summary],
        sensitivity,
        outer_fold,
        counter_atoms,
        counter_words: counter_atoms,
    })
}

pub(crate) fn normalized_midranks(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let mut order = (0..values.len()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        values[left]
            .total_cmp(&values[right])
            .then_with(|| left.cmp(&right))
    });
    let mut output = vec![0.0; values.len()];
    let mut begin = 0;
    while begin < order.len() {
        checkpoint_chunk(interrupt, begin, "structured_variance_midranks")?;
        let mut end = begin + 1;
        while end < order.len()
            && values[order[begin]].total_cmp(&values[order[end]]) == Ordering::Equal
        {
            end += 1;
        }
        let midrank = 0.5 * ((begin + 1) as f64 + end as f64);
        let normalized = 2.0 * ((midrank - 0.5) / values.len() as f64) - 1.0;
        for &row in &order[begin..end] {
            output[row] = normalized;
        }
        begin = end;
    }
    Ok(output)
}

fn fold_classes(entity: &[u64], interrupt: &mut dyn InterruptCheck) -> Result<Vec<FoldClass>> {
    let mut order = (0..entity.len()).collect::<Vec<_>>();
    order.sort_by_key(|&row| (entity[row], row));
    let mut output = Vec::new();
    let mut begin = 0;
    while begin < order.len() {
        checkpoint_chunk(interrupt, begin, "structured_variance_fold_classes")?;
        let key = entity[order[begin]];
        let mut end = begin + 1;
        while end < order.len() && entity[order[end]] == key {
            end += 1;
        }
        output.push(FoldClass {
            entity: key,
            rows: order[begin..end].to_vec(),
        });
        begin = end;
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn assign_classes_to_folds(
    rows: usize,
    classes: &[FoldClass],
    eligible: Option<&[bool]>,
    folds: usize,
    rng: CounterRng,
    purpose: u64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u8>> {
    let mut keyed = Vec::new();
    for (index, class) in classes.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "structured_variance_fold_keys")?;
        let active = eligible.is_none_or(|mask| class.rows.iter().any(|&row| mask[row]));
        if active {
            if let Some(mask) = eligible {
                if class.rows.iter().any(|&row| !mask[row]) {
                    return Err(BackendError::invariant(
                        "structured_variance_folds",
                        "one outcome-free design class crosses an outer-fold boundary",
                    ));
                }
            }
            keyed.push((
                rng.word(ProbeDomain::ComponentVarianceFold, purpose, class.entity, 0),
                class.entity,
                index,
            ));
        }
    }
    if keyed.len() < folds {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "structured_variance_folds",
            "a variance-regression training sample has too few outcome-free design classes",
        ));
    }
    keyed.sort_by_key(|&(word, entity, _)| (word, entity));
    let mut load = vec![0_usize; folds];
    let mut output = vec![u8::MAX; rows];
    for (position, &(_, _, class_index)) in keyed.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "structured_variance_fold_balance")?;
        let fold = load
            .iter()
            .enumerate()
            .min_by_key(|&(fold, count)| (*count, fold))
            .map(|(fold, _)| fold)
            .expect("the registered fold count is positive");
        for &row in &classes[class_index].rows {
            output[row] = u8::try_from(fold).expect("fold count fits u8");
        }
        load[fold] = load[fold]
            .checked_add(classes[class_index].rows.len())
            .ok_or_else(|| resource("structured fold load"))?;
    }
    if load.contains(&0) {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "structured_variance_folds",
            "deterministic variance folds contain an empty validation set",
        ));
    }
    Ok(output)
}

fn canonical_response_order(
    entity: &[u64],
    proxy: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut order = (0..entity.len()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        entity[left]
            .cmp(&entity[right])
            .then_with(|| proxy[left].total_cmp(&proxy[right]))
            .then_with(|| left.cmp(&right))
    });
    interrupt.checkpoint("structured_variance_response_order")?;
    Ok(order)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn fit_cross_fitted_model(
    model: StructuredVarianceModel,
    proxy: &[f64],
    residual: &[f64],
    maker_inverse: &[f64],
    ranks: &[Vec<f64>],
    outer_fold: &[u8],
    inner_fold: &[Vec<u8>],
    canonical_order: &[usize],
    options: StructuredVarianceOptions,
    fold_diagnostics: &mut Vec<StructuredVarianceFoldDiagnostic>,
    cv_diagnostics: &mut Vec<StructuredVarianceCvDiagnostic>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, StructuredVarianceSummary)> {
    let rows = proxy.len();
    let mut output = vec![f64::NAN; rows];
    let mut total_floor = 0_u64;
    let mut total_boundary = 0_u64;
    let mut maximum_boundary_excess = 0.0_f64;
    let mut maximum_prediction_leverage = 0.0_f64;
    let mut minimum_fitted_rcond = f64::INFINITY;
    for outer in 0..STRUCTURED_OUTER_FOLDS {
        interrupt.checkpoint("structured_variance_outer_fold")?;
        let outer_training = outer_fold
            .iter()
            .map(|&fold| usize::from(fold) != outer)
            .collect::<Vec<_>>();
        let outer_validation = outer_fold
            .iter()
            .map(|&fold| usize::from(fold) == outer)
            .collect::<Vec<_>>();
        let mut grid_sse = [CompensatedSum::default(); STRUCTURED_RIDGE_GRID.len()];
        let mut grid_count = [0_u64; STRUCTURED_RIDGE_GRID.len()];
        let mut grid_available = [true; STRUCTURED_RIDGE_GRID.len()];
        for inner in 0..STRUCTURED_INNER_FOLDS {
            let training = (0..rows)
                .map(|row| outer_training[row] && usize::from(inner_fold[outer][row]) != inner)
                .collect::<Vec<_>>();
            let validation = (0..rows)
                .map(|row| outer_training[row] && usize::from(inner_fold[outer][row]) == inner)
                .collect::<Vec<_>>();
            for (grid, &lambda) in STRUCTURED_RIDGE_GRID.iter().enumerate() {
                if !grid_available[grid] {
                    continue;
                }
                let fit = fit_ridge(
                    model,
                    proxy,
                    ranks,
                    &training,
                    canonical_order,
                    lambda,
                    options,
                    interrupt,
                );
                let Ok(fit) = fit else {
                    grid_available[grid] = false;
                    continue;
                };
                for &row in canonical_order {
                    if validation[row] {
                        let prediction = predict(&fit, ranks, row)?;
                        let error = proxy[row] - prediction;
                        grid_sse[grid].add(error * error);
                        grid_count[grid] = grid_count[grid]
                            .checked_add(1)
                            .ok_or_else(|| resource("structured CV observation count"))?;
                    }
                }
            }
        }
        let mut selected: Option<(usize, f64)> = None;
        for (grid, &lambda) in STRUCTURED_RIDGE_GRID.iter().enumerate() {
            let available = grid_available[grid] && grid_count[grid] > 0;
            let mse = if available {
                grid_sse[grid].finish() / grid_count[grid] as f64
            } else {
                f64::NAN
            };
            cv_diagnostics.push(StructuredVarianceCvDiagnostic {
                model: model as u32,
                outer_fold: outer as u32,
                grid_index: grid as u32,
                lambda,
                validation_observations: grid_count[grid],
                mse,
                available,
            });
            if available
                && selected.is_none_or(|(best, best_mse)| {
                    mse < best_mse || (mse.total_cmp(&best_mse) == Ordering::Equal && grid < best)
                })
            {
                selected = Some((grid, mse));
            }
        }
        let Some((selected_grid, selected_mse)) = selected else {
            return Err(BackendError::new(
                ErrorCode::SingularInformation,
                "structured_variance_cv",
                "no registered ridge penalty produced a valid nested-fold fit",
            ));
        };
        let fit = fit_ridge(
            model,
            proxy,
            ranks,
            &outer_training,
            canonical_order,
            STRUCTURED_RIDGE_GRID[selected_grid],
            options,
            interrupt,
        )?;
        let scale = training_positive_scale(
            residual,
            maker_inverse,
            &outer_training,
            canonical_order,
            interrupt,
        )?;
        let floor = options.positivity_multiplier * scale;
        if !floor.is_finite() || floor <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "structured_variance_positivity",
                "the structured variance positivity floor is nonpositive or nonfinite",
            ));
        }
        let bounds = training_bounds(ranks, &outer_training);
        let mut fold_floor = 0_u64;
        let mut fold_boundary = 0_u64;
        let mut fold_maximum_excess = 0.0_f64;
        let mut fold_maximum_prediction_leverage = 0.0_f64;
        let mut training_count = 0_u64;
        let mut validation_count = 0_u64;
        for &row in canonical_order {
            if outer_training[row] {
                training_count += 1;
            }
            if !outer_validation[row] {
                continue;
            }
            validation_count += 1;
            let raw_prediction = predict(&fit, ranks, row)?;
            output[row] = raw_prediction.max(floor);
            if raw_prediction < floor {
                fold_floor += 1;
            }
            let excess = boundary_excess(ranks, row, &bounds);
            if excess > 0.0 {
                fold_boundary += 1;
                fold_maximum_excess = fold_maximum_excess.max(excess);
            }
            fold_maximum_prediction_leverage =
                fold_maximum_prediction_leverage.max(prediction_leverage(&fit, ranks, row)?);
        }
        if validation_count == 0 || fold_floor == validation_count {
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "structured_variance_positivity",
                "a structured variance fold has no usable unfloored prediction",
            ));
        }
        total_floor += fold_floor;
        total_boundary += fold_boundary;
        maximum_boundary_excess = maximum_boundary_excess.max(fold_maximum_excess);
        maximum_prediction_leverage =
            maximum_prediction_leverage.max(fold_maximum_prediction_leverage);
        minimum_fitted_rcond = minimum_fitted_rcond.min(fit.rcond);
        fold_diagnostics.push(StructuredVarianceFoldDiagnostic {
            model: model as u32,
            outer_fold: outer as u32,
            training_observations: training_count,
            validation_observations: validation_count,
            active_terms: u32::try_from(fit.coefficients.len())
                .map_err(|_| resource("active structured term count"))?,
            selected_lambda: STRUCTURED_RIDGE_GRID[selected_grid],
            selected_cv_mse: selected_mse,
            variance_scale: scale,
            positivity_floor: floor,
            floored_predictions: fold_floor,
            boundary_predictions: fold_boundary,
            maximum_boundary_excess: fold_maximum_excess,
            maximum_prediction_leverage: fold_maximum_prediction_leverage,
            fitted_rcond: fit.rcond,
            fitted_relres: fit.relres,
        });
    }
    if output
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(BackendError::invariant(
            "structured_variance_finish",
            "cross-fitted structured variances are incomplete or nonpositive",
        ));
    }
    let summary = summarize_variance(
        model,
        &output,
        total_floor,
        total_boundary,
        maximum_boundary_excess,
        maximum_prediction_leverage,
        minimum_fitted_rcond,
    )?;
    Ok((output, summary))
}

fn model_terms(model: StructuredVarianceModel, diagnostics: usize) -> usize {
    match model {
        StructuredVarianceModel::Common => {
            1 + 2 * diagnostics + diagnostics * (diagnostics - 1) / 2
        }
        StructuredVarianceModel::LeverageOnly => STRUCTURED_LEVERAGE_TERMS,
    }
}

pub(crate) fn basis_row(
    model: StructuredVarianceModel,
    ranks: &[Vec<f64>],
    row: usize,
) -> [f64; GROUPED_STRUCTURED_PRIMARY_TERMS] {
    let h = ranks[0][row];
    if model == StructuredVarianceModel::LeverageOnly {
        let mut output = [0.0; GROUPED_STRUCTURED_PRIMARY_TERMS];
        output[0] = 1.0;
        output[1] = h;
        output[2] = h * h;
        return output;
    }
    let diagnostics = ranks.len();
    debug_assert!(matches!(diagnostics, 4 | 5));
    let mut output = [0.0; GROUPED_STRUCTURED_PRIMARY_TERMS];
    output[0] = 1.0;
    for index in 0..diagnostics {
        output[1 + index] = ranks[index][row];
        output[1 + diagnostics + index] = ranks[index][row] * ranks[index][row];
    }
    let mut term = 1 + 2 * diagnostics;
    for left in 0..diagnostics {
        for right in (left + 1)..diagnostics {
            output[term] = ranks[left][row] * ranks[right][row];
            term += 1;
        }
    }
    output
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn fit_ridge(
    model: StructuredVarianceModel,
    response: &[f64],
    ranks: &[Vec<f64>],
    training: &[bool],
    order: &[usize],
    lambda: f64,
    options: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ModelFit> {
    let diagnostics = ranks.len();
    let terms = model_terms(model, diagnostics);
    let training_rows = training.iter().filter(|&&value| value).count();
    if training_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "structured_variance_support",
            "a structured variance training set is empty",
        ));
    }
    let mut means = vec![0.0; terms];
    for term in 1..terms {
        let mut sum = CompensatedSum::default();
        for &row in order {
            if training[row] {
                sum.add(basis_row(model, ranks, row)[term]);
            }
        }
        means[term] = sum.finish() / training_rows as f64;
    }
    let mut scales = vec![0.0; terms];
    let mut active = Vec::new();
    for term in 1..terms {
        let mut square = CompensatedSum::default();
        for &row in order {
            if training[row] {
                let value = basis_row(model, ranks, row)[term] - means[term];
                square.add(value * value);
            }
        }
        scales[term] = (square.finish() / training_rows as f64).max(0.0).sqrt();
        if scales[term] > 64.0 * f64::EPSILON {
            active.push(term);
        }
    }
    let dimension = 1 + active.len();
    let required = options
        .observations_per_term
        .checked_mul(dimension)
        .ok_or_else(|| resource("structured variance support count"))?;
    if training_rows < required {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "structured_variance_support",
            format!(
                "a structured variance training set has {training_rows} observations for {dimension} active coefficients; at least {required} are required"
            ),
        ));
    }
    let mut gram = vec![0.0; dimension * dimension];
    let mut rhs = vec![0.0; dimension];
    let mut z = vec![0.0; dimension];
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "structured_variance_gram")?;
        if !training[row] {
            continue;
        }
        standardized_row(model, ranks, row, &means, &scales, &active, &mut z);
        for left in 0..dimension {
            rhs[left] += z[left] * response[row] / training_rows as f64;
            for right in 0..dimension {
                gram[left * dimension + right] += z[left] * z[right] / training_rows as f64;
            }
        }
    }
    for term in 1..dimension {
        gram[term * dimension + term] += lambda;
    }
    let inverse = invert_scaled_spd(
        &gram,
        dimension,
        options.rank_tolerance,
        interrupt,
        "structured_variance_ridge",
    )?;
    let mut coefficients = vec![0.0; dimension];
    for row in 0..dimension {
        for column in 0..dimension {
            coefficients[row] += inverse.inverse[row * dimension + column] * rhs[column];
        }
    }
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "structured_variance_ridge",
            "structured variance ridge coefficients are nonfinite",
        ));
    }
    Ok(ModelFit {
        model,
        diagnostics,
        means,
        scales,
        active,
        coefficients,
        inverse: inverse.inverse,
        rcond: inverse.rcond,
        relres: inverse.original_relres.max(inverse.relres),
    })
}

fn standardized_row(
    model: StructuredVarianceModel,
    ranks: &[Vec<f64>],
    row: usize,
    means: &[f64],
    scales: &[f64],
    active: &[usize],
    output: &mut [f64],
) {
    let raw = basis_row(model, ranks, row);
    output[0] = 1.0;
    for (column, &term) in active.iter().enumerate() {
        output[column + 1] = (raw[term] - means[term]) / scales[term];
    }
}

fn predict(fit: &ModelFit, ranks: &[Vec<f64>], row: usize) -> Result<f64> {
    let mut z = [0.0; GROUPED_STRUCTURED_PRIMARY_TERMS];
    if ranks.len() != fit.diagnostics {
        return Err(BackendError::invariant(
            "structured_variance_predict",
            "structured variance diagnostic width changed after fitting",
        ));
    }
    standardized_row(
        fit.model,
        ranks,
        row,
        &fit.means,
        &fit.scales,
        &fit.active,
        &mut z[..fit.coefficients.len()],
    );
    let value = z[..fit.coefficients.len()]
        .iter()
        .zip(&fit.coefficients)
        .map(|(&left, &right)| left * right)
        .sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "structured_variance_predict",
            "a structured variance prediction is nonfinite",
        ))
    }
}

fn prediction_leverage(fit: &ModelFit, ranks: &[Vec<f64>], row: usize) -> Result<f64> {
    let mut z = [0.0; GROUPED_STRUCTURED_PRIMARY_TERMS];
    if ranks.len() != fit.diagnostics {
        return Err(BackendError::invariant(
            "structured_variance_prediction_leverage",
            "structured variance diagnostic width changed after fitting",
        ));
    }
    standardized_row(
        fit.model,
        ranks,
        row,
        &fit.means,
        &fit.scales,
        &fit.active,
        &mut z[..fit.coefficients.len()],
    );
    let dimension = fit.coefficients.len();
    let mut value = 0.0;
    for left in 0..dimension {
        for right in 0..dimension {
            value += z[left] * fit.inverse[left * dimension + right] * z[right];
        }
    }
    if value.is_finite() && value >= -1.0e-10 {
        Ok(value.max(0.0))
    } else {
        Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            "structured_variance_prediction_leverage",
            "structured variance prediction leverage is invalid",
        ))
    }
}

fn training_positive_scale(
    residual: &[f64],
    maker_inverse: &[f64],
    training: &[bool],
    order: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut values = Vec::new();
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "structured_variance_scale")?;
        if training[row] {
            let value = residual[row] * residual[row] * maker_inverse[row];
            if !value.is_finite() || value < 0.0 {
                return Err(BackendError::new(
                    ErrorCode::JlaConstraintFailed,
                    "structured_variance_scale",
                    "structured variance scale observations are invalid",
                ));
            }
            values.push(value);
        }
    }
    values.sort_by(f64::total_cmp);
    median_sorted(&values)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "structured_variance_scale",
                "structured variance training scale is nonpositive or nonfinite",
            )
        })
}

fn training_bounds(ranks: &[Vec<f64>], training: &[bool]) -> Vec<(f64, f64)> {
    (0..ranks.len())
        .map(|column| {
            let mut minimum = f64::INFINITY;
            let mut maximum = f64::NEG_INFINITY;
            for row in 0..training.len() {
                if training[row] {
                    minimum = minimum.min(ranks[column][row]);
                    maximum = maximum.max(ranks[column][row]);
                }
            }
            (minimum, maximum)
        })
        .collect()
}

fn boundary_excess(ranks: &[Vec<f64>], row: usize, bounds: &[(f64, f64)]) -> f64 {
    let mut maximum = 0.0_f64;
    for column in 0..ranks.len() {
        maximum = maximum
            .max((bounds[column].0 - ranks[column][row]).max(0.0))
            .max((ranks[column][row] - bounds[column].1).max(0.0));
    }
    maximum
}

#[allow(clippy::too_many_arguments)]
fn summarize_variance(
    model: StructuredVarianceModel,
    values: &[f64],
    floor_count: u64,
    boundary_count: u64,
    maximum_boundary_excess: f64,
    maximum_prediction_leverage: f64,
    minimum_fitted_rcond: f64,
) -> Result<StructuredVarianceSummary> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let observations =
        u64::try_from(values.len()).map_err(|_| resource("variance observations"))?;
    Ok(StructuredVarianceSummary {
        model: model as u32,
        observations,
        minimum: sorted[0],
        median: median_sorted(&sorted).expect("nonempty variance vector"),
        maximum: sorted[sorted.len() - 1],
        floor_count,
        floor_share: floor_count as f64 / observations as f64,
        boundary_count,
        boundary_share: boundary_count as f64 / observations as f64,
        maximum_boundary_excess,
        maximum_prediction_leverage,
        minimum_fitted_rcond,
    })
}

fn variance_sensitivity(
    common: &[f64],
    leverage: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<StructuredVarianceSensitivity> {
    let mut difference = Vec::with_capacity(common.len());
    let mut left_mean = CompensatedSum::default();
    let mut right_mean = CompensatedSum::default();
    for row in 0..common.len() {
        checkpoint_chunk(interrupt, row, "structured_variance_sensitivity")?;
        let left = common[row].ln();
        let right = leverage[row].ln();
        difference.push((left - right).abs());
        left_mean.add(left);
        right_mean.add(right);
    }
    difference.sort_by(f64::total_cmp);
    let left_mean = left_mean.finish() / common.len() as f64;
    let right_mean = right_mean.finish() / common.len() as f64;
    let mut covariance = CompensatedSum::default();
    let mut left_square = CompensatedSum::default();
    let mut right_square = CompensatedSum::default();
    for row in 0..common.len() {
        let left = common[row].ln() - left_mean;
        let right = leverage[row].ln() - right_mean;
        covariance.add(left * right);
        left_square.add(left * left);
        right_square.add(right * right);
    }
    let denominator = (left_square.finish() * right_square.finish()).sqrt();
    let correlation = if denominator > 0.0 {
        (covariance.finish() / denominator).clamp(-1.0, 1.0)
    } else if difference.last().copied().unwrap_or(0.0) == 0.0 {
        1.0
    } else {
        0.0
    };
    let p90_index = ((9 * difference.len()).div_ceil(10)).saturating_sub(1);
    Ok(StructuredVarianceSensitivity {
        median_absolute_log_ratio: median_sorted(&difference).expect("nonempty sensitivity"),
        p90_absolute_log_ratio: difference[p90_index],
        maximum_absolute_log_ratio: *difference.last().expect("nonempty sensitivity"),
        log_variance_correlation: correlation,
    })
}

fn median_sorted(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else if values.len() % 2 == 1 {
        Some(values[values.len() / 2])
    } else {
        let right = values.len() / 2;
        Some(0.5 * (values[right - 1] + values[right]))
    }
}

fn resource(what: &'static str) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "structured_variance",
        format!("{what} overflow"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::NeverInterrupt;

    fn fixture(
        rows: usize,
    ) -> (
        Vec<f64>,
        Vec<f64>,
        Vec<f64>,
        Vec<f64>,
        [Vec<f64>; 3],
        Vec<u64>,
    ) {
        let leverage = (0..rows)
            .map(|row| 0.05 + 0.8 * row as f64 / (rows - 1) as f64)
            .collect::<Vec<_>>();
        let diagonal = core::array::from_fn(|target| {
            (0..rows)
                .map(|row| {
                    let x = row as f64 / rows as f64;
                    match target {
                        0 => 0.1 + x,
                        1 => 0.2 + (3.0 * x).sin(),
                        _ => x - 0.5,
                    }
                })
                .collect::<Vec<_>>()
        });
        let variance = leverage
            .iter()
            .zip(&diagonal[0])
            .map(|(&h, &b)| 0.8 + 0.4 * h + 0.2 * b)
            .collect::<Vec<_>>();
        let residual = variance
            .iter()
            .map(|value| value.sqrt())
            .collect::<Vec<_>>();
        let maker = leverage
            .iter()
            .map(|value| 1.0 / (1.0 - value))
            .collect::<Vec<_>>();
        let proxy = variance
            .iter()
            .enumerate()
            .map(|(row, &value)| value + 0.05 * ((row * 17 % 11) as f64 - 5.0))
            .collect::<Vec<_>>();
        let entity = (0..rows).map(|row| row as u64 + 1).collect::<Vec<_>>();
        (proxy, residual, maker, leverage, diagonal, entity)
    }

    #[test]
    fn midranks_share_ties_and_are_symmetric() {
        let values = [2.0, 1.0, 2.0, 4.0];
        let ranks = normalized_midranks(&values, &mut NeverInterrupt).expect("midranks");
        assert_eq!(ranks[0], ranks[2]);
        assert!((ranks.iter().sum::<f64>()).abs() < 1.0e-15);
        assert!(ranks.iter().all(|value| (-1.0..1.0).contains(value)));
    }

    #[test]
    fn folds_do_not_depend_on_variance_responses() {
        let (proxy, residual, maker, leverage, diagonal, entity) = fixture(240);
        let first = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &maker,
            &leverage,
            &diagonal,
            &entity,
            StructuredVarianceOptions::default(),
            &mut NeverInterrupt,
        )
        .expect("first fit");
        let changed = proxy
            .iter()
            .rev()
            .map(|value| value * 7.0)
            .collect::<Vec<_>>();
        let second = fit_structured_variance_with_interrupt(
            &changed,
            &residual,
            &maker,
            &leverage,
            &diagonal,
            &entity,
            StructuredVarianceOptions::default(),
            &mut NeverInterrupt,
        )
        .expect("second fit");
        assert_eq!(first.outer_fold, second.outer_fold);
    }

    #[test]
    fn structured_models_are_positive_deterministic_and_distinct() {
        let (proxy, residual, maker, leverage, diagonal, entity) = fixture(160);
        let fit = || {
            fit_structured_variance_with_interrupt(
                &proxy,
                &residual,
                &maker,
                &leverage,
                &diagonal,
                &entity,
                StructuredVarianceOptions::default(),
                &mut NeverInterrupt,
            )
            .expect("structured fit")
        };
        let left = fit();
        let right = fit();
        assert_eq!(left.common, right.common);
        assert_eq!(left.leverage_only, right.leverage_only);
        assert!(left
            .common
            .iter()
            .all(|value| value.is_finite() && *value > 0.0));
        assert!(left
            .leverage_only
            .iter()
            .all(|value| value.is_finite() && *value > 0.0));
        assert_eq!(left.folds.len(), 10);
        assert_eq!(left.cv.len(), 70);
        assert!(left.sensitivity.maximum_absolute_log_ratio > 0.0);
    }

    #[test]
    fn omitted_variance_driver_can_escape_the_registered_sensitivity() {
        let rows = 250;
        let proxy = (0..rows)
            .map(|row| if row % 2 == 0 { 0.25_f64 } else { 4.0_f64 })
            .collect::<Vec<_>>();
        let residual = proxy.iter().map(|value| value.sqrt()).collect::<Vec<_>>();
        let maker = vec![1.0; rows];
        let leverage = vec![0.2; rows];
        let diagonal = [vec![0.1; rows], vec![0.2; rows], vec![-0.1; rows]];
        let entity = (0..rows).map(|row| row as u64 + 1).collect::<Vec<_>>();
        let fit = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &maker,
            &leverage,
            &diagonal,
            &entity,
            StructuredVarianceOptions::default(),
            &mut NeverInterrupt,
        )
        .expect("intercept-only structured fit");

        // An omitted outcome-free variance driver leaves both registered
        // designs at the same cross-fitted intercept. Their agreement is
        // therefore a sensitivity comparison, never a specification test.
        assert_eq!(fit.common, fit.leverage_only);
        assert_eq!(fit.sensitivity.maximum_absolute_log_ratio, 0.0);
        assert_eq!(fit.sensitivity.log_variance_correlation, 1.0);
        let mse = fit
            .common
            .iter()
            .zip(&proxy)
            .map(|(&prediction, &truth)| (prediction - truth).powi(2))
            .sum::<f64>()
            / rows as f64;
        assert!(mse > 2.0);
    }

    #[test]
    fn grouped_design_classes_remain_in_one_fold() {
        let (proxy, residual, maker, leverage, diagonal, mut entity) = fixture(240);
        for pair in 0..120 {
            entity[2 * pair] = pair as u64;
            entity[2 * pair + 1] = pair as u64;
        }
        let result = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &maker,
            &leverage,
            &diagonal,
            &entity,
            StructuredVarianceOptions::default(),
            &mut NeverInterrupt,
        )
        .expect("grouped fold fit");
        for pair in 0..120 {
            assert_eq!(result.outer_fold[2 * pair], result.outer_fold[2 * pair + 1]);
        }
    }

    #[test]
    fn insufficient_fold_support_fails_closed() {
        let (proxy, residual, maker, leverage, diagonal, _) = fixture(40);
        let entity = vec![1_u64; 40];
        let error = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &maker,
            &leverage,
            &diagonal,
            &entity,
            StructuredVarianceOptions::default(),
            &mut NeverInterrupt,
        )
        .expect_err("one design class cannot support five folds");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
        assert_eq!(error.phase, "structured_variance_folds");
    }
}
