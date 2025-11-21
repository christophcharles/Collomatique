use super::*;

#[test]
fn simple_number() {
    let input = "let f() -> Int = 42;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn negative_number() {
    let input = "let f() -> Int = -5;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-5));
}

#[test]
fn boolean_true() {
    let input = "let f() -> Bool = true;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn boolean_false() {
    let input = "let f() -> Bool = false;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn boolean_list() {
    let input = "let f() -> [Bool] = [true, false, true];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Bool,
            vec![
                ExprValue::Bool(true),
                ExprValue::Bool(false),
                ExprValue::Bool(true),
            ]
        )
    );
}

#[test]
fn number_list() {
    let input = "let f() -> [Int] = [0, 42, -1];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            vec![ExprValue::Int(0), ExprValue::Int(42), ExprValue::Int(-1),]
        )
    );
}

#[test]
fn cardinality_of_fixed_list() {
    let input = "let f() -> Int = |[0, 42, -1]|;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn cardinality_of_list_in_param() {
    let input = "let f(list: [Int]) -> Int = |list|;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(
                ExprType::Int,
                vec![ExprValue::Int(0), ExprValue::Int(42), ExprValue::Int(-1)],
            )],
        )
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn range() {
    let input = "let f() -> [Int] = [-3..2];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            vec![
                ExprValue::Int(-3),
                ExprValue::Int(-2),
                ExprValue::Int(-1),
                ExprValue::Int(0),
                ExprValue::Int(1),
            ]
        )
    );
}

#[test]
fn empty_range() {
    let input = "let f() -> [Int] = [0..0];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, vec![]));
}

#[test]
fn empty_range_with_end_below_start() {
    let input = "let f() -> [Int] = [3..-2];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, vec![]));
}

#[test]
fn range_with_one_element() {
    let input = "let f() -> [Int] = [4..5];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(ExprType::Int, vec![ExprValue::Int(4),])
    );
}
