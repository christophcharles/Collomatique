use super::*;

// ========== IN Operator Tests ==========

#[test]
fn in_element_present() {
    let input = "pub let f() -> Bool = 5 in [1, 5, 10];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn in_element_absent() {
    let input = "pub let f() -> Bool = 7 in [1, 5, 10];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn in_empty_list() {
    let input = "pub let f() -> Bool = 5 in [];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn in_with_param_element() {
    let input = "pub let f(x: Int) -> Bool = x in [1, 2, 3];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn in_with_param_list() {
    let input = "pub let f(list: [Int]) -> Bool = 5 in list;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let list_with = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(5), ExprValue::Int(10)]),
    );
    let result_true = checked_ast
        .quick_eval_fn("f", vec![list_with])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let list_without = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(10)]),
    );
    let result_false = checked_ast
        .quick_eval_fn("f", vec![list_without])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn in_with_both_params() {
    let input = "pub let f(x: Int, list: [Int]) -> Bool = x in list;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(2), list.clone()])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5), list])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn in_with_range() {
    let input = "pub let f(x: Int) -> Bool = x in [0..10];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn in_bool_list() {
    let input = "pub let f() -> Bool = true in [true, false];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn in_nested_usage() {
    let input = "pub let f(x: Int) -> Bool = (x in [1, 2, 3]) and (x in [2, 3, 4]);";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

// ========== UNION Operator Tests ==========

#[test]
fn union_two_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] union [4, 5, 6];";
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
            BTreeSet::from([
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
                ExprValue::Int(4),
                ExprValue::Int(5),
                ExprValue::Int(6),
            ])
        )
    );
}

#[test]
fn union_overlapping_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] union [2, 3, 4];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Since lists are sets, duplicates are removed
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
                ExprValue::Int(4),
            ])
        )
    );
}

#[test]
fn union_with_empty_list_left() {
    let input = "pub let f() -> [Int] = [] union [1, 2, 3];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn union_with_empty_list_right() {
    let input = "pub let f() -> [Int] = [1, 2, 3] union [];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn union_two_empty_lists() {
    let input = "pub let f() -> [Int] = [] union [];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn union_with_params() {
    let input = "pub let f(list1: [Int], list2: [Int]) -> [Int] = list1 union list2;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let list1 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let list2 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(3), ExprValue::Int(4)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list1, list2])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
                ExprValue::Int(4),
            ])
        )
    );
}

#[test]
fn union_chain() {
    let input = "pub let f() -> [Int] = [1] union [2] union [3];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn union_with_ranges() {
    let input = "pub let f() -> [Int] = [1..3] union [5..7];";
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
            BTreeSet::from([
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(5),
                ExprValue::Int(6),
            ])
        )
    );
}

#[test]
fn union_bool_lists() {
    let input = "pub let f() -> [Bool] = [true] union [false];";
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
            BTreeSet::from([ExprValue::Bool(true), ExprValue::Bool(false)])
        )
    );
}

// ========== INTER (Intersection) Operator Tests ==========

#[test]
fn inter_overlapping_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3, 4] inter [2, 3, 4, 5];";
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
            BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3), ExprValue::Int(4)])
        )
    );
}

#[test]
fn inter_disjoint_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] inter [4, 5, 6];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn inter_identical_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] inter [1, 2, 3];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn inter_with_empty_list_left() {
    let input = "pub let f() -> [Int] = [] inter [1, 2, 3];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn inter_with_empty_list_right() {
    let input = "pub let f() -> [Int] = [1, 2, 3] inter [];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn inter_two_empty_lists() {
    let input = "pub let f() -> [Int] = [] inter [];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn inter_with_params() {
    let input = "pub let f(list1: [Int], list2: [Int]) -> [Int] = list1 inter list2;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let list1 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );
    let list2 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3), ExprValue::Int(4)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list1, list2])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn inter_chain() {
    let input = "pub let f() -> [Int] = [1, 2, 3, 4] inter [2, 3, 4, 5] inter [3, 4, 5, 6];";
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
            BTreeSet::from([ExprValue::Int(3), ExprValue::Int(4)])
        )
    );
}

