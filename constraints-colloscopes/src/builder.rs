use std::time::Instant;

use crate::ColloscopeModel;
use crate::extras::build_extras;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp_modeler::Modeler;
use collomatique_ilp_modeler::bundle::ReifyError;

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

pub async fn build_model(db: &sqlx::SqlitePool) -> ColloscopeModel {
    build_model_with_log(db, &mut |_: &str| {}).await
}

pub async fn build_model_with_log(
    db: &sqlx::SqlitePool,
    log: &mut (dyn FnMut(&str) + Send),
) -> ColloscopeModel {
    let t_total = Instant::now();

    let t = Instant::now();
    log("[build_model] Loading environment...");
    let env = VarEnv::load(db).await;
    log(&format!(
        "[build_model] Environment loaded ({:.2?})",
        t.elapsed()
    ));

    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);

    macro_rules! apply {
        ($name:expr, $bundle:expr) => {{
            let t = Instant::now();
            log(&format!("[build_model] Applying bundle: {}...", $name));
            modeler
                .apply_bundle($bundle.into_general())
                .unwrap_or_else(|_| panic!("no duplicate extras from {}", $name));
            log(&format!(
                "[build_model] Bundle applied ({:.2?})",
                t.elapsed()
            ));
        }};
    }

    apply!("extras", build_extras(&env));
    apply!("groups", crate::groups::build(&env));
    apply!("schedule_structure", crate::schedule_structure::build(&env));
    apply!("pairings", crate::pairings::build(&env));
    apply!("misc", crate::misc::build(&env));
    apply!("periodicity", crate::periodicity::build(&env));
    apply!("balancing", crate::balancing::build(&env));

    log("[build_model] Running Modeler::build()...");
    let t = Instant::now();
    let model = modeler
        .build_with_log(&env, log)
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e));
    log(&format!(
        "[build_model] Modeler::build() complete ({:.2?})",
        t.elapsed()
    ));

    log(&format!("[build_model] Total ({:.2?})", t_total.elapsed()));
    model
}
