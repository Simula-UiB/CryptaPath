use crate::algebra;

#[test]
fn solving_linear_system_test() {
    let m = matrix![vec![
        vob![true, false, true, false],
        vob![false, true, true, true],
        vob![false, false, true, true],
        vob![false, false, false, true]
    ]];
    let v = vob![true, false, true, true];
    let sol = algebra::solve_linear_system(m, v);
    assert_eq!(sol, vec![Some(true), Some(true), Some(false), Some(true)])
}

#[test]
fn transpose_test() {
    let m = matrix![vec![
        vob![true, false, true, false],
        vob![false, true, true, true],
        vob![false, false, true, true],
        vob![false, false, false, true]
    ]];

    let trans = algebra::transpose(&m);
    let expected_result = matrix![vec![
        vob![true, false, false, false],
        vob![false, true, false, false],
        vob![true, true, true, false],
        vob![false, true, true, true]
    ]];
    assert_eq!(trans, expected_result);
}

#[test]
fn identity_test() {
    let id = algebra::identity(4, 4);
    let expected = matrix![vec![
        vob![true, false, false, false],
        vob![false, true, false, false],
        vob![false, false, true, false],
        vob![false, false, false, true]
    ]];
    assert_eq!(id, expected);
}