#[test]
fn inter_with_ranges() {
    let input = "pub let f() -> [Int] = [1..5] inter [3..7];";
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
            BTreeSet::from([ExprValue::Int(3), ExprValue::Int(4)])
        )
    );
}

#[test]
fn inter_single_element_overlap() {
    let input = "pub let f() -> [Int] = [1, 2, 3] inter [3, 4, 5];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(ExprType::Int, BTreeSet::from([ExprValue::Int(3)]))
    );
}

// ========== DIFF (Difference) Operator Tests ==========

#[test]
fn diff_disjoint_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] \\ [4, 5, 6];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn diff_overlapping_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3, 4] \\ [2, 3];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(4)])
        )
    );
}

#[test]
fn diff_identical_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] \\ [1, 2, 3];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn diff_with_empty_list_left() {
    let input = "pub let f() -> [Int] = [] \\ [1, 2, 3];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn diff_with_empty_list_right() {
    let input = "pub let f() -> [Int] = [1, 2, 3] \\ [];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn diff_two_empty_lists() {
    let input = "pub let f() -> [Int] = [] \\ [];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn diff_with_params() {
    let input = "pub let f(list1: [Int], list2: [Int]) -> [Int] = list1 \\ list2;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let list1 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(3),
            ExprValue::Int(4),
        ]),
    );
    let list2 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(4)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list1, list2])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(3)])
        )
    );
}

#[test]
fn diff_partial_overlap() {
    let input = "pub let f() -> [Int] = [1, 2, 3, 4, 5] \\ [3, 4, 5, 6, 7];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)])
        )
    );
}

#[test]
fn diff_with_ranges() {
    let input = "pub let f() -> [Int] = [1..6] \\ [3..5];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(5)])
        )
    );
}

#[test]
fn diff_removing_single_element() {
    let input = "pub let f() -> [Int] = [1, 2, 3] \\ [2];";
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
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(3)])
        )
    );
}

// ========== Combined Operations Tests ==========

#[test]
fn union_then_inter() {
    let input = "pub let f() -> [Int] = ([1, 2] union [3, 4]) inter [2, 3, 5];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // [1, 2, 3, 4] inter [2, 3, 5] = [2, 3]
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn inter_then_diff() {
    let input = "pub let f() -> [Int] = ([1, 2, 3, 4] inter [2, 3, 4, 5]) \\ [3];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // [2, 3, 4] \\ [3] = [2, 4]
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(2), ExprValue::Int(4)])
        )
    );
}

#[test]
fn union_diff_inter_combination() {
    let input = "pub let f() -> [Int] = ([1, 2, 3] union [4, 5]) \\ [2, 4] inter [1, 3, 5];";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // [1, 2, 3, 4, 5] \\ [2, 4] = [1, 3, 5]
    // [1, 3, 5] inter [1, 3, 5] = [1, 3, 5]
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(3), ExprValue::Int(5)])
        )
    );
}

#[test]
fn in_with_union_result() {
    let input = "pub let f(x: Int) -> Bool = x in ([1, 2] union [3, 4]);";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn in_with_inter_result() {
    let input = "pub let f(x: Int) -> Bool = x in ([1, 2, 3] inter [2, 3, 4]);";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn in_with_diff_result() {
    let input = "pub let f(x: Int) -> Bool = x in ([1, 2, 3, 4] \\ [2, 4]);";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn cardinality_of_union() {
    let input = "pub let f() -> Int = |[1, 2] union [2, 3]|;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // union removes duplicates, so [1, 2, 3] has cardinality 3
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn cardinality_of_inter() {
    let input = "pub let f() -> Int = |[1, 2, 3] inter [2, 3, 4]|;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // intersection gives [2, 3], cardinality is 2
    assert_eq!(result, ExprValue::Int(2));
}

#[test]
fn cardinality_of_diff() {
    let input = "pub let f() -> Int = |[1, 2, 3, 4, 5] \\ [2, 4]|;";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // difference gives [1, 3, 5], cardinality is 3
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn collection_operations_with_if() {
    let input =
        "pub let f(x: Int) -> [Int] = if x > 0 { [1, 2] union [3] } else { [4, 5] \\ [5] };";
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::List(ExprType::Int, BTreeSet::from([ExprValue::Int(4)]))
    );
}
