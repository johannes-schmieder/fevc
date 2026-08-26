use cmg::{CmgError, SddmMatrix, ValidationOptions};

fn assert_vector_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (left_value, right_value) in left.iter().zip(right) {
        let scale = 1.0_f64.max(left_value.abs()).max(right_value.abs());
        assert!((left_value - right_value).abs() <= tolerance * scale);
    }
}

#[test]
fn laplacian_sddm_requires_no_extra_vertex() {
    let matrix = SddmMatrix::from_dense(
        &[vec![2.0, -2.0], vec![-2.0, 2.0]],
        ValidationOptions::default(),
    )
    .unwrap();
    let augmentation = matrix.augment(ValidationOptions::default()).unwrap();
    assert!(!augmentation.is_augmented());
    assert_eq!(augmentation.graph().vertex_count(), 2);
    assert_eq!(augmentation.lift_rhs(&[1.0, -1.0]).unwrap(), [1.0, -1.0]);
}

#[test]
fn strictly_dominant_sddm_matches_upstream_extra_vertex_construction() {
    let matrix = SddmMatrix::from_dense(
        &[vec![3.0, -1.0], vec![-1.0, 2.0]],
        ValidationOptions::default(),
    )
    .unwrap();
    assert_eq!(matrix.matvec(&[2.0, -1.0]).unwrap(), [7.0, -4.0]);

    let augmentation = matrix.augment(ValidationOptions::default()).unwrap();
    assert!(augmentation.is_augmented());
    assert_eq!(augmentation.excess(), &[2.0, 1.0]);
    assert_eq!(
        augmentation.graph().to_dense(),
        vec![
            vec![3.0, -1.0, -2.0],
            vec![-1.0, 2.0, -1.0],
            vec![-2.0, -1.0, 3.0]
        ]
    );
    let lifted = augmentation.lift_rhs(&[4.0, -2.0]).unwrap();
    assert_eq!(lifted, [4.0, -2.0, -2.0]);

    let original_x = [2.0, -1.0];
    let augmented_x = [original_x[0], original_x[1], 0.0];
    let augmented_product = augmentation.graph().matvec(&augmented_x).unwrap();
    let original_product = matrix.matvec(&original_x).unwrap();
    assert_vector_close(&augmented_product[..2], &original_product, 1.0e-15);

    assert_eq!(
        augmentation.extract_solution(&[7.0, 4.0, 5.0]).unwrap(),
        [2.0, -1.0]
    );
}

#[test]
fn arbitrarily_small_positive_excess_is_preserved_exactly() {
    let excess = 1.0e-15;
    let matrix = SddmMatrix::from_dense(
        &[vec![1.0 + excess, -1.0], vec![-1.0, 1.0]],
        ValidationOptions::default(),
    )
    .unwrap();
    let augmentation = matrix.augment(ValidationOptions::default()).unwrap();
    assert!(augmentation.is_augmented());
    assert_eq!(augmentation.graph().vertex_count(), 3);
    assert!(augmentation.excess()[0] > 0.0);
}

#[test]
fn dense_validation_rejects_invalid_sddm_inputs() {
    assert!(matches!(
        SddmMatrix::from_dense(
            &[vec![1.0, -1.0], vec![-2.0, 2.0]],
            ValidationOptions::default()
        ),
        Err(CmgError::NotSymmetric { .. })
    ));
    assert!(matches!(
        SddmMatrix::from_dense(
            &[vec![2.0, 1.0], vec![1.0, 2.0]],
            ValidationOptions::default()
        ),
        Err(CmgError::PositiveOffDiagonal { .. })
    ));
    assert!(matches!(
        SddmMatrix::from_dense(
            &[vec![0.5, -1.0], vec![-1.0, 1.0]],
            ValidationOptions::default()
        ),
        Err(CmgError::NotDiagonallyDominant { row: 0, .. })
    ));
}

#[test]
fn parts_constructor_aggregates_duplicate_off_diagonals() {
    let matrix = SddmMatrix::from_parts(
        vec![3.0, 3.0],
        [(1, 0, -0.25), (0, 1, -0.75)],
        ValidationOptions::default(),
    )
    .unwrap();
    assert_eq!(matrix.to_dense(), vec![vec![3.0, -1.0], vec![-1.0, 3.0]]);
}
