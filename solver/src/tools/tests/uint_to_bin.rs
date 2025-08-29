use super::*;

#[test]
fn uint_to_bin_variables_get_variables_def() {
    let uint_to_bin_vars = UIntToBinVariables {
        original_variable: String::from("a"),
        original_range: 2..=4,
        variable_name_builder: |i| i.to_string(),
    };

    let expected = vec![
        (String::from("2"), Variable::binary()),
        (String::from("3"), Variable::binary()),
        (String::from("4"), Variable::binary()),
    ];

    assert_eq!(uint_to_bin_vars.get_variables_def(), expected);
}

#[test]
fn uint_to_bin_variables_get_structure_constraints() {
    let uint_to_bin_vars = UIntToBinVariables {
        original_variable: String::from("a"),
        original_range: 2..=4,
        variable_name_builder: |i| i.to_string(),
    };

    let var_a = LinExpr::<String>::var("a");
    let var_2 = LinExpr::<String>::var("2");
    let var_3 = LinExpr::<String>::var("3");
    let var_4 = LinExpr::<String>::var("4");
    let one = LinExpr::constant(1.);
    let constraints = vec![
        (2 * &var_2 + 3 * &var_3 + 4 * &var_4).eq(&var_a),
        (&var_2 + &var_3 + &var_4).eq(&one),
    ];
    let output = uint_to_bin_vars.get_structure_constraints();

    assert_eq!(constraints.len(), output.len());
    for (c1, (c2, _)) in constraints.iter().zip(output.iter()) {
        assert_eq!(c1, c2);
    }
}

#[test]
fn uint_to_bin_variables_reconstruct_2() {
    let uint_to_bin_vars = UIntToBinVariables {
        original_variable: String::from("a"),
        original_range: 2..=4,
        variable_name_builder: |i| i.to_string(),
    };

    let config = ConfigData::new().set("a", 2.);

    let expected = vec![Some(1.), Some(0.), Some(0.)];
    let output = uint_to_bin_vars.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}

#[test]
fn uint_to_bin_variables_reconstruct_3() {
    let uint_to_bin_vars = UIntToBinVariables {
        original_variable: String::from("a"),
        original_range: 2..=4,
        variable_name_builder: |i| i.to_string(),
    };

    let config = ConfigData::new().set("a", 3.);

    let expected = vec![Some(0.), Some(1.), Some(0.)];
    let output = uint_to_bin_vars.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}

#[test]
fn uint_to_bin_variables_reconstruct_4() {
    let uint_to_bin_vars = UIntToBinVariables {
        original_variable: String::from("a"),
        original_range: 2..=4,
        variable_name_builder: |i| i.to_string(),
    };

    let config = ConfigData::new().set("a", 4.);

    let expected = vec![Some(0.), Some(0.), Some(1.)];
    let output = uint_to_bin_vars.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}

#[test]
fn uint_to_bin_variables_reconstruct_undefined() {
    let uint_to_bin_vars = UIntToBinVariables {
        original_variable: String::from("a"),
        original_range: 2..=4,
        variable_name_builder: |i| i.to_string(),
    };

    let config = ConfigData::new();

    let expected = vec![None, None, None];
    let output = uint_to_bin_vars.reconstruct_structure_variables(&config);

    assert_eq!(expected, output);
}
