// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic qualification-only q1 reference; never used for production RNG.

pub fn reference_normal_cdf(value: f64) -> f64 {
    let absolute = value.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * absolute);
    let polynomial = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let upper =
        (-0.5 * absolute * absolute).exp() * polynomial / (2.0 * core::f64::consts::PI).sqrt();
    if value >= 0.0 {
        1.0 - upper
    } else {
        upper
    }
}

pub fn reference_q1_cdf(distance: f64, curvature: f64) -> f64 {
    if distance <= 0.0 {
        return 0.0;
    }
    if curvature <= 1.0e-10 {
        return 2.0 * reference_normal_cdf(distance) - 1.0;
    }
    // Positive nodes and weights for a 32-point Gauss--Legendre rule.
    // Substituting x=d(1-s^2) removes the square-root endpoint at x=d.
    const NODE: [f64; 16] = [
        0.048_307_665_687_738_32,
        0.144_471_961_582_796_5,
        0.239_287_362_252_137_07,
        0.331_868_602_282_127_67,
        0.421_351_276_130_635_33,
        0.506_899_908_932_229_4,
        0.587_715_757_240_762_3,
        0.663_044_266_930_215_2,
        0.732_182_118_740_289_7,
        0.794_483_795_967_942_4,
        0.849_367_613_732_57,
        0.896_321_155_766_052_1,
        0.934_906_075_937_739_7,
        0.964_762_255_587_506_4,
        0.985_611_511_545_268_4,
        0.997_263_861_849_481_6,
    ];
    const WEIGHT: [f64; 16] = [
        0.096_540_088_514_727_8,
        0.095_638_720_079_274_86,
        0.093_844_399_080_804_57,
        0.091_173_878_695_763_88,
        0.087_652_093_004_403_81,
        0.083_311_924_226_946_76,
        0.078_193_895_787_070_31,
        0.072_345_794_108_848_5,
        0.065_822_222_776_361_85,
        0.058_684_093_478_535_55,
        0.050_998_059_262_376_18,
        0.042_835_898_022_226_68,
        0.034_273_862_913_021_43,
        0.025_392_065_309_262_06,
        0.016_274_394_730_905_67,
        0.007_018_610_009_470_097,
    ];
    let inverse = curvature.recip();
    let transformed = |s: f64| {
        let x = distance * (1.0 - s * s);
        let y_squared = ((distance - x) * (2.0 * inverse + distance + x)).max(0.0);
        let half_normal_density = (2.0 / core::f64::consts::PI).sqrt() * (-0.5 * x * x).exp();
        let half_normal_cdf = (2.0 * reference_normal_cdf(y_squared.sqrt()) - 1.0).clamp(0.0, 1.0);
        2.0 * distance * s * half_normal_density * half_normal_cdf
    };
    let mut integral = 0.0;
    for (&node, &weight) in NODE.iter().zip(&WEIGHT) {
        integral +=
            0.5 * weight * (transformed(0.5 * (1.0 - node)) + transformed(0.5 * (1.0 + node)));
    }
    integral.clamp(0.0, 1.0)
}

pub fn reference_q1_critical(curvature: f64, level: f64) -> f64 {
    assert!(curvature.is_finite() && curvature >= 0.0);
    assert!(level.is_finite() && level > 0.0 && level < 1.0);
    let mut lower = 0.0;
    let mut upper = 2.0;
    while reference_q1_cdf(upper, curvature) < level {
        upper *= 2.0;
        assert!(upper.is_finite());
    }
    for _ in 0..64 {
        let midpoint = 0.5 * (lower + upper);
        if reference_q1_cdf(midpoint, curvature) < level {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    0.5 * (lower + upper)
}
