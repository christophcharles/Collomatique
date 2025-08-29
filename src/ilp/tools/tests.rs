use super::*;
#[test]
fn test_mat_repr() {
    use crate::ilp::linexpr::Expr;

    let pb = crate::ilp::ProblemBuilder::new()
        .add((2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c") - 3).leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))))
        .add((- Expr::var("a") + Expr::var("b") + 3 * Expr::var("c") + 3).leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))))
        .add((2 * Expr::var("c") - 3 * Expr::var("d") + 4 * Expr::var("e") + 2).eq(&(-1 * Expr::var("e") + Expr::var("c"))))
        .build();

    let mat_repr = MatRepr::new(&pb);

    use ndarray::array;

    assert_eq!(mat_repr.leq_mat, array![
        [0,-3,4,5,0],
        [-3,1,3,5,0],
    ]);
    assert_eq!(mat_repr.eq_mat, array![
        [0,0,1,-3,5],
    ]);

    assert_eq!(mat_repr.leq_constants, array![-3, 3]);
    assert_eq!(mat_repr.eq_constants, array![2]);
}
