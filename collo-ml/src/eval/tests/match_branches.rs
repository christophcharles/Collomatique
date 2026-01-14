use super::*;

// ========== Basic Match Expression Tests ==========

#[test]
fn match_simple_int() {
    let input = "pub let f(x: Int) -> Int = match x { y as Int { y + 10 } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn match_with_two_branches() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { i } 
            b as Bool { 0 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(42));

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Int(0));
}

#[test]
fn match_with_catchall() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Int = match x { 
            i as Int { i } 
            other { 0 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(42));

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Int(0));

    let result_none = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result_none, ExprValue::Int(0));
}

#[test]
fn match_only_catchall() {
    let input = "pub let f(x: Int) -> Int = match x { y { y * 2 } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(21)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn match_binding_uses_refined_type() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { i * 2 } 
            b as Bool { 1 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(20));
}

#[test]
fn match_multiple_int_branches() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Int = match x { 
            i as Int { 1 } 
            b as Bool { 2 } 
            n as None { 3 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(1));

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Int(2));

    let result_none = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result_none, ExprValue::Int(3));
}

// ========== Type Conversion in Match Tests ==========
// Note: The `into` keyword was removed from match branches.
// Type conversions should be done explicitly in the body using C-like syntax: LinExpr(x)

#[test]
fn match_int_to_linexpr_conversion() {
    // Conversion is now done in the body using LinExpr(i)
    let input = r#"
        pub let f(x: Int) -> LinExpr = match x {
            i as Int { $V(LinExpr(i)) }
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::LinExpr.into()])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // After conversion, i is LinExpr(5.0), so $V(LinExpr(5.0))
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::LinExpr(LinExpr::constant(5.))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn match_int_branch_with_conversion_in_body() {
    // The `into` in match branches was removed - conversion is done in the body
    let input = r#"
        pub let f(x: Int | Bool) -> LinExpr | Bool = match x {
            b as Bool { b }
            i as Int { $V(LinExpr(i)) }
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::LinExpr.into()])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Bool(true));

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_int {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::LinExpr(LinExpr::constant(5.))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn match_emptylist_to_list_conversion() {
    // Note: The `into` in match branches was removed
    // Using `as []` to match empty list, conversion in body
    let input = r#"
        pub let f(x: [] | Int) -> [Int] | Int = match x {
            i as Int { i }
            lst as [] { [1, 2, 3] }
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(42));

    let result_list = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::List(vec![])])
        .expect("Should evaluate");
    assert_eq!(
        result_list,
        ExprValue::List(vec![
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(3)
        ])
    );
}

// ========== Where Filter Tests ==========

#[test]
fn match_with_where_filter() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            i as Int where i > 0 { i * 2 } 
            j as Int { 0 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_positive = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_positive, ExprValue::Int(10));

    let result_negative = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_negative, ExprValue::Int(0));

    let result_zero = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result_zero, ExprValue::Int(0));
}

#[test]
fn match_multiple_filtered_branches() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            i as Int where i > 10 { 100 } 
            j as Int where j > 0 { 10 } 
            k as Int { 0 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_large = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_large, ExprValue::Int(100));

    let result_medium = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_medium, ExprValue::Int(10));

    let result_negative = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_negative, ExprValue::Int(0));
}

#[test]
fn match_where_with_original_variable() {
    let input = r#"
        pub let f(x: Int) -> LinExpr = match x {
            i as Int where x > 0 { $V(LinExpr(i)) }
            j as Int { $V2() }
        };
    "#;

    let vars = HashMap::from([
        ("V".to_string(), vec![SimpleType::LinExpr.into()]),
        ("V2".to_string(), vec![]),
    ]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_positive = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_positive {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::LinExpr(LinExpr::constant(5.))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_negative = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");

    match result_negative {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V2".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== List Matching Tests ==========

#[test]
fn match_list_vs_int() {
    let input = r#"
        pub let f(x: [Int] | Int) -> Int = match x { 
            lst as [Int] { |lst| } 
            i as Int { i } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_list = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
            ])],
        )
        .expect("Should evaluate");
    assert_eq!(result_list, ExprValue::Int(3));

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(42));
}

#[test]
fn match_emptylist_separately() {
    let input = r#"
        pub let f(x: [Int]) -> Int = match x { 
            empty as [] { 0 } 
            lst as [Int] { |lst| } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_empty = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::List(vec![])])
        .expect("Should evaluate");
    assert_eq!(result_empty, ExprValue::Int(0));

    let result_nonempty = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![ExprValue::Int(1), ExprValue::Int(2)])],
        )
        .expect("Should evaluate");
    assert_eq!(result_nonempty, ExprValue::Int(2));
}

#[test]
fn match_list_with_filter() {
    let input = r#"
        pub let f(items: [Int] | Int) -> Int = match items { 
            lst as [Int] where |lst| > 0 { |lst| } 
            empty as [Int] { 0 } 
            i as Int { i } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_nonempty = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![ExprValue::Int(1), ExprValue::Int(2)])],
        )
        .expect("Should evaluate");
    assert_eq!(result_nonempty, ExprValue::Int(2));

    let result_empty = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::List(vec![])])
        .expect("Should evaluate");
    assert_eq!(result_empty, ExprValue::Int(0));

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(42));
}

