use super::*;

// ========== Simple List Comprehensions ==========

#[test]
fn list_comp_simple_identity() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(3)
        ]))
    );
}

#[test]
fn list_comp_with_arithmetic() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(4),
            ExprValue::Int(6)
        ]))
    );
}

#[test]
fn list_comp_with_addition() {
    let input = "pub let f() -> [Int] = [x + 10 for x in [1, 2, 3]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(11),
            ExprValue::Int(12),
            ExprValue::Int(13)
        ]))
    );
}

#[test]
fn list_comp_with_range() {
    let input = "pub let f() -> [Int] = [x * x for x in [1..5]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Squares: 1, 4, 9, 16
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(4),
            ExprValue::Int(9),
            ExprValue::Int(16),
        ]))
    );
}

#[test]
fn list_comp_with_param() {
    let input = "pub let f(list: [Int]) -> [Int] = [x * 2 for x in list];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(5), ExprValue::Int(10)]));

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([ExprValue::Int(10), ExprValue::Int(20)]))
    );
}

#[test]
fn list_comp_boolean_expression() {
    let input = "pub let f() -> [Bool] = [x > 2 for x in [1, 2, 3, 4]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Bool(false),
            ExprValue::Bool(false),
            ExprValue::Bool(true),
            ExprValue::Bool(true)
        ]))
    );
}

#[test]
fn list_comp_constant_body() {
    let input = "pub let f() -> [Int] = [42 for x in [1, 2, 3]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // All elements are 42
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(42),
            ExprValue::Int(42),
            ExprValue::Int(42)
        ]))
    );
}

// ========== List Comprehensions with Filter ==========

#[test]
fn list_comp_with_simple_filter() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3, 4, 5] where x > 3];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([ExprValue::Int(4), ExprValue::Int(5)]))
    );
}

#[test]
fn list_comp_filter_even_numbers() {
    let input = "pub let f() -> [Int] = [x for x in [1..10] where x % 2 == 0];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(4),
            ExprValue::Int(6),
            ExprValue::Int(8),
        ]))
    );
}

#[test]
fn list_comp_filter_with_transformation() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1..10] where x % 2 == 1];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Odd numbers: 1, 3, 5, 7, 9 -> doubled: 2, 6, 10, 14, 18
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(6),
            ExprValue::Int(10),
            ExprValue::Int(14),
            ExprValue::Int(18),
        ]))
    );
}

#[test]
fn list_comp_filter_no_matches() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3] where x > 10];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn list_comp_filter_all_match() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3] where x > 0];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(3)
        ]))
    );
}

#[test]
fn list_comp_filter_complex_condition() {
    let input = "pub let f() -> [Int] = [x for x in [1..10] where x > 3 and x < 7];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(4),
            ExprValue::Int(5),
            ExprValue::Int(6),
        ]))
    );
}

#[test]
fn list_comp_filter_with_param() {
    let input = "pub let f(threshold: Int) -> [Int] = [x for x in [1..10] where x > threshold];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(6),
            ExprValue::Int(7),
            ExprValue::Int(8),
            ExprValue::Int(9),
        ]))
    );
}

// ========== Nested List Comprehensions (Two Variables) ==========

#[test]
fn list_comp_two_vars_simple() {
    let input = "pub let f() -> [Int] = [x + y for x in [1, 2] for y in [10, 20]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // (1+10), (1+20), (2+10), (2+20) = 11, 21, 12, 22
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(11),
            ExprValue::Int(21),
            ExprValue::Int(12),
            ExprValue::Int(22),
        ]))
    );
}

#[test]
fn list_comp_two_vars_multiplication() {
    let input = "pub let f() -> [Int] = [x * y for x in [2, 3] for y in [4, 5]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // 2*4=8, 2*5=10, 3*4=12, 3*5=15
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(8),
            ExprValue::Int(10),
            ExprValue::Int(12),
            ExprValue::Int(15),
        ]))
    );
}

#[test]
fn list_comp_two_vars_with_ranges() {
    let input = "pub let f() -> [Int] = [x + y for x in [1..3] for y in [10..12]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // (1+10), (1+11), (2+10), (2+11) = 11, 12, 12, 13
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(11),
            ExprValue::Int(12),
            ExprValue::Int(12),
            ExprValue::Int(13)
        ]))
    );
}

#[test]
fn list_comp_two_vars_with_filter() {
    let input = "pub let f() -> [Int] = [x + y for x in [1..5] for y in [1..5] where x + y < 6];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Pairs where x+y < 6: many combinations, results are 2, 3, 4, 5
    // But order matters now! It gives:
    // 2, 3, 4, 5, 3, 4, 5, 4, 5, 5
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(3),
            ExprValue::Int(4),
            ExprValue::Int(5),
            ExprValue::Int(3),
            ExprValue::Int(4),
            ExprValue::Int(5),
            ExprValue::Int(4),
            ExprValue::Int(5),
            ExprValue::Int(5)
        ]))
    );
}

