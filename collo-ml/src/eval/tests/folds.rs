use super::*;

// ========== FOLD with Int Tests ==========

#[test]
fn fold_simple_sum() {
    let input = "pub let f() -> Int = fold x in [1, 2, 3] with acc = 0 { acc + x };";

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
    // 0 + 1 + 2 + 3 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn fold_simple_product() {
    let input = "pub let f() -> Int = fold x in [2, 3, 4] with acc = 1 { acc * x };";

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
    // 1 * 2 * 3 * 4 = 24
    assert_eq!(result, ExprValue::Int(24));
}

#[test]
fn fold_with_range() {
    let input = "pub let f() -> Int = fold x in [1..5] with acc = 0 { acc + x };";

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
    // 0 + 1 + 2 + 3 + 4 = 10
    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn fold_empty_list() {
    let input = "pub let f() -> Int = fold x in [] as [Int] with acc = 42 { acc + x };";

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
    // Empty list, should return initial value
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn fold_ignoring_elements() {
    let input = "pub let f() -> Int = fold x in [1, 2, 3, 4, 5] with acc = 100 { acc };";

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
    // Body just returns acc without using x
    assert_eq!(result, ExprValue::Int(100));
}

#[test]
fn fold_counting_elements() {
    let input = "pub let f() -> Int = fold x in [10, 20, 30, 40] with acc = 0 { acc + 1 };";

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
    // Count elements: 4
    assert_eq!(result, ExprValue::Int(4));
}

#[test]
fn fold_with_arithmetic_in_body() {
    let input = "pub let f() -> Int = fold x in [1..4] with acc = 0 { acc + (x * 2) };";

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
    // 0 + (1*2) + (2*2) + (3*2) = 0 + 2 + 4 + 6 = 12
    assert_eq!(result, ExprValue::Int(12));
}

// ========== FOLD with Where Clause ==========

#[test]
fn fold_with_simple_filter() {
    let input = "pub let f() -> Int = fold x in [1..6] with acc = 0 where x % 2 == 0 { acc + x };";

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
    // Only even numbers: 2 + 4 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn fold_with_filter_no_matches() {
    let input = "pub let f() -> Int = fold x in [1..5] with acc = 100 where x > 10 { acc + x };";

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
    // No elements pass the filter, returns init value
    assert_eq!(result, ExprValue::Int(100));
}

#[test]
fn fold_with_complex_filter() {
    let input =
        "pub let f() -> Int = fold x in [1..10] with acc = 0 where x > 3 and x < 7 { acc + x };";

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
    // Elements 4, 5, 6: 0 + 4 + 5 + 6 = 15
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn fold_filter_using_accumulator() {
    let input = "pub let f() -> Int = fold x in [1..6] with acc = 0 where acc < 5 { acc + x };";

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
    // acc=0, x=1: 0<5   -> acc=1
    // acc=1, x=2: 1<5   -> acc=3
    // acc=3, x=3: 3<5   -> acc=6
    // acc=6, x=4: 6<5 ! -> acc=6
    // acc=6, x=5: 6<5 ! -> acc=6
    assert_eq!(result, ExprValue::Int(6));
}

// ========== FOLD with Parameters ==========

#[test]
fn fold_with_param_list() {
    let input = "pub let f(list: [Int]) -> Int = fold x in list with acc = 0 { acc + x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let list = ExprValue::List(vec![
        ExprValue::Int(10),
        ExprValue::Int(20),
        ExprValue::Int(30),
    ]);

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![list])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(60));
}

#[test]
fn fold_with_param_in_body() {
    let input = "pub let f(multiplier: Int) -> Int = fold x in [1..4] with acc = 0 { acc + (x * multiplier) };";

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
    // (1*5) + (2*5) + (3*5) = 5 + 10 + 15 = 30
    assert_eq!(result, ExprValue::Int(30));
}

#[test]
fn fold_with_param_in_filter() {
    let input = "pub let f(threshold: Int) -> Int = fold x in [1..10] with acc = 0 where x > threshold { acc + x };";

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
    // Elements > 5: 6 + 7 + 8 + 9 = 30
    assert_eq!(result, ExprValue::Int(30));
}

#[test]
fn fold_with_param_as_init() {
    let input = "pub let f(initial: Int) -> Int = fold x in [1..4] with acc = initial { acc + x };";

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
    // 10 + 1 + 2 + 3 = 16
    assert_eq!(result, ExprValue::Int(16));
}

// ========== FOLD Building Lists ==========

