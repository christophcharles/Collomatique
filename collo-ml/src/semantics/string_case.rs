#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingConvention {
    SnakeCase,
    PascalCase,
}

pub fn generate_suggestion_for_naming_convention(
    identifier: &str,
    convention: NamingConvention,
) -> Option<String> {
    match convention {
        NamingConvention::SnakeCase => {
            if is_snake_case(identifier) {
                return None;
            }
            Some(to_snake_case(identifier))
        }
        NamingConvention::PascalCase => {
            if is_pascal_case(identifier) {
                return None;
            }
            Some(to_pascal_case(identifier))
        }
    }
}

fn is_snake_case(s: &str) -> bool {
    // Strip leading underscores
    let trimmed = s.trim_start_matches('_');

    // Empty after trimming means the identifier was only underscores -> reject
    if trimmed.is_empty() {
        return false;
    }

    trimmed
        .chars()
        .all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
        && !trimmed.ends_with('_')
        && !trimmed.contains("__")
}

fn is_pascal_case(s: &str) -> bool {
    // Strip leading underscores
    let trimmed = s.trim_start_matches('_');

    // Empty after trimming means the identifier was only underscores -> reject
    if trimmed.is_empty() {
        return false;
    }

    trimmed.chars().next().unwrap().is_uppercase()
        && trimmed.chars().all(|c| c.is_alphanumeric())
        && !trimmed.contains('_')
}

pub fn to_snake_case(s: &str) -> String {
    // If the whole string is underscores
    if s.chars().all(|c| c == '_') {
        return s.to_string() + "name";
    }

    // Count leading underscores
    let leading_count = s.chars().take_while(|&c| c == '_').count();

    // Trim leading underscores
    let trimmed = s.trim_start_matches('_');

    let mut result = String::new();
    for (i, c) in trimmed.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    // Normalize: remove leading/trailing underscores and collapse double underscores
    // Prepend the original leading underscores
    "_".repeat(leading_count) + &normalize_snake_case(&result)
}

fn normalize_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_underscore = false;

    // Trim trailing underscores
    let trimmed = s.trim_end_matches('_');

    for c in trimmed.chars() {
        if c == '_' {
            if !prev_was_underscore {
                result.push(c);
            }
            prev_was_underscore = true;
        } else {
            result.push(c);
            prev_was_underscore = false;
        }
    }

    result
}