#[test]
fn list_comp_two_vars_filter_on_first() {
    let input = "pub let f() -> [Int] = [x * y for x in [1..5] for y in [1..3] where x > 2];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // x > 2: x=3,4
    // 3*1=3, 3*2=6, 4*1=4, 4*2=8
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(3),
            ExprValue::Int(6),
            ExprValue::Int(4),
            ExprValue::Int(8),
        ]))
    );
}

#[test]
fn list_comp_two_vars_filter_on_second() {
    let input = "pub let f() -> [Int] = [x + y for x in [1..3] for y in [1..5] where y % 2 == 0];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // y even: y=2,4
    // 1+2=3, 1+4=5, 2+2=4, 2+4=6
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(3),
            ExprValue::Int(5),
            ExprValue::Int(4),
            ExprValue::Int(6),
        ]))
    );
}

#[test]
fn list_comp_two_vars_filter_on_both() {
    let input =
        "pub let f() -> [Int] = [x * y for x in [1..5] for y in [1..5] where x > 2 and y < 3];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // x > 2: x=3,4 and y < 3: y=1,2
    // 3*1=3, 3*2=6, 4*1=4, 4*2=8
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(3),
            ExprValue::Int(6),
            ExprValue::Int(4),
            ExprValue::Int(8),
        ]))
    );
}

#[test]
fn list_comp_cartesian_product() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2] for y in [3, 4]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Body only uses x, but iterates over y too
    // Results: 1 (twice), 2 (twice)
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(2)
        ]))
    );
}

#[test]
fn list_comp_with_dependent_limit() {
    let input = "pub let f() -> [Int] = [y for x in [1, 2] for y in [2*x, 2*x+1]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(3),
            ExprValue::Int(4),
            ExprValue::Int(5)
        ]))
    );
}

// ========== List Comprehensions with Collection Operations ==========

#[test]
fn list_comp_over_union() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2] + [3, 4]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(4),
            ExprValue::Int(6),
            ExprValue::Int(8),
        ]))
    );
}

#[test]
fn list_comp_over_difference() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3, 4] - [2, 4]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // [1, 3] * 2 = [2, 6]
    assert_eq!(
        result,
        ExprValue::List(Vec::from([ExprValue::Int(2), ExprValue::Int(6)]))
    );
}

// ========== List Comprehensions with LinExpr ==========

#[test]
fn list_comp_linexpr_simple() {
    let input = "pub let f() -> [LinExpr] = [$V(x) for x in [1, 2, 3]];";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 3);
            assert!(list.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn list_comp_linexpr_with_coefficient() {
    let input = "pub let f() -> [LinExpr] = [x * $V() for x in [1, 2, 3]];";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 3);
            assert!(list.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
            // Could verify each LinExpr is coef * $V()
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn list_comp_linexpr_with_filter() {
    let input = "pub let f() -> [LinExpr] = [$V(x) for x in [1..6] where x % 2 == 0];";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            // Even numbers in [1..6): 2, 4
            assert_eq!(list.len(), 2);
            assert!(list.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

// ========== Complex List Comprehensions ==========

#[test]
fn list_comp_nested_in_sum() {
    let input = "pub let f() -> Int = sum x in [x * 2 for x in [1, 2, 3]] { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // [2, 4, 6], sum = 12
    assert_eq!(result, ExprValue::Int(12));
}

#[test]
fn list_comp_in_if_condition() {
    let input = "pub let f(x: Int) -> Bool = x in [y * 2 for y in [1, 2, 3]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(4)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn list_comp_with_if_expression_body() {
    let input = "pub let f() -> [Int] = [if x > 2 { x * 2 } else { x } for x in [1, 2, 3, 4]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // 1->1, 2->2, 3->6, 4->8
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(6),
            ExprValue::Int(8),
        ]))
    );
}

#[test]
fn list_comp_cardinality() {
    let input = "pub let f() -> Int = |[x * 2 for x in [1..5]]|;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // [2, 4, 6, 8], cardinality = 4
    assert_eq!(result, ExprValue::Int(4));
}

#[test]
fn list_comp_two_vars_one_used() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2] for y in [10, 20, 30]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Creates 6 copies (2*3)
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(1),
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(2),
            ExprValue::Int(2)
        ]))
    );
}

#[test]
fn list_comp_with_multiple_operations() {
    let input =
        "pub let f() -> [Int] = [(x + y) * 2 for x in [1, 2] for y in [3, 4] where x + y < 6];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // x=1,y=3: (1+3)*2=8, x=1,y=4: (1+4)*2=10, x=2,y=3: (2+3)*2=10
    // x=2,y=4 filtered out (2+4=6, not < 6)
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(8),
            ExprValue::Int(10),
            ExprValue::Int(10)
        ]))
    );
}