#[test]
fn fold_building_list() {
    let input = "pub let f() -> [Int] = fold x in [1, 2, 3] with acc = [] as [Int] { acc + [x] };";

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
        ExprValue::List(elements) => {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0], ExprValue::Int(1));
            assert_eq!(elements[1], ExprValue::Int(2));
            assert_eq!(elements[2], ExprValue::Int(3));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn fold_reverse_list() {
    let input =
        "pub let f() -> [Int] = fold x in [1, 2, 3, 4] with acc = [] as [Int] { [x] + acc };";

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
        ExprValue::List(elements) => {
            assert_eq!(elements.len(), 4);
            assert_eq!(elements[0], ExprValue::Int(4));
            assert_eq!(elements[1], ExprValue::Int(3));
            assert_eq!(elements[2], ExprValue::Int(2));
            assert_eq!(elements[3], ExprValue::Int(1));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn fold_filter_list() {
    let input = "pub let f() -> [Int] = fold x in [1..6] with acc = [] as [Int] where x % 2 == 0 { acc + [x] };";

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
        ExprValue::List(elements) => {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0], ExprValue::Int(2));
            assert_eq!(elements[1], ExprValue::Int(4));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn fold_conditional_list_building() {
    let input = r#"
        pub let f() -> [Int] = 
            fold x in [1..6] with acc = [] as [Int] { 
                if x % 2 == 0 { acc + [x] } else { acc } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(elements) => {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0], ExprValue::Int(2));
            assert_eq!(elements[1], ExprValue::Int(4));
        }
        _ => panic!("Expected List"),
    }
}

// ========== FOLD with Nested Structures ==========