pub fn to_pascal_case(s: &str) -> String {
    // If the whole string is underscores
    if s.chars().all(|c| c == '_') {
        return s.to_string() + "Name";
    }

    // Count leading underscores
    let leading_count = s.chars().take_while(|&c| c == '_').count();

    // Trim leading underscores
    let trimmed = s.trim_start_matches('_');

    let mut result = String::new();
    let mut capitalize_next = true;
    for c in trimmed.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    // Prepend the original leading underscores
    "_".repeat(leading_count) + &result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== is_snake_case tests ==========

    #[test]
    fn is_snake_case_accepts_valid() {
        let cases = vec![
            "hello",
            "hello_world",
            "my_function_name",
            "with_numbers_123",
            "a",
            "test_1_2_3",
        ];
        for case in cases {
            assert!(is_snake_case(case), "{} should be snake_case", case);
        }
    }

    #[test]
    fn is_snake_case_rejects_pascal_case() {
        let cases = vec!["HelloWorld", "MyFunction", "PascalCase"];
        for case in cases {
            assert!(!is_snake_case(case), "{} should not be snake_case", case);
        }
    }

    #[test]
    fn is_snake_case_accepts_leading_underscore() {
        assert!(is_snake_case("_hello"));
        assert!(is_snake_case("_world"));
    }

    #[test]
    fn is_snake_case_rejects_only_underscores() {
        assert!(!is_snake_case("_"));
        assert!(!is_snake_case("__"));
    }

    #[test]
    fn is_snake_case_rejects_trailing_underscore() {
        assert!(!is_snake_case("hello_"));
        assert!(!is_snake_case("world_"));
    }

    #[test]
    fn is_snake_case_rejects_double_underscore() {
        assert!(!is_snake_case("hello__world"));
        assert!(!is_snake_case("my__function"));
    }

    #[test]
    fn is_snake_case_rejects_uppercase() {
        assert!(!is_snake_case("Hello"));
        assert!(!is_snake_case("helloWorld"));
        assert!(!is_snake_case("HELLO"));
    }

    // ========== is_pascal_case tests ==========

    #[test]
    fn is_pascal_case_accepts_valid() {
        let cases = vec![
            "Hello",
            "HelloWorld",
            "MyFunction",
            "PascalCase",
            "WithNumbers123",
            "A",
        ];
        for case in cases {
            assert!(is_pascal_case(case), "{} should be PascalCase", case);
        }
    }

    #[test]
    fn is_pascal_case_rejects_snake_case() {
        let cases = vec!["hello_world", "my_function", "snake_case"];
        for case in cases {
            assert!(!is_pascal_case(case), "{} should not be PascalCase", case);
        }
    }

    #[test]
    fn is_pascal_case_rejects_lowercase_start() {
        assert!(!is_pascal_case("helloWorld"));
        assert!(!is_pascal_case("myFunction"));
    }

    #[test]
    fn is_pascal_case_rejects_underscore() {
        assert!(!is_pascal_case("Hello_World"));
        assert!(!is_pascal_case("My_Function"));
    }

    #[test]
    fn is_pascal_case_rejects_empty() {
        assert!(!is_pascal_case(""));
    }

    #[test]
    fn is_pascal_case_accepts_leading_underscores() {
        assert!(is_pascal_case("_Hello"));
        assert!(is_pascal_case("__World"));
    }

    #[test]
    fn is_pascal_case_rejects_only_underscores() {
        assert!(!is_pascal_case("_"));
        assert!(!is_pascal_case("__"));
    }

    #[test]
    fn is_pascal_case_rejects_trailing_underscore() {
        assert!(!is_pascal_case("Hello_"));
        assert!(!is_pascal_case("World_"));
    }

    // ========== to_snake_case tests ==========

    #[test]
    fn to_snake_case_converts_pascal_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("MyFunction"), "my_function");
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
    }

    #[test]
    fn to_snake_case_handles_single_word() {
        assert_eq!(to_snake_case("Hello"), "hello");
        assert_eq!(to_snake_case("World"), "world");
    }

    #[test]
    fn to_snake_case_handles_numbers() {
        assert_eq!(to_snake_case("HelloWorld123"), "hello_world123");
        assert_eq!(to_snake_case("Test1Test2"), "test1_test2");
    }

    #[test]
    fn to_snake_case_already_snake_case() {
        assert_eq!(to_snake_case("already_snake"), "already_snake");
        assert_eq!(to_snake_case("hello_world"), "hello_world");
    }

    #[test]
    fn to_snake_case_handles_consecutive_capitals() {
        // Note: This might not handle acronyms perfectly
        // "HTTPServer" -> "h_t_t_p_server" (might want "http_server")
        assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
    }

    // ========== to_pascal_case tests ==========

    #[test]
    fn to_pascal_case_converts_snake_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("my_function"), "MyFunction");
        assert_eq!(to_pascal_case("snake_case"), "SnakeCase");
    }

    #[test]
    fn to_pascal_case_handles_single_word() {
        assert_eq!(to_pascal_case("hello"), "Hello");
        assert_eq!(to_pascal_case("world"), "World");
    }

    #[test]
    fn to_pascal_case_handles_numbers() {
        assert_eq!(to_pascal_case("hello_world_123"), "HelloWorld123");
        assert_eq!(to_pascal_case("test_1_test_2"), "Test1Test2");
    }

    #[test]
    fn to_pascal_case_already_pascal_case() {
        assert_eq!(to_pascal_case("AlreadyPascal"), "AlreadyPascal");
        assert_eq!(to_pascal_case("HelloWorld"), "HelloWorld");
    }

    #[test]
    fn to_pascal_case_handles_multiple_underscores() {
        assert_eq!(to_pascal_case("hello__world"), "HelloWorld");
        assert_eq!(to_pascal_case("my___function"), "MyFunction");
    }

    // ========== generate_suggestion_for_naming_convention tests ==========

    #[test]
    fn generate_suggestion_returns_none_when_correct() {
        assert_eq!(
            generate_suggestion_for_naming_convention("hello_world", NamingConvention::SnakeCase),
            None
        );
        assert_eq!(
            generate_suggestion_for_naming_convention("HelloWorld", NamingConvention::PascalCase),
            None
        );
    }

    #[test]
    fn generate_suggestion_returns_some_when_incorrect() {
        assert_eq!(
            generate_suggestion_for_naming_convention("HelloWorld", NamingConvention::SnakeCase),
            Some("hello_world".to_string())
        );
        assert_eq!(
            generate_suggestion_for_naming_convention("hello_world", NamingConvention::PascalCase),
            Some("HelloWorld".to_string())
        );
    }

    #[test]
    fn generate_suggestion_snake_case_examples() {
        assert_eq!(
            generate_suggestion_for_naming_convention("MyFunction", NamingConvention::SnakeCase),
            Some("my_function".to_string())
        );
        assert_eq!(
            generate_suggestion_for_naming_convention(
                "calculateTotal",
                NamingConvention::SnakeCase
            ),
            Some("calculate_total".to_string())
        );
    }

    #[test]
    fn generate_suggestion_pascal_case_examples() {
        assert_eq!(
            generate_suggestion_for_naming_convention("my_variable", NamingConvention::PascalCase),
            Some("MyVariable".to_string())
        );
        assert_eq!(
            generate_suggestion_for_naming_convention(
                "student_in_slot",
                NamingConvention::PascalCase
            ),
            Some("StudentInSlot".to_string())
        );
    }

    #[test]
    fn to_snake_case_does_not_remove_leading_underscore() {
        assert_eq!(to_snake_case("_hello"), "_hello");
        assert_eq!(to_snake_case("__world"), "__world");
    }

    #[test]
    fn to_snake_case_removes_trailing_underscore() {
        assert_eq!(to_snake_case("hello_"), "hello");
        assert_eq!(to_snake_case("world__"), "world");
    }

    #[test]
    fn to_snake_case_collapses_double_underscores() {
        assert_eq!(to_snake_case("hello__world"), "hello_world");
        assert_eq!(to_snake_case("my___function"), "my_function");
    }

    #[test]
    fn to_snake_case_fixes_multiple_issues() {
        assert_eq!(to_snake_case("_Hello__World_"), "_hello_world");
    }

    #[test]
    fn test_to_snake_case_only_underscores() {
        let input = "__";
        let output = to_snake_case(input);
        assert_eq!(
            output, "__name",
            "Expected '__name' for input '__', got {}",
            output
        );
    }

    #[test]
    fn test_to_pascal_case_only_underscores() {
        let input = "__";
        let output = to_pascal_case(input);
        assert_eq!(
            output, "__Name",
            "Expected '__Name' for input '__', got {}",
            output
        );
    }
}
