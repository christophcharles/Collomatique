use super::*;

#[test]
fn simple_number() {
    let input = "pub let f() -> Int = 42;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn negative_number() {
    let input = "pub let f() -> Int = -5;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-5));
}

#[test]
fn boolean_true() {
    let input = "pub let f() -> Bool = true;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn boolean_false() {
    let input = "pub let f() -> Bool = false;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn boolean_list() {
    let input = "pub let f() -> [Bool] = [true, false, true];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            SimpleType::Bool,
            Vec::from([
                ExprValue::Bool(true),
                ExprValue::Bool(false),
                ExprValue::Bool(true),
            ])
        )
    );
}

#[test]
fn number_list() {
    let input = "pub let f() -> [Int] = [0, 42, -1];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            SimpleType::Int,
            Vec::from([ExprValue::Int(0), ExprValue::Int(42), ExprValue::Int(-1)])
        )
    );
}

#[test]
fn cardinality_of_fixed_list() {
    let input = "pub let f() -> Int = |[0, 42, -1]|;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn cardinality_of_list_in_param() {
    let input = "pub let f(list: [Int]) -> Int = |list|;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(
                SimpleType::Int,
                Vec::from([ExprValue::Int(0), ExprValue::Int(42), ExprValue::Int(-1)]),
            )],
        )
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn range() {
    let input = "pub let f() -> [Int] = [-3..2];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            SimpleType::Int,
            Vec::from([
                ExprValue::Int(-3),
                ExprValue::Int(-2),
                ExprValue::Int(-1),
                ExprValue::Int(0),
                ExprValue::Int(1),
            ])
        )
    );
}

#[test]
fn empty_range() {
    let input = "pub let f() -> [Int] = [0..0];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(SimpleType::Int, Vec::new()));
}

#[test]
fn empty_range_with_end_below_start() {
    let input = "pub let f() -> [Int] = [3..-2];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(SimpleType::Int, Vec::new()));
}

#[test]
fn range_with_one_element() {
    let input = "pub let f() -> [Int] = [4..5];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(SimpleType::Int, Vec::from([ExprValue::Int(4)]))
    );
}
