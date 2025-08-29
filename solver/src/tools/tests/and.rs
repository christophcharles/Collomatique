use super::*;

#[test]
fn and_variable_get_variable_def() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let expected = (String::from("c"), Variable::binary());

    assert_eq!(and_var.get_variable_def(), expected);
}

#[test]
fn and_variable_get_structure_constraints() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let var_a = LinExpr::<String>::var("a");
    let var_b = LinExpr::<String>::var("b");
    let var_c = LinExpr::<String>::var("c");
    let one = LinExpr::constant(1.);
    let constraints = vec![
        (&var_a + &var_b - &one).leq(&var_c),
        var_a.geq(&var_c),
        var_b.geq(&var_c),
    ];
    let output = and_var.get_structure_constraints();

    assert_eq!(constraints.len(), output.len());
    for (c1, (c2, _)) in constraints.iter().zip(output.iter()) {
        assert_eq!(c1, c2);
    }
}

#[test]
fn and_variable_reconstruct_one_one() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("a", 1.).set("b", 1.);

    let expected = Some(1.);
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_one_zero() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("a", 1.).set("b", 0.);

    let expected = Some(0.);
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_zero_one() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("a", 0.).set("b", 1.);

    let expected = Some(0.);
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_zero_zero() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("a", 0.).set("b", 0.);

    let expected = Some(0.);
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_one_undefined() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("a", 1.);

    let expected = None;
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_undefined_one() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("b", 1.);

    let expected = None;
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_zero_undefined() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("a", 0.);

    let expected = Some(0.);
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_undefined_zero() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new().set("b", 0.);

    let expected = Some(0.);
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}

#[test]
fn and_variable_reconstruct_undefined_undefined() {
    let and_var = AndVariable {
        variable_name: String::from("c"),
        original_variables: BTreeSet::from([String::from("a"), String::from("b")]),
    };

    let config = ConfigData::new();

    let expected = None;
    let output = and_var.reconstruct_structure_variable(&config);

    assert_eq!(expected, output);
}
