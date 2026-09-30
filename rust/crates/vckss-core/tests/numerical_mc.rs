// SPDX-License-Identifier: GPL-3.0-only
// Complex-step golden values from the independent Python dense expression.
use vckss_core::numerical_mc::{
    conditional_covariance, cross_covariance, finalize, FiniteDerivative, Primitive, Status,
};
#[test]
fn scaled_covariance_policy_and_signed_stage() {
    for scale in [1e-120, 1.0, 1e120] {
        let conditional = [
            [scale, 0.0, 0.0],
            [0.0, 2.0 * scale, 0.0],
            [0.0, 0.0, 3.0 * scale],
        ];
        let signed = [
            [-0.2 * scale, 0.0, 0.0],
            [0.0, 0.4 * scale, 0.0],
            [0.0, 0.0, -0.1 * scale],
        ];
        let cov = finalize(conditional, signed);
        assert_eq!(cov.status, Status::OkLocal);
        let mcse = cov.mcse.unwrap();
        assert!((mcse[3] / scale.sqrt() - 14.8_f64.sqrt()).abs() < 1e-12);
        assert!(mcse[0] < scale.sqrt());
    }
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(
        finalize(
            identity,
            [[-2.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
        )
        .status,
        Status::UnstableNonPsd
    );
    assert_eq!(
        finalize(
            identity,
            [[-1.0 - 1e-13, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
        )
        .status,
        Status::OkLocalPsdAdjusted
    );
    let large = finalize(identity.map(|row| row.map(|x| x * 1e308)), [[0.0; 3]; 3]);
    assert!(large.mcse.unwrap().iter().all(|v| v.is_finite()));
    let draws = [1e154, -1e154, 1e154, -1e154].map(|worker| Primitive {
        worker,
        ..Primitive::default()
    });
    assert!((conditional_covariance(&draws).unwrap()[0][0] / 1e307 - 10.0 / 3.0).abs() < 1e-14);
}

#[test]
fn bilinear_exact_enumeration_odd_folds_and_pure_interaction() {
    for targets in [2_usize, 4, 5] {
        for (mx, my) in [(0.0, 0.0), (0.7, -0.4)] {
            let mut sum = 0.0;
            let mut naive = 0.0;
            let cases = (1 << 3) * (1 << targets);
            for xb in 0..(1 << 3) {
                let x: Vec<f64> = (0..3)
                    .map(|r| mx + if (xb >> r) & 1 == 0 { -0.3 } else { 0.3 })
                    .collect();
                let xm = x.iter().sum::<f64>() / 3.0;
                for yb in 0..(1 << targets) {
                    let y: Vec<f64> = (0..targets)
                        .map(|t| my + if (yb >> t) & 1 == 0 { -0.8 } else { 0.8 })
                        .collect();
                    let ym = y.iter().sum::<f64>() / targets as f64;
                    let folds: [f64; 2] = std::array::from_fn(|parity| {
                        y.iter().skip(parity).step_by(2).sum::<f64>()
                            / y.iter().skip(parity).step_by(2).count() as f64
                    });
                    let scores: [Vec<Primitive>; 2] = folds.map(|mean| {
                        x.iter()
                            .map(|x| Primitive {
                                worker: (x - xm) * mean,
                                ..Primitive::default()
                            })
                            .collect()
                    });
                    let draws: Vec<Primitive> = y
                        .iter()
                        .map(|y| Primitive {
                            worker: xm * y,
                            ..Primitive::default()
                        })
                        .collect();
                    let all_scores: Vec<Primitive> = x
                        .iter()
                        .map(|x| Primitive {
                            worker: (x - xm) * ym,
                            ..Primitive::default()
                        })
                        .collect();
                    let cond = conditional_covariance(&draws).unwrap()[0][0];
                    sum += cond + cross_covariance(&scores[0], &scores[1]).unwrap()[0][0];
                    naive += cond + conditional_covariance(&all_scores).unwrap()[0][0];
                }
            }
            let truth = my * my * 0.09 / 3.0
                + mx * mx * 0.64 / targets as f64
                + 0.09 * 0.64 / (3 * targets) as f64;
            assert!((sum / cases as f64 - truth).abs() < 1e-13);
            assert!(naive / cases as f64 > truth * 1.001);
        }
    }
}

#[test]
fn clipped_adjustment_and_exact_zero_variation() {
    let clipped = FiniteDerivative::new([0.2, 0.8, 0.01, 0.01, 1.0], 32).unwrap();
    assert!(clipped.nonsmooth);
    assert!(clipped.observation(0.0).is_none());
    assert!(clipped.block(1.0).is_none());
    let scores = [Primitive::default(); 3];
    assert_eq!(cross_covariance(&scores, &scores).unwrap(), [[0.0; 3]; 3]);
    assert_eq!(finalize([[0.0; 3]; 3], [[0.0; 3]; 3]).mcse, Some([0.0; 4]));
}
#[test]
#[allow(clippy::excessive_precision)] // Independent binary64 oracle literals.
fn independent_complex_step_derivatives() {
    {
        let u = [
            0.034345271732428746_f64,
            0.5316187128630201_f64,
            0.002597179249604178_f64,
            0.8257651474304468_f64,
            0.020506604509730863_f64,
        ];
        let expected = [
            1.9871782180652244_f64,
            -0.1283818161942682_f64,
            -0.0016143039841723325_f64,
            -0.0025030138775913365_f64,
            0.03884758786503552_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.04508998649165008_f64,
            0.0029130423760427185_f64,
            0.029353607701458978_f64,
            -0.0018963922985410211_f64,
            0.027457215402917956_f64,
        ];
        let expected_dv = [
            0.003075985247360219_f64,
            -0.00019872428605189076_f64,
            0.02757229712291686_f64,
            0.00011508171999890231_f64,
            -0.0035626211570842373_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.023582648943206306_f64,
            0.19921621180934082_f64,
            0.0012441072794023558_f64,
            0.07991341178050078_f64,
            0.0062261648872195844_f64,
        ];
        let expected = [
            5.482265801915762_f64,
            -0.6489750439771337_f64,
            -0.0017935566956364009_f64,
            -0.0050955874559136995_f64,
            0.043257676839269815_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.011740059865950222_f64,
            0.0013897549194234217_f64,
            0.027942273124799754_f64,
            -0.0033077268752002443_f64,
            0.024634546249599508_f64,
        ];
        let expected_dv = [
            0.0006115410551425906_f64,
            -7.239249199050115e-05_f64,
            0.024984660076189012_f64,
            0.0003501138265895031_f64,
            -0.005915226097221482_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.03380530654000659_f64,
            0.5070462270117443_f64,
            0.0030313676456463733_f64,
            0.6828470583529644_f64,
            0.01913510482935192_f64,
        ];
        let expected = [
            2.095527601767355_f64,
            -0.13971103455060216_f64,
            -0.001620995201984189_f64,
            -0.0025937630196397696_f64,
            0.03901195830677232_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.0392255609482153_f64,
            0.0026152095044135212_f64,
            0.029296754490205912_f64,
            -0.0019532455097940916_f64,
            0.02734350898041182_f64,
        ];
        let expected_dv = [
            0.002502078178833361_f64,
            -0.0001668161901549148_f64,
            0.027465594357100807_f64,
            0.00012208537668898498_f64,
            -0.0036623202662102138_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.05851497805781126_f64,
            1.2748851562699488_f64,
            0.008982498876590429_f64,
            5.00300889268547_f64,
            0.10102768896452768_f64,
        ];
        let expected = [
            0.7004213928889998_f64,
            -0.032148105446639264_f64,
            -0.0015544230519198485_f64,
            -0.0017122865756806073_f64,
            0.03737749852006313_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.11683555330277612_f64,
            0.005362545641277016_f64,
            0.02987862390873502_f64,
            -0.0013713760912649792_f64,
            0.028507247817470042_f64,
        ];
        let expected_dv = [
            0.005324248465056149_f64,
            -0.0002443736053987967_f64,
            0.028567429333748225_f64,
            6.01815162781828e-05_f64,
            -0.0026223891499735927_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.04349441541438542_f64,
            0.7198222446768423_f64,
            0.00823496001668613_f64,
            1.242938358792351_f64,
            0.0471487518194308_f64,
        ];
        let expected = [
            1.4457719377807974_f64,
            -0.08735907471785609_f64,
            -0.0016008093881350956_f64,
            -0.0023214487423812453_f64,
            0.038516152346174753_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.05194456176835_f64,
            0.0031386892594366957_f64,
            0.02946934912106189_f64,
            -0.0017806508789381063_f64,
            0.027688698242123784_f64,
        ];
        let expected_dv = [
            0.0016432928590266694_f64,
            -9.92940448653218e-05_f64,
            0.027790161203808995_f64,
            0.00010146296168521761_f64,
            -0.003358375834505777_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.05296142714892435_f64,
            0.6687455103594685_f64,
            0.00679556710784556_f64,
            1.680150765805858_f64,
            0.045846238818983316_f64,
        ];
        let expected = [
            1.5229045080242578_f64,
            -0.12060671048563897_f64,
            -0.0016618915451215765_f64,
            -0.0031587315640854304_f64,
            0.040017010898088075_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.07136362551267386_f64,
            0.005651655816935509_f64,
            0.028956763628852272_f64,
            -0.0022932363711477264_f64,
            0.026663527257704545_f64,
        ];
        let expected_dv = [
            0.00624958138892013_f64,
            -0.000494936779856141_f64,
            0.026831813115431098_f64,
            0.00016828585772655336_f64,
            -0.0042499010268423456_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.03826361674238576_f64,
            1.041677756142046_f64,
            0.0041586116225051225_f64,
            2.5312838144526144_f64,
            0.051707344493504816_f64,
        ];
        let expected = [
            0.9529700856169894_f64,
            -0.03500514617692839_f64,
            -0.0015255467451877475_f64,
            -0.0013448981273436736_f64,
            0.036669159306478745_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.07365443942472191_f64,
            0.0027055250291228433_f64,
            0.03014277505869986_f64,
            -0.0011072249413001374_f64,
            0.029035550117399722_f64,
        ];
        let expected_dv = [
            0.0021007256900419367_f64,
            -7.716528668361756e-05_f64,
            0.02907478042366011_f64,
            3.923030626038697e-05_f64,
            -0.002135989270079501_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.041840086184790105_f64,
            1.1516603960416218_f64,
            0.0035079748313156423_f64,
            3.2574054075193795_f64,
            0.07486523836969826_f64,
        ];
        let expected = [
            0.8388235007120881_f64,
            -0.03047465006546308_f64,
            -0.0015242857633763983_f64,
            -0.0013290636287389464_f64,
            0.03663823597223101_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.08617189209117566_f64,
            0.0031306445929663797_f64,
            0.030154480800178973_f64,
            -0.001095519199821029_f64,
            0.029058961600357945_f64,
        ];
        let expected_dv = [
            0.002081500355602333_f64,
            -7.562138506404399e-05_f64,
            0.029097366794507595_f64,
            3.8405194149648245e-05_f64,
            -0.0021142280113427615_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.04025713755925736_f64,
            0.8461152946527105_f64,
            0.004753697922722501_f64,
            1.522033120307071_f64,
            0.019637950641710526_f64,
        ];
        let expected = [
            1.2154463517989684_f64,
            -0.057829460464193835_f64,
            -0.0015597506214542686_f64,
            -0.0017810649415610231_f64,
            0.03750822595413411_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.05270553643863789_f64,
            0.0025076653784110855_f64,
            0.02983069192699582_f64,
            -0.0014193080730041756_f64,
            0.028411383853991643_f64,
        ];
        let expected_dv = [
            0.0031457527918561127_f64,
            -0.00014967109526267173_f64,
            0.028475845786986673_f64,
            6.446193299503444e-05_f64,
            -0.002709692280018282_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.019696172018063236_f64,
            0.43835988350764704_f64,
            0.0009606223409609589_f64,
            0.5474178489125411_f64,
            0.009337809664030467_f64,
        ];
        let expected = [
            2.435478568687554_f64,
            -0.10942973260083855_f64,
            -0.0015513630401596667_f64,
            -0.001672922059762376_f64,
            0.03730241804965543_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.0370226971426005_f64,
            0.0016634857315372857_f64,
            0.029906266262306996_f64,
            -0.0013437337376930023_f64,
            0.028562532524613996_f64,
        ];
        let expected_dv = [
            0.0018391537751175416_f64,
            -8.263595845616094e-05_f64,
            0.028620312376064053_f64,
            5.777985145006101e-05_f64,
            -0.0025719077724858826_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.055070840461092334_f64,
            0.6584419992176918_f64,
            0.009262584575297408_f64,
            1.7721137036933357_f64,
            0.021711745903801098_f64,
        ];
        let expected = [
            1.5359687231625012_f64,
            -0.12846551193120936_f64,
            -0.0016765373198242553_f64,
            -0.0033653376683798197_f64,
            0.0403771180786313_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.07375291350421932_f64,
            0.006168553855855664_f64,
            0.028838040931143026_f64,
            -0.002411959068856974_f64,
            0.026426081862286052_f64,
        ];
        let expected_dv = [
            0.008881118171291742_f64,
            -0.0007427998859556561_f64,
            0.026612243351880976_f64,
            0.00018616148959492483_f64,
            -0.004451595158524098_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.03824070330140023_f64,
            0.5940470797173165_f64,
            0.0038408425936313764_f64,
            0.77928016191164_f64,
            0.04180704012036737_f64,
        ];
        let expected = [
            1.7846243612156152_f64,
            -0.11488195638327617_f64,
            -0.0016135538888335283_f64,
            -0.0024928705200833214_f64,
            0.038829162937008194_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.04024645963371192_f64,
            0.002590792841733913_f64,
            0.029360003055154103_f64,
            -0.0018899969448458922_f64,
            0.027470006110308212_f64,
        ];
        let expected_dv = [
            0.0006289254709017642_f64,
            -4.048593647306953e-05_f64,
            0.027584312940757062_f64,
            0.00011430683044885779_f64,
            -0.003551380228794068_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.043763351158364903_f64,
            0.977469724955087_f64,
            0.004319242806643373_f64,
            2.751348228799443_f64,
            0.050550456571720095_f64,
        ];
        let expected = [
            1.0038985942027328_f64,
            -0.04494662656431806_f64,
            -0.0015508587053746884_f64,
            -0.0016664440210283893_f64,
            0.0372900440965353_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.08367170573874261_f64,
            0.003746156169115334_f64,
            0.02991082997536307_f64,
            -0.001339170024636934_f64,
            0.028571659950726132_f64,
        ];
        let expected_dv = [
            0.003957090773142926_f64,
            -0.00017716717832722304_f64,
            0.02862904799408249_f64,
            5.738804335635476e-05_f64,
            -0.0025635639625611584_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.05168933970556828_f64,
            0.807264320145609_f64,
            0.005579772507040597_f64,
            2.1365702922701537_f64,
            0.04053027013845289_f64,
        ];
        let expected = [
            1.239724986118699_f64,
            -0.0793799061222129_f64,
            -0.0016124450030297573_f64,
            -0.0024778863260980343_f64,
            0.03880192533630163_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.07601633305851344_f64,
            0.004867345136626861_f64,
            0.029369465646052573_f64,
            -0.0018805343539474233_f64,
            0.02748893129210515_f64,
        ];
        let expected_dv = [
            0.005995682583538738_f64,
            -0.00038390508052107763_f64,
            0.027602096394709192_f64,
            0.0001131651026040465_f64,
            -0.0035347385026867533_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.036455234971772464_f64,
            0.5914981617305967_f64,
            0.0045474611888085765_f64,
            0.8731381526927787_f64,
            0.03225025926932938_f64,
        ];
        let expected = [
            1.7843027616929732_f64,
            -0.10997020894872417_f64,
            -0.0016047022191821927_f64,
            -0.002373625491806254_f64,
            0.038611754322531354_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.044165718577716166_f64,
            0.0027220230807431336_f64,
            0.02943581108271662_f64,
            -0.0018141889172833762_f64,
            0.027621622165433245_f64,
        ];
        let expected_dv = [
            0.0016781528805622137_f64,
            -0.00010342797583759211_f64,
            0.027726943171116243_f64,
            0.00010532100568300252_f64,
            -0.003417735823200747_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
    {
        let u = [
            0.06414909255288581_f64,
            0.4272907340255577_f64,
            0.01596342088680802_f64,
            0.38034670392010633_f64,
            0.04668010534401357_f64,
        ];
        let expected = [
            2.5288580631592845_f64,
            -0.37965707427910994_f64,
            -0.0019044331398517574_f64,
            -0.006861894144898108_f64,
            0.04599230761247944_f64,
        ];
        let d = FiniteDerivative::new(u, 32).unwrap();
        for (a, b) in d.observation(0.04).unwrap().into_iter().zip(expected) {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
        let expected_db = [
            -0.0270730131146902_f64,
            0.004064467318581907_f64,
            0.02717084516992703_f64,
            -0.004079154830072972_f64,
            0.02309169033985406_f64,
        ];
        let expected_dv = [
            0.00014093962267955974_f64,
            -2.1159243998722108e-05_f64,
            0.023624154471940703_f64,
            0.0005324641320866451_f64,
            -0.007093381395972654_f64,
        ];
        for (a, b) in
            d.db.into_iter()
                .zip(expected_db)
                .chain(d.dv.into_iter().zip(expected_dv))
        {
            assert!((a - b).abs() <= 1e-11 * b.abs().max(1.0));
        }
    }
}
