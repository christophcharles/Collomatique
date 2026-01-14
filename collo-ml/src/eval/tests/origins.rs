use super::*;
use std::collections::HashMap;

/// Test that a simple constraint gets an origin when returned from a function
#[test]
fn simple_constraint_gets_origin() {
    let input = r#"
    pub let make_constraint(x: Int) -> Constraint = $V(x) === 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_constraint", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraint_with_origin = constraints.iter().next().unwrap();

            // Verify origin exists
            assert!(constraint_with_origin.origin.is_some());

            let origin = constraint_with_origin.origin.as_ref().unwrap();

            // Verify function name
            assert_eq!(origin.fn_name.node, "make_constraint");

            // Verify arguments
            assert_eq!(origin.args.len(), 1);
            assert_eq!(origin.args[0], ExprValue::Int(42));
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that nested function calls preserve the INNER function's origin
/// The origin should track the innermost function that creates the constraint
#[test]
fn nested_function_origin_is_inner() {
    let input = r#"
    let inner(x: Int) -> Constraint = $V(x) === 0;
    pub let outer(y: Int) -> Constraint = inner(y + 1);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "outer", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraint_with_origin = constraints.iter().next().unwrap();

            assert!(constraint_with_origin.origin.is_some());
            let origin = constraint_with_origin.origin.as_ref().unwrap();

            // The origin should be from "inner", not "outer"
            // This is because "inner" is the function that directly creates the constraint
            assert_eq!(origin.fn_name.node, "inner");

            // The args should be from the inner function call: y + 1 = 11
            assert_eq!(origin.args.len(), 1);
            assert_eq!(origin.args[0], ExprValue::Int(11));
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that multiple constraints from the same function have the same origin
#[test]
fn multiple_constraints_same_origin() {
    let input = r#"
    pub let make_two(x: Int) -> Constraint = 
        ($V(x) === 1) and ($V(x + 1) === 2);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_two", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);

            // Both constraints should have origins
            for constraint_with_origin in &constraints {
                assert!(constraint_with_origin.origin.is_some());
                let origin = constraint_with_origin.origin.as_ref().unwrap();
                assert_eq!(origin.fn_name.node, "make_two");
                assert_eq!(origin.args.len(), 1);
                assert_eq!(origin.args[0], ExprValue::Int(5));
            }
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test origin with multiple parameters
#[test]
fn origin_with_multiple_params() {
    let input = r#"
    pub let complex(x: Int, y: Int, z: Int) -> Constraint = 
        $V(x) === y + z;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "complex",
            vec![ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)],
        )
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraint_with_origin = constraints.iter().next().unwrap();

            assert!(constraint_with_origin.origin.is_some());
            let origin = constraint_with_origin.origin.as_ref().unwrap();

            assert_eq!(origin.fn_name.node, "complex");
            assert_eq!(origin.args.len(), 3);
            assert_eq!(origin.args[0], ExprValue::Int(1));
            assert_eq!(origin.args[1], ExprValue::Int(2));
            assert_eq!(origin.args[2], ExprValue::Int(3));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn reified_constraint_origin() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 0;
    reify base as $BaseVar;
    pub let use_reified(y: Int) -> Constraint = $BaseVar(y) <== 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "use_reified", vec![ExprValue::Int(7)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);

            let has_use_reified_origin = constraints.iter().any(|c| {
                if let Some(origin) = &c.origin {
                    origin.fn_name.node == "use_reified"
                        && origin.args.len() == 1
                        && origin.args[0] == ExprValue::Int(7)
                } else {
                    false
                }
            });

            assert!(
                has_use_reified_origin,
                "Should have constraint with use_reified origin and that's all"
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test origin tracking with forall expressions
#[test]
fn forall_constraint_origin() {
    let input = r#"
    pub let forall_constraints(n: Int) -> Constraint = 
        forall i in [0..n] { $V(i) === 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "forall_constraints", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should have 3 constraints (for i = 0, 1, 2)
            assert_eq!(constraints.len(), 3);

            // All should have the same origin
            for constraint_with_origin in &constraints {
                assert!(constraint_with_origin.origin.is_some());
                let origin = constraint_with_origin.origin.as_ref().unwrap();
                assert_eq!(origin.fn_name.node, "forall_constraints");
                assert_eq!(origin.args.len(), 1);
                assert_eq!(origin.args[0], ExprValue::Int(3));
            }
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that combining constraints from multiple sources preserves each origin
#[test]
fn combined_constraints_preserve_separate_origins() {
    let input = r#"
    let c1(x: Int) -> Constraint = $V(x) === 0;
    let c2(x: Int) -> Constraint = $V(x) === 1;
    pub let combined(x: Int) -> Constraint = c1(x) and c2(x);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "combined", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);

            // Should have constraints from c1
            let has_c1_origin = constraints.iter().any(|c| {
                if let Some(origin) = &c.origin {
                    origin.fn_name.node == "c1"
                        && origin.args.len() == 1
                        && origin.args[0] == ExprValue::Int(5)
                } else {
                    false
                }
            });

            // Should have constraints from c2
            let has_c2_origin = constraints.iter().any(|c| {
                if let Some(origin) = &c.origin {
                    origin.fn_name.node == "c2"
                        && origin.args.len() == 1
                        && origin.args[0] == ExprValue::Int(5)
                } else {
                    false
                }
            });

            assert!(has_c1_origin, "Should have constraint with c1 origin");
            assert!(has_c2_origin, "Should have constraint with c2 origin");
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test origin with list parameters - using Vec for List
#[test]
fn origin_with_list_param() {
    let input = r#"
    pub let list_constraint(items: [Int]) -> Constraint = 
        forall x in items { $V(x) === 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let mut list_items = Vec::new();
    list_items.push(ExprValue::Int(1));
    list_items.push(ExprValue::Int(2));
    list_items.push(ExprValue::Int(3));

    let list_arg = ExprValue::List(list_items.clone());

    let result = checked_ast
        .quick_eval_fn("main", "list_constraint", vec![list_arg.clone()])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);

            for constraint_with_origin in &constraints {
                assert!(constraint_with_origin.origin.is_some());
                let origin = constraint_with_origin.origin.as_ref().unwrap();
                assert_eq!(origin.fn_name.node, "list_constraint");
                assert_eq!(origin.args.len(), 1);
                assert_eq!(origin.args[0], list_arg);
            }
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that inner function origin is preserved, not the wrapper's
#[test]
fn inner_function_origin_preserved() {
    let input = r#"
    let helper(x: Int) -> Constraint = $V(x) === 0;
    pub let wrapper(y: Int) -> Constraint = helper(y * 2);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "wrapper", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraint_with_origin = constraints.iter().next().unwrap();

            assert!(constraint_with_origin.origin.is_some());
            let origin = constraint_with_origin.origin.as_ref().unwrap();

            // Origin should be "helper", NOT "wrapper"
            // because "helper" is the function that directly creates the constraint
            assert_eq!(origin.fn_name.node, "helper");
            assert_ne!(origin.fn_name.node, "wrapper");

            // Args should be from helper call: y * 2 = 6
            assert_eq!(origin.args.len(), 1);
            assert_eq!(origin.args[0], ExprValue::Int(6));
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test deeply nested function calls - origin should be the deepest function
#[test]
fn deeply_nested_function_origin() {
    let input = r#"
    let innermost(x: Int) -> Constraint = $V(x) === 42;
    let middle(x: Int) -> Constraint = innermost(x + 10);
    pub let outer(x: Int) -> Constraint = middle(x + 5);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "outer", vec![ExprValue::Int(1)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraint_with_origin = constraints.iter().next().unwrap();

            assert!(constraint_with_origin.origin.is_some());
            let origin = constraint_with_origin.origin.as_ref().unwrap();

            // Origin should be "innermost", the deepest function that creates the constraint
            assert_eq!(origin.fn_name.node, "innermost");

            // Args should be from innermost call: (1 + 5) + 10 = 16
            assert_eq!(origin.args.len(), 1);
            assert_eq!(origin.args[0], ExprValue::Int(16));
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that docstrings are correctly substituted with actual argument values
#[test]
fn docstring_substitution_with_args() {
    let input = r#"
    /// `x` must be smaller than 1.
    let h(x: Int) -> Constraint = x <== 1;
    pub let f() -> Constraint = h(1) and h(2);
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);

            let mut constraints_vec: Vec<_> = constraints.iter().collect();
            constraints_vec.sort_by_key(|c| &c.origin.as_ref().unwrap().args[0]);

            // First constraint: h(1)
            let constraint1 = &constraints_vec[0];
            assert!(constraint1.origin.is_some());
            let origin1 = constraint1.origin.as_ref().unwrap();
            assert_eq!(origin1.fn_name.node, "h");
            assert_eq!(origin1.args.len(), 1);
            assert_eq!(origin1.args[0], ExprValue::Int(1));
            assert_eq!(origin1.pretty_docstring.len(), 1);
            assert_eq!(origin1.pretty_docstring[0], "1 must be smaller than 1.");

            // Second constraint: h(2)
            let constraint2 = &constraints_vec[1];
            assert!(constraint2.origin.is_some());
            let origin2 = constraint2.origin.as_ref().unwrap();
            assert_eq!(origin2.fn_name.node, "h");
            assert_eq!(origin2.args.len(), 1);
            assert_eq!(origin2.args[0], ExprValue::Int(2));
            assert_eq!(origin2.pretty_docstring.len(), 1);
            assert_eq!(origin2.pretty_docstring[0], "2 must be smaller than 1.");
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test docstring substitution with multiple parameters and multi-line docstrings
#[test]
fn multiline_docstring_multiple_params() {
    let input = r#"
    /// Constraint on `x` and `y`:
    /// - `x` must be less than `y`
    /// - Their sum must be positive
    let range_check(x: Int, y: Int) -> Constraint =
        (x <== y) and ((x + y) >== 0);
    pub let test() -> Constraint = range_check(5, 10);
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
        .quick_eval_fn("main", "test", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Get any constraint to check the origin
            let constraint = constraints.iter().next().unwrap();
            assert!(constraint.origin.is_some());
            let origin = constraint.origin.as_ref().unwrap();

            assert_eq!(origin.fn_name.node, "range_check");
            assert_eq!(origin.args.len(), 2);
            assert_eq!(origin.args[0], ExprValue::Int(5));
            assert_eq!(origin.args[1], ExprValue::Int(10));

            assert_eq!(origin.pretty_docstring.len(), 3);
            assert_eq!(origin.pretty_docstring[0], "Constraint on 5 and 10:");
            assert_eq!(origin.pretty_docstring[1], "- 5 must be less than 10");
            assert_eq!(origin.pretty_docstring[2], "- Their sum must be positive");
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that the same parameter can be substituted multiple times in one line
#[test]
fn repeated_parameter_substitution() {
    let input = r#"
    /// The value `val` is compared to itself: `val` === `val`
    let self_compare(val: Int) -> Constraint = val === val;
    pub let test() -> Constraint = self_compare(42);
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
        .quick_eval_fn("main", "test", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            let constraint = constraints.iter().next().unwrap();
            let origin = constraint.origin.as_ref().unwrap();

            assert_eq!(origin.pretty_docstring.len(), 1);
            assert_eq!(
                origin.pretty_docstring[0],
                "The value 42 is compared to itself: 42 === 42"
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test that arbitrary expressions work in docstrings (not just parameter names)
#[test]
fn docstring_expression_evaluation() {
    let input = r#"
    /// Index `i + 1` of `total` items.
    let describe_index(i: Int, total: Int) -> Constraint = i <== total;
    pub let test() -> Constraint = describe_index(5, 10);
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
        .quick_eval_fn("main", "test", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            let constraint = constraints.iter().next().unwrap();
            assert!(constraint.origin.is_some());
            let origin = constraint.origin.as_ref().unwrap();

            assert_eq!(origin.fn_name.node, "describe_index");
            assert_eq!(origin.pretty_docstring.len(), 1);
            // i=5, so i+1=6; total=10
            assert_eq!(origin.pretty_docstring[0], "Index 6 of 10 items.");
        }
        _ => panic!("Expected Constraint"),
    }
}

/// Test double backticks for expressions containing single backticks
#[test]
fn docstring_double_backticks() {
    let input = r#"
    /// Value is ``x``.
    let show(x: Int) -> Constraint = x >== 0;
    pub let test() -> Constraint = show(42);
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
        .quick_eval_fn("main", "test", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            let constraint = constraints.iter().next().unwrap();
            assert!(constraint.origin.is_some());
            let origin = constraint.origin.as_ref().unwrap();

            assert_eq!(origin.pretty_docstring.len(), 1);
            assert_eq!(origin.pretty_docstring[0], "Value is 42.");
        }
        _ => panic!("Expected Constraint"),
    }
}