// ========== Nested Match Tests ==========

#[test]
fn match_nested() {
    let input = r#"
        pub let f(x: Int | Bool, y: Int | Bool) -> Int = match x { 
            i as Int { 
                match y { 
                    j as Int { i + j } 
                    b as Bool { i } 
                } 
            } 
            b as Bool { 0 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5), ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result_int_int, ExprValue::Int(8));

    let result_int_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5), ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_int_bool, ExprValue::Int(5));

    let result_bool_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true), ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result_bool_int, ExprValue::Int(0));
}

#[test]
fn match_in_branch_body() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { i } 
            b as Bool { match 10 { y as Int { y } } } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(10));
}

// ========== Match with Other Expressions ==========

#[test]
fn match_with_if_in_branch() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { if i > 0 { i } else { 0 } } 
            b as Bool { 1 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_positive = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_positive, ExprValue::Int(5));

    let result_negative = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_negative, ExprValue::Int(0));

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Int(1));
}

#[test]
fn match_with_sum_in_branch() {
    let input = r#"
        pub let f(x: [Int] | Int) -> Int = match x { 
            lst as [Int] { sum i in lst { i } } 
            i as Int { i } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_list = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
            ])],
        )
        .expect("Should evaluate");
    assert_eq!(result_list, ExprValue::Int(6));

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(10));
}

#[test]
fn match_with_list_comprehension_in_branch() {
    let input = r#"
        pub let f(x: [Int] | Int) -> [Int] = match x { 
            lst as [Int] { [i * 2 for i in lst] } 
            i as Int { [i] } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_list = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
            ])],
        )
        .expect("Should evaluate");
    assert_eq!(
        result_list,
        ExprValue::List(vec![
            ExprValue::Int(2),
            ExprValue::Int(4),
            ExprValue::Int(6)
        ])
    );

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::List(vec![ExprValue::Int(10)]));
}

// ========== Match with LinExpr and Constraints ==========

#[test]
fn match_returning_linexpr() {
    let input = r#"
        pub let f(x: Int | Bool) -> LinExpr = match x {
            i as Int { $V(LinExpr(i)) }
            b as Bool { $V2() }
        };
    "#;

    let vars = HashMap::from([
        ("V".to_string(), vec![SimpleType::LinExpr.into()]),
        ("V2".to_string(), vec![]),
    ]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_int {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::LinExpr(LinExpr::constant(5.))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");

    match result_bool {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V2".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn match_returning_constraint() {
    let input = r#"
        pub let f(x: Int | Bool) -> Constraint = match x {
            i as Int { $V(LinExpr(i)) === 0 }
            b as Bool { if b { 0 === 0 } else { 1 === 0 } }
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::LinExpr.into()])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_int {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);
            let var_expr = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::LinExpr(LinExpr::constant(5.))],
            )));
            let expected = var_expr.eq(&LinExpr::constant(0.));
            assert!(constraints.contains(&expected));
        }
        _ => panic!("Expected Constraint"),
    }

    let result_bool_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");

    match result_bool_true {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);
            let expected = LinExpr::constant(0.).eq(&LinExpr::constant(0.));
            assert!(constraints.contains(&expected));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Complex Real-World Examples ==========

#[test]
fn match_complex_type_dispatch() {
    let input = r#"
        pub let f(value: Int | Bool | [Int]) -> Constraint = match value {
            i as Int { $V(LinExpr(i)) === 0 }
            b as Bool { if b { 0 === 0 } else { 1 === 0 } }
            lst as [Int] { sum x in lst { $V(LinExpr(x)) } === 10 }
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::LinExpr.into()])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_int {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    let result_list = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Int(2),
                ExprValue::Int(3),
                ExprValue::Int(5),
            ])],
        )
        .expect("Should evaluate");

    match result_list {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn match_in_arithmetic() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = 
            (match x { i as Int { i } b as Bool { 0 } }) + 5;
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(15));

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Int(5));
}

#[test]
fn match_optional_handling() {
    let input = r#"
        pub let f(x: Int | None) -> Int = match x { 
            i as Int { i } 
            n as None { 0 } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result_int, ExprValue::Int(42));

    let result_none = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result_none, ExprValue::Int(0));
}

#[test]
fn match_returning_list() {
    let input = r#"
        pub let f(x: Int | Bool) -> [Int] = match x { 
            i as Int { [i, i * 2, i * 3] } 
            b as Bool { [] } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_int = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(
        result_int,
        ExprValue::List(vec![
            ExprValue::Int(5),
            ExprValue::Int(10),
            ExprValue::Int(15)
        ])
    );

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::List(vec![]));
}

#[test]
fn match_with_boolean_result() {
    let input = r#"
        pub let f(x: Int | Bool) -> Bool = match x { 
            i as Int { i > 0 } 
            b as Bool { b } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result_positive = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_positive, ExprValue::Bool(true));

    let result_negative = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_negative, ExprValue::Bool(false));

    let result_bool = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_bool, ExprValue::Bool(true));
}
