use super::*;
use std::sync::Arc;

// =============================================================================
// CUSTOM TYPE BASIC OPERATIONS
// =============================================================================

#[tokio::test]
async fn custom_type_wrap_and_unwrap() {
    let input = r#"
        type MyInt = Int;
        pub let wrap(x: Int) -> MyInt = MyInt(x);
        pub let unwrap(x: MyInt) -> Int = Int(x);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    // Test wrapping
    let wrapped = checked_ast
        .eval_fn("main", "wrap", vec![ExprValue::Int(42)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        wrapped,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(42)),
        })
    );

    // Test unwrapping
    let unwrapped = checked_ast
        .eval_fn("main", "unwrap", vec![wrapped])
        .await
        .expect("Should evaluate");

    assert_eq!(unwrapped, ExprValue::Int(42));
}

#[tokio::test]
async fn custom_type_roundtrip() {
    let input = r#"
        type MyInt = Int;
        pub let roundtrip(x: Int) -> Int = Int(MyInt(x));
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "roundtrip", vec![ExprValue::Int(123)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(123));
}

#[tokio::test]
async fn custom_type_with_tuple() {
    let input = r#"
        type Point = (Int, Int);
        pub let make_point(x: Int, y: Int) -> Point = Point(x, y);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "make_point",
            vec![ExprValue::Int(3), ExprValue::Int(4)],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Point".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(3)),
                Arc::new(ExprValue::Int(4))
            ])),
        })
    );
}

#[tokio::test]
async fn custom_type_with_list() {
    let input = r#"
        type IntList = [Int];
        pub let make_list() -> IntList = IntList([1, 2, 3]);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "make_list", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "IntList".to_string(),
            variant: None,
            content: Arc::new(ExprValue::List(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Int(3))
            ])),
        })
    );
}

// =============================================================================
// FIELD ACCESS THROUGH CUSTOM TYPES
// =============================================================================

#[tokio::test]
async fn custom_type_tuple_field_access() {
    let input = r#"
        type Point = (Int, Int);
        pub let get_x(p: Point) -> Int = p.0;
        pub let get_y(p: Point) -> Int = p.1;
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let point = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "Point".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(10)),
            Arc::new(ExprValue::Int(20)),
        ])),
    });

    let x = checked_ast
        .eval_fn("main", "get_x", vec![point.clone()])
        .await
        .expect("Should evaluate");
    assert_eq!(x, ExprValue::Int(10));

    let y = checked_ast
        .eval_fn("main", "get_y", vec![point])
        .await
        .expect("Should evaluate");
    assert_eq!(y, ExprValue::Int(20));
}

#[tokio::test]
async fn custom_type_nested_tuple_field_access() {
    let input = r#"
        type Point = (Int, Int);
        type NamedPoint = (String, Point);
        pub let get_x(np: NamedPoint) -> Int = np.1.0;
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let named_point = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "NamedPoint".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::String("origin".to_string())),
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "Point".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(0)),
                    Arc::new(ExprValue::Int(0)),
                ])),
            })),
        ])),
    });

    let x = checked_ast
        .eval_fn("main", "get_x", vec![named_point])
        .await
        .expect("Should evaluate");
    assert_eq!(x, ExprValue::Int(0));
}

// =============================================================================
// CUSTOM TYPES IN COLLECTIONS
// =============================================================================

#[tokio::test]
async fn custom_type_in_list() {
    let input = r#"
        type MyInt = Int;
        pub let make_list() -> [MyInt] = [MyInt(1), MyInt(2), MyInt(3)];
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "make_list", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Int(1)),
            })),
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Int(2)),
            })),
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Int(3)),
            })),
        ])
    );
}

#[tokio::test]
async fn sum_over_custom_type_list() {
    let input = r#"
        type MyInt = Int;
        pub let total(xs: [MyInt]) -> Int = sum x in xs { Int(x) };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let list = ExprValue::List(vec![
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(1)),
        })),
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(2)),
        })),
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(3)),
        })),
    ]);

    let result = checked_ast
        .eval_fn("main", "total", vec![list])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

// =============================================================================
// CUSTOM TYPES IN CONTROL FLOW
// =============================================================================

#[tokio::test]
async fn custom_type_in_if_expression() {
    let input = r#"
        type MyInt = Int;
        pub let f(b: Bool) -> MyInt = if b { MyInt(1) } else { MyInt(0) };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result_true = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(1)),
        })
    );

    let result_false = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(0)),
        })
    );
}

#[tokio::test]
async fn custom_type_in_let_expression() {
    let input = r#"
        type MyInt = Int;
        pub let f() -> Int = let x = MyInt(42) { Int(x) };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

// =============================================================================
// CUSTOM TYPE STRING CONVERSION
// =============================================================================

#[tokio::test]
async fn custom_type_to_string() {
    let input = r#"
        type MyInt = Int;
        pub let to_str(x: MyInt) -> String = String(x);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let value = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "MyInt".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Int(42)),
    });

    let result = checked_ast
        .eval_fn("main", "to_str", vec![value])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("MyInt(42)".to_string()));
}

#[tokio::test]
async fn custom_type_tuple_to_string() {
    let input = r#"
        type Point = (Int, Int);
        pub let to_str(p: Point) -> String = String(p);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let value = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "Point".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(3)),
            Arc::new(ExprValue::Int(4)),
        ])),
    });

    let result = checked_ast
        .eval_fn("main", "to_str", vec![value])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("Point((3, 4))".to_string()));
}

