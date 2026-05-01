use std::sync::Arc;

use super::*;

// ========== Constraint Equality Tests (===) ==========

#[tokio::test]
async fn constraint_eq_two_ints() {
    let input = "pub let f() -> Constraint = 5 === 3;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            // Should create: LinExpr(5) == LinExpr(3)
            let constraint = IntLinExpr::constant(5).eq(&IntLinExpr::constant(3));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_eq_var_with_int() {
    let input = "pub let f() -> Constraint = $V() === 42;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = IntLinExpr::var(IlpVar::Base(ExternVar::new("V".into(), vec![])))
                .eq(&IntLinExpr::constant(42));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_eq_two_vars() {
    let input = "pub let f() -> Constraint = $V1() === $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);
            let constraint = IntLinExpr::var(IlpVar::Base(ExternVar::new("V1".into(), vec![]))).eq(
                &IntLinExpr::var(IlpVar::Base(ExternVar::new("V2".into(), vec![]))),
            );
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_eq_with_arithmetic() {
    let input = "pub let f() -> Constraint = 2 * $V() + 3 === 10;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (2_i32
                * IntLinExpr::var(IlpVar::Base(ExternVar::new("V".into(), vec![])))
                + IntLinExpr::constant(3))
            .eq(&IntLinExpr::constant(10));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_eq_with_params() {
    let input = "pub let f(x: Int) -> Constraint = $V(x) === 1;";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Int(5)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = IntLinExpr::var(IlpVar::Base(ExternVar::new(
                "V".into(),
                vec![Arc::new(ExprValue::Int(5))],
            )))
            .eq(&IntLinExpr::constant(1));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Constraint Less Than or Equal Tests (<==) ==========

#[tokio::test]
async fn constraint_le_two_ints() {
    let input = "pub let f() -> Constraint = 5 <== 10;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = IntLinExpr::constant(5).leq(&IntLinExpr::constant(10));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_le_var_with_int() {
    let input = "pub let f() -> Constraint = $V() <== 100;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = IntLinExpr::var(IlpVar::Base(ExternVar::new("V".into(), vec![])))
                .leq(&IntLinExpr::constant(100));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_le_with_arithmetic() {
    let input = "pub let f() -> Constraint = 3 * $V1() + 2 * $V2() <== 50;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (3_i32
                * IntLinExpr::var(IlpVar::Base(ExternVar::new("V1".into(), vec![])))
                + 2_i32 * IntLinExpr::var(IlpVar::Base(ExternVar::new("V2".into(), vec![]))))
            .leq(&IntLinExpr::constant(50));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_le_two_vars() {
    let input = "pub let f() -> Constraint = $V1() <== $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint =
                IntLinExpr::var(IlpVar::Base(ExternVar::new("V1".into(), vec![]))).leq(
                    &IntLinExpr::var(IlpVar::Base(ExternVar::new("V2".into(), vec![]))),
                );
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Constraint Greater Than or Equal Tests (>==) ==========

#[tokio::test]
async fn constraint_ge_two_ints() {
    let input = "pub let f() -> Constraint = 10 >== 5;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = IntLinExpr::constant(10).geq(&IntLinExpr::constant(5));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_ge_var_with_int() {
    let input = "pub let f() -> Constraint = $V() >== 0;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = IntLinExpr::var(IlpVar::Base(ExternVar::new("V".into(), vec![])))
                .geq(&IntLinExpr::constant(0));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_ge_with_arithmetic() {
    let input = "pub let f() -> Constraint = $V1() + $V2() >== 10;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (IntLinExpr::var(IlpVar::Base(ExternVar::new("V1".into(), vec![])))
                + IntLinExpr::var(IlpVar::Base(ExternVar::new("V2".into(), vec![]))))
            .geq(&IntLinExpr::constant(10));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn constraint_ge_two_vars() {
    let input = "pub let f() -> Constraint = $V1() >== $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint =
                IntLinExpr::var(IlpVar::Base(ExternVar::new("V1".into(), vec![]))).geq(
                    &IntLinExpr::var(IlpVar::Base(ExternVar::new("V2".into(), vec![]))),
                );
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Boolean AND with Constraints Tests ==========

#[tokio::test]
async fn and_two_constraints() {
    let input = "pub let f() -> Constraint = $V1() === 1 and $V2() === 2;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should combine both constraints into one list
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn and_constraint_chain() {
    let input = "pub let f() -> Constraint = $V1() === 1 and $V2() === 2 and $V3() === 3;";

    let vars = HashMap::from([
        ("V1".to_string(), vec![]),
        ("V2".to_string(), vec![]),
        ("V3".to_string(), vec![]),
    ]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn and_mixed_constraint_types() {
    let input = "pub let f() -> Constraint = $V1() === 1 and $V2() <== 5 and $V3() >== 0;";

    let vars = HashMap::from([
        ("V1".to_string(), vec![]),
        ("V2".to_string(), vec![]),
        ("V3".to_string(), vec![]),
    ]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);
        }
        _ => panic!("Expected Constraint"),
    }
}
