use super::*;

#[test]
fn sign_default_is_less_than() {
    let sign = Sign::default();

    assert_eq!(sign, Sign::LessThan);
}

#[test]
fn leq_gives_right_sign() {
    let expr1 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B");
    let expr2 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B") + 2.0*Expr::<String>::constant(3.0);

    let constraint = expr1.leq(&expr2);

    assert_eq!(constraint.get_sign(), Sign::LessThan);
}

#[test]
fn leq_gives_right_lhs() {
    let expr1 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B");
    let expr2 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B") + 2.0*Expr::<String>::constant(3.0);

    let expr = &expr1 - &expr2;

    let constraint = expr1.leq(&expr2);

    assert_eq!(*constraint.get_lhs(), expr);
}

#[test]
fn geq_gives_right_sign() {
    let expr1 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B");
    let expr2 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B") + 2.0*Expr::<String>::constant(3.0);

    let constraint = expr1.geq(&expr2);

    assert_eq!(constraint.get_sign(), Sign::LessThan);
}

#[test]
fn geq_gives_right_lhs() {
    let expr1 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B");
    let expr2 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B") + 2.0*Expr::<String>::constant(3.0);

    let expr = &expr2 - &expr1;

    let constraint = expr1.geq(&expr2);

    assert_eq!(*constraint.get_lhs(), expr);
}


#[test]
fn eq_gives_right_sign() {
    let expr1 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B");
    let expr2 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B") + 2.0*Expr::<String>::constant(3.0);

    let constraint = expr1.eq(&expr2);

    assert_eq!(constraint.get_sign(), Sign::Equals);
}

#[test]
fn eq_gives_right_lhs() {
    let expr1 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B");
    let expr2 = 2.0*Expr::<String>::var("A") - 3.0*Expr::<String>::var("B") + 2.0*Expr::<String>::constant(3.0);

    let expr = &expr1 - &expr2;

    let constraint = expr1.eq(&expr2);

    assert_eq!(*constraint.get_lhs(), expr);
}