// =============================================================================
// MULTIPLE CUSTOM TYPES
// =============================================================================

#[tokio::test]
async fn multiple_custom_types() {
    let input = r#"
        type TypeA = Int;
        type TypeB = Int;
        pub let make_a(x: Int) -> TypeA = TypeA(x);
        pub let make_b(x: Int) -> TypeB = TypeB(x);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let a = checked_ast
        .eval_fn("main", "make_a", vec![ExprValue::Int(1)])
        .await
        .expect("Should evaluate");
    let b = checked_ast
        .eval_fn("main", "make_b", vec![ExprValue::Int(1)])
        .await
        .expect("Should evaluate");

    // Even though both are Int underneath, they should be different custom types
    assert_eq!(
        a,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "TypeA".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(1)),
        })
    );
    assert_eq!(
        b,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "TypeB".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(1)),
        })
    );
    assert_ne!(a, b);
}

#[tokio::test]
async fn custom_type_referencing_another() {
    let input = r#"
        type Inner = Int;
        type Outer = [Inner];
        pub let make() -> Outer = Outer([Inner(1), Inner(2)]);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "make", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Outer".to_string(),
            variant: None,
            content: Arc::new(ExprValue::List(vec![
                Arc::new(ExprValue::Custom(CustomValue {
                    module: "main".to_string(),
                    type_name: "Inner".to_string(),
                    variant: None,
                    content: Arc::new(ExprValue::Int(1)),
                })),
                Arc::new(ExprValue::Custom(CustomValue {
                    module: "main".to_string(),
                    type_name: "Inner".to_string(),
                    variant: None,
                    content: Arc::new(ExprValue::Int(2)),
                })),
            ])),
        })
    );
}

// =============================================================================
// CUSTOM TYPES WITH FOLDS
// =============================================================================

#[tokio::test]
async fn custom_type_in_fold() {
    let input = r#"
        type MyInt = Int;
        pub let sum_custom(xs: [MyInt]) -> Int = fold x in xs with acc = 0 { acc + (Int(x)) };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let list = ExprValue::List(vec![
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(1)),
        })),
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(2)),
        })),
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(3)),
        })),
    ]);

    let result = checked_ast
        .eval_fn("main", "sum_custom", vec![list])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

// =============================================================================
// CUSTOM TYPES WITH LIST COMPREHENSIONS
// =============================================================================

#[tokio::test]
async fn custom_type_in_list_comprehension() {
    let input = r#"
        type MyInt = Int;
        pub let double_all(xs: [MyInt]) -> [MyInt] = [MyInt(Int(x) * 2) for x in xs];
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let list = ExprValue::List(vec![
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(1)),
        })),
        Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Int(2)),
        })),
    ]);

    let result = checked_ast
        .eval_fn("main", "double_all", vec![list])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Int(2)),
            })),
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Int(4)),
            })),
        ])
    );
}

// =============================================================================
// CUSTOM TYPES WRAPPING UNION TYPES
// =============================================================================

#[tokio::test]
async fn custom_type_wrapping_union_tuple_index() {
    // Custom type wraps union of tuples, tuple index access should work
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool);
        pub let get_second(x: MyType) -> Bool = x.1;
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    // Test with first variant (Int, Bool)
    let value1 = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(42)),
            Arc::new(ExprValue::Bool(true)),
        ])),
    });
    let result1 = checked_ast
        .eval_fn("main", "get_second", vec![value1])
        .await
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Bool(true));

    // Test with second variant (String, Bool)
    let value2 = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::String("hello".to_string())),
            Arc::new(ExprValue::Bool(false)),
        ])),
    });
    let result2 = checked_ast
        .eval_fn("main", "get_second", vec![value2])
        .await
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Bool(false));
}

#[tokio::test]
async fn custom_type_wrapping_union_tuple_index_returns_union() {
    // Custom type wraps union of tuples with different first element types
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool);
        pub let get_first(x: MyType) -> Int | String = x.0;
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    // Test with first variant (Int, Bool)
    let value1 = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(42)),
            Arc::new(ExprValue::Bool(true)),
        ])),
    });
    let result1 = checked_ast
        .eval_fn("main", "get_first", vec![value1])
        .await
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    // Test with second variant (String, Bool)
    let value2 = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::String("hello".to_string())),
            Arc::new(ExprValue::Bool(false)),
        ])),
    });
    let result2 = checked_ast
        .eval_fn("main", "get_first", vec![value2])
        .await
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::String("hello".to_string()));
}

#[tokio::test]
async fn custom_type_wrapping_nested_custom_type_union() {
    // type A wraps tuple, type B is union containing A
    let input = r#"
        type A = (Int, Int);
        type B = A | (String, Int);
        pub let get_second(x: B) -> Int = x.1;
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    // Test with A variant (wrapped in B)
    let value1 = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "B".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "A".to_string(),
            variant: None,
            content: Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Int(2)),
            ])),
        })),
    });
    let result1 = checked_ast
        .eval_fn("main", "get_second", vec![value1])
        .await
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(2));

    // Test with (String, Int) variant
    let value2 = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "B".to_string(),
        variant: None,
        content: Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::String("test".to_string())),
            Arc::new(ExprValue::Int(99)),
        ])),
    });
    let result2 = checked_ast
        .eval_fn("main", "get_second", vec![value2])
        .await
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(99));
}
