use std::collections::BTreeMap;

use super::*;
use crate::vars::Var;
use collo_ml::eval::Origin;
use collo_ml::script_feeder::{ReifiedVar, ScriptFeeder};

type E = ReifiedVar<collo_ml::SqliteDatabaseConnection>;
type C = Option<Origin<collo_ml::SqliteDatabaseConnection>>;

#[tokio::test]
async fn modules_should_compile() {
    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", get_default_main_module());
    let _feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Should compile modules");
}

#[tokio::test]
async fn modules_should_compile_without_warnings() {
    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", get_default_main_module());
    let feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Should compile modules");

    let warnings = feeder.get_warnings();
    if !warnings.is_empty() {
        let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
        panic!(
            "Modules should compile without warnings!\nWarnings:\n{}",
            warnings_str.join("\n")
        );
    }
}
