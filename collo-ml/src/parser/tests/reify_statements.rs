use super::*;

// =============================================================================
// REIFY STATEMENT GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of reify statements only.
// They do NOT validate semantic correctness - checking if the constraint
// exists, type compatibility, etc. are handled elsewhere.
//
// Grammar: reify_statement = docstring* "reify" ident "as" var_name ";"
// where var_name = "$" ~ ident
//
// Semantic note: reify binds a constraint function to a reified variable.
// Example: `reify my_constraint as $MyVar;` means that $MyVar can be called
// later as $MyVar() or $MyVar(args) depending on my_constraint's signature.

// =============================================================================
// BASIC STRUCTURE
// =============================================================================

#[test]
fn reify_statement_minimal_structure() {
    // Most basic valid reify statements
    let cases = vec![
        "reify x as $X;",
        "reify constraint as $Var;",
        "reify check as $Check;",
        "reify rule as $Rule;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_minimal_structure_with_var_list() {
    // Most basic valid reify statements
    let cases = vec![
        "reify x as $[X];",
        "reify constraint as $[Var];",
        "reify check as $[Check];",
        "reify rule as $[Rule];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_with_descriptive_names() {
    let cases = vec![
        "reify my_constraint as $MyVar;",
        "reify student_has_subject as $HasSubject;",
        "reify is_valid as $Valid;",
        "reify check123 as $Check123;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_with_single_docstring() {
    let cases = vec![
        "/// This reifies a constraint\nreify my_constraint as $MyVar;",
        "/// Check validity\nreify is_valid as $Valid;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_with_multiple_docstrings() {
    let cases = vec![
        "/// First line\n/// Second line\nreify student_has_subject as $HasSubject;",
        "/// Documentation\n/// More docs\n/// Even more\nreify check as $Check;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_with_varied_whitespace() {
    let cases = vec![
        "reify x as $X;",                       // minimal
        "reify   constraint   as   $Var  ;",    // extra spaces
        "reify\nconstraint\nas\n$Var\n;",       // newlines
        "reify constraint as $Var; // comment", // trailing comment
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// IDENTIFIER VARIATIONS - CONSTRAINT NAMES
// =============================================================================

#[test]
fn reify_statement_constraint_names_with_underscores() {
    let cases = vec![
        "reify has_any_slot as $HasAnySlot;",
        "reify student_in_slot_check as $StudentInSlotCheck;",
        "reify my_long_constraint_name as $MyLongVar;",
        "reify _private_constraint as $Private;",
        "reify _underscore as $Underscore;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_constraint_names_with_numbers() {
    let cases = vec![
        "reify check1 as $Check1;",
        "reify rule2test as $Rule2Test;",
        "reify constraint_v2 as $V2;",
        "reify test123 as $Test123;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_constraint_names_with_mixed_case() {
    let cases = vec![
        "reify myConstraint as $MyConstraint;",
        "reify studentCheck as $StudentCheck;",
        "reify isValid as $IsValid;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// IDENTIFIER VARIATIONS - VARIABLE NAMES
// =============================================================================

#[test]
fn reify_statement_variable_names_with_underscores() {
    let cases = vec![
        "reify constraint as $My_Var;",
        "reify check as $Student_Has_Subject;",
        "reify rule as $Is_Valid_123;",
        "reify test as $_PrivateVar;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_variable_names_with_numbers() {
    let cases = vec![
        "reify constraint as $Var1;",
        "reify check as $Check123;",
        "reify rule as $Rule2;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_variable_names_with_mixed_case() {
    let cases = vec![
        "reify constraint as $MyVar;",
        "reify check as $StudentHasSubject;",
        "reify rule as $IsValid;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// KEYWORD HANDLING
// =============================================================================

#[test]
fn reify_statement_allows_keyword_prefixes_in_names() {
    // Identifiers can contain keywords as prefixes/substrings
    let cases = vec![
        "reify sum_constraint as $SumConstraint;",
        "reify forall_check as $ForallCheck;",
        "reify if_valid as $IfValid;",
        "reify let_binding as $LetBinding;",
        "reify reify_test as $ReifyTest;",
        "reify where_clause as $WhereClause;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// REALISTIC EXAMPLES
// =============================================================================

#[test]
fn reify_statement_realistic_examples() {
    let cases = vec![
        "/// Ensure each student has at least one slot per week\nreify has_any_slot_per_week as $HasSlot;",
        "/// Room capacity constraint\nreify room_capacity_check as $CapacityOK;",
        "/// Student availability constraint\nreify student_available as $Available;",
        "/// Maximum colles per week\nreify max_colles_per_week as $MaxColles;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_statement_complete_example() {
    let input = r#"/// Check if student has any slot in subject for a week
/// This is used to ensure proper distribution
reify has_any_slot_in_subject as $HasSubject;"#;
    let result = ColloMLParser::parse(Rule::reify_statement, input);
    assert!(
        result.is_ok(),
        "Should parse complete example: {:?}",
        result
    );
}

// =============================================================================
// SEMANTICALLY INCORRECT BUT GRAMMATICALLY VALID
// =============================================================================
// These tests explicitly check that the parser accepts code that is
// syntactically correct but semantically wrong. Semantic analysis should
// catch these errors later.

#[test]
fn reify_statement_accepts_undefined_constraints() {
    // Parser should accept these even though the constraint doesn't exist
    let cases = vec![
        "reify nonexistent_constraint as $Var;",
        "reify undefined_function as $Undefined;",
        "reify not_defined_anywhere as $NotDefined;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically wrong): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_accepts_any_constraint_name() {
    // Parser doesn't validate that these are actually constraint-returning functions
    let cases = vec![
        "reify some_linexpr_function as $Var;", // Might return LinExpr, not Constraint
        "reify some_int_function as $Var;",     // Might return Int, not Constraint
        "reify some_bool_function as $Var;",    // Might return Bool, not Constraint
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically wrong): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS - MISSING COMPONENTS
// =============================================================================

#[test]
fn reify_statement_rejects_missing_dollar_sign() {
    let cases = vec![
        "reify my_constraint as MyVar;",
        "reify check as Valid;",
        "reify rule as X;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing $ in variable name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_missing_as_keyword() {
    let cases = vec![
        "reify my_constraint $MyVar;",
        "reify check = $Valid;",
        "reify rule: $Rule;",
        "reify test $Test;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing 'as' keyword): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_missing_semicolon() {
    let cases = vec!["reify my_constraint as $MyVar", "reify check as $Valid\n"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing semicolon): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_missing_constraint_name() {
    let cases = vec!["reify as $MyVar;", "reify  as $Valid;"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing constraint name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_missing_variable_name() {
    let cases = vec![
        "reify my_constraint as;",
        "reify check as $;",
        "reify rule as ;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing or invalid variable name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_missing_reify_keyword() {
    let cases = vec!["my_constraint as $MyVar;", "check as $Valid;"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing 'reify' keyword): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS - INVALID SYNTAX
// =============================================================================

#[test]
fn reify_statement_rejects_inline_expressions() {
    // Reify only accepts identifiers, not inline constraint expressions
    let cases = vec![
        "reify ($V() <= 10) as $MyVar;",
        "reify (forall x in @[X] { $V(x) >= 0 }) as $Check;",
        "reify $V() == 1 as $IsOne;",
        "reify 5 + 3 as $Result;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (inline expression not allowed): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_function_calls() {
    // Cannot reify function calls directly - only identifiers
    let cases = vec![
        "reify compute_constraint(x) as $Result;",
        "reify my_func(a, b) as $MyVar;",
        "reify get_constraint() as $Constraint;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (function call not allowed): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_invalid_constraint_names() {
    let cases = vec![
        "reify 123constraint as $MyVar;", // starts with digit
        "reify my-constraint as $MyVar;", // hyphen not allowed
        "reify my constraint as $MyVar;", // space not allowed
        "reify my.constraint as $MyVar;", // dot not allowed
        "reify my@constraint as $MyVar;", // @ not allowed
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (invalid constraint name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_invalid_variable_names() {
    let cases = vec![
        "reify constraint as $123Var;", // starts with digit
        "reify check as $My-Var;",      // hyphen not allowed
        "reify rule as $My Var;",       // space not allowed
        "reify test as $$MyVar;",       // double dollar
        "reify test as $My.Var;",       // dot not allowed
        "reify test as $My@Var;",       // @ not allowed
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (invalid variable name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_reserved_keywords_as_constraint_names() {
    // Cannot use reserved keywords as constraint names
    let cases = vec![
        "reify let as $MyVar;",
        "reify forall as $Check;",
        "reify sum as $Total;",
        "reify if as $Condition;",
        "reify reify as $Reify;",
        "reify where as $Where;",
        "reify in as $In;",
        "reify and as $And;",
        "reify or as $Or;",
        "reify not as $Not;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (reserved keyword as constraint name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_reserved_keywords_as_variable_names() {
    // Variable names can't be keywords (though they start with $ so this is less likely)
    let cases = vec![
        "reify constraint as $let;",
        "reify constraint as $sum;",
        "reify constraint as $if;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (reserved keyword as variable name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_pub_modifier() {
    // Reify statements cannot have pub modifier (only let statements can)
    let cases = vec![
        "pub reify constraint as $Var;",
        "pub reify check as $Check;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (pub not allowed on reify): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_statement_rejects_wrong_syntax_variants() {
    let cases = vec![
        "reify constraint = $Var;",    // = instead of as
        "reify constraint : $Var;",    // : instead of as
        "reify constraint -> $Var;",   // -> instead of as
        "reify [constraint] as $Var;", // brackets around name
        "reify constraint as [$Var];", // brackets around var
        "reify constraint as $Var();", // parens after var
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (wrong syntax): {:?}",
            case,
            result
        );
    }
}
