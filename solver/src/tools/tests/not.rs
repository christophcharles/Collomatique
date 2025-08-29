use super::*;

#[test]
fn not_variable_get_variables_def() {
    let not_var = NotVariable {
        original_variable: String::from("a"),
        variable_name: String::from("b"),
    };

    let expected = vec![(String::from("b"), Variable::binary())];

    assert_eq!(not_var.get_variables_def(), expected);
}

#[test]
fn not_variable_get_structure_constraints() {
    let not_var = NotVariable {
        original_variable: String::from("a"),
        variable_name: String::from("b"),
    };

    let var_a = LinExpr::<String>::var("a");
    let var_b = LinExpr::<String>::var("b");
    let one = LinExpr::constant(1.);
    let constraints = vec![var_b.eq(&(one - var_a))];
    let output = not_var.get_structure_constraints();

    assert_eq!(constraints.len(), output.len());
    for (c1, (c2, _)) in constraints.iter().zip(output.iter()) {
        assert_eq!(c1, c2);
    }
}

#[test]
fn not_variable_reconstruct_one() {
    let not_var = NotVariable {
        original_variable: String::from("a"),
        variable_name: String::from("b"),
    };

    let config = ConfigData::new().set("a", 1.);

    let expected = vec![Some(0.)];
    let output = not_var.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}

#[test]
fn not_variable_reconstruct_zero() {
    let not_var = NotVariable {
        original_variable: String::from("a"),
        variable_name: String::from("b"),
    };

    let config = ConfigData::new().set("a", 0.);

    let expected = vec![Some(1.)];
    let output = not_var.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}

#[test]
fn not_variable_reconstruct_undefined() {
    let not_var = NotVariable {
        original_variable: String::from("a"),
        variable_name: String::from("b"),
    };

    let config = ConfigData::new();

    let expected = vec![None];
    let output = not_var.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}
