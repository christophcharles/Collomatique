use super::*;

// ========== STRING CONCATENATION Tests ==========

#[test]
fn concat_two_strings() {
    let input = r#"pub let f() -> String = "hello" + "world";"#;

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
    assert_eq!(result, ExprValue::String("helloworld".to_string()));
}

#[test]
fn concat_with_space() {
    let input = r#"pub let f() -> String = "hello" + " " + "world";"#;

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
    assert_eq!(result, ExprValue::String("hello world".to_string()));
}

#[test]
fn concat_with_empty_string_left() {
    let input = r#"pub let f() -> String = "" + "hello";"#;

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
    assert_eq!(result, ExprValue::String("hello".to_string()));
}

#[test]
fn concat_with_empty_string_right() {
    let input = r#"pub let f() -> String = "hello" + "";"#;

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
    assert_eq!(result, ExprValue::String("hello".to_string()));
}

#[test]
fn concat_two_empty_strings() {
    let input = r#"pub let f() -> String = "" + "";"#;

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
    assert_eq!(result, ExprValue::String("".to_string()));
}

#[test]
fn concat_with_params() {
    let input = r#"pub let f(str1: String, str2: String) -> String = str1 + str2;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let str1 = ExprValue::String("Hello, ".to_string());
    let str2 = ExprValue::String("World!".to_string());

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![str1, str2])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::String("Hello, World!".to_string()));
}

#[test]
fn concat_chain() {
    let input = r#"pub let f() -> String = "a" + "b" + "c";"#;

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
    assert_eq!(result, ExprValue::String("abc".to_string()));
}

#[test]
fn concat_longer_chain() {
    let input = r#"pub let f() -> String = "Hello" + ", " + "how" + " " + "are" + " " + "you?";"#;

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
    assert_eq!(result, ExprValue::String("Hello, how are you?".to_string()));
}

#[test]
fn concat_with_unicode() {
    let input = r#"pub let f() -> String = "Hello " + "世界";"#;

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
    assert_eq!(result, ExprValue::String("Hello 世界".to_string()));
}

#[test]
fn concat_with_newlines() {
    let input = "pub let f() -> String = \"line1\n\" + \"line2\";";

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
    assert_eq!(result, ExprValue::String("line1\nline2".to_string()));
}

#[test]
fn concat_with_special_characters() {
    let input = r#"pub let f() -> String = "tab:	" + ~"quote:""~;"#;

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
    assert_eq!(result, ExprValue::String("tab:\tquote:\"".to_string()));
}

#[test]
fn concat_with_parentheses() {
    let input = r#"pub let f() -> String = ("a" + "b") + ("c" + "d");"#;

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
    assert_eq!(result, ExprValue::String("abcd".to_string()));
}

// ========== STRING SUM Tests ==========

#[test]
fn sum_strings_in_list() {
    let input = r#"pub let f() -> String = sum s in ["a", "b", "c"] { s };"#;

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
    assert_eq!(result, ExprValue::String("abc".to_string()));
}

#[test]
fn sum_strings_with_separator() {
    let input = r#"pub let f() -> String = sum s in ["hello", "world", "test"] { s + " " };"#;

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
    assert_eq!(result, ExprValue::String("hello world test ".to_string()));
}

#[test]
fn sum_empty_string_list() {
    let input = r#"pub let f() -> String = sum s in [<String>] { s };"#;

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
    assert_eq!(result, ExprValue::String("".to_string()));
}

#[test]
fn sum_strings_with_filter() {
    let input = r#"pub let f() -> String = sum s in ["a", "", "b", "", "c"] where s != "" { s };"#;

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
    assert_eq!(result, ExprValue::String("abc".to_string()));
}

#[test]
fn sum_strings_with_transformation() {
    let input = r#"pub let f() -> String = sum s in ["a", "b", "c"] { s + s };"#;

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
    assert_eq!(result, ExprValue::String("aabbcc".to_string()));
}

// ========== COMPLEX STRING OPERATIONS Tests ==========

#[test]
fn concat_in_if_expression() {
    let input = r#"pub let f(x: Bool) -> String = if x { "yes" + "!" } else { "no" + "." };"#;

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
    assert_eq!(result_true, ExprValue::String("yes!".to_string()));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::String("no.".to_string()));
}

#[test]
fn concat_with_function_calls() {
    let input = r#"
        pub let prefix() -> String = "Hello, ";
        pub let suffix() -> String = "World!";
        pub let greeting() -> String = prefix() + suffix();
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
        .quick_eval_fn("main", "greeting", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::String("Hello, World!".to_string()));
}

#[test]
fn concat_in_let_expression() {
    let input = r#"
        pub let f() -> String = 
            let prefix = "Hello" {
                prefix + ", " + "World!"
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
    assert_eq!(result, ExprValue::String("Hello, World!".to_string()));
}

#[test]
fn concat_strings_with_tildes() {
    let input = r#"pub let f() -> String = ~"He said "hello""~ + " to me";"#;

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
    assert_eq!(
        result,
        ExprValue::String(r#"He said "hello" to me"#.to_string())
    );
}
