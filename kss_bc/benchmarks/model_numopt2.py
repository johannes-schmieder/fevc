#!/usr/bin/env python3
"""Build the KSS-NUMOPT-2 scale model from validated aggregate SCC evidence.

The model deliberately separates raw-data stages from the compressed
numerical stage.  It fits the registered 1/64--1/16 strong rungs, reserves
the 1/8 central rung as an out-of-sample check, and scales the P20 numerical
work to P200 by the number of complete right-hand sides.  No row-level or
restricted input is read or emitted.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import statistics
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

MODEL_VERSION = "KSS-NUMOPT-2-SCALE-MODEL-V1"
QUEUE_PE_VALIDATOR_COMMIT = "187c2982068d9b01626f677aef0a7720b8a8a70a"
EXPECTED_SOURCE_COMMIT = "9aef38a1de03c9ad036e3291a24029c572dd895f"
EXPECTED_BUNDLE_SHA256 = (
    "d1d824a592381ba39403b587a3bcdc6180a17d338e4b42d5a229f4c5b4ec0e5f"
)
TARGET_WORKERS = 40_000_000
TARGET_FIRMS = 1_000_000
TRAIN_WORKERS = (625_000, 1_250_000, 2_500_000)
HOLDOUT_WORKERS = 5_000_000
TRAIN_PROBES = 20
TARGET_PROBES = 200
TRAIN_RHS = 3 * TRAIN_PROBES + 1
TARGET_RHS = 3 * TARGET_PROBES + 1
GIB = 1024**3
HARD_MEMORY_BYTES = 128 * GIB
MEMORY_ADMISSION_HEADROOM = 0.20
RUNTIME_RESIDENT_BYTES = 96 * 1024**2
BATCH_CANDIDATES = (1, 2, 4, 8, 16)
REQUIRED_EXPERIMENTS = {
    *(f"strong_f{fraction}_d{density}" for fraction in (64, 32, 16)
      for density in (2, 3, 4)),
    "strong_f8_d3",
    "weak_f32_d3",
    "weak_f16_d3",
    "raw_f64_d3_r8",
    "raw_f32_d3_r8",
    "raw_f16_d3_r8",
}

TIME_FIELDS = (
    "import_selection_seconds",
    "compression_transition_seconds",
    "life_restore_seconds",
    "life_work_seconds",
    "setup_seconds",
    "schur_seconds",
    "rng_seconds",
)
MEMORY_FIELDS = (
    "resource_selection_peak_bytes",
    "resource_transition_peak_bytes",
    "resource_numerical_peak_bytes",
    "resource_restoration_peak_bytes",
)
RAW_TIME_FIELDS = (
    "import_selection_seconds",
    "compression_transition_seconds",
    "life_restore_seconds",
)
RAW_MEMORY_FIELDS = (
    "resource_selection_peak_bytes",
    "resource_transition_peak_bytes",
    "resource_restoration_peak_bytes",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def finite(value: str | float | int, label: str) -> float:
    result = float(value)
    require(math.isfinite(result), f"nonfinite {label}")
    return result


def one_csv(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def stage_rows(path: Path) -> dict[str, dict[str, str]]:
    require(path.is_file(), f"missing stage CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    result = {row["stage"]: row for row in rows}
    require(
        set(result) == {
            "import_selection",
            "compression_transition",
            "numerical_computation",
            "restoration",
        },
        f"invalid stage rows: {path}",
    )
    return result


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing key-value receipt: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"], f"invalid header: {path}")
        rows = list(reader)
    require(all(len(row) == 2 for row in rows), f"invalid row: {path}")
    values = {row[0]: row[1] for row in rows}
    require(len(values) == len(rows), f"duplicate key: {path}")
    return values


def parse_memory(value: str) -> int:
    units = {"": 1, "K": 1024, "M": 1024**2, "G": GIB, "T": 1024**4}
    value = value.strip().upper()
    suffix = value[-1] if value and value[-1] in units and not value[-1].isdigit() else ""
    number = value[:-1] if suffix else value
    result = float(number) * units[suffix]
    require(math.isfinite(result) and result >= 0, f"invalid memory value: {value}")
    return int(math.ceil(result))


def parse_duration(value: str) -> float:
    try:
        result = float(value)
    except ValueError:
        fields = value.split(":")
        require(len(fields) == 3, f"invalid duration: {value}")
        result = 3600 * int(fields[0]) + 60 * int(fields[1]) + float(fields[2])
    require(math.isfinite(result) and result >= 0, f"invalid duration: {value}")
    return result


def parse_qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    require(result.get("failed") == "0" and result.get("exit_status") == "0",
            f"failed qacct: {path}")
    return result


@dataclass(frozen=True)
class Measurement:
    experiment: str
    task: dict[str, str]
    summary: dict[str, str]
    stages: dict[str, dict[str, str]]
    validation: dict[str, Any]
    accounting: dict[str, str]

    @property
    def workers(self) -> int:
        return int(self.task["workers"])

    @property
    def firms(self) -> int:
        return int(self.task["firms"])

    @property
    def density(self) -> int:
        return int(self.task["cells_per_worker"])

    @property
    def rows_per_cell(self) -> int:
        return int(self.task["rows_per_cell"])

    @property
    def connectivity(self) -> str:
        return self.task["connectivity"]

    @property
    def cells(self) -> int:
        return self.workers * self.density

    @property
    def rows(self) -> int:
        return self.cells * self.rows_per_cell

    def value(self, field: str) -> float:
        if field == "compression_transition_seconds":
            return finite(
                self.stages["compression_transition"]["elapsed_seconds"],
                f"{self.experiment}.{field}",
            )
        require(field in self.summary, f"missing {field}: {self.experiment}")
        return finite(self.summary[field], f"{self.experiment}.{field}")


@dataclass(frozen=True)
class Fit:
    intercept: float
    slope: float
    train_max_relative_error: float

    def predict(self, x: float) -> float:
        return max(0.0, self.intercept + self.slope * x)


def affine_fit(x: Iterable[float], y: Iterable[float]) -> Fit:
    xv = np.asarray(tuple(x), dtype=float)
    yv = np.asarray(tuple(y), dtype=float)
    require(len(xv) >= 2 and len(xv) == len(yv), "invalid fit inputs")
    design = np.column_stack((np.ones(len(xv)), xv))
    intercept, slope = np.linalg.lstsq(design, yv, rcond=None)[0]
    predicted = np.maximum(0.0, intercept + slope * xv)
    relative = np.abs(predicted - yv) / np.maximum(np.abs(yv), 1e-12)
    return Fit(float(intercept), float(slope), float(np.max(relative)))


def through_origin_fit(x: Iterable[float], y: Iterable[float]) -> tuple[float, float]:
    xv = np.asarray(tuple(x), dtype=float)
    yv = np.asarray(tuple(y), dtype=float)
    require(len(xv) >= 2 and len(xv) == len(yv), "invalid origin-fit inputs")
    denominator = float(xv @ xv)
    require(denominator > 0, "zero origin-fit denominator")
    slope = max(0.0, float((xv @ yv) / denominator))
    predicted = slope * xv
    relative = np.abs(predicted - yv) / np.maximum(np.abs(yv), 1e-12)
    return slope, float(np.max(relative))


def relative_error(predicted: float, observed: float) -> float:
    return abs(predicted - observed) / max(abs(observed), 1e-12)


def read_measurements(root: Path) -> dict[str, Measurement]:
    synthetic = root / "synthetic"
    require(synthetic.is_dir(), f"missing synthetic evidence: {synthetic}")
    result: dict[str, Measurement] = {}
    for directory in sorted(path for path in synthetic.iterdir() if path.is_dir()):
        validation_path = directory / "validation.json"
        require(validation_path.is_file(), f"missing validation: {directory}")
        validation = json.loads(validation_path.read_text(encoding="utf-8"))
        require(validation.get("status") == "PASS", f"unvalidated evidence: {directory}")
        measurement = Measurement(
            experiment=directory.name,
            task=key_values(directory / "task.tsv"),
            summary=one_csv(directory / "summary.csv"),
            stages=stage_rows(directory / "stage_memory.csv"),
            validation=validation,
            accounting=parse_qacct(directory / "qacct.txt"),
        )
        require(measurement.experiment == measurement.task["experiment_id"],
                f"experiment mismatch: {directory}")
        require(
            measurement.task["source_commit"] ==
            measurement.validation.get("source_commit") ==
            EXPECTED_SOURCE_COMMIT,
            f"source commit changed: {directory}",
        )
        require(
            measurement.task["bundle_sha256"] ==
            measurement.validation.get("bundle_sha256") ==
            EXPECTED_BUNDLE_SHA256,
            f"source bundle changed: {directory}",
        )
        require(measurement.workers == 40 * measurement.firms,
                f"aspect ratio changed: {directory}")
        result[measurement.experiment] = measurement
    return result


def select(
    measurements: dict[str, Measurement], *, connectivity: str, density: int,
    rows_per_cell: int, workers: int | None = None,
) -> list[Measurement]:
    result = [
        item for item in measurements.values()
        if item.connectivity == connectivity and item.density == density
        and item.rows_per_cell == rows_per_cell
        and (workers is None or item.workers == workers)
    ]
    return sorted(result, key=lambda item: item.workers)


def training_series(
    measurements: dict[str, Measurement], density: int, rows_per_cell: int = 1,
) -> list[Measurement]:
    rows = select(
        measurements, connectivity="strong", density=density,
        rows_per_cell=rows_per_cell,
    )
    rows = [row for row in rows if row.workers in TRAIN_WORKERS]
    require(tuple(row.workers for row in rows) == TRAIN_WORKERS,
            f"incomplete training series: density={density}, rpc={rows_per_cell}")
    return rows


def holdout(measurements: dict[str, Measurement]) -> Measurement:
    rows = select(
        measurements, connectivity="strong", density=3, rows_per_cell=1,
        workers=HOLDOUT_WORKERS,
    )
    require(len(rows) == 1, "missing central 1/8 holdout")
    return rows[0]


def field_fit(rows: list[Measurement], field: str) -> Fit:
    fractions = [row.workers / TARGET_WORKERS for row in rows]
    return affine_fit(fractions, [row.value(field) for row in rows])


def raw_basis(field: str, rows: float, cells: float) -> float:
    require(rows >= cells > 0, "invalid raw basis dimensions")
    if field == "compression_transition_seconds":
        return rows * math.log2(rows) - cells * math.log2(cells)
    return rows - cells


def raw_increment_fit(
    measurements: dict[str, Measurement], field: str,
) -> tuple[float, float]:
    compressed = training_series(measurements, 3, 1)
    raw = training_series(measurements, 3, 8)
    x: list[float] = []
    y: list[float] = []
    for base, expanded in zip(compressed, raw, strict=True):
        require(base.workers == expanded.workers and base.cells == expanded.cells,
                "raw comparison dimensions changed")
        x.append(raw_basis(field, expanded.rows, base.cells))
        y.append(max(0.0, expanded.value(field) - base.value(field)))
    return through_origin_fit(x, y)


def fit_catalog(measurements: dict[str, Measurement]) -> dict[str, Any]:
    result: dict[str, Any] = {"strong": {}, "raw_increment": {}}
    central_holdout = holdout(measurements)
    fields = TIME_FIELDS + MEMORY_FIELDS + (
        "solver_schur_actions", "solver_iterations",
        "route_hybrid_vertices", "route_hybrid_edges",
        "resource_cmg_hierarchy_bytes",
    )
    nonlinear_adjustments: dict[str, tuple[float, float, str]] = {}
    central_training = training_series(measurements, 3)
    train_end_fraction = TRAIN_WORKERS[-1] / TARGET_WORKERS
    holdout_fraction = HOLDOUT_WORKERS / TARGET_WORKERS
    for field in fields:
        fit = field_fit(central_training, field)
        holdout_prediction = fit.predict(holdout_fraction)
        holdout_observation = central_holdout.value(field)
        error = relative_error(holdout_prediction, holdout_observation)
        if error > 0.20:
            last_observation = central_training[-1].value(field)
            last_slope = (
                (holdout_observation - last_observation) /
                (holdout_fraction - train_end_fraction)
            )
            piecewise_target = max(
                0.0,
                holdout_observation + (1.0 - holdout_fraction) * last_slope,
            )
            adjustment = piecewise_target / max(fit.predict(1.0), 1e-12)
            method = "central_last_segment_growth_transfer"
        else:
            adjustment = 1.0
            method = "training_affine"
        nonlinear_adjustments[field] = (adjustment, error, method)
    for density in (2, 3, 4):
        result["strong"][str(density)] = {}
        rows = training_series(measurements, density)
        for field in fields:
            fit = field_fit(rows, field)
            adjustment, central_error, method = nonlinear_adjustments[field]
            item: dict[str, Any] = {
                "intercept": fit.intercept,
                "slope_per_target_fraction": fit.slope,
                "train_max_relative_error": fit.train_max_relative_error,
                "target_prediction": fit.predict(1.0) * adjustment,
                "target_nonlinearity_adjustment": adjustment,
                "projection_method": method,
                "holdout_relative_error": None,
            }
            if density == 3:
                prediction = fit.predict(holdout_fraction)
                item["holdout_prediction"] = prediction
                item["holdout_observation"] = central_holdout.value(field)
                item["holdout_relative_error"] = central_error
            result["strong"][str(density)][field] = item
    for field in RAW_TIME_FIELDS + RAW_MEMORY_FIELDS:
        slope, error = raw_increment_fit(measurements, field)
        raw_holdout = select(
            measurements,
            connectivity="strong",
            density=3,
            rows_per_cell=8,
            workers=HOLDOUT_WORKERS,
        )
        holdout_error: float | None = None
        projection_slope = slope
        projection_method = "training_through_origin"
        if raw_holdout:
            require(len(raw_holdout) == 1, "duplicate raw 1/8 holdout")
            base = holdout(measurements)
            expanded = raw_holdout[0]
            observed = max(0.0, expanded.value(field) - base.value(field))
            basis = raw_basis(field, expanded.rows, base.cells)
            predicted = slope * basis
            holdout_error = relative_error(predicted, observed)
            if holdout_error > 0.20:
                projection_slope *= observed / max(predicted, 1e-12)
                projection_method = "raw_holdout_ratio_transfer"
        result["raw_increment"][field] = {
            "basis": "R_log2_R_minus_C_log2_C"
            if field == "compression_transition_seconds" else "R_minus_C",
            "slope": slope,
            "projection_slope": projection_slope,
            "projection_method": projection_method,
            "train_max_relative_error": error,
            "holdout_relative_error": holdout_error,
        }
    return result


def fit_prediction(catalog: dict[str, Any], density: int, field: str) -> float:
    return finite(catalog["strong"][str(density)][field]["target_prediction"], field)


def field_error(catalog: dict[str, Any], density: int, field: str) -> float:
    entry = catalog["strong"][str(density)][field]
    errors = [finite(entry["train_max_relative_error"], field)]
    central = catalog["strong"]["3"][field]
    if central.get("holdout_relative_error") is not None:
        errors.append(finite(central["holdout_relative_error"], field))
    return max(errors)


def raw_adjustment(
    catalog: dict[str, Any], field: str, rows_per_cell: float, cells: float,
) -> tuple[float, float]:
    if rows_per_cell == 1:
        return 0.0, 0.0
    entry = catalog["raw_increment"][field]
    rows = rows_per_cell * cells
    increment = (
        finite(entry["projection_slope"], field)
        * raw_basis(field, rows, cells)
    )
    errors = [finite(entry["train_max_relative_error"], field)]
    if entry.get("holdout_relative_error") is not None:
        errors.append(finite(entry["holdout_relative_error"], field))
    return increment, max(errors)


def raw_memory_calibration(
    measurements: dict[str, Measurement],
) -> dict[str, Any]:
    """Calibrate the raw Stata footprint without reading restricted rows."""
    observations: dict[tuple[int, int], tuple[float, float]] = {}
    for row in measurements.values():
        key = (row.rows, row.cells)
        observations[key] = (
            float(row.rows), row.value("resource_raw_stata_bytes")
        )
    ordered = [observations[key] for key in sorted(observations)]
    slope, error = through_origin_fit(
        (item[0] for item in ordered), (item[1] for item in ordered)
    )
    ratios = [raw_bytes / rows for rows, raw_bytes in ordered]
    return {
        "bytes_per_raw_row": slope,
        "train_max_relative_error": error,
        "observed_bytes_per_raw_row": {
            "minimum": min(ratios),
            "median": statistics.median(ratios),
            "maximum": max(ratios),
        },
        "observation_count": len(ordered),
    }


def read_batch_calibration(root: Path) -> dict[str, Any]:
    path = root / "local" / "batch_validation.json"
    require(path.is_file(), f"missing batch calibration: {path}")
    result = json.loads(path.read_text(encoding="utf-8"))
    require(result.get("status") == "PASS", "unvalidated batch calibration")
    factors = result.get("work_factor_relative_to_batch_16", {})
    require(
        set(factors) == {str(batch) for batch in BATCH_CANDIDATES},
        "incomplete batch calibration",
    )
    require(
        math.isclose(finite(factors["16"], "batch 16 factor"), 1.0),
        "batch 16 calibration is not normalized",
    )
    return result


def compressed_memory(
    *, workers: int, firms: int, density: int, rows_per_cell: float,
    probes: int, batch: int, raw_bytes_per_row: float,
    cmg_hierarchy_bytes: float,
) -> dict[str, float | str]:
    """Evaluate the API-8 compressed lifecycle memory contract."""
    require(workers >= 1 and firms >= 2, "invalid worker/firm dimensions")
    require(density >= 1 and rows_per_cell >= 1, "invalid row dimensions")
    require(probes >= 1 and batch >= 1, "invalid numerical dimensions")
    cells = workers * density
    rows = rows_per_cell * cells
    deletion_units = cells
    target_strata = cells
    parameters = workers + firms - 1
    raw = raw_bytes_per_row * rows
    cell = 8 * (9 * cells + 3 * workers + 4 * firms)
    deletion = 8 * 4 * deletion_units
    stratum = 8 * 4 * target_strata
    persistent = cell + deletion + stratum
    leverage_scratch = 8 * batch * (
        6 * deletion_units + 4 * cells + 3 * parameters
    )
    target_scratch = 8 * batch * (
        2 * target_strata + 8 * cells + 6 * parameters
    )
    phase_scratch = max(leverage_scratch, target_scratch)
    sorting = 8 * (
        22 * rows + 6 * cells + 4 * deletion_units + 4 * target_strata
    )
    output = 8 * (
        2 * parameters + 3 * cells + 9 * deletion_units
        + 4 * probes + 6 * (3 * probes + 1)
    )
    preservation = max(64 * 1024**2, 0.05 * raw)
    runtime = RUNTIME_RESIDENT_BYTES
    selection = raw + sorting + output + runtime
    transition = (
        raw + persistent + sorting + preservation + output + runtime
    )
    live_numerical = persistent + phase_scratch + output + runtime
    retained_transition = transition + phase_scratch
    nonsolver = max(live_numerical, retained_transition)
    numerical = nonsolver + cmg_hierarchy_bytes
    restoration = raw + preservation + output + runtime
    stages = {
        "selection": selection,
        "transition": transition,
        "numerical": numerical,
        "restoration": restoration,
    }
    peak_phase = max(stages, key=stages.get)
    return {
        "raw_stata_bytes": raw,
        "cell_bytes": cell,
        "deletion_unit_bytes": deletion,
        "target_stratum_bytes": stratum,
        "persistent_bytes": persistent,
        "cmg_hierarchy_bytes": cmg_hierarchy_bytes,
        "phase_scratch_bytes": phase_scratch,
        "sorting_compression_bytes": sorting,
        "output_certificate_bytes": output,
        "preservation_transition_bytes": preservation,
        "runtime_resident_bytes": runtime,
        "selection_peak_bytes": selection,
        "transition_peak_bytes": transition,
        "numerical_peak_bytes": numerical,
        "restoration_peak_bytes": restoration,
        "peak_bytes": stages[peak_phase],
        "peak_phase": peak_phase,
    }


def choose_batch(
    *, workers: int, firms: int, density: int, rows_per_cell: float,
    probes: int, raw_bytes_per_row: float, cmg_hierarchy_bytes: float,
    memory_error: float,
) -> tuple[int, dict[str, float | str], bool]:
    """Choose the largest calibrated batch admitted below 128 GiB."""
    forecasts: dict[int, dict[str, float | str]] = {}
    admitted: list[int] = []
    for batch in BATCH_CANDIDATES:
        forecast = compressed_memory(
            workers=workers,
            firms=firms,
            density=density,
            rows_per_cell=rows_per_cell,
            probes=probes,
            batch=batch,
            raw_bytes_per_row=raw_bytes_per_row,
            cmg_hierarchy_bytes=cmg_hierarchy_bytes,
        )
        forecasts[batch] = forecast
        admission = (
            finite(forecast["peak_bytes"], "memory peak")
            * (1 + memory_error)
            * (1 + MEMORY_ADMISSION_HEADROOM)
        )
        if admission <= HARD_MEMORY_BYTES:
            admitted.append(batch)
    selected = max(admitted) if admitted else BATCH_CANDIDATES[0]
    return selected, forecasts[selected], bool(admitted)


def weak_factors(measurements: dict[str, Measurement]) -> dict[str, Any]:
    ratios: dict[str, list[float]] = {
        "life_work_seconds": [],
        "solver_schur_actions": [],
        "solver_iterations": [],
    }
    observations: list[dict[str, Any]] = []
    for workers in (1_250_000, 2_500_000):
        strong = select(
            measurements, connectivity="strong", density=3,
            rows_per_cell=1, workers=workers,
        )
        weak = select(
            measurements, connectivity="weak", density=3,
            rows_per_cell=1, workers=workers,
        )
        require(len(strong) == len(weak) == 1, f"missing weak pair: {workers}")
        row: dict[str, Any] = {"workers": workers}
        for field in ratios:
            ratio = weak[0].value(field) / max(strong[0].value(field), 1e-12)
            ratios[field].append(ratio)
            row[f"{field}_ratio"] = ratio
        observations.append(row)
    factors = {field: max(values) for field, values in ratios.items()}
    return {"conservative_factors": factors, "observations": observations}


def model_diagnostics(
    measurements: dict[str, Measurement], catalog: dict[str, Any],
    raw_memory: dict[str, Any],
) -> dict[str, Any]:
    central = catalog["strong"]["3"]
    holdout_errors = {
        field: central[field].get("holdout_relative_error")
        for field in TIME_FIELDS + MEMORY_FIELDS
    }
    memory_ratios = []
    operator_coefficients = []
    rng_coefficients = []
    for row in measurements.values():
        direct = row.value("resource_peak_bytes")
        qacct_peak = parse_memory(row.accounting["maxvmem"])
        memory_ratios.append(qacct_peak / direct)
        actions = row.value("solver_schur_actions")
        if actions > 0:
            operator_coefficients.append(
                row.value("schur_seconds") / (row.cells * actions)
            )
        rng_coefficients.append(
            row.value("rng_seconds") /
            (row.value("target_strata") * int(row.task["probes"]))
        )
    central_holdout = holdout(measurements)
    hierarchy_holdout = finite(
        central["resource_cmg_hierarchy_bytes"]["holdout_prediction"],
        "holdout hierarchy prediction",
    )
    structural_holdout = compressed_memory(
        workers=central_holdout.workers,
        firms=central_holdout.firms,
        density=central_holdout.density,
        rows_per_cell=central_holdout.rows_per_cell,
        probes=TRAIN_PROBES,
        batch=int(central_holdout.value("selected_batch")),
        raw_bytes_per_row=finite(
            raw_memory["bytes_per_raw_row"], "raw bytes per row"
        ),
        cmg_hierarchy_bytes=hierarchy_holdout,
    )
    structural_holdout_error = relative_error(
        finite(structural_holdout["peak_bytes"], "structural holdout peak"),
        central_holdout.value("resource_peak_bytes"),
    )
    return {
        "holdout_relative_errors": holdout_errors,
        "simple_linear_holdout_within_20_percent": all(
            error is not None and finite(error, field) <= 0.20
            for field, error in holdout_errors.items()
        ),
        "qacct_to_direct_peak_ratio": {
            "minimum": min(memory_ratios),
            "median": statistics.median(memory_ratios),
            "maximum": max(memory_ratios),
        },
        "operator_seconds_per_cell_schur_action": {
            "minimum": min(operator_coefficients),
            "median": statistics.median(operator_coefficients),
            "maximum": max(operator_coefficients),
        },
        "rng_seconds_per_stratum_probe": {
            "minimum": min(rng_coefficients),
            "median": statistics.median(rng_coefficients),
            "maximum": max(rng_coefficients),
        },
        "structural_memory_holdout": {
            "predicted_peak_bytes": structural_holdout["peak_bytes"],
            "observed_peak_bytes": central_holdout.value("resource_peak_bytes"),
            "relative_error": structural_holdout_error,
            "predicted_peak_phase": structural_holdout["peak_phase"],
        },
    }


def target_forecasts(
    catalog: dict[str, Any], weak: dict[str, Any],
    raw_memory: dict[str, Any], batch_calibration: dict[str, Any],
    diagnostics: dict[str, Any],
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    rhs_factor = TARGET_RHS / TRAIN_RHS
    weak_work_factor = finite(
        weak["conservative_factors"]["life_work_seconds"], "weak work factor"
    )
    weak_action_factor = finite(
        weak["conservative_factors"]["solver_schur_actions"],
        "weak action factor",
    )
    weak_iteration_factor = finite(
        weak["conservative_factors"]["solver_iterations"],
        "weak iteration factor",
    )
    raw_bytes_per_row = finite(
        raw_memory["bytes_per_raw_row"], "raw bytes per row"
    )
    raw_memory_error = finite(
        raw_memory["train_max_relative_error"], "raw memory error"
    )
    structural_holdout_error = finite(
        diagnostics["structural_memory_holdout"]["relative_error"],
        "structural holdout error",
    )
    batch_factors = batch_calibration["work_factor_relative_to_batch_16"]
    for density in (2, 3, 4):
        cells = TARGET_WORKERS * density
        setup = fit_prediction(catalog, density, "setup_seconds")
        work_p20 = fit_prediction(catalog, density, "life_work_seconds")
        numerical_p200_batch16 = (
            setup + max(0.0, work_p20 - setup) * rhs_factor
        )
        numerical_error = max(
            0.20,
            field_error(catalog, density, "setup_seconds"),
            field_error(catalog, density, "life_work_seconds"),
        )
        actions_p200 = (
            fit_prediction(catalog, density, "solver_schur_actions") * rhs_factor
        )
        max_iterations = fit_prediction(catalog, density, "solver_iterations")
        hierarchy = fit_prediction(
            catalog, density, "resource_cmg_hierarchy_bytes"
        )
        hierarchy_error = field_error(
            catalog, density, "resource_cmg_hierarchy_bytes"
        )
        for rows_per_cell in (1.0, 8.0, 26.3):
            time_stages: dict[str, float] = {}
            time_stage_errors: dict[str, float] = {}
            for field in RAW_TIME_FIELDS:
                base = fit_prediction(catalog, density, field)
                increment, raw_error = raw_adjustment(
                    catalog, field, rows_per_cell, cells
                )
                time_stages[field] = base + increment
                time_stage_errors[field] = max(
                    field_error(catalog, density, field), raw_error
                )
            memory_error = max(
                0.20,
                raw_memory_error,
                hierarchy_error,
                structural_holdout_error,
            )
            if rows_per_cell > 8:
                memory_error = max(memory_error, 0.35)
            selected_batch, memory, any_batch_admitted = choose_batch(
                workers=TARGET_WORKERS,
                firms=TARGET_FIRMS,
                density=density,
                rows_per_cell=rows_per_cell,
                probes=TARGET_PROBES,
                raw_bytes_per_row=raw_bytes_per_row,
                cmg_hierarchy_bytes=hierarchy,
                memory_error=memory_error,
            )
            batch_factor = finite(
                batch_factors[str(selected_batch)], "selected batch factor"
            )
            time_stages["numerical_p200_seconds"] = (
                numerical_p200_batch16 * batch_factor
            )
            time_stage_errors["numerical_p200_seconds"] = numerical_error
            memory_point = finite(memory["peak_bytes"], "memory point")
            memory_low = memory_point * max(0.0, 1 - memory_error)
            memory_high = memory_point * (1 + memory_error)
            memory_admission = memory_high * (
                1 + MEMORY_ADMISSION_HEADROOM
            )

            for connectivity, work_factor, action_factor, iteration_factor in (
                ("strong", 1.0, 1.0, 1.0),
                (
                    "weak_sensitivity",
                    weak_work_factor,
                    weak_action_factor,
                    weak_iteration_factor,
                ),
            ):
                adjusted_stages = dict(time_stages)
                adjusted_stages["numerical_p200_seconds"] *= work_factor
                adjusted_time = sum(adjusted_stages.values())
                weighted_error = sum(
                    adjusted_stages[field] * time_stage_errors[field]
                    for field in adjusted_stages
                ) / max(adjusted_time, 1e-12)
                adjusted_error = max(
                    0.35 if rows_per_cell > 8 else 0.20,
                    0.30 if work_factor > 1 else 0.20,
                    weighted_error,
                )
                time_low = adjusted_time * max(0.0, 1 - adjusted_error)
                time_high = adjusted_time * (1 + adjusted_error)
                time_admission = time_high * 1.20
                rows.append({
                    "connectivity": connectivity,
                    "workers": TARGET_WORKERS,
                    "firms": TARGET_FIRMS,
                    "cells_per_worker": density,
                    "cells": cells,
                    "rows_per_cell": rows_per_cell,
                    "raw_rows": rows_per_cell * cells,
                    "target_probes": TARGET_PROBES,
                    "target_rhs": TARGET_RHS,
                    "selected_batch": selected_batch,
                    "batch_work_factor": batch_factor,
                    "any_batch_admitted": any_batch_admitted,
                    "solver_actions_point": actions_p200 * action_factor,
                    "solver_max_iterations_point": (
                        max_iterations * iteration_factor
                    ),
                    "import_selection_seconds": adjusted_stages[
                        "import_selection_seconds"
                    ],
                    "compression_transition_seconds": adjusted_stages[
                        "compression_transition_seconds"
                    ],
                    "numerical_p200_seconds": adjusted_stages[
                        "numerical_p200_seconds"
                    ],
                    "restoration_seconds": adjusted_stages["life_restore_seconds"],
                    "wall_point_hours": adjusted_time / 3600,
                    "wall_low_hours": time_low / 3600,
                    "wall_high_hours": time_high / 3600,
                    "wall_admission_hours": time_admission / 3600,
                    "wall_error_fraction": adjusted_error,
                    "direct_peak_point_gib": memory_point / GIB,
                    "direct_peak_low_gib": memory_low / GIB,
                    "direct_peak_high_gib": memory_high / GIB,
                    "memory_admission_gib": memory_admission / GIB,
                    "memory_error_fraction": memory_error,
                    "fits_128_gib": memory_admission <= HARD_MEMORY_BYTES,
                    "fits_24_hours": time_admission <= 24 * 3600,
                    "fits_48_hours": time_admission <= 48 * 3600,
                    "peak_phase": memory["peak_phase"],
                    "selection_peak_gib": finite(
                        memory["selection_peak_bytes"], "selection peak"
                    ) / GIB,
                    "transition_peak_gib": finite(
                        memory["transition_peak_bytes"], "transition peak"
                    ) / GIB,
                    "numerical_peak_gib": finite(
                        memory["numerical_peak_bytes"], "numerical peak"
                    ) / GIB,
                    "restoration_peak_gib": finite(
                        memory["restoration_peak_bytes"], "restoration peak"
                    ) / GIB,
                    "raw_stata_gib": finite(
                        memory["raw_stata_bytes"], "raw Stata"
                    ) / GIB,
                    "persistent_compressed_gib": finite(
                        memory["persistent_bytes"], "persistent compressed"
                    ) / GIB,
                    "cmg_hierarchy_gib": finite(
                        memory["cmg_hierarchy_bytes"], "CMG hierarchy"
                    ) / GIB,
                    "phase_scratch_gib": finite(
                        memory["phase_scratch_bytes"], "phase scratch"
                    ) / GIB,
                })
    return rows


def measurement_rows(measurements: dict[str, Measurement]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for row in sorted(measurements.values(), key=lambda item: item.experiment):
        result.append({
            "experiment": row.experiment,
            "job_id": row.validation["job_id"],
            "connectivity": row.connectivity,
            "workers": row.workers,
            "firms": row.firms,
            "cells_per_worker": row.density,
            "rows_per_cell": row.rows_per_cell,
            "cells": row.cells,
            "rows": row.rows,
            "deletion_units": row.value("deletion_units"),
            "target_strata": row.value("target_strata"),
            "rhs_count": row.value("rhs_count"),
            "selected_batch": row.value("selected_batch"),
            "seed": row.value("seed"),
            "actual_stata_processors": row.value("actual_stata_processors"),
            "command_seconds": row.value("command_seconds"),
            "import_selection_seconds": row.value("import_selection_seconds"),
            "compression_transition_seconds": row.value(
                "compression_transition_seconds"
            ),
            "life_work_seconds": row.value("life_work_seconds"),
            "life_restore_seconds": row.value("life_restore_seconds"),
            "setup_seconds": row.value("setup_seconds"),
            "fit_seconds": row.value("fit_seconds"),
            "leverage_seconds": row.value("leverage_seconds"),
            "target_seconds": row.value("target_seconds"),
            "correction_seconds": row.value("correction_seconds"),
            "schur_seconds": row.value("schur_seconds"),
            "preconditioner_apply_seconds": row.value(
                "preconditioner_apply_seconds"
            ),
            "pcg_seconds": row.value("pcg_seconds"),
            "rng_seconds": row.value("rng_seconds"),
            "solver_iterations": row.value("solver_iterations"),
            "solver_schur_actions": row.value("solver_schur_actions"),
            "solver_precond_applications": row.value(
                "solver_precond_applications"
            ),
            "route_hierarchy_levels": row.value("route_hierarchy_levels"),
            "route_hybrid_vertices": row.value("route_hybrid_vertices"),
            "route_hybrid_edges": row.value("route_hybrid_edges"),
            "route_terminal_vertices": row.value("route_terminal_vertices"),
            "solver_max_residual": row.value("solver_max_residual"),
            "resource_peak_bytes": row.value("resource_peak_bytes"),
            "qacct_wall_seconds": parse_duration(row.accounting["ru_wallclock"]),
            "qacct_cpu_seconds": parse_duration(row.accounting["cpu"]),
            "qacct_maxvmem_bytes": parse_memory(row.accounting["maxvmem"]),
            "qacct_hostname": row.accounting["hostname"],
            "qacct_queue": row.accounting["qname"],
            "source_commit": row.validation["source_commit"],
            "bundle_sha256": row.validation["bundle_sha256"],
            "input_sha256": row.validation["input_sha256"],
            "status": row.validation["status"],
        })
    return result


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    require(rows, f"no rows for {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle, fieldnames=list(rows[0]), lineterminator="\n"
        )
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    measurements = read_measurements(args.evidence_root)
    missing = REQUIRED_EXPERIMENTS - measurements.keys()
    require(not missing, "missing required experiments: " + ", ".join(sorted(missing)))
    catalog = fit_catalog(measurements)
    weak = weak_factors(measurements)
    raw_memory = raw_memory_calibration(measurements)
    batch_calibration = read_batch_calibration(args.evidence_root)
    diagnostics = model_diagnostics(measurements, catalog, raw_memory)
    forecasts = target_forecasts(
        catalog, weak, raw_memory, batch_calibration, diagnostics
    )

    args.output_dir.mkdir(parents=True, exist_ok=True)
    write_csv(args.output_dir / "measurements.csv", measurement_rows(measurements))
    write_csv(args.output_dir / "target_forecasts.csv", forecasts)
    payload = {
        "model_version": MODEL_VERSION,
        "source_commit": EXPECTED_SOURCE_COMMIT,
        "bundle_sha256": EXPECTED_BUNDLE_SHA256,
        "queue_pe_validator_commit": QUEUE_PE_VALIDATOR_COMMIT,
        "evidence_files": {
            str(path.relative_to(args.evidence_root)): sha256(path)
            for path in sorted(args.evidence_root.rglob("*"))
            if path.is_file() and not path.is_relative_to(args.output_dir)
        },
        "target": {
            "workers": TARGET_WORKERS,
            "firms": TARGET_FIRMS,
            "cell_densities": [2, 3, 4],
            "raw_rows_per_cell": [1, 8, 26.3],
            "probes": TARGET_PROBES,
            "rhs": TARGET_RHS,
        },
        "dimension_assumptions": {
            "deletion_units_per_cell": 1.0,
            "target_strata_per_cell": 1.0,
            "basis": (
                "finalized CZ18 observed 311730 deletion units and 311730 "
                "target strata for 311730 cells; synthetic tasks enforce the "
                "same replaceable ratios"
            ),
        },
        "fit_rungs": list(TRAIN_WORKERS),
        "holdout_workers": HOLDOUT_WORKERS,
        "p20_to_p200_rhs_factor": TARGET_RHS / TRAIN_RHS,
        "coefficients": catalog,
        "raw_memory_calibration": raw_memory,
        "batch_calibration": batch_calibration,
        "weak_connectivity": weak,
        "diagnostics": diagnostics,
        "memory_contract": {
            "version": "KSS-RESOURCE-API-8",
            "hard_memory_gib": HARD_MEMORY_BYTES / GIB,
            "batch_candidates": list(BATCH_CANDIDATES),
            "selection_rule": (
                "largest calibrated batch whose model-high peak plus 20% "
                "admission headroom is at most 128 GiB; batch 1 is retained "
                "as the diagnostic fallback when no batch is admitted"
            ),
        },
        "uncertainty_contract": {
            "minimum_model_error_fraction": 0.20,
            "minimum_rpc_26_3_error_fraction": 0.35,
            "admission_headroom_fraction_after_model_high": 0.20,
        },
    }
    (args.output_dir / "model.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"KSS_NUMOPT2_MODEL_PASS {len(measurements)} {len(forecasts)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
