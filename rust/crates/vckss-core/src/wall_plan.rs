// SPDX-License-Identifier: GPL-3.0-only

//! Advisory wall-work forecasting.
//!
//! Wall forecasts are diagnostics only.  They cannot change engine routing,
//! batching, tolerances, probe counts, or failure behavior.

use crate::error::{BackendError, ErrorCode, Result};

pub const WALL_WORK_SCHEMA_VERSION: u32 = 1;
pub const UNCALIBRATED_WALL_MODEL_ID: &str = "VCKSS-WALL-UNCALIBRATED";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WallRoutingEffect {
    AdvisoryOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WallAdvisoryStatus {
    NotRequested,
    Uncalibrated,
    WithinRequestedEnvelope,
    ExceedsRequestedEnvelope,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WallWork {
    pub preparation: u64,
    pub engine_setup: u64,
    pub full_fit: u64,
    pub leverage: u64,
    pub target: u64,
    pub result_export: u64,
}

impl WallWork {
    pub fn total(self) -> Result<u64> {
        [
            self.preparation,
            self.engine_setup,
            self.full_fit,
            self.leverage,
            self.target,
            self.result_export,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            total.checked_add(value).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "wall_plan",
                    "wall-work total overflow",
                )
            })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WallCalibration {
    Uncalibrated,
    Calibrated {
        model_id: &'static str,
        intercept_seconds: f64,
        seconds_per_work_unit: f64,
        advisory_margin_fraction: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallWorkReceipt {
    pub schema_version: u32,
    pub work: WallWork,
    pub total_work: u64,
    pub model_id: &'static str,
    pub requested_seconds: Option<f64>,
    pub forecast_seconds: Option<f64>,
    pub advisory_seconds: Option<f64>,
    pub advisory_margin_fraction: Option<f64>,
    pub status: WallAdvisoryStatus,
    pub routing_effect: WallRoutingEffect,
}

pub fn wall_work_receipt(
    work: WallWork,
    requested_seconds: Option<f64>,
    calibration: WallCalibration,
) -> Result<WallWorkReceipt> {
    if requested_seconds.is_some_and(|seconds| !seconds.is_finite() || seconds <= 0.0) {
        return Err(BackendError::invalid(
            "wall_plan",
            "wallseconds must be positive and finite when supplied",
        ));
    }
    let total_work = work.total()?;
    let (model_id, forecast_seconds, advisory_seconds, margin, status) = match calibration {
        WallCalibration::Uncalibrated => (
            UNCALIBRATED_WALL_MODEL_ID,
            None,
            None,
            None,
            WallAdvisoryStatus::Uncalibrated,
        ),
        WallCalibration::Calibrated {
            model_id,
            intercept_seconds,
            seconds_per_work_unit,
            advisory_margin_fraction,
        } => {
            if model_id.is_empty()
                || !intercept_seconds.is_finite()
                || intercept_seconds < 0.0
                || !seconds_per_work_unit.is_finite()
                || seconds_per_work_unit <= 0.0
                || !advisory_margin_fraction.is_finite()
                || advisory_margin_fraction < 0.0
            {
                return Err(BackendError::invalid(
                    "wall_plan",
                    "registered wall calibration is invalid",
                ));
            }
            let forecast = intercept_seconds + seconds_per_work_unit * total_work as f64;
            let advisory = forecast * (1.0 + advisory_margin_fraction);
            if !forecast.is_finite() || !advisory.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "wall_plan",
                    "wall forecast is not finite",
                ));
            }
            let status = requested_seconds.map_or(WallAdvisoryStatus::NotRequested, |requested| {
                if advisory <= requested {
                    WallAdvisoryStatus::WithinRequestedEnvelope
                } else {
                    WallAdvisoryStatus::ExceedsRequestedEnvelope
                }
            });
            (
                model_id,
                Some(forecast),
                Some(advisory),
                Some(advisory_margin_fraction),
                status,
            )
        }
    };
    Ok(WallWorkReceipt {
        schema_version: WALL_WORK_SCHEMA_VERSION,
        work,
        total_work,
        model_id,
        requested_seconds,
        forecast_seconds,
        advisory_seconds,
        advisory_margin_fraction: margin,
        status,
        routing_effect: WallRoutingEffect::AdvisoryOnly,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work() -> WallWork {
        WallWork {
            preparation: 10,
            engine_setup: 20,
            full_fit: 30,
            leverage: 40,
            target: 50,
            result_export: 60,
        }
    }

    fn calibration() -> WallCalibration {
        WallCalibration::Calibrated {
            model_id: "TEST-WALL-V1",
            intercept_seconds: 1.0,
            seconds_per_work_unit: 0.1,
            advisory_margin_fraction: 0.5,
        }
    }

    #[test]
    fn uncalibrated_is_explicit_even_when_wallseconds_is_supplied() {
        let receipt = wall_work_receipt(work(), Some(10.0), WallCalibration::Uncalibrated)
            .expect("uncalibrated receipt");
        assert_eq!(receipt.status, WallAdvisoryStatus::Uncalibrated);
        assert_eq!(receipt.model_id, UNCALIBRATED_WALL_MODEL_ID);
        assert_eq!(receipt.forecast_seconds, None);
        assert_eq!(receipt.advisory_seconds, None);
        assert_eq!(receipt.routing_effect, WallRoutingEffect::AdvisoryOnly);
    }

    #[test]
    fn calibrated_status_compares_only_the_advisory_envelope() {
        let within = wall_work_receipt(work(), Some(40.0), calibration()).expect("within");
        let exceeds = wall_work_receipt(work(), Some(30.0), calibration()).expect("exceeds");
        let omitted = wall_work_receipt(work(), None, calibration()).expect("omitted");
        assert_eq!(within.total_work, 210);
        assert_eq!(within.forecast_seconds, Some(22.0));
        assert_eq!(within.advisory_seconds, Some(33.0));
        assert_eq!(within.status, WallAdvisoryStatus::WithinRequestedEnvelope);
        assert_eq!(exceeds.status, WallAdvisoryStatus::ExceedsRequestedEnvelope);
        assert_eq!(omitted.status, WallAdvisoryStatus::NotRequested);
        assert_eq!(within.routing_effect, exceeds.routing_effect);
    }

    #[test]
    fn invalid_inputs_and_work_overflow_are_typed() {
        let invalid =
            wall_work_receipt(work(), Some(0.0), calibration()).expect_err("invalid wallseconds");
        assert_eq!(invalid.code, ErrorCode::InvalidInput);

        let overflow = WallWork {
            preparation: u64::MAX,
            engine_setup: 1,
            ..WallWork::default()
        }
        .total()
        .expect_err("work overflow");
        assert_eq!(overflow.code, ErrorCode::ResourceLimit);
    }
}