#[test]
fn fold_nested_lists() {
    let input = r#"
        pub let f() -> Int = 
            fold row in [[1, 2], [3, 4], [5]] with acc = 0 { 
                fold x in row with inner = acc { inner + x } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Sum of all elements: 1+2+3+4+5 = 15
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn fold_with_sum_in_body() {
    let input = r#"
        pub let f() -> Int = 
            fold row in [[1, 2], [3, 4]] with acc = 0 { 
                acc + sum x in row { x } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // acc + (1+2) + (3+4) = 0 + 3 + 7 = 10
    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn fold_with_if_in_body() {
    let input = r#"
        pub let f() -> Int = 
            fold x in [1..6] with acc = 0 { 
                if x > 3 { acc + x } else { acc } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Only add x when x > 3: 4 + 5 = 9
    assert_eq!(result, ExprValue::Int(9));
}

#[test]
fn fold_with_let_in_body() {
    let input = r#"
        pub let f() -> Int = 
            fold x in [1..4] with acc = 0 { 
                let double = x * 2 { acc + double } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // (1*2) + (2*2) + (3*2) = 2 + 4 + 6 = 12
    assert_eq!(result, ExprValue::Int(12));
}

// ========== FOLD with LinExpr ==========

#[test]
fn fold_linexpr_simple() {
    let input = "pub let f() -> LinExpr = fold x in [1..3] with acc = $V(0) { acc + $V(x) };";

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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: $V(0) + $V(1) + $V(2)
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::Int(0)],
            ))) + LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::Int(1)],
            ))) + LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::Int(2)],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn fold_linexpr_with_coefficients() {
    let input =
        "pub let f() -> LinExpr = fold x in [1..3] with acc = LinExpr(0) { acc + (x * $V()) };";

    let vars = HashMap::from([("V".to_string(), vec![])]);

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
        ExprValue::LinExpr(lin_expr) => {
            // Should be: 1*$V() + 2*$V() = 3*$V()
            let expected =
                3 * LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn fold_linexpr_empty_list() {
    let input = "pub let f() -> LinExpr = fold x in [<Int>] with acc = LinExpr(5) { acc + $V(x) };";

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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be constant 5
            assert_eq!(lin_expr, LinExpr::constant(5.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== RFOLD Tests ==========

#[test]
fn rfold_simple() {
    let input = "pub let f() -> Int = rfold x in [1, 2, 3] with acc = 0 { acc + x };";

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
    // Same as fold for addition: 0 + 3 + 2 + 1 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn rfold_order_matters() {
    let input = "pub let f() -> Int = rfold x in [48, 2] with acc = 2 { x / acc };";

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

    // rfold processes [2, 48] right-to-left: acc=2, x=2 -> 2//2=1, then acc=1, x=48 -> 48//1=48
    assert_eq!(result, ExprValue::Int(48));
}

#[test]
fn rfold_vs_fold_division() {
    let fold_input = "pub let fold_f() -> Int = fold x in [48, 2] with acc = 2 { x / acc };";
    let rfold_input = "pub let rfold_f() -> Int = rfold x in [48, 2] with acc = 2 { x / acc };";

    let vars = HashMap::new();

    let fold_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: fold_input.to_string(),
        }],
        vars.clone(),
    )
    .expect("Should compile");
    let rfold_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: rfold_input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let fold_result = fold_ast
        .quick_eval_fn("main", "fold_f", vec![])
        .expect("Should evaluate");
    let rfold_result = rfold_ast
        .quick_eval_fn("main", "rfold_f", vec![])
        .expect("Should evaluate");

    // fold processes [48, 2] left-to-right: acc=2, x=48 -> 48//2=24, then acc=24, x=2 -> 2//24=0
    assert_eq!(fold_result, ExprValue::Int(0));
    // rfold processes [48, 2] right-to-left: acc=2, x=2 -> 2//2=1, then acc=1, x=48 -> 48//1=48
    assert_eq!(rfold_result, ExprValue::Int(48));
}

#[test]
fn rfold_list_building() {
    let input =
        "pub let f() -> [Int] = rfold x in [1, 2, 3, 4] with acc = [] as [Int] { acc + [x] };";

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
        ExprValue::List(elements) => {
            assert_eq!(elements.len(), 4);
            // Processes right to left, so builds [4, 3, 2, 1]
            assert_eq!(elements[0], ExprValue::Int(4));
            assert_eq!(elements[1], ExprValue::Int(3));
            assert_eq!(elements[2], ExprValue::Int(2));
            assert_eq!(elements[3], ExprValue::Int(1));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn rfold_with_filter() {
    let input = "pub let f() -> Int = rfold x in [1..6] with acc = 0 where x % 2 == 0 { acc + x };";

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
    // Only even numbers, right to left: 4 + 2 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn rfold_nested() {
    let input = r#"
        pub let f() -> Int = 
            rfold row in [[1, 2], [3, 4]] with acc = 0 { 
                rfold x in row with inner = acc { inner + x } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Processes right to left at both levels: 1+2+3+4 = 10
    assert_eq!(result, ExprValue::Int(10));
}

// ========== Complex Fold Scenarios ==========

#[test]
fn fold_max_value() {
    let input = r#"
        pub let f() -> Int = 
            fold x in [3, 7, 2, 9, 1] with acc = 0 { 
                if x > acc { x } else { acc } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Maximum value is 9
    assert_eq!(result, ExprValue::Int(9));
}

#[test]
fn fold_count_condition() {
    let input = r#"
        pub let f() -> Int = 
            fold x in [1..10] with acc = 0 { 
                if x > 5 { acc + 1 } else { acc } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Count elements > 5: 6, 7, 8, 9 = 4
    assert_eq!(result, ExprValue::Int(4));
}

#[test]
fn fold_alternating_operation() {
    let input = r#"
        pub let f() -> Int = 
            fold x in [1..6] with acc = 0 { 
                if x % 2 == 0 { acc + x } else { acc - x } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // 0 - 1 + 2 - 3 + 4 - 5 = -3
    assert_eq!(result, ExprValue::Int(-3));
}

#[test]
fn fold_with_function_call() {
    let input = r#"
        pub let double(x: Int) -> Int = x * 2;
        pub let f() -> Int = fold x in [1..4] with acc = 0 { acc + double(x) };
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
    // 0 + 2 + 4 + 6 = 12
    assert_eq!(result, ExprValue::Int(12));
}

#[test]
fn fold_flatten_nested_list() {
    let input = r#"
        pub let f() -> [Int] = 
            fold row in [[1, 2], [3], [4, 5, 6]] with acc = [] as [Int] { 
                acc + row 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(elements) => {
            assert_eq!(elements.len(), 6);
            assert_eq!(elements[0], ExprValue::Int(1));
            assert_eq!(elements[5], ExprValue::Int(6));
        }
        _ => panic!("Expected List"),
    }
}

// ========== Fold with Bool ==========

#[test]
fn fold_all_condition() {
    let input = r#"
        pub let f() -> Bool = 
            fold x in [2, 4, 6, 8] with acc = true { 
                acc and (x % 2 == 0) 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // All are even
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn fold_any_condition() {
    let input = r#"
        pub let f() -> Bool = 
            fold x in [1, 3, 5, 6] with acc = false { 
                acc or (x % 2 == 0) 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // At least one is even (6)
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Combined with Other Constructs ==========

#[test]
fn fold_inside_if() {
    let input = r#"
        pub let f(flag: Bool) -> Int = 
            if flag { 
                fold x in [1..4] with acc = 0 { acc + x } 
            } else { 
                0 
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

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(6));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(0));
}

#[test]
fn sum_with_fold_in_body() {
    let input = r#"
        pub let f() -> Int = 
            sum row in [[1, 2], [3, 4], [5]] { 
                fold x in row with acc = 0 { acc + x } 
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
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // sum of (1+2), (3+4), (5) = 3 + 7 + 5 = 15
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn fold_with_forall_in_filter() {
    let input = r#"
        pub let f() -> Int = 
            fold xs in [[2, 4], [1, 3], [6, 8]] with acc = 0 
            where (forall x in xs { x % 2 == 0 }) 
            { acc + |xs| };
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
    // Only [2,4] and [6,8] have all even elements, so count = 2 + 2 = 4
    assert_eq!(result, ExprValue::Int(4));
}

#[test]
fn mixing_fold_and_rfold() {
    let input = r#"
        pub let f() -> Int = 
            (fold x in [1, 2, 3] with a = 0 { a + x }) + 
            (rfold y in [1, 2, 3] with b = 0 { b + y });
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
    // Both sum to 6, so 6 + 6 = 12
    assert_eq!(result, ExprValue::Int(12));
}
