use crate::native_extras::register_native_extras;
use crate::problem::Problem;
use crate::types::{ConstraintDesc, ReifiedVarName};
use collo_ml::script_feeder::{ScriptError, ScriptFeeder};
use collo_ml::{SemWarning, SqliteDatabaseConnection, SqliteDatabaseDriver};
use collomatique_binding_colloscopes::scripts::SimpleScriptError;
use collomatique_binding_colloscopes::vars::{Var, VarEnv};
use collomatique_ilp::{ObjectiveSense, Variable};
use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{DescribeVar, LoadEnv, Modeler};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemBuilder {
    feeder: ScriptFeeder<SqliteDatabaseDriver, Var, ReifiedVarName, ConstraintDesc>,
}

pub(crate) type MyModeler<'m> = Modeler<
    'm,
    Var,
    ReifiedVarName,
    ConstraintDesc,
    sqlx::SqlitePool,
    ReifyError<Var, ReifiedVarName>,
>;

impl ProblemBuilder {
    pub fn get_warnings(&self) -> &[SemWarning] {
        self.feeder.get_warnings()
    }

    pub async fn build(
        self,
        db: &sqlx::SqlitePool,
        db_connection: Option<SqliteDatabaseConnection>,
    ) -> Result<Problem, ScriptError<SqliteDatabaseConnection>> {
        let t0 = Instant::now();
        let script_bundle = self.feeder.build(db_connection).await?;
        eprintln!("[build] feeder.build: {:?}", t0.elapsed());

        let t1 = Instant::now();
        let env = Arc::new(VarEnv::load(db).await);
        eprintln!("[build] VarEnv::load: {:?}", t1.elapsed());

        let t2 = Instant::now();
        let base_vars = Var::enumerate(&env);
        let mut modeler: MyModeler<'_> = Modeler::new(base_vars);
        let env_for_fixer = env.clone();
        modeler.add_fixer(move |b: &Var, _db: &sqlx::SqlitePool| {
            let result = b.check_fix(&env_for_fixer);
            Box::pin(async move { result })
        });
        eprintln!("[build] modeler setup: {:?}", t2.elapsed());

        let original_var_list: HashMap<Var, Variable> = modeler
            .base_vars()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, desc) in &original_var_list {
            if !desc.is_integer() {
                return Err(ScriptError::NonIntegerVariable(format!("{:?}", name)));
            }
        }

        let t3 = Instant::now();
        modeler
            .apply_bundle(script_bundle.into_general())
            .expect("no duplicate extras from script");
        eprintln!("[build] apply script bundle: {:?}", t3.elapsed());

        let t4 = Instant::now();
        register_native_extras(&mut modeler, env);
        eprintln!("[build] register native extras: {:?}", t4.elapsed());

        let t5 = Instant::now();
        let model = modeler
            .build(db)
            .await
            .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e));
        eprintln!("[build] modeler.build: {:?}", t5.elapsed());

        eprintln!("[build] total: {:?}", t0.elapsed());
        Ok(Problem::from_model(model, original_var_list))
    }
}

pub async fn default_problem_builder(
    main_module: &str,
) -> Result<ProblemBuilder, SimpleScriptError> {
    use collo_ml::eval::CompileError;
    use collo_ml::script_feeder::ScriptError;
    use collomatique_binding_colloscopes::scripts::MODULES;

    let t0 = Instant::now();

    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", main_module);

    let mut feeder = ScriptFeeder::new(&modules).await.map_err(|e| match e {
        ScriptError::CompileError(compile_error) => match compile_error {
            CompileError::ParsingError(parse_err) => SimpleScriptError::ParsingError(parse_err),
            CompileError::SemanticsError { errors, warnings } => {
                SimpleScriptError::SemanticErrors { errors, warnings }
            }
            other => SimpleScriptError::UnexpectedError(format!("{}", other)),
        },
        other => SimpleScriptError::UnexpectedError(format!("{}", other)),
    })?;

    let functions = feeder.get_fn_from_module("main");

    for (fn_name, _) in &functions {
        if fn_name == "constraint" || fn_name.starts_with("constraint_") {
            feeder
                .add_constraint("main", fn_name, vec![])
                .map_err(|e| SimpleScriptError::UnexpectedError(format!("{}", e)))?;
        } else if fn_name == "objective" || fn_name.starts_with("objective_") {
            feeder
                .add_objective("main", fn_name, vec![], 1.0, ObjectiveSense::Minimize)
                .map_err(|e| SimpleScriptError::UnexpectedError(format!("{}", e)))?;
        }
    }

    eprintln!("[default_problem_builder] total: {:?}", t0.elapsed());
    Ok(ProblemBuilder { feeder })
}
