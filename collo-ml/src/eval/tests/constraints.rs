use super::*;

// ========== Constraint Equality Tests (===) ==========

#[test]
fn constraint_eq_two_ints() {
    let input = "pub let f() -> Constraint = 5 === 3;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            // Should create: LinExpr(5) == LinExpr(3)
            let constraint = LinExpr::constant(5.).eq(&LinExpr::constant(3.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_eq_var_with_int() {
    let input = "pub let f() -> Constraint = $V() === 42;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![],
            }))
            .eq(&LinExpr::constant(42.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_eq_two_vars() {
    let input = "pub let f() -> Constraint = $V1() === $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);
            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![],
            }))
            .eq(&LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![],
            })));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_eq_with_arithmetic() {
    let input = "pub let f() -> Constraint = 2 * $V() + 3 === 10;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (2 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![],
            })) + LinExpr::constant(3.))
            .eq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_eq_with_params() {
    let input = "pub let f(x: Int) -> Constraint = $V(x) === 1;";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(5)],
            }))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Constraint Less Than or Equal Tests (<==) ==========

#[test]
fn constraint_le_two_ints() {
    let input = "pub let f() -> Constraint = 5 <== 10;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::constant(5.).leq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_le_var_with_int() {
    let input = "pub let f() -> Constraint = $V() <== 100;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![],
            }))
            .leq(&LinExpr::constant(100.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_le_with_arithmetic() {
    let input = "pub let f() -> Constraint = 3 * $V1() + 2 * $V2() <== 50;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (3 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![],
            })) + 2.
                * LinExpr::var(IlpVar::Base(ExternVar {
                    name: "V2".into(),
                    params: vec![],
                })))
            .leq(&LinExpr::constant(50.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_le_two_vars() {
    let input = "pub let f() -> Constraint = $V1() <== $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![],
            }))
            .leq(&LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![],
            })));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Constraint Greater Than or Equal Tests (>==) ==========

#[test]
fn constraint_ge_two_ints() {
    let input = "pub let f() -> Constraint = 10 >== 5;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::constant(10.).geq(&LinExpr::constant(5.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_ge_var_with_int() {
    let input = "pub let f() -> Constraint = $V() >== 0;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![],
            }))
            .geq(&LinExpr::constant(0.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_ge_with_arithmetic() {
    let input = "pub let f() -> Constraint = $V1() + $V2() >== 10;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![],
            })))
            .geq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn constraint_ge_two_vars() {
    let input = "pub let f() -> Constraint = $V1() >== $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![],
            }))
            .geq(&LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![],
            })));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Boolean AND with Constraints Tests ==========

#[test]
fn and_two_constraints() {
    let input = "pub let f() -> Constraint = $V1() === 1 and $V2() === 2;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should combine both constraints into one list
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn and_constraint_chain() {
    let input = "pub let f() -> Constraint = $V1() === 1 and $V2() === 2 and $V3() === 3;";

    let vars = HashMap::from([
        ("V1".to_string(), vec![]),
        ("V2".to_string(), vec![]),
        ("V3".to_string(), vec![]),
    ]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn and_mixed_constraint_types() {
    let input = "pub let f() -> Constraint = $V1() === 1 and $V2() <== 5 and $V3() >== 0;";

    let vars = HashMap::from([
        ("V1".to_string(), vec![]),
        ("V2".to_string(), vec![]),
        ("V3".to_string(), vec![]),
    ]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);
        }
        _ => panic!("Expected Constraint"),
    }
}
