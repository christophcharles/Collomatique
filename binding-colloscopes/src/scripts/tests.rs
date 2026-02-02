use std::collections::BTreeMap;

use super::*;

#[tokio::test]
async fn modules_should_compile() {
    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", get_default_main_module());
    let _builder = ProblemBuilder::<ObjectId, SqliteDatabaseDriver, Var>::new(&modules)
        .await
        .expect("Should compile modules");
}

#[tokio::test]
async fn modules_should_compile_without_warnings() {
    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", get_default_main_module());
    let builder = ProblemBuilder::<ObjectId, SqliteDatabaseDriver, Var>::new(&modules)
        .await
        .expect("Should compile modules");

    let warnings = builder.get_warnings();
    if !warnings.is_empty() {
        let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
        panic!(
            "Modules should compile without warnings!\nWarnings:\n{}",
            warnings_str.join("\n")
        );
    }
}
