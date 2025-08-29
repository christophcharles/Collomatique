use super::*;

#[test]
fn test_config_from_iterator() {
    let config = Config::from_iter(["x", "y", "z"]);

    assert_eq!(
        config.variables,
        BTreeSet::from_iter([String::from("x"), String::from("y"), String::from("z"),])
    );
}
